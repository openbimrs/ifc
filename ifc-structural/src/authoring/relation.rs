use std::collections::HashSet;

use ifc_model::guid::Guid;
use ifc_model::{Edit, EntityId, Model, Transaction, Value};
use ifc_schema::Schema;

use super::{
    build_named, optional_ref, optional_text, projected_entity, validate_optional_ref, validate_ref,
};
use crate::error::{StructuralError, StructuralResult};

#[derive(Debug, Clone)]
pub struct RelationshipRootDraft {
    pub global_id: String,
    pub owner_history: Option<EntityId>,
    pub name: Option<String>,
    pub description: Option<String>,
}

impl Default for RelationshipRootDraft {
    fn default() -> Self {
        Self {
            global_id: "0000000000000000000000".into(),
            owner_history: None,
            name: None,
            description: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MemberConnectionDraft {
    pub root: RelationshipRootDraft,
    pub member: EntityId,
    pub connection: EntityId,
    pub applied_condition: Option<EntityId>,
    pub additional_conditions: Option<EntityId>,
    pub supported_length: Option<f64>,
    pub condition_coordinate_system: Option<EntityId>,
}

#[derive(Debug, Clone)]
pub struct ActivityAssignmentDraft {
    pub root: RelationshipRootDraft,
    pub relating_element: EntityId,
    pub activity: EntityId,
}

fn validate_root(
    tx: &Transaction,
    model: &Model,
    schema: &Schema,
    root: &RelationshipRootDraft,
) -> StructuralResult<()> {
    if Guid::parse(&root.global_id).is_none() {
        return Err(StructuralError::InvalidGlobalId);
    }
    validate_optional_ref(tx, model, schema, root.owner_history, "IfcOwnerHistory")
}

fn root_fields(root: RelationshipRootDraft) -> Vec<(&'static str, Value)> {
    vec![
        ("GlobalId", Value::Text(root.global_id.into())),
        ("OwnerHistory", optional_ref(root.owner_history)),
        ("Name", optional_text(root.name)),
        ("Description", optional_text(root.description)),
    ]
}

pub fn stage_member_connection(
    tx: &mut Transaction,
    model: &Model,
    schema: &Schema,
    draft: MemberConnectionDraft,
) -> StructuralResult<EntityId> {
    const ENTITY: &str = "IfcRelConnectsStructuralMember";
    validate_root(tx, model, schema, &draft.root)?;
    validate_ref(tx, model, schema, draft.member, "IfcStructuralMember")?;
    validate_ref(
        tx,
        model,
        schema,
        draft.connection,
        "IfcStructuralConnection",
    )?;
    validate_optional_ref(
        tx,
        model,
        schema,
        draft.applied_condition,
        "IfcBoundaryCondition",
    )?;
    validate_optional_ref(
        tx,
        model,
        schema,
        draft.additional_conditions,
        "IfcStructuralConnectionCondition",
    )?;
    validate_optional_ref(
        tx,
        model,
        schema,
        draft.condition_coordinate_system,
        "IfcAxis2Placement3D",
    )?;
    if draft
        .supported_length
        .is_some_and(|value| !value.is_finite() || value <= 0.0)
    {
        return Err(StructuralError::InvalidDraftValue {
            entity_type: ENTITY,
            attribute: "SupportedLength",
            expected: "positive finite length or null",
        });
    }
    let mut fields = root_fields(draft.root);
    fields.extend([
        ("RelatingStructuralMember", Value::Ref(draft.member)),
        ("RelatedStructuralConnection", Value::Ref(draft.connection)),
        ("AppliedCondition", optional_ref(draft.applied_condition)),
        (
            "AdditionalConditions",
            optional_ref(draft.additional_conditions),
        ),
        (
            "SupportedLength",
            draft.supported_length.map_or(Value::Null, Value::Real),
        ),
        (
            "ConditionCoordinateSystem",
            optional_ref(draft.condition_coordinate_system),
        ),
    ]);
    Ok(tx.create(build_named(schema, ENTITY, fields)?))
}

pub fn stage_activity_assignment(
    tx: &mut Transaction,
    model: &Model,
    schema: &Schema,
    draft: ActivityAssignmentDraft,
) -> StructuralResult<EntityId> {
    const ENTITY: &str = "IfcRelConnectsStructuralActivity";
    validate_root(tx, model, schema, &draft.root)?;
    let relating = projected_entity(tx, model, draft.relating_element).ok_or(
        StructuralError::DanglingReference {
            entity: EntityId(0),
            attribute: "draft reference",
            target: draft.relating_element,
        },
    )?;
    if !schema.accepts_type("IfcStructuralActivityAssignmentSelect", &relating.type_name) {
        return Err(StructuralError::WrongReferenceType {
            entity: EntityId(0),
            attribute: "draft reference",
            target: draft.relating_element,
            expected: "IfcStructuralActivityAssignmentSelect",
            actual: relating.type_name.to_string(),
        });
    }
    validate_ref(tx, model, schema, draft.activity, "IfcStructuralActivity")?;
    if has_activity_attachment(tx, model, schema, draft.activity) {
        return Err(StructuralError::SemanticViolation {
            entity: Some(draft.activity),
            rule: "IfcStructuralActivity.AssignedToStructuralItem SET [0:1]",
        });
    }
    let mut fields = root_fields(draft.root);
    fields.extend([
        ("RelatingElement", Value::Ref(draft.relating_element)),
        ("RelatedStructuralActivity", Value::Ref(draft.activity)),
    ]);
    Ok(tx.create(build_named(schema, ENTITY, fields)?))
}

fn has_activity_attachment(
    tx: &Transaction,
    model: &Model,
    schema: &Schema,
    activity: EntityId,
) -> bool {
    let mut ids: HashSet<EntityId> = model.iter().map(|(id, _)| id).collect();
    for edit in tx.edits() {
        if let Edit::Create { id, .. } = edit {
            ids.insert(*id);
        }
    }
    ids.into_iter().any(|id| {
        let Some(entity) = projected_entity(tx, model, id) else { return false; };
        if !schema.is_a(&entity.type_name, "IfcRelConnectsStructuralActivity") { return false; }
        let Some(slot) = schema.attributes(&entity.type_name).iter()
            .position(|attribute| attribute.name.eq_ignore_ascii_case("RelatedStructuralActivity"))
        else { return false; };
        matches!(entity.attribute(slot).map(Value::unwrap_typed), Some(Value::Ref(target)) if *target == activity)
    })
}
