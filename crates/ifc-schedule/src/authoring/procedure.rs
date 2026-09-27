//! Staging `IfcProcedure`.
//!
//! A procedure is a process, not a task: it describes how work is
//! carried out rather than when. It is split from the task and
//! calendar writers because it shares none of their time slots.

use ifc_model::guid::Guid;
use ifc_model::{Entity, EntityId, Transaction, Value};

use super::{optional_text, ScheduleAuthoringError, ScheduleAuthoringResult};
/// `IfcProcedureTypeEnum`.
///
/// Closed: a token outside it names a procedure kind the schema
/// does not define.
const PROCEDURE_KIND: &[&str] = &[
    "ADVICE_CAUTION",
    "ADVICE_NOTE",
    "ADVICE_WARNING",
    "CALIBRATION",
    "DIAGNOSTIC",
    "SHUTDOWN",
    "STARTUP",
    "USERDEFINED",
    "NOTDEFINED",
];

/// Attributes of an `IfcProcedure`.
#[derive(Debug, Clone, Copy, Default)]
pub struct ProcedureDraft<'a> {
    /// `GlobalId`, a compressed IFC GUID.
    pub global_id: &'a str,
    /// `Name`. Required by `HasName`.
    pub name: Option<&'a str>,
    /// `Description`.
    pub description: Option<&'a str>,
    /// `ObjectType`. Required when `predefined_type` is `USERDEFINED`.
    pub object_type: Option<&'a str>,
    /// `Identification`.
    pub identification: Option<&'a str>,
    /// `LongDescription`.
    pub long_description: Option<&'a str>,
    /// `PredefinedType`, an `IfcProcedureTypeEnum` token.
    pub predefined_type: Option<&'a str>,
}

/// Stage an `IfcProcedure`.
///
/// `HasName` makes `Name` mandatory even though the slot is
/// OPTIONAL: a procedure nothing can name cannot be referred to by
/// the work that must follow it.
///
/// # Errors
///
/// Refuses a malformed GlobalId, a blank name (`HasName`), a token
/// outside `IfcProcedureTypeEnum`, and `USERDEFINED` without
/// `ObjectType` (`CorrectPredefinedType`).
pub fn create_procedure(
    tx: &mut Transaction,
    draft: ProcedureDraft<'_>,
) -> ScheduleAuthoringResult<EntityId> {
    const ENTITY: &str = "IFCPROCEDURE";
    if Guid::parse(draft.global_id).is_none() {
        return Err(ScheduleAuthoringError::InvalidValue {
            entity: ENTITY,
            attribute: "GlobalId",
            expected: "an IFC compressed GUID",
        });
    }
    if draft.name.is_none_or(|value| value.trim().is_empty()) {
        return Err(ScheduleAuthoringError::InvalidValue {
            entity: ENTITY,
            attribute: "Name",
            expected: "a name, required by HasName",
        });
    }
    if let Some(token) = draft.predefined_type {
        if !PROCEDURE_KIND.contains(&token) {
            return Err(ScheduleAuthoringError::InvalidValue {
                entity: ENTITY,
                attribute: "PredefinedType",
                expected: "a token of IfcProcedureTypeEnum",
            });
        }
        if token == "USERDEFINED"
            && draft
                .object_type
                .is_none_or(|value| value.trim().is_empty())
        {
            return Err(ScheduleAuthoringError::InvalidValue {
                entity: ENTITY,
                attribute: "ObjectType",
                expected: "a name, required by USERDEFINED",
            });
        }
    }
    let mut attributes = vec![Value::Null; 8];
    attributes[0] = Value::Text(draft.global_id.into());
    attributes[2] = optional_text(draft.name);
    attributes[3] = optional_text(draft.description);
    attributes[4] = optional_text(draft.object_type);
    attributes[5] = optional_text(draft.identification);
    attributes[6] = optional_text(draft.long_description);
    attributes[7] = draft
        .predefined_type
        .map_or(Value::Null, |t| Value::Enum(t.into()));
    Ok(tx.create(Entity::new(ENTITY, attributes)))
}
