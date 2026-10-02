//! The IFC release a systems record is authored against (#202).
//!
//! The readers bind a release in `crate::release`; this is the authoring
//! counterpart, and follows the rule `ifc-material` (#77), `ifc-properties`
//! (#191) and `ifc-classification` (#194) use, from `FILE_SCHEMA`:
//! - one recognised declaration binds that release's bundled table;
//! - one unrecognised declaration fails with
//!   [`SystemAuthoringError::UnsupportedSchema`];
//! - several fail with [`SystemAuthoringError::MultipleSchemas`];
//! - none at all (an in-memory [`Model::new`]) binds IFC4.
//!
//! Every record is built by attribute name from that table, so an IFC2X3
//! `IfcDistributionPort` has eight attributes and an IFC2X3 `IfcZone` five.
//! No attribute these writers set is renamed between IFC2X3 TC1, IFC4 ADD2
//! TC1 and IFC4X3 ADD2, so names need no per-release alias.

use ifc_model::{Edit, Entity, EntityId, Model, Transaction, Value};
use ifc_schema::{for_version, Attribute, Schema, SchemaVersion, TypeKind};

use super::{SystemAuthoringError, SystemAuthoringResult};

/// The bound release and its table.
#[derive(Clone, Copy)]
pub(super) struct Release {
    version: SchemaVersion,
    schema: &'static Schema,
}

/// Releases this crate's layouts are proven against.
///
/// `ifc-schema` also bundles IFC4X1 and IFC4X2, but nothing here is verified
/// against their tables, so a header declaring either is refused with the
/// unsupported-schema error rather than read through a neighbour's layout.
const fn proven(version: SchemaVersion) -> bool {
    matches!(
        version,
        SchemaVersion::Ifc2x3 | SchemaVersion::Ifc4 | SchemaVersion::Ifc4x3
    )
}

/// Bind `model`'s declared release.
pub(super) fn bind(model: &Model) -> SystemAuthoringResult<Release> {
    let version = match model.header().schema.as_slice() {
        [] => SchemaVersion::Ifc4,
        [token] => SchemaVersion::from_header_token(token)
            .filter(|version| proven(*version))
            .ok_or_else(|| SystemAuthoringError::UnsupportedSchema {
                schema: token.clone(),
            })?,
        tokens => {
            return Err(SystemAuthoringError::MultipleSchemas {
                schemas: tokens.len(),
            })
        }
    };
    let schema = for_version(version).expect("every SchemaVersion has a bundled table");
    Ok(Release { version, schema })
}

