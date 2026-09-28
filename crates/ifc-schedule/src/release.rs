//! The IFC release schedule records are laid out against (#202).
//!
//! The `*_with_owner_history` writers bind the model's declared release, as
//! `ifc-material` (#77), `ifc-properties` (#191), `ifc-classification`
//! (#194) and `ifc-control` (#198) do, from `FILE_SCHEMA`:
//! - one recognised declaration binds that release's bundled table;
//! - one unrecognised declaration fails with
//!   [`ScheduleAuthoringError::UnsupportedSchema`];
//! - several fail with [`ScheduleAuthoringError::MultipleSchemas`];
//! - none at all (an in-memory [`Model::new`]) binds IFC4.
//!
//! Every record is built by attribute name from that table, so an IFC2X3
//! `IfcTask` (ten attributes, `TaskId` required, no `PredefinedType`) is
//! never written with IFC4's thirteen slots, and a value the release cannot
//! hold is refused instead of written into a slot that means something else.

use ifc_model::{Edit, Entity, EntityId, Model, Transaction, Value};
use ifc_schema::{for_version, Attribute, Schema, SchemaVersion, TypeKind};

use crate::error::ScheduleAuthoringError;

type Result<T> = std::result::Result<T, ScheduleAuthoringError>;

/// The bound release and its table.
#[derive(Clone, Copy)]
pub(crate) struct Release {
    version: SchemaVersion,
    schema: &'static Schema,
}

/// Bind `model`'s declared release.
pub(crate) fn bind(model: &Model) -> Result<Release> {
    let version = match model.header().schema.as_slice() {
        [] => SchemaVersion::Ifc4,
        [token] => SchemaVersion::from_header_token(token).ok_or_else(|| {
            ScheduleAuthoringError::UnsupportedSchema {
                schema: token.clone(),
            }
        })?,
        tokens => {
            return Err(ScheduleAuthoringError::MultipleSchemas {
                schemas: tokens.len(),
            })
        }
    };
    let schema = for_version(version).expect("every SchemaVersion has a bundled table");
    Ok(Release { version, schema })
}

impl Release {
    /// The bound version.
    pub(crate) const fn version(self) -> SchemaVersion {
        self.version
    }

    /// Build `entity`'s record in this release's layout from values named
    /// by their IFC4 attribute names.
    ///
    /// An entity the release does not declare is `EntityNotInSchema`; a
    /// non-null value for an attribute it does not declare is
    /// `AuthoringNotInSchema`, one it cannot hold `AuthoringValueType`, and
    /// a null in a slot it requires (IFC2X3 `IfcRoot.OwnerHistory`)
    /// `AuthoringRequired`. Nothing is ever dropped or invented.
    pub(crate) fn record(
        self,
        entity: &'static str,
        values: Vec<(&'static str, Value)>,
    ) -> Result<Entity> {
        if self.schema.entity(entity).is_none_or(|e| e.abstract_) {
            return Err(ScheduleAuthoringError::EntityNotInSchema {
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
                return Err(ScheduleAuthoringError::AuthoringNotInSchema {
                    entity,
                    attribute,
                    schema: self.version,
                });
            };
            if !conforms(self.schema, declared[slot], &value) {
                return Err(ScheduleAuthoringError::AuthoringValueType {
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
            return Err(ScheduleAuthoringError::AuthoringRequired {
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
    ) -> Result<()> {
        let actual =
            projected_type(tx, model, id).ok_or(ScheduleAuthoringError::MissingReference {
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
            Err(ScheduleAuthoringError::WrongReferenceType {
                entity,
                attribute: "OwnerHistory",
                target: id,
                actual,
                expected: "IFCOWNERHISTORY",
            })
        }
    }
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
/// From the IFC4 ADD2 TC1 documentation: `IfcProcess.Identification` "has
/// been promoted from subtypes IfcTask and others", which is IFC2X3
/// `IfcTask.TaskId` and `IfcProcedure.ProcedureID`; and on `IfcProcedure`,
/// "ProcedureType renamed to PredefinedType". On `IfcWorkControl`,
/// `Identification` is the "attribute unified by promoting from various
/// subtypes of IfcControl", IFC2X3's `IfcWorkControl.Identifier`. Each keeps
/// its meaning.
fn release_name(release: SchemaVersion, entity: &str, attribute: &'static str) -> &'static str {
    match (release, entity, attribute) {
        (SchemaVersion::Ifc2x3, "IFCWORKPLAN" | "IFCWORKSCHEDULE", "Identification") => {
            "Identifier"
        }
        (SchemaVersion::Ifc2x3, "IFCTASK", "Identification") => "TaskId",
        (SchemaVersion::Ifc2x3, "IFCPROCEDURE", "Identification") => "ProcedureID",
        (SchemaVersion::Ifc2x3, "IFCPROCEDURE", "PredefinedType") => "ProcedureType",
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
