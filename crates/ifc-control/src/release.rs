//! The IFC release a control record is laid out against (#198, #202).
//!
//! Every record is built by attribute name from one release's table, so an
//! IFC2X3 `IfcPermit` (six attributes, `PermitID` required, no
//! `PredefinedType`) is never written with IFC4's nine slots, and a value
//! the release cannot hold is refused instead of written past the record.
//!
//! [`create_control`](crate::create_control) and
//! [`assign_to_control`](crate::assign_to_control) take the table from the
//! caller. The `*_with_owner_history` writers bind the model's declared
//! release, as `ifc-material` (#77), `ifc-properties` (#191) and
//! `ifc-classification` (#194) do, from `FILE_SCHEMA`:
//! - one recognised declaration binds that release's bundled table;
//! - one unrecognised declaration fails with
//!   [`ControlError::UnsupportedSchema`];
//! - several fail with [`ControlError::MultipleSchemas`];
//! - none at all (an in-memory [`Model::new`]) binds IFC4.

use ifc_model::{Edit, Entity, EntityId, Model, Transaction, Value};
use ifc_schema::{for_version, Attribute, Schema, SchemaVersion, TypeKind};

use crate::error::{ControlError, ControlResult};

/// The table a record is laid out against.
#[derive(Clone, Copy)]
pub(crate) struct Release<'s> {
    schema: &'s Schema,
}

impl<'s> Release<'s> {
    /// Lay records out against a caller-supplied table.
    pub(crate) const fn of_schema(schema: &'s Schema) -> Self {
        Self { schema }
    }

    /// The table itself.
    pub(crate) const fn schema(self) -> &'s Schema {
        self.schema
    }

    fn name(self) -> String {
        self.schema.name().to_owned()
    }

    /// Build `entity`'s record in this release's layout from values named
    /// by their IFC4 attribute names.
    ///
    /// A non-null value the release does not declare is refused with
    /// `AuthoringNotInSchema`, one it cannot hold (text where IFC2X3 wants
    /// an `IfcDateTimeSelect`, a token outside the release's enumeration)
    /// with `AuthoringValueType`, and a null in a slot it requires (IFC2X3
    /// `IfcRoot.OwnerHistory`, `IfcPermit.PermitID`) with
    /// `AuthoringRequired`. Nothing is ever dropped or invented.
    pub(crate) fn record(
        self,
        entity: &'static str,
        values: Vec<(&'static str, Value)>,
    ) -> ControlResult<Entity> {
        if self.schema.entity(entity).is_none_or(|e| e.abstract_) {
            return Err(ControlError::UnsupportedEntity {
                schema: self.name(),
                entity,
            });
        }
        let declared = self.schema.attributes(entity);
        let mut slots = vec![Value::Null; declared.len()];
        for (attribute, value) in values {
            let name = release_name(self.schema.version(), entity, attribute);
            let Some(slot) = declared
                .iter()
                .position(|d| d.name.eq_ignore_ascii_case(name))
            else {
                if value == Value::Null {
                    continue;
                }
                return Err(ControlError::AuthoringNotInSchema {
                    entity,
                    attribute,
                    schema: self.name(),
                });
            };
            if !conforms(self.schema, declared[slot], &value) {
                return Err(ControlError::AuthoringValueType {
                    entity,
                    attribute,
                    declared: declared[slot].type_name.clone(),
                    schema: self.name(),
                });
            }
            slots[slot] = value;
        }
        if let Some(missing) = declared
            .iter()
            .zip(&slots)
            .find(|(d, value)| !d.optional && **value == Value::Null)
        {
            return Err(ControlError::AuthoringRequired {
                entity,
                attribute: missing.0.name.clone(),
                schema: self.name(),
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
    ) -> ControlResult<()> {
        let actual = projected_type(tx, model, id).ok_or(ControlError::UnknownEntity { id })?;
        let accepted = self
            .schema
            .attributes(entity)
            .iter()
            .find(|d| d.name.eq_ignore_ascii_case("OwnerHistory"))
            .is_some_and(|d| self.schema.accepts_type(&d.type_name, &actual));
        if accepted {
            Ok(())
        } else {
            Err(ControlError::AuthoringInvalid {
                entity,
                attribute: "OwnerHistory",
                value: format!("{id} is {actual}, not an IfcOwnerHistory"),
            })
        }
    }
}

/// Bind `model`'s declared release.
pub(crate) fn bind(model: &Model) -> ControlResult<Release<'static>> {
    let version = match model.header().schema.as_slice() {
        [] => SchemaVersion::Ifc4,
        [token] => SchemaVersion::from_header_token(token).ok_or_else(|| {
            ControlError::UnsupportedSchema {
                schema: token.clone(),
            }
        })?,
        tokens => {
            return Err(ControlError::MultipleSchemas {
                schemas: tokens.len(),
            })
        }
    };
    let schema = for_version(version).expect("every SchemaVersion has a bundled table");
    Ok(Release::of_schema(schema))
}

/// The type `target` will have once `tx` commits, or `None` if absent.
///
/// The latest staged edit touching `target` wins; otherwise the model's
/// record answers.
pub(crate) fn projected_type(tx: &Transaction, model: &Model, target: EntityId) -> Option<String> {
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
/// IFC4 promoted each control's own identifier to
/// `IfcControl.Identification` with the same meaning. The IFC4 ADD2 TC1
/// documentation states each rename: "Attribute PermitID renamed to
/// Identification", "Atribute RequestID renamed to Identification" and
/// (`IfcProjectOrder`) "Attribute ID renamed to Identification". IFC2X3
/// `IfcPerformanceHistory` declares no identifier at all.
fn release_name(
    release: Option<SchemaVersion>,
    entity: &str,
    attribute: &'static str,
) -> &'static str {
    match (release, entity, attribute) {
        (Some(SchemaVersion::Ifc2x3), "IFCPERMIT", "Identification") => "PermitID",
        (Some(SchemaVersion::Ifc2x3), "IFCACTIONREQUEST", "Identification") => "RequestID",
        (Some(SchemaVersion::Ifc2x3), "IFCPROJECTORDER", "Identification") => "ID",
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