impl Release {
    /// The bound release's table.
    pub(super) const fn schema(self) -> &'static Schema {
        self.schema
    }

    /// The bound release.
    pub(super) const fn version(self) -> SchemaVersion {
        self.version
    }

    /// Fail with `EntityNotInSchema` unless the release can instantiate
    /// `entity` (IFC2X3 has no `IfcSpatialZone`, IFC4 no `IfcBuiltSystem`).
    pub(super) fn require_entity(self, entity: &'static str) -> SystemAuthoringResult<()> {
        if self.schema.entity(entity).is_some_and(|e| !e.abstract_) {
            Ok(())
        } else {
            Err(SystemAuthoringError::EntityNotInSchema {
                entity,
                schema: self.version,
            })
        }
    }

    /// Build `entity`'s record in this release's layout from named values.
    ///
    /// A non-null value the release does not declare is refused with
    /// `AuthoringNotInSchema` (an IFC2X3 zone's `LongName`), one it cannot
    /// hold with `AuthoringValueType` (a token outside the release's
    /// enumeration), and a null in a slot it requires (IFC2X3
    /// `IfcRoot.OwnerHistory`) with `AuthoringRequired`. Nothing is dropped
    /// or invented.
    pub(super) fn record(
        self,
        entity: &'static str,
        values: Vec<(&'static str, Value)>,
    ) -> SystemAuthoringResult<Entity> {
        self.require_entity(entity)?;
        let declared = self.schema.attributes(entity);
        let mut slots = vec![Value::Null; declared.len()];
        for (attribute, value) in values {
            let Some(slot) = declared
                .iter()
                .position(|d| d.name.eq_ignore_ascii_case(attribute))
            else {
                if value == Value::Null {
                    continue;
                }
                return Err(SystemAuthoringError::AuthoringNotInSchema {
                    entity,
                    attribute,
                    schema: self.version,
                });
            };
            if !conforms(self.schema, declared[slot], &value) {
                return Err(SystemAuthoringError::AuthoringValueType {
                    entity,
                    attribute,
                    declared: declared[slot].type_name.as_str(),
                    schema: self.version,
                });
            }
            slots[slot] = value;
        }
        if let Some(missing) = declared
            .iter()
            .zip(&slots)
            .find(|(d, value)| !d.optional && **value == Value::Null)
        {
            return Err(SystemAuthoringError::AuthoringRequired {
                entity,
                attribute: missing.0.name.as_str(),
                schema: self.version,
            });
        }
        Ok(Entity::new(entity, slots))
    }

    /// Fail unless `id` is, in the model or staged on `tx`, a record this
    /// release accepts as `entity.OwnerHistory` (an `IfcOwnerHistory`).
    pub(super) fn require_owner_history(
        self,
        tx: &Transaction,
        model: &Model,
        entity: &'static str,
        id: EntityId,
    ) -> SystemAuthoringResult<()> {
        let actual =
            projected_type(tx, model, id).ok_or(SystemAuthoringError::MissingReference {
                entity,
                attribute: "OwnerHistory",
                target: id,
            })?;
        let accepted = self
            .schema
            .attributes(entity)
            .iter()
            .find(|d| d.name.eq_ignore_ascii_case("OwnerHistory"))
            .is_some_and(|d| self.schema.accepts_type(&d.type_name, &actual));
        if accepted {
            Ok(())
        } else {
            Err(SystemAuthoringError::WrongReferenceType {
                entity,
                attribute: "OwnerHistory",
                target: id,
                actual,
                expected: "IFCOWNERHISTORY",
            })
        }
    }

    /// Lay out `entity` with `OwnerHistory` set, check the owner history,
    /// and only then stage the record: a refusal stages nothing.
    pub(super) fn stage(
        self,
        tx: &mut Transaction,
        model: &Model,
        entity: &'static str,
        mut values: Vec<(&'static str, Value)>,
        owner_history: EntityId,
    ) -> SystemAuthoringResult<EntityId> {
        values.push(("OwnerHistory", Value::Ref(owner_history)));
        let record = self.record(entity, values)?;
        self.require_owner_history(tx, model, entity, owner_history)?;
        Ok(tx.create(record))
    }
}

/// The type `target` will have once `tx` commits, or `None` if absent.
pub(super) fn projected_type(tx: &Transaction, model: &Model, target: EntityId) -> Option<String> {
    for edit in tx.edits().iter().rev() {
        match edit {
            Edit::Remove { id } if *id == target => return None,
            Edit::Retype { id, type_name } if *id == target => return Some(type_name.to_string()),
            Edit::Create { id, entity } if *id == target => {
                return Some(entity.type_name.to_string())
            }
            _ => {}
        }
    }
    model.get(target).map(|entity| entity.type_name.to_string())
}

/// Whether `value` is a legal instance of `declared` in `schema`.
fn conforms(schema: &Schema, declared: &Attribute, value: &Value) -> bool {
    match value {
        Value::Null => true,
        Value::List(items) => {
            (declared.aggregate || is_aggregate(schema, &declared.type_name))
                && items
                    .iter()
                    .all(|item| scalar(schema, &declared.type_name, item))
        }
        _ => !declared.aggregate && scalar(schema, &declared.type_name, value),
    }
}

fn is_aggregate(schema: &Schema, declared: &str) -> bool {
    let base = schema.resolve_defined(declared).to_ascii_uppercase();
    ["LIST", "SET", "BAG", "ARRAY"]
        .iter()
        .any(|kind| base.starts_with(kind))
}

/// Whether a single (non-aggregate) `value` is a legal `declared`.
fn scalar(schema: &Schema, declared: &str, value: &Value) -> bool {
    let base = schema.resolve_defined(declared).to_ascii_uppercase();
    let kind = schema.type_def(declared).map(|t| &t.kind);
    match value {
        Value::Text(_) => base.starts_with("STRING"),
        Value::Enum(token) => matches!(kind, Some(TypeKind::Enumeration(members))
            if members.iter().any(|m| m.eq_ignore_ascii_case(token))),
        Value::Bool(_) => base == "BOOLEAN" || base == "LOGICAL",
        Value::LogicalUnknown => base == "LOGICAL",
        Value::Integer(_) => base == "INTEGER" || base == "NUMBER",
        Value::Real(_) => base.starts_with("REAL") || base == "NUMBER",
        Value::Ref(_) => admits_entity(schema, declared, 8),
        Value::Typed { type_name, .. } => {
            matches!(kind, Some(TypeKind::Select(_))) && schema.accepts_type(declared, type_name)
        }
        _ => false,
    }
}

/// Whether `declared` is an entity, or a SELECT that reaches one.
fn admits_entity(schema: &Schema, declared: &str, depth: usize) -> bool {
    if schema.entity(declared).is_some() {
        return true;
    }
    match schema.type_def(declared).map(|t| &t.kind) {
        Some(TypeKind::Select(members)) if depth > 0 => members
            .iter()
            .any(|member| admits_entity(schema, member, depth - 1)),
        _ => false,
    }
}

#[cfg(test)]
mod intermediate_release_tests {
    use super::*;

    /// IFC4X1 and IFC4X2 have bundled tables but no verified layout here:
    /// refused with the unsupported-schema error, never read as IFC4/IFC4X3.
    #[test]
    fn ifc4x1_and_ifc4x2_are_refused_not_aliased() {
        for token in ["IFC4X1", "IFC4X2"] {
            let mut model = Model::new();
            model.header_mut().schema = vec![token.to_owned()];
            assert!(
                matches!(bind(&model), Err(SystemAuthoringError::UnsupportedSchema { schema }) if schema == token),
                "{token} must be refused"
            );
        }
    }
}
