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
//! The positions above are IFC4's. IFC2X3 declares an `IfcControl`
//! without `Identification`, and each control its own tail (#198):
//!
//! ```text
//! IFC2X3_TC1  IfcPermit              ... ObjectType, PermitID
//! IFC2X3_TC1  IfcActionRequest       ... ObjectType, RequestID
//! IFC2X3_TC1  IfcProjectOrder        ... ObjectType, ID, PredefinedType, Status
//! IFC2X3_TC1  IfcPerformanceHistory  ... ObjectType, LifeCyclePhase
//! ```
//!
//! So records are laid out by attribute name from the release's own table
//! (`release.rs`), never by these positions.
//!
//! # USERDEFINED
//!
//! These entities declare no WHERE rules. The `USERDEFINED` token
//! still asserts that a name is given elsewhere, and `ObjectType` is
//! where `IfcObject` puts it. Writing `USERDEFINED` without it
//! produces a record that claims a specific kind and withholds which,
//! so it is refused here even though the schema does not say so.
//!
//! # Work orders
//!
//! `IfcWorkOrder` does not exist in any IFC release: a work order is
//! an `IfcProjectOrder` whose `PredefinedType` is `WORKORDER`. There is
//! deliberately no separate entity or `ControlKind` for it.

use ifc_model::guid::Guid;
use ifc_model::{Entity, EntityId, Model, Transaction, Value};
use ifc_schema::Schema;

use crate::error::{ControlError, ControlResult};
use crate::release::{bind, Release};

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
    /// Every control this crate owns, in declaration order.
    pub const ALL: [Self; 4] = [
        Self::Permit,
        Self::ProjectOrder,
        Self::ActionRequest,
        Self::PerformanceHistory,
    ];

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

/// Stage one control record, laid out by attribute name in `schema`.
///
/// `OwnerHistory` is left unset, which IFC4 and IFC4X3 allow. IFC2X3
/// requires it, so an IFC2X3 `schema` is refused with
/// [`ControlError::AuthoringRequired`] instead of written as `$`; use
/// [`create_control_with_owner_history`] there. Before #198 an IFC2X3
/// `IfcPermit`, `IfcActionRequest` or `IfcPerformanceHistory` panicked
/// here, indexing past their six declared attributes.
///
/// # Errors
///
/// Refuses a malformed GlobalId; a blank name; a token outside the
/// entity's own enum; `USERDEFINED` without `object_type`; a
/// `life_cycle_phase` on an entity that does not declare it and a
/// missing one on `IfcPerformanceHistory`; `status` or
/// `long_description` on `IfcPerformanceHistory`; and an entity the
/// schema does not declare. Against the release's own table: a value
/// the entity does not declare there (`AuthoringNotInSchema`, such as an
/// IFC2X3 `PredefinedType` on a permit), one it cannot hold
/// (`AuthoringValueType`), and a required one left unset
/// (`AuthoringRequired`, such as the IFC2X3 `OwnerHistory` or
/// `PermitID`). Nothing is staged on an error.
pub fn create_control(
    tx: &mut Transaction,
    schema: &Schema,
    kind: ControlKind,
    global_id: &str,
    predefined_type: Option<&str>,
    draft: ControlDraft<'_>,
) -> ControlResult<EntityId> {
    let release = Release::of_schema(schema);
    let record = control_record(
        release,
        kind,
        global_id,
        predefined_type,
        draft,
        Value::Null,
    )?;
    Ok(tx.create(record))
}

/// [`create_control`] in `model`'s declared release, with a caller-supplied
/// `IfcOwnerHistory`, which IFC2X3 requires (#198, #202).
///
/// The release is bound from `FILE_SCHEMA` (none binds IFC4). In IFC2X3
/// `Identification` is written as the entity's own identifier
/// (`PermitID`, `RequestID`, `ID`), which IFC2X3 requires; IFC2X3 declares
/// no `PredefinedType` for a permit, an action request or a performance
/// history, no `Status` for a permit or an action request, and no
/// `LongDescription` at all. In IFC4 and IFC4X3 the record is that of
/// [`create_control`] with the reference in the optional slot. The
/// owner history is never invented: build it with `ifc-author`.
///
/// # Errors
///
/// Those of [`create_control`] except the IFC2X3 refusal, and:
/// [`ControlError::MultipleSchemas`] or [`ControlError::UnsupportedSchema`]
/// if the model binds no single known release;
/// [`ControlError::UnknownEntity`] if `owner_history` is neither in the
/// model nor staged; [`ControlError::AuthoringInvalid`] if it is not an
/// `IfcOwnerHistory`. Nothing is staged on an error.
pub fn create_control_with_owner_history(
    tx: &mut Transaction,
    model: &Model,
    kind: ControlKind,
    global_id: &str,
    predefined_type: Option<&str>,
    draft: ControlDraft<'_>,
    owner_history: EntityId,
) -> ControlResult<EntityId> {
    let release = bind(model)?;
    let record = control_record(
        release,
        kind,
        global_id,
        predefined_type,
        draft,
        Value::Ref(owner_history),
    )?;
    release.require_owner_history(tx, model, kind.type_name(), owner_history)?;
    Ok(tx.create(record))
}

/// Validate a draft and lay its record out in `release`.
fn control_record(
    release: Release<'_>,
    kind: ControlKind,
    global_id: &str,
    predefined_type: Option<&str>,
    draft: ControlDraft<'_>,
    owner_history: Value,
) -> ControlResult<Entity> {
    let entity = kind.type_name();
    if release.schema().attributes(entity).is_empty() {
        return Err(ControlError::UnsupportedEntity {
            schema: release.schema().name().to_owned(),
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

    // Placed by name from the release's own table: IFC2X3 declares six
    // attributes for a permit, where IFC4 declares nine. Indexing by the
    // IFC4 positions is what panicked (#198).
    let mut values = vec![
        ("GlobalId", Value::Text(global_id.into())),
        ("OwnerHistory", owner_history),
        ("Name", text(draft.name)),
        ("Description", text(draft.description)),
        ("ObjectType", text(draft.object_type)),
        ("Identification", text(draft.identification)),
        (
            "PredefinedType",
            predefined_type.map_or(Value::Null, |t| Value::Enum(t.into())),
        ),
    ];
    if history {
        values.push(("LifeCyclePhase", text(draft.life_cycle_phase)));
    } else {
        values.push(("Status", text(draft.status)));
        values.push(("LongDescription", text(draft.long_description)));
    }
    release.record(entity, values)
}
