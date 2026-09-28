//! Cost items, schedules and their relationships, laid out by name in the
//! model's declared release (#202).
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
//! The writers that leave it unset refuse an IFC2X3 model with
//! [`CostAuthoringError::AuthoringRequired`] instead of writing `$`. The
//! `*_with_owner_history` variants take a caller-supplied `IfcOwnerHistory`
//! (in the model or staged on the transaction); one is never invented here.
//! This follows `ifc-material` (#77), `ifc-properties` (#191) and
//! `ifc-classification` (#194).
//!
//! IFC2X3 also declares less: `IfcCostItem` has only the five `IfcControl`
//! attributes (no `Identification`, `PredefinedType`, `CostValues` or
//! `CostQuantities`), and `IfcCostSchedule` requires `ID` and
//! `PredefinedType` and types its dates as `IfcDateTimeSelect` records,
//! which [`CostScheduleDraft`] carries as [`DateTimeValue`] record forms and
//! the writer stages (#214). A draft value the release cannot hold is
//! refused, never dropped.
//!
//! [`CostAuthoringError::AuthoringRequired`]: super::CostAuthoringError::AuthoringRequired

use ifc_model::{EntityId, Model, Transaction, Value};

use super::datetime::{patch, DateTimeValue, PLACEHOLDERS};
use super::draft::{
    CostItemDraft, CostItemType, CostScheduleDraft, CostScheduleType, NestingDraft,
    ScheduleAssignmentDraft,
};
use super::release::{bind, Release};
use super::validate::{guid, invalid, non_empty_unique, reference_type, validate_nesting};
use super::value::{optional_enum, optional_text, refs};
use super::CostAuthoringResult;

/// Validate and stage one cost item in the model's declared release, with
/// `OwnerHistory` unset.
///
/// All references may target the model or entities staged earlier in `tx`.
///
/// # Errors
///
/// A malformed or duplicate GlobalId; `USERDEFINED` without `ObjectType`; a
/// cost value that is not an `IfcCostValue`; a header binding no single
/// known release (`MultipleSchemas`, `UnsupportedSchema`); a value the
/// release does not declare (`AuthoringNotInSchema`: IFC2X3 has no
/// `Identification`, `PredefinedType` or `CostValues`); and an IFC2X3
/// model, which requires `OwnerHistory` (`AuthoringRequired`: use
/// [`create_cost_item_with_owner_history`]). Nothing is staged on an error.
pub fn create_cost_item(
    tx: &mut Transaction,
    model: &Model,
    draft: CostItemDraft<'_>,
) -> CostAuthoringResult<EntityId> {
    cost_item(tx, model, draft, None)
}

/// [`create_cost_item`] with a caller-supplied `IfcOwnerHistory`, which
/// IFC2X3 requires.
///
/// # Errors
///
/// Those of [`create_cost_item`] except the IFC2X3 refusal, and an
/// `owner_history` that does not resolve (`MissingReference`) or is not an
/// `IfcOwnerHistory` (`WrongReferenceType`).
pub fn create_cost_item_with_owner_history(
    tx: &mut Transaction,
    model: &Model,
    draft: CostItemDraft<'_>,
    owner_history: EntityId,
) -> CostAuthoringResult<EntityId> {
    cost_item(tx, model, draft, Some(owner_history))
}

fn cost_item(
    tx: &mut Transaction,
    model: &Model,
    draft: CostItemDraft<'_>,
    owner_history: Option<EntityId>,
) -> CostAuthoringResult<EntityId> {
    const ENTITY: &str = "IFCCOSTITEM";
    let release = bind(model)?;
    guid(tx, model, ENTITY, draft.global_id)?;
    if draft.predefined_type == Some(CostItemType::UserDefined) && draft.object_type.is_none() {
        return Err(invalid(
            ENTITY,
            "ObjectType",
            "required for USERDEFINED PredefinedType",
        ));
    }
    for target in draft.cost_values {
        reference_type(tx, model, ENTITY, "CostValues", *target, "IFCCOSTVALUE")?;
    }
    let record = release.record(
        ENTITY,
        vec![
            ("GlobalId", Value::Text(draft.global_id.into())),
            (
                "OwnerHistory",
                owner_history.map_or(Value::Null, Value::Ref),
            ),
            ("Name", optional_text(draft.name)),
            ("Description", optional_text(draft.description)),
            ("ObjectType", optional_text(draft.object_type)),
            ("Identification", optional_text(draft.identification)),
            (
                "PredefinedType",
                optional_enum(draft.predefined_type.map(CostItemType::token)),
            ),
            ("CostValues", optional_refs(draft.cost_values)),
        ],
    )?;
    stage(tx, model, release, ENTITY, record, owner_history)
}

