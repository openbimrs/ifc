use ifc_model::{Entity, EntityId, Model, Transaction, Value};

use super::draft::{
    CostItemDraft, CostItemType, CostScheduleDraft, CostScheduleType, NestingDraft,
    ScheduleAssignmentDraft,
};
use super::validate::{guid, invalid, non_empty_unique, reference_type, validate_nesting};
use super::value::{optional_enum, optional_text, refs};
use super::CostAuthoringResult;

/// Validate and stage one selected IFC4 cost item.
///
/// All references may target the model or entities staged earlier in `tx`.
pub fn create_cost_item(
    tx: &mut Transaction,
    model: &Model,
    draft: CostItemDraft<'_>,
) -> CostAuthoringResult<EntityId> {
    guid(tx, model, "IFCCOSTITEM", draft.global_id)?;
    if draft.predefined_type == Some(CostItemType::UserDefined) && draft.object_type.is_none() {
        return Err(invalid(
            "IFCCOSTITEM",
            "ObjectType",
            "required for USERDEFINED PredefinedType",
        ));
    }
    for target in draft.cost_values {
        reference_type(
            tx,
            model,
            "IFCCOSTITEM",
            "CostValues",
            *target,
            "IFCCOSTVALUE",
        )?;
    }
    Ok(tx.create(Entity::new(
        "IFCCOSTITEM",
        vec![
            Value::Text(draft.global_id.into()),
            Value::Null,
            optional_text(draft.name),
            optional_text(draft.description),
            optional_text(draft.object_type),
            optional_text(draft.identification),
            optional_enum(draft.predefined_type.map(CostItemType::token)),
            optional_refs(draft.cost_values),
            Value::Null,
        ],
    )))
}

/// Validate and stage one IFC4 cost schedule.
pub fn create_cost_schedule(
    tx: &mut Transaction,
    model: &Model,
    draft: CostScheduleDraft<'_>,
) -> CostAuthoringResult<EntityId> {
    guid(tx, model, "IFCCOSTSCHEDULE", draft.global_id)?;
    if draft.predefined_type == Some(CostScheduleType::UserDefined) && draft.object_type.is_none() {
        return Err(invalid(
            "IFCCOSTSCHEDULE",
            "ObjectType",
            "required for USERDEFINED PredefinedType",
        ));
    }
    Ok(tx.create(Entity::new(
        "IFCCOSTSCHEDULE",
        vec![
            Value::Text(draft.global_id.into()),
            Value::Null,
            optional_text(draft.name),
            optional_text(draft.description),
            optional_text(draft.object_type),
            optional_text(draft.identification),
            optional_enum(draft.predefined_type.map(CostScheduleType::token)),
            optional_text(draft.status),
            optional_text(draft.submitted_on),
            optional_text(draft.update_date),
        ],
    )))
}

/// Validate and stage ordered cost-item nesting.
///
/// Self-reference, duplicate children, second parents, and projected cycles are refused.
pub fn nest_cost_items(
    tx: &mut Transaction,
    model: &Model,
    draft: NestingDraft<'_>,
) -> CostAuthoringResult<EntityId> {
    guid(tx, model, "IFCRELNESTS", draft.global_id)?;
    non_empty_unique("IFCRELNESTS", "RelatedObjects", draft.children)?;
    reference_type(
        tx,
        model,
        "IFCRELNESTS",
        "RelatingObject",
        draft.parent,
        "IFCCOSTITEM",
    )?;
    for child in draft.children {
        reference_type(
            tx,
            model,
            "IFCRELNESTS",
            "RelatedObjects",
            *child,
            "IFCCOSTITEM",
        )?;
    }
    validate_nesting(tx, model, draft.parent, draft.children)?;
    Ok(tx.create(Entity::new(
        "IFCRELNESTS",
        root_relation(
            draft.global_id,
            Value::Ref(draft.parent),
            refs(draft.children),
        ),
    )))
}

/// Validate and stage a schedule-to-items `IfcRelAssignsToControl`.
///
/// `RelatedObjectsType` is authored as `.CONTROL.` because every related target
/// is required to be an exact `IfcCostItem`.
pub fn assign_schedule_items(
    tx: &mut Transaction,
    model: &Model,
    draft: ScheduleAssignmentDraft<'_>,
) -> CostAuthoringResult<EntityId> {
    guid(tx, model, "IFCRELASSIGNSTOCONTROL", draft.global_id)?;
    non_empty_unique("IFCRELASSIGNSTOCONTROL", "RelatedObjects", draft.items)?;
    reference_type(
        tx,
        model,
        "IFCRELASSIGNSTOCONTROL",
        "RelatingControl",
        draft.schedule,
        "IFCCOSTSCHEDULE",
    )?;
    for item in draft.items {
        reference_type(
            tx,
            model,
            "IFCRELASSIGNSTOCONTROL",
            "RelatedObjects",
            *item,
            "IFCCOSTITEM",
        )?;
    }
    let mut attributes = root_prefix(draft.global_id);
    attributes.extend([
        refs(draft.items),
        Value::Enum("CONTROL".into()),
        Value::Ref(draft.schedule),
    ]);
    Ok(tx.create(Entity::new("IFCRELASSIGNSTOCONTROL", attributes)))
}

fn root_prefix(global_id: &str) -> Vec<Value> {
    vec![
        Value::Text(global_id.into()),
        Value::Null,
        Value::Null,
        Value::Null,
    ]
}
fn root_relation(global_id: &str, relating: Value, related: Value) -> Vec<Value> {
    let mut attributes = root_prefix(global_id);
    attributes.extend([relating, related]);
    attributes
}
fn optional_refs(ids: &[EntityId]) -> Value {
    if ids.is_empty() {
        Value::Null
    } else {
        refs(ids)
    }
}
