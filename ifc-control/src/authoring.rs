//! Staging the four `IfcControl` subtypes this crate owns.
//!
//! # What a control is
//!
//! A control is a record that governs work rather than describing
//! physical form: a permit that authorises it, an order that
//! commissions it, a request that asks for it, a performance history
//! that records how it behaved. They carry no geometry.
//!
//! # The shape they share
//!
//! All four are `IfcControl` subtypes, so slots 0-5 are fixed:
//! the four `IfcRoot` slots, `ObjectType`, and `Identification`.
//! Three of them then take `PredefinedType`, `Status` and
//! `LongDescription`. `IfcPerformanceHistory` does not: it takes a
//! required `LifeCyclePhase` at slot 6 and its predefined type at
//! slot 7, so a writer that assumes the common tail would file the
//! phase as a status.
//!
//! # USERDEFINED
//!
//! These entities declare no WHERE rules. The `USERDEFINED` token
//! still asserts that a name is given elsewhere, and `ObjectType` is
//! where `IfcObject` puts it. Writing `USERDEFINED` without it
//! produces a record that claims a specific kind and withholds which,
//! so it is refused here even though the schema does not say so.

use ifc_model::guid::Guid;
use ifc_model::{Entity, EntityId, Transaction, Value};
use ifc_schema::Schema;

use crate::error::{ControlError, ControlResult};

/// `IfcPermitTypeEnum`.
const PERMIT: &[&str] = &["ACCESS", "BUILDING", "WORK", "USERDEFINED", "NOTDEFINED"];

/// `IfcProjectOrderTypeEnum`.
const PROJECT_ORDER: &[&str] = &[
    "CHANGEORDER",
    "MAINTENANCEWORKORDER",
    "MOVEORDER",
    "PURCHASEORDER",
    "WORKORDER",
    "USERDEFINED",
    "NOTDEFINED",
];

/// `IfcActionRequestTypeEnum`.
const ACTION_REQUEST: &[&str] = &[
    "EMAIL",
    "FAX",
    "PHONE",
    "POST",
    "VERBAL",
    "USERDEFINED",
    "NOTDEFINED",
];

/// `IfcPerformanceHistoryTypeEnum`.
const PERFORMANCE_HISTORY: &[&str] = &["USERDEFINED", "NOTDEFINED"];

/// Which control is being staged.
///
/// A single enum rather than four writers: the four differ only in
/// type name, predefined-type enum, and whether they carry the
/// `Status`/`LongDescription` tail. Four near-identical functions
/// would drift apart on the next schema revision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ControlKind {
    /// `IfcPermit`: authorisation to proceed.
    Permit,
    /// `IfcProjectOrder`: an instruction to carry work out.
    ProjectOrder,
    /// `IfcActionRequest`: a request that work be done.
    ActionRequest,
    /// `IfcPerformanceHistory`: recorded in-use behaviour.
    PerformanceHistory,
}

impl ControlKind {
    /// STEP type name, upper-case as the catalogue stores it.
    ///
    /// Upper-case because `Entity::new` does not normalise and the
    /// model indexes by the stored string.
    #[must_use]
    pub const fn type_name(self) -> &'static str {
        match self {
            Self::Permit => "IFCPERMIT",
            Self::ProjectOrder => "IFCPROJECTORDER",
            Self::ActionRequest => "IFCACTIONREQUEST",
            Self::PerformanceHistory => "IFCPERFORMANCEHISTORY",
        }
    }

    /// The tokens this entity's own `PredefinedType` enum declares.
    #[must_use]
    pub const fn members(self) -> &'static [&'static str] {
        match self {
            Self::Permit => PERMIT,
            Self::ProjectOrder => PROJECT_ORDER,
            Self::ActionRequest => ACTION_REQUEST,
            Self::PerformanceHistory => PERFORMANCE_HISTORY,
        }
    }

    /// Slot holding `PredefinedType`.
    ///
    /// `IfcPerformanceHistory` puts it at 7, after the required
    /// `LifeCyclePhase`; the others at 6.
    const fn predefined_slot(self) -> usize {
        match self {
            Self::PerformanceHistory => 7,
            _ => 6,
        }
    }
}