/// Validate and stage one cost schedule in the model's declared release,
/// with `OwnerHistory` unset.
///
/// # Errors
///
/// A malformed or duplicate GlobalId; `USERDEFINED` without `ObjectType`;
/// a header binding no single known release; a date record form the
/// schema's rules refuse (`InvalidValue`: a month outside 1..=12, a day the
/// month does not have, an hour, minute or second out of range, a second
/// without a minute); a date in the form the release does not declare
/// (`AuthoringValueType`: IFC2X3 types `SubmittedOn` and `UpdateDate` as
/// `IfcDateTimeSelect` records, IFC4 and IFC4X3 as `IfcDateTime` text); a
/// required value left unset (`AuthoringRequired`: IFC2X3 requires `ID`,
/// written from `identification`, and `PredefinedType`); and an IFC2X3
/// model, which requires `OwnerHistory` (use
/// [`create_cost_schedule_with_owner_history`]). Nothing is staged on an
/// error: IFC2X3 date records are staged only once the schedule itself is
/// accepted.
pub fn create_cost_schedule(
    tx: &mut Transaction,
    model: &Model,
    draft: CostScheduleDraft<'_>,
) -> CostAuthoringResult<EntityId> {
    cost_schedule(tx, model, draft, None)
}

/// [`create_cost_schedule`] with a caller-supplied `IfcOwnerHistory`, which
/// IFC2X3 requires.
///
/// # Errors
///
/// Those of [`create_cost_schedule`] except the IFC2X3 owner-history
/// refusal, and the owner-history refusals of
/// [`create_cost_item_with_owner_history`].
pub fn create_cost_schedule_with_owner_history(
    tx: &mut Transaction,
    model: &Model,
    draft: CostScheduleDraft<'_>,
    owner_history: EntityId,
) -> CostAuthoringResult<EntityId> {
    cost_schedule(tx, model, draft, Some(owner_history))
}

fn cost_schedule(
    tx: &mut Transaction,
    model: &Model,
    draft: CostScheduleDraft<'_>,
    owner_history: Option<EntityId>,
) -> CostAuthoringResult<EntityId> {
    const ENTITY: &str = "IFCCOSTSCHEDULE";
    let release = bind(model)?;
    guid(tx, model, ENTITY, draft.global_id)?;
    if draft.predefined_type == Some(CostScheduleType::UserDefined) && draft.object_type.is_none() {
        return Err(invalid(
            ENTITY,
            "ObjectType",
            "required for USERDEFINED PredefinedType",
        ));
    }
    let dates = [draft.submitted_on, draft.update_date];
    for date in dates.iter().flatten() {
        date.check()?;
    }
    let provisional = |slot: usize| {
        dates[slot].map_or(Value::Null, |date: DateTimeValue<'_>| {
            date.provisional(PLACEHOLDERS[slot])
        })
    };
    let mut record = release.record(
        ENTITY,
        vec![
            ("GlobalId", Value::Text(draft.global_id.into())),
            (
                "OwnerHistory",
                owner_history.map_or(Value::Null, Value::Ref),
            ),
            ("Name", optional_text(draft.name)),
            ("Description", optional_text(draft.description)),
            ("ObjectType", optional_text(draft.object_type)),
            ("Identification", optional_text(draft.identification)),
            (
                "PredefinedType",
                optional_enum(draft.predefined_type.map(CostScheduleType::token)),
            ),
            ("Status", optional_text(draft.status)),
            ("SubmittedOn", provisional(0)),
            ("UpdateDate", provisional(1)),
        ],
    )?;
    // Every refusal before the first edit: the record with placeholders,
    // then the owner history; only then the date records.
    if let Some(owner_history) = owner_history {
        release.require_owner_history(tx, model, ENTITY, owner_history)?;
    }
    patch(tx, &mut record, &dates);
    Ok(tx.create(record))
}

/// Validate and stage ordered cost-item nesting, with `OwnerHistory` unset.
///
/// Self-reference, duplicate children, second parents, and projected cycles
/// are refused.
///
/// # Errors
///
/// Those refusals, a malformed or duplicate GlobalId, a parent or child
/// that is not an `IfcCostItem`, a header binding no single known release,
/// and an IFC2X3 model, which requires `OwnerHistory` (use
/// [`nest_cost_items_with_owner_history`]). Nothing is staged on an error.
pub fn nest_cost_items(
    tx: &mut Transaction,
    model: &Model,
    draft: NestingDraft<'_>,
) -> CostAuthoringResult<EntityId> {
    nesting(tx, model, draft, None)
}

/// [`nest_cost_items`] with a caller-supplied `IfcOwnerHistory`, which
/// IFC2X3 requires.
///
/// # Errors
///
/// Those of [`nest_cost_items`] except the IFC2X3 refusal, and the
/// owner-history refusals of [`create_cost_item_with_owner_history`].
pub fn nest_cost_items_with_owner_history(
    tx: &mut Transaction,
    model: &Model,
    draft: NestingDraft<'_>,
    owner_history: EntityId,
) -> CostAuthoringResult<EntityId> {
    nesting(tx, model, draft, Some(owner_history))
}

