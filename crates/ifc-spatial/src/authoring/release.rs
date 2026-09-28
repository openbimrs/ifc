//! The IFC release a spatial record is laid out against (#202).
//!
//! `IfcRoot.OwnerHistory` is required in IFC2X3 TC1 and `OPTIONAL` from
//! IFC4 on:
//!
//! ```text
//! IFC2X3_TC1   OwnerHistory : IfcOwnerHistory;
//! IFC4         OwnerHistory : OPTIONAL IfcOwnerHistory;
//! IFC4X3_ADD2  OwnerHistory : OPTIONAL IfcOwnerHistory;
//! ```
//!
//! The `*_with_owner_history` writers, and [`create_space_boundary`], bind
//! the model's declared release as `ifc-material` (#77), `ifc-properties`
//! (#191) and `ifc-classification` (#194) do, from `FILE_SCHEMA`:
//! - one recognised declaration binds that release's bundled table;
//! - one unrecognised declaration fails with
//!   [`SpatialAuthoringError::UnsupportedSchema`];
//! - several fail with [`SpatialAuthoringError::MultipleSchemas`];
//! - none at all (an in-memory [`Model::new`]) binds IFC4.
//!
//! Every record is then built by attribute name from that release's table,
//! so a value the release cannot hold is refused instead of written into a
//! slot the release gives another meaning.
//!
//! [`create_space_boundary`]: super::create_space_boundary

use ifc_model::{Edit, Entity, EntityId, Model, Transaction, Value};
use ifc_schema::{for_version, Attribute, Schema, SchemaVersion, TypeKind};

use super::{SpatialAuthoringError, SpatialAuthoringResult};

/// The bound release and its table.
#[derive(Clone, Copy)]
pub(crate) struct Release {
    version: SchemaVersion,
    schema: &'static Schema,
}

/// Bind `model`'s declared release.
pub(crate) fn bind(model: &Model) -> SpatialAuthoringResult<Release> {
    let version = match model.header().schema.as_slice() {
        [] => SchemaVersion::Ifc4,
        [token] => SchemaVersion::from_header_token(token).ok_or_else(|| {
            SpatialAuthoringError::UnsupportedSchema {
                schema: token.clone(),
            }
        })?,
        tokens => {
            return Err(SpatialAuthoringError::MultipleSchemas {
                schemas: tokens.len(),
            })
        }
    };
    // A release this build carries no table for (a future `SchemaVersion`
    // member, or one whose table is feature-gated off) is refused, not
    // assumed to share another release's layout.
    let schema = for_version(version).ok_or_else(|| SpatialAuthoringError::UnsupportedSchema {
        schema: version.release_id().to_owned(),
    })?;
    Ok(Release { version, schema })
}

