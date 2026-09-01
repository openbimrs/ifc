//! Selected schema-resolved structural authoring.

use std::collections::HashSet;
use std::sync::Arc;

use ifc_model::guid::Guid;
use ifc_model::{Edit, Entity, EntityId, Model, Transaction, Value};
use ifc_schema::Schema;

use crate::error::{StructuralError, StructuralResult};
use crate::AnalysisModelType;

mod action;
mod item;
mod relation;

pub use action::{stage_action, ActionDraft, ActionDraftKind, ProjectedOrTrue};
pub use item::{
    stage_connection, stage_member, ConnectionDraft, ConnectionDraftKind, MemberDraft,
    MemberDraftKind, MemberPredefinedType, StructuralRootDraft,
};
pub use relation::{
    stage_activity_assignment, stage_member_connection, ActivityAssignmentDraft,
    MemberConnectionDraft, RelationshipRootDraft,
};

#[derive(Debug, Clone)]
pub struct AnalysisModelDraft {
    pub global_id: String,
    pub owner_history: Option<EntityId>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub object_type: Option<String>,
    pub predefined_type: AnalysisModelType,
    pub orientation_of_2d_plane: Option<EntityId>,
    pub loaded_by: Vec<EntityId>,
    pub result_groups: Vec<EntityId>,
    pub shared_placement: Option<EntityId>,
}