fn nesting(
    tx: &mut Transaction,
    model: &Model,
    draft: NestingDraft<'_>,
    owner_history: Option<EntityId>,
) -> CostAuthoringResult<EntityId> {
    const ENTITY: &str = "IFCRELNESTS";
    let release = bind(model)?;
    guid(tx, model, ENTITY, draft.global_id)?;
    non_empty_unique(ENTITY, "RelatedObjects", draft.children)?;
    reference_type(
        tx,
        model,
        ENTITY,
        "RelatingObject",
        draft.parent,
        "IFCCOSTITEM",
    )?;
    for child in draft.children {
        reference_type(tx, model, ENTITY, "RelatedObjects", *child, "IFCCOSTITEM")?;
    }
    validate_nesting(tx, model, draft.parent, draft.children)?;
    let record = release.record(
        ENTITY,
        vec![
            ("GlobalId", Value::Text(draft.global_id.into())),
            (
                "OwnerHistory",
                owner_history.map_or(Value::Null, Value::Ref),
            ),
            ("RelatingObject", Value::Ref(draft.parent)),
            ("RelatedObjects", refs(draft.children)),
        ],
    )?;
    stage(tx, model, release, ENTITY, record, owner_history)
}

/// Validate and stage a schedule-to-items `IfcRelAssignsToControl`, with
/// `OwnerHistory` unset.
///
/// `RelatedObjectsType` is left unset. IFC4 redeclares it as
/// `OPTIONAL IfcStrippedOptional`, a BOOLEAN retained only so older
/// files still parse; the IFC2x3 reading, where it named the common
/// type of the related objects, no longer applies. Writing `.CONTROL.`
/// there put an enumeration token in a boolean slot -- a file that
/// parses and is wrong. The constraint that every target is an exact
/// `IfcCostItem` is still enforced, by the reference check below.
///
/// # Errors
///
/// A malformed or duplicate GlobalId; an empty or duplicated item list; a
/// schedule that is not an `IfcCostSchedule` or an item that is not an
/// `IfcCostItem`; a header binding no single known release; and an IFC2X3
/// model, which requires `OwnerHistory` (use
/// [`assign_schedule_items_with_owner_history`]). Nothing is staged on an
/// error.
pub fn assign_schedule_items(
    tx: &mut Transaction,
    model: &Model,
    draft: ScheduleAssignmentDraft<'_>,
) -> CostAuthoringResult<EntityId> {
    schedule_assignment(tx, model, draft, None)
}

/// [`assign_schedule_items`] with a caller-supplied `IfcOwnerHistory`,
/// which IFC2X3 requires.
///
/// # Errors
///
/// Those of [`assign_schedule_items`] except the IFC2X3 refusal, and the
/// owner-history refusals of [`create_cost_item_with_owner_history`].
pub fn assign_schedule_items_with_owner_history(
    tx: &mut Transaction,
    model: &Model,
    draft: ScheduleAssignmentDraft<'_>,
    owner_history: EntityId,
) -> CostAuthoringResult<EntityId> {
    schedule_assignment(tx, model, draft, Some(owner_history))
}

fn schedule_assignment(
    tx: &mut Transaction,
    model: &Model,
    draft: ScheduleAssignmentDraft<'_>,
    owner_history: Option<EntityId>,
) -> CostAuthoringResult<EntityId> {
    const ENTITY: &str = "IFCRELASSIGNSTOCONTROL";
    let release = bind(model)?;
    guid(tx, model, ENTITY, draft.global_id)?;
    non_empty_unique(ENTITY, "RelatedObjects", draft.items)?;
    reference_type(
        tx,
        model,
        ENTITY,
        "RelatingControl",
        draft.schedule,
        "IFCCOSTSCHEDULE",
    )?;
    for item in draft.items {
        reference_type(tx, model, ENTITY, "RelatedObjects", *item, "IFCCOSTITEM")?;
    }
    let record = release.record(
        ENTITY,
        vec![
            ("GlobalId", Value::Text(draft.global_id.into())),
            (
                "OwnerHistory",
                owner_history.map_or(Value::Null, Value::Ref),
            ),
            ("RelatedObjects", refs(draft.items)),
            // RelatedObjectsType: stripped in IFC4, deliberately unset.
            ("RelatedObjectsType", Value::Null),
            ("RelatingControl", Value::Ref(draft.schedule)),
        ],
    )?;
    stage(tx, model, release, ENTITY, record, owner_history)
}

/// Check the owner history, then stage `record`: every refusal comes
/// before the one edit.
fn stage(
    tx: &mut Transaction,
    model: &Model,
    release: Release,
    entity: &'static str,
    record: ifc_model::Entity,
    owner_history: Option<EntityId>,
) -> CostAuthoringResult<EntityId> {
    if let Some(owner_history) = owner_history {
        release.require_owner_history(tx, model, entity, owner_history)?;
    }
    Ok(tx.create(record))
}

fn optional_refs(ids: &[EntityId]) -> Value {
    if ids.is_empty() {
        Value::Null
    } else {
        refs(ids)
    }
}
