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

use crate::error::{ScheduleAuthoringError, ScheduleReadError};

type Result<T> = std::result::Result<T, ScheduleAuthoringError>;

/// The bound release and its table.
#[derive(Clone, Copy)]
pub(crate) struct Release {
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
pub(crate) fn bind(model: &Model) -> Result<Release> {
    let version = match model.header().schema.as_slice() {
        [] => SchemaVersion::Ifc4,
        [token] => SchemaVersion::from_header_token(token)
            .filter(|version| proven(*version))
            .ok_or_else(|| ScheduleAuthoringError::UnsupportedSchema {
                schema: token.clone(),
            })?,
        tokens => {
            return Err(ScheduleAuthoringError::MultipleSchemas {
                schemas: tokens.len(),
            })
        }
    };
    // A release this build carries no table for (a future `SchemaVersion`)
    // is refused, never assumed to be laid out like another.
    let schema = for_version(version).map_err(|_| ScheduleAuthoringError::UnsupportedSchema {
        schema: format!("{version:?}"),
    })?;
    Ok(Release { version, schema })
}

impl Release {
    /// The bound version.
    pub(crate) const fn version(self) -> SchemaVersion {
        self.version
    }

    /// Fail with `EntityNotInSchema` unless this release declares `entity`
    /// as an instantiable entity.
    pub(crate) fn require_entity(self, entity: &'static str) -> Result<()> {
        if self.schema.entity(entity).is_none_or(|e| e.abstract_) {
            return Err(ScheduleAuthoringError::EntityNotInSchema {
                entity,
                schema: self.version,
            });
        }
        Ok(())
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
        self.require_entity(entity)?;
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

/// The release the readers interpret records against (#212).
///
/// The same rule as [`bind`]: one declaration of IFC2X3, IFC4 or IFC4X3
/// binds that release's table; any other single declaration (IFC4X1 and
/// IFC4X2 included) is [`ScheduleReadError::UnsupportedSchema`]; several
/// are [`ScheduleReadError::MultipleSchemas`]; none reads as IFC4.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ReadRelease {
    version: SchemaVersion,
    schema: &'static Schema,
}

impl ReadRelease {
    /// Bind `version`, refusing a release the readers are not verified for.
    pub(crate) fn of_version(
        version: SchemaVersion,
    ) -> std::result::Result<Self, ScheduleReadError> {
        let unsupported = || ScheduleReadError::UnsupportedSchema {
            schema: version.release_id().to_owned(),
        };
        if !proven(version) {
            return Err(unsupported());
        }
        let schema = for_version(version).map_err(|_| unsupported())?;
        Ok(Self { version, schema })
    }

    /// Bind `model`'s declared release.
    pub(crate) fn of(model: &Model) -> std::result::Result<Self, ScheduleReadError> {
        match model.header().schema.as_slice() {
            [] => Self::of_version(SchemaVersion::Ifc4),
            [token] => SchemaVersion::from_header_token(token)
                .filter(|version| proven(*version))
                .ok_or_else(|| ScheduleReadError::UnsupportedSchema {
                    schema: token.clone(),
                })
                .and_then(Self::of_version),
            tokens => Err(ScheduleReadError::MultipleSchemas {
                schemas: tokens.len(),
            }),
        }
    }

    /// The bound release.
    pub(crate) const fn version(self) -> SchemaVersion {
        self.version
    }

    /// Every instance of `ancestor` or one of its subtypes in this
    /// release, in file order (#236).
    ///
    /// `IfcRelSequence` relates any `IfcProcess`: `IfcTask`, `IfcProcedure`
    /// and, from IFC4, `IfcEvent`, plus IFC2X3's `IfcMove` and
    /// `IfcOrderAction` under `IfcTask`. The subtypes come from the bound
    /// release's table, never from a list written here.
    pub(crate) fn instances_of(self, model: &Model, ancestor: &str) -> Vec<EntityId> {
        let mut types: std::collections::HashSet<String> = self
            .schema
            .subtypes(ancestor)
            .into_iter()
            .map(str::to_ascii_uppercase)
            .collect();
        types.insert(ancestor.to_ascii_uppercase());
        model
            .iter()
            .filter(|(_, entity)| types.contains(&entity.type_name.to_ascii_uppercase()))
            .map(|(id, _)| id)
            .collect()
    }