impl Default for AnalysisModelDraft {
    fn default() -> Self {
        Self {
            global_id: "0000000000000000000000".to_owned(),
            owner_history: None,
            name: None,
            description: None,
            object_type: None,
            predefined_type: AnalysisModelType::NotDefined,
            orientation_of_2d_plane: None,
            loaded_by: Vec::new(),
            result_groups: Vec::new(),
            shared_placement: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum LoadDraft {
    SingleForce {
        name: Option<String>,
        force: [Option<f64>; 3],
        moment: [Option<f64>; 3],
    },
    LinearForce {
        name: Option<String>,
        force: [Option<f64>; 3],
        moment: [Option<f64>; 3],
    },
    PlanarForce {
        name: Option<String>,
        force: [Option<f64>; 3],
    },
    Temperature {
        name: Option<String>,
        delta: [Option<f64>; 3],
    },
}

pub fn stage_analysis_model(
    tx: &mut Transaction,
    model: &Model,
    schema: &Schema,
    draft: AnalysisModelDraft,
) -> StructuralResult<EntityId> {
    if Guid::parse(&draft.global_id).is_none() {
        return Err(StructuralError::InvalidGlobalId);
    }
    if draft.predefined_type == AnalysisModelType::UserDefined
        && draft
            .object_type
            .as_deref()
            .is_none_or(|value| value.trim().is_empty())
    {
        return Err(StructuralError::SemanticViolation {
            entity: None,
            rule: "USERDEFINED analysis model requires ObjectType",
        });
    }

    validate_unique_set_members(&draft.loaded_by, "IfcStructuralAnalysisModel", "LoadedBy")?;
    validate_unique_set_members(
        &draft.result_groups,
        "IfcStructuralAnalysisModel",
        "HasResults",
    )?;
    validate_optional_ref(tx, model, schema, draft.owner_history, "IfcOwnerHistory")?;
    validate_optional_ref(
        tx,
        model,
        schema,
        draft.orientation_of_2d_plane,
        "IfcAxis2Placement3D",
    )?;
    validate_refs(
        tx,
        model,
        schema,
        &draft.loaded_by,
        "IfcStructuralLoadGroup",
    )?;
    validate_refs(
        tx,
        model,
        schema,
        &draft.result_groups,
        "IfcStructuralResultGroup",
    )?;
    validate_optional_ref(
        tx,
        model,
        schema,
        draft.shared_placement,
        "IfcObjectPlacement",
    )?;

    let mut fields = vec![
        ("GlobalId", Value::Text(Arc::from(draft.global_id))),
        ("Name", optional_text(draft.name)),
        ("Description", optional_text(draft.description)),
        ("ObjectType", optional_text(draft.object_type)),
        (
            "PredefinedType",
            Value::Enum(Arc::from(draft.predefined_type.token())),
        ),
        (
            "OrientationOf2DPlane",
            optional_ref(draft.orientation_of_2d_plane),
        ),
        ("LoadedBy", optional_refs(draft.loaded_by)),
        ("HasResults", optional_refs(draft.result_groups)),
    ];
    if let Some(owner_history) = draft.owner_history {
        fields.push(("OwnerHistory", Value::Ref(owner_history)));
    }
    if schema
        .attribute_names("IfcStructuralAnalysisModel")
        .iter()
        .any(|name| name.eq_ignore_ascii_case("SharedPlacement"))
    {
        fields.push(("SharedPlacement", optional_ref(draft.shared_placement)));
    } else if draft.shared_placement.is_some() {
        return Err(StructuralError::UnsupportedAttribute {
            entity_type: "IfcStructuralAnalysisModel".to_owned(),
            attribute: "SharedPlacement".to_owned(),
        });
    }

    let entity = build_named(schema, "IfcStructuralAnalysisModel", fields)?;
    Ok(tx.create(entity))
}

pub fn stage_load(
    tx: &mut Transaction,
    schema: &Schema,
    draft: LoadDraft,
) -> StructuralResult<EntityId> {
    let (entity_type, name, attributes, values): (&str, Option<String>, &[&str], Vec<Option<f64>>) =
        match draft {
            LoadDraft::SingleForce {
                name,
                force,
                moment,
            } => (
                "IfcStructuralLoadSingleForce",
                name,
                &[
                    "ForceX", "ForceY", "ForceZ", "MomentX", "MomentY", "MomentZ",
                ],
                force.into_iter().chain(moment).collect(),
            ),
            LoadDraft::LinearForce {
                name,
                force,
                moment,
            } => (
                "IfcStructuralLoadLinearForce",
                name,
                &[
                    "LinearForceX",
                    "LinearForceY",
                    "LinearForceZ",
                    "LinearMomentX",
                    "LinearMomentY",
                    "LinearMomentZ",
                ],
                force.into_iter().chain(moment).collect(),
            ),
            LoadDraft::PlanarForce { name, force } => (
                "IfcStructuralLoadPlanarForce",
                name,
                &["PlanarForceX", "PlanarForceY", "PlanarForceZ"],
                force.to_vec(),
            ),
            LoadDraft::Temperature { name, delta } => {
                let attributes: &[&str] = if schema
                    .attribute_names("IfcStructuralLoadTemperature")
                    .iter()
                    .any(|attribute| attribute.eq_ignore_ascii_case("DeltaTConstant"))
                {
                    &["DeltaTConstant", "DeltaTY", "DeltaTZ"]
                } else {
                    &["DeltaT_Constant", "DeltaT_Y", "DeltaT_Z"]
                };
                (
                    "IfcStructuralLoadTemperature",
                    name,
                    attributes,
                    delta.to_vec(),
                )
            }
        };
    for (attribute, value) in attributes.iter().zip(&values) {
        if value.is_some_and(|number| !number.is_finite()) {
            return Err(StructuralError::InvalidDraftValue {
                entity_type,
                attribute,
                expected: "finite load value or null",
            });
        }
    }
    let mut fields = vec![("Name", optional_text(name))];
    fields.extend(
        attributes
            .iter()
            .zip(values)
            .map(|(name, value)| (*name, value.map_or(Value::Null, Value::Real))),
    );
    Ok(tx.create(build_named(schema, entity_type, fields)?))
}

pub(super) fn build_named(
    schema: &Schema,
    entity_type: &str,
    fields: Vec<(&str, Value)>,
) -> StructuralResult<Entity> {
    let attributes = schema.attributes(entity_type);
    if attributes.is_empty() {
        return Err(StructuralError::UnsupportedSchema {
            token: schema.name().to_owned(),
        });
    }
    let mut values = vec![Value::Null; attributes.len()];
    for (name, value) in fields {
        let slot = attributes
            .iter()
            .position(|attribute| attribute.name.eq_ignore_ascii_case(name))
            .ok_or_else(|| StructuralError::UnsupportedAttribute {
                entity_type: entity_type.to_owned(),
                attribute: name.to_owned(),
            })?;
        values[slot] = value;
    }
    for (attribute, value) in attributes.iter().zip(&values) {
        if !attribute.optional && matches!(value, Value::Null) {
            return Err(StructuralError::MissingRequired {
                entity_type: entity_type.to_owned(),
                attribute: attribute.name.clone(),
            });
        }
    }
    Ok(Entity::new(entity_type.to_ascii_uppercase(), values))
}

pub(super) fn optional_text(value: Option<String>) -> Value {
    value.map_or(Value::Null, |value| Value::Text(Arc::from(value)))
}

pub(super) fn optional_ref(value: Option<EntityId>) -> Value {
    value.map_or(Value::Null, Value::Ref)
}

fn optional_refs(values: Vec<EntityId>) -> Value {
    if values.is_empty() {
        Value::Null
    } else {
        Value::List(values.into_iter().map(Value::Ref).collect())
    }
}

pub(super) fn validate_optional_ref(
    tx: &Transaction,
    model: &Model,
    schema: &Schema,
    target: Option<EntityId>,
    expected: &'static str,
) -> StructuralResult<()> {
    if let Some(target) = target {
        validate_ref(tx, model, schema, target, expected)?;
    }
    Ok(())
}

fn validate_refs(
    tx: &Transaction,
    model: &Model,
    schema: &Schema,
    targets: &[EntityId],
    expected: &'static str,
) -> StructuralResult<()> {
    for target in targets {
        validate_ref(tx, model, schema, *target, expected)?;
    }
    Ok(())
}

fn validate_unique_set_members(
    targets: &[EntityId],
    entity_type: &'static str,
    attribute: &'static str,
) -> StructuralResult<()> {
    let mut unique = HashSet::with_capacity(targets.len());
    if targets.iter().all(|target| unique.insert(*target)) {
        return Ok(());
    }
    Err(StructuralError::InvalidDraftValue {
        entity_type,
        attribute,
        expected: "SET of unique entity references",
    })
}

pub(super) fn validate_ref(
    tx: &Transaction,
    model: &Model,
    schema: &Schema,
    target: EntityId,
    expected: &'static str,
) -> StructuralResult<()> {
    validate_ref_select(tx, model, schema, target, expected, &[expected])
}

pub(super) fn validate_ref_select(
    tx: &Transaction,
    model: &Model,
    schema: &Schema,
    target: EntityId,
    expected: &'static str,
    members: &[&str],
) -> StructuralResult<()> {
    let entity = projected_entity(tx, model, target).ok_or(StructuralError::DanglingReference {
        entity: EntityId(0),
        attribute: "draft reference",
        target,
    })?;
    if !members
        .iter()
        .any(|member| schema.is_a(&entity.type_name, member))
    {
        return Err(StructuralError::WrongReferenceType {
            entity: EntityId(0),
            attribute: "draft reference",
            target,
            expected,
            actual: entity.type_name.to_string(),
        });
    }
    Ok(())
}

pub(super) fn projected_entity(
    tx: &Transaction,
    model: &Model,
    target: EntityId,
) -> Option<Entity> {
    let mut projected = model.get(target).cloned();
    for edit in tx.edits() {
        match edit {
            Edit::Create { id, entity } if *id == target => projected = Some(entity.clone()),
            Edit::SetAttribute { id, slot, value } if *id == target => {
                let entity = projected.as_mut()?;
                let attribute = entity.attributes.get_mut(*slot)?;
                *attribute = value.clone();
            }
            Edit::Retype { id, type_name } if *id == target => {
                projected.as_mut()?.type_name = type_name.clone();
            }
            Edit::Remove { id } if *id == target => projected = None,
            _ => {}
        }
    }
    projected
}