impl Release {
    /// Build `entity`'s record in this release's layout from values named
    /// by their IFC4 attribute names.
    ///
    /// An entity the release cannot instantiate is `EntityNotInSchema`; a
    /// non-null value it does not declare is `AuthoringNotInSchema`; one it
    /// cannot hold (a token outside its enumeration) is
    /// `AuthoringValueType`; a null in a slot it requires (the IFC2X3
    /// `OwnerHistory`, `CompositionType`) is `AuthoringRequired`. Nothing is
    /// dropped or invented.
    pub(crate) fn record(
        self,
        entity: &'static str,
        values: Vec<(&'static str, Value)>,
    ) -> SpatialAuthoringResult<Entity> {
        if self.schema.entity(entity).is_none_or(|e| e.abstract_) {
            return Err(SpatialAuthoringError::EntityNotInSchema {
                entity,
                schema: self.version,
            });
        }
        let declared = self.schema.attributes(entity);
        let mut slots = vec![Value::Null; declared.len()];
        for (attribute, value) in values {
            let name = release_name(self.version, entity, attribute);
            let Some(slot) = declared
                .iter()
                .position(|d| d.name.eq_ignore_ascii_case(name))
            else {
                if value == Value::Null {
                    continue;
                }
                return Err(SpatialAuthoringError::AuthoringNotInSchema {
                    entity,
                    attribute,
                    schema: self.version,
                });
            };
            if !conforms(self.schema, declared[slot], &value) {
                return Err(SpatialAuthoringError::AuthoringValueType {
                    entity,
                    attribute,
                    declared: declared[slot].type_name.as_str(),
                    schema: self.version,
                });
            }
            slots[slot] = value;
        }
        if let Some((missing, _)) = declared
            .iter()
            .zip(&slots)
            .find(|(d, value)| !d.optional && **value == Value::Null)
        {
            return Err(SpatialAuthoringError::AuthoringRequired {
                entity,
                attribute: missing.name.as_str(),
                schema: self.version,
            });
        }
        Ok(Entity::new(entity, slots))
    }

    /// Fail unless `id` is, in the model or staged on `tx`, a record this
    /// release accepts as `entity.OwnerHistory` (an `IfcOwnerHistory`).
    pub(crate) fn require_owner_history(
        self,
        tx: &Transaction,
        model: &Model,
        entity: &'static str,
        id: EntityId,
    ) -> SpatialAuthoringResult<()> {
        let actual =
            projected_type(tx, model, id).ok_or(SpatialAuthoringError::MissingReference {
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
            Err(SpatialAuthoringError::WrongReferenceType {
                entity,
                attribute: "OwnerHistory",
                target: id,
                actual,
                expected: "IFCOWNERHISTORY",
            })
        }
    }
}

impl Release {
    /// Fail unless every id in `contexts` is, in the model or staged on
    /// `tx`, an `IfcRepresentationContext` of this release that is not an
    /// `IfcGeometricRepresentationSubContext`, and none repeats.
    ///
    /// `IfcContext.RepresentationContexts` is a `SET`, and every release's
    /// `IfcProject` forbids a sub-context there (IFC2X3 TC1 `WR32`, IFC4 ADD2
    /// TC1 and IFC4X3 ADD2 `CorrectContext`).
    pub(crate) fn require_contexts(
        self,
        tx: &Transaction,
        model: &Model,
        entity: &'static str,
        contexts: &[EntityId],
    ) -> SpatialAuthoringResult<()> {
        const ATTRIBUTE: &str = "RepresentationContexts";
        const CONTEXT: &str = "IFCREPRESENTATIONCONTEXT";
        const SUBCONTEXT: &str = "IFCGEOMETRICREPRESENTATIONSUBCONTEXT";
        for (index, &id) in contexts.iter().enumerate() {
            if contexts[..index].contains(&id) {
                return Err(super::invalid(
                    entity,
                    ATTRIBUTE,
                    format!("#{} repeats in a SET", id.0),
                ));
            }
            let actual = projected_type(tx, model, id)
                .ok_or(SpatialAuthoringError::MissingReference {
                    entity,
                    attribute: ATTRIBUTE,
                    target: id,
                })?
                .to_ascii_uppercase();
            if !self.schema.is_a(&actual, CONTEXT) {
                return Err(SpatialAuthoringError::WrongReferenceType {
                    entity,
                    attribute: ATTRIBUTE,
                    target: id,
                    actual,
                    expected: CONTEXT,
                });
            }
            if self.schema.is_a(&actual, SUBCONTEXT) {
                return Err(super::invalid(
                    entity,
                    ATTRIBUTE,
                    format!("#{} is an IfcGeometricRepresentationSubContext", id.0),
                ));
            }
        }
        Ok(())
    }
}

/// Validate and stage `entity` in `model`'s declared release.
///
/// `values` name every attribute but `OwnerHistory`, which is `owner` or
/// `$`. The owner history, when given, must exist and be accepted.
pub(crate) fn stage(
    tx: &mut Transaction,
    model: &Model,
    entity: &'static str,
    mut values: Vec<(&'static str, Value)>,
    owner: Option<EntityId>,
) -> SpatialAuthoringResult<EntityId> {
    let release = bind(model)?;
    values.push(("OwnerHistory", owner.map_or(Value::Null, Value::Ref)));
    let record = release.record(entity, values)?;
    if let Some(owner) = owner {
        release.require_owner_history(tx, model, entity, owner)?;
    }
    Ok(tx.create(record))
}

/// The type `target` will have once `tx` commits, or `None` if absent.
fn projected_type(tx: &Transaction, model: &Model, target: EntityId) -> Option<String> {
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

/// The name `release` gives the attribute this crate knows by its IFC4
/// name.
///
/// IFC4 renamed IFC2X3 `IfcRelCoversSpaces.RelatedSpace` to `RelatingSpace`
/// in place (IFC4 ADD2 TC1, `IfcRelCoversSpaces`: "The attribute name has
/// been changed from RelatedSpace to RelatingSpace with upward
/// compatibility for file based exchange"). No other attribute these
/// writers name differs between IFC2X3 TC1, IFC4 ADD2 TC1 and IFC4X3 ADD2.
fn release_name(release: SchemaVersion, entity: &str, attribute: &'static str) -> &'static str {
    match (release, entity, attribute) {
        (SchemaVersion::Ifc2x3, "IFCRELCOVERSSPACES", "RelatingSpace") => "RelatedSpace",
        _ => attribute,
    }
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