    /// Whether this release declares `entity` as an entity (#234).
    ///
    /// IFC2X3 declares no `IfcWorkCalendar` or `IfcEvent`, and IFC4 and
    /// IFC4X3 no `IfcRelAssignsTasks` or `IfcScheduleTimeControl`; a
    /// record of an undeclared type is not one of that release's.
    pub(crate) fn declares(self, entity: &str) -> bool {
        self.schema.entity(entity).is_some()
    }

    /// Whether `type_name` is `ancestor` or one of its subtypes in this
    /// release (#235), so an `IfcTaskTimeRecurring` is an `IfcTaskTime`.
    /// An undeclared type is nothing.
    pub(crate) fn is_a(self, type_name: &str, ancestor: &str) -> bool {
        self.schema.is_a(type_name, ancestor)
    }

    fn slot(self, entity: &str, attribute: &'static str) -> Option<usize> {
        let entity = entity.to_ascii_uppercase();
        let name = release_name(self.version, &entity, attribute);
        self.schema
            .attribute_names(&entity)
            .iter()
            .position(|declared| declared.eq_ignore_ascii_case(name))
    }

    /// The value of `attribute` (IFC4 name) on a record of `entity`; `None`
    /// when the release does not declare it, the record leaves it `$`, or
    /// the record ends before it.
    pub(crate) fn value<'m>(
        self,
        entity: &str,
        record: &'m Entity,
        attribute: &'static str,
    ) -> Option<&'m Value> {
        match record.attribute(self.slot(entity, attribute)?)? {
            Value::Null => None,
            value => Some(value),
        }
    }

    /// The text of `attribute`, or `None`.
    pub(crate) fn text<'m>(
        self,
        entity: &str,
        record: &'m Entity,
        attribute: &'static str,
    ) -> Option<&'m str> {
        self.value(entity, record, attribute)?
            .unwrap_typed()
            .as_text()
    }

    /// The enumeration token of `attribute`, without its dots, or `None`.
    pub(crate) fn token<'m>(
        self,
        entity: &str,
        record: &'m Entity,
        attribute: &'static str,
    ) -> Option<&'m str> {
        match self.value(entity, record, attribute)? {
            Value::Enum(token) => Some(token),
            _ => None,
        }
    }

    /// The reference in `attribute`, or `None`.
    pub(crate) fn reference(
        self,
        entity: &str,
        record: &Entity,
        attribute: &'static str,
    ) -> Option<EntityId> {
        match self.value(entity, record, attribute)? {
            Value::Ref(id) => Some(*id),
            _ => None,
        }
    }
}

/// The name `release` gives the attribute this crate knows by its IFC4
/// name.
///
/// From the IFC4 ADD2 TC1 documentation: `IfcProcess.Identification` "has
/// been promoted from subtypes IfcTask and others", which is IFC2X3
/// `IfcTask.TaskId` and `IfcProcedure.ProcedureID`; and on `IfcProcedure`,
/// "ProcedureType renamed to PredefinedType". On `IfcWorkControl`,
/// `Identification` is the "attribute unified by promoting from various
/// subtypes of IfcControl", IFC2X3's `IfcWorkControl.Identifier`. IFC4X3
/// ADD2 renames `IfcWorkTime.Start` and `Finish` to `StartDate` and
/// `FinishDate` at the same positions, both still `IfcDate` (#234). Each
/// keeps its meaning.
fn release_name(release: SchemaVersion, entity: &str, attribute: &'static str) -> &'static str {
    match (release, entity, attribute) {
        (SchemaVersion::Ifc2x3, "IFCWORKPLAN" | "IFCWORKSCHEDULE", "Identification") => {
            "Identifier"
        }
        (SchemaVersion::Ifc2x3, "IFCTASK", "Identification") => "TaskId",
        (SchemaVersion::Ifc2x3, "IFCPROCEDURE", "Identification") => "ProcedureID",
        (SchemaVersion::Ifc2x3, "IFCPROCEDURE", "PredefinedType") => "ProcedureType",
        (SchemaVersion::Ifc4x3, "IFCWORKTIME", "Start") => "StartDate",
        (SchemaVersion::Ifc4x3, "IFCWORKTIME", "Finish") => "FinishDate",
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
                matches!(bind(&model), Err(ScheduleAuthoringError::UnsupportedSchema { schema }) if schema == token),
                "{token} must be refused"
            );
        }
    }
}