/// Attributes shared by every control.
#[derive(Debug, Clone, Copy, Default)]
pub struct ControlDraft<'a> {
    /// `Name`.
    pub name: Option<&'a str>,
    /// `Description`.
    pub description: Option<&'a str>,
    /// `ObjectType`, slot 4. Required when the predefined type is
    /// `USERDEFINED`.
    pub object_type: Option<&'a str>,
    /// `Identification`, slot 5: the permit or order number.
    pub identification: Option<&'a str>,
    /// `Status`, slot 7. Not declared by `IfcPerformanceHistory`.
    pub status: Option<&'a str>,
    /// `LongDescription`, slot 8. Not declared by
    /// `IfcPerformanceHistory`.
    pub long_description: Option<&'a str>,
    /// `LifeCyclePhase`, slot 6. Required by
    /// `IfcPerformanceHistory` and declared by no other control.
    pub life_cycle_phase: Option<&'a str>,
}

fn invalid(
    entity: &'static str,
    attribute: &'static str,
    value: impl Into<String>,
) -> ControlError {
    ControlError::AuthoringInvalid {
        entity,
        attribute,
        value: value.into(),
    }
}

fn text(value: Option<&str>) -> Value {
    value.map_or(Value::Null, |v| Value::Text(v.into()))
}

fn blank(value: Option<&str>) -> bool {
    value.is_none_or(|v| v.trim().is_empty())
}

/// Stage one control record.
///
/// The arity is taken from `schema`, not hardcoded, so a schema that
/// declares a different tail produces a correctly-sized record
/// instead of a silently truncated one.
///
/// # Errors
///
/// Refuses a malformed GlobalId; a blank name; a token outside the
/// entity's own enum; `USERDEFINED` without `object_type`; a
/// `life_cycle_phase` on an entity that does not declare it and a
/// missing one on `IfcPerformanceHistory`; `status` or
/// `long_description` on `IfcPerformanceHistory`; and an entity the
/// schema does not declare.
pub fn create_control(
    tx: &mut Transaction,
    schema: &Schema,
    kind: ControlKind,
    global_id: &str,
    predefined_type: Option<&str>,
    draft: ControlDraft<'_>,
) -> ControlResult<EntityId> {
    let entity = kind.type_name();
    let declared = schema.attributes(entity);
    if declared.is_empty() {
        return Err(ControlError::UnsupportedEntity {
            schema: schema.name().to_owned(),
            entity,
        });
    }

    if Guid::parse(global_id).is_none() {
        return Err(invalid(entity, "GlobalId", global_id));
    }
    // `IfcRoot.Name` is optional in the slot table, but a control
    // nobody can name is a record nobody can cite in correspondence.
    if blank(draft.name) {
        return Err(invalid(entity, "Name", "required"));
    }

    if let Some(token) = predefined_type {
        if !kind.members().contains(&token) {
            return Err(invalid(entity, "PredefinedType", token));
        }
        if token == "USERDEFINED" && blank(draft.object_type) {
            return Err(invalid(entity, "ObjectType", "required by USERDEFINED"));
        }
    }

    let history = kind == ControlKind::PerformanceHistory;
    // Attributes the entity does not declare are refused, not
    // dropped: silently discarding a Status writes a file missing
    // data the caller believes they supplied.
    if history {
        if blank(draft.life_cycle_phase) {
            return Err(invalid(entity, "LifeCyclePhase", "required"));
        }
        if draft.status.is_some() {
            return Err(invalid(entity, "Status", "not declared"));
        }
        if draft.long_description.is_some() {
            return Err(invalid(entity, "LongDescription", "not declared"));
        }
    } else if draft.life_cycle_phase.is_some() {
        return Err(invalid(entity, "LifeCyclePhase", "not declared"));
    }

    let mut attrs = vec![Value::Null; declared.len()];
    attrs[0] = Value::Text(global_id.into());
    attrs[2] = text(draft.name);
    attrs[3] = text(draft.description);
    attrs[4] = text(draft.object_type);
    attrs[5] = text(draft.identification);
    if history {
        attrs[6] = text(draft.life_cycle_phase);
    } else {
        // Indexing is bounded by the schema's own count rather than the
        // 9 slots IFC4 happens to declare: a panic here would turn a
        // schema difference into a crash instead of a refusal.
        let tail = [
            (7, "Status", draft.status),
            (8, "LongDescription", draft.long_description),
        ];
        for (slot, attribute, value) in tail {
            if let Some(cell) = attrs.get_mut(slot) {
                *cell = text(value);
            } else if value.is_some() {
                return Err(invalid(entity, attribute, "not declared"));
            }
        }
    }
    attrs[kind.predefined_slot()] = predefined_type.map_or(Value::Null, |t| Value::Enum(t.into()));

    Ok(tx.create(Entity::new(entity, attrs)))
}
