//! Transaction-staged authoring for bounded IFC4 constraints.

use std::collections::HashSet;
use std::sync::Arc;

use ifc_model::guid::Guid;
use ifc_model::{Edit, Entity, EntityId, Model, Transaction, Value};

use crate::types::{
    Benchmark, ConstraintGrade, LogicalOperator, MetricValueDraft, ObjectiveQualifier,
};
use crate::{ConstraintError, ConstraintResult};

const METRIC: &str = "IFCMETRIC";
const OBJECTIVE: &str = "IFCOBJECTIVE";
const RESOURCE_REL: &str = "IFCRESOURCECONSTRAINTRELATIONSHIP";
const ASSIGNMENT: &str = "IFCRELASSOCIATESCONSTRAINT";

/// Common inherited `IfcConstraint` fields.
#[derive(Debug, Clone, Copy)]
pub struct ConstraintBaseDraft<'a> {
    /// Required constraint name.
    pub name: &'a str,
    /// Optional description.
    pub description: Option<&'a str>,
    /// Typed constraint grade.
    pub grade: ConstraintGrade,
    /// Optional source label.
    pub source: Option<&'a str>,
    /// Optional existing or earlier-staged actor-select target.
    pub creating_actor: Option<EntityId>,
    /// Optional IFC date-time lexical value.
    pub creation_time: Option<&'a str>,
    /// Required when `grade` is user-defined.
    pub user_defined_grade: Option<&'a str>,
}

/// Draft for one `IfcMetric`.
#[derive(Debug, Clone, Copy)]
pub struct MetricDraft<'a> {
    /// Inherited constraint fields.
    pub base: ConstraintBaseDraft<'a>,
    /// Comparison benchmark.
    pub benchmark: Benchmark,
    /// Optional source label for the data value.
    pub value_source: Option<&'a str>,
    /// Optional preserved metric SELECT value.
    pub data_value: Option<MetricValueDraft<'a>>,
    /// Optional existing or earlier-staged `IfcReference`.
    pub reference_path: Option<EntityId>,
}

/// Draft for one `IfcObjective`.
#[derive(Debug, Clone, Copy)]
pub struct ObjectiveDraft<'a> {
    /// Inherited constraint fields.
    pub base: ConstraintBaseDraft<'a>,
    /// Optional non-empty ordered constraints.
    pub benchmark_values: Option<&'a [EntityId]>,
    /// Optional logical operator.
    pub logical_aggregator: Option<LogicalOperator>,
    /// Objective purpose qualifier.
    pub qualifier: ObjectiveQualifier,
    /// Required when `qualifier` is user-defined.
    pub user_defined_qualifier: Option<&'a str>,
}

/// Draft for one resource-level constraint relationship.
#[derive(Debug, Clone, Copy)]
pub struct ResourceConstraintDraft<'a> {
    /// Optional relationship name.
    pub name: Option<&'a str>,
    /// Optional relationship description.
    pub description: Option<&'a str>,
    /// Existing or earlier-staged metric/objective.
    pub relating_constraint: EntityId,
    /// Non-empty unique resource-select targets.
    pub related_resources: &'a [EntityId],
}

/// Draft for one rooted constraint association.
#[derive(Debug, Clone, Copy)]
pub struct ConstraintAssociationDraft<'a> {
    /// Compressed IFC GlobalId.
    pub global_id: &'a str,
    /// Optional relationship name.
    pub name: Option<&'a str>,
    /// Optional relationship description.
    pub description: Option<&'a str>,
    /// Non-empty unique definition-select targets.
    pub related_objects: &'a [EntityId],
    /// Optional association intent.
    pub intent: Option<&'a str>,
    /// Existing or earlier-staged metric/objective.
    pub relating_constraint: EntityId,
}

/// Validate and stage one metric.
pub fn create_metric(
    tx: &mut Transaction,
    model: &Model,
    draft: MetricDraft<'_>,
) -> ConstraintResult<EntityId> {
    validate_base(tx, model, METRIC, draft.base)?;
    if let Some(path) = draft.reference_path {
        validate_target(tx, model, path, "IfcReference")?;
    }
    let data_value = match draft.data_value {
        None => Value::Null,
        Some(MetricValueDraft::Entity(target)) => {
            validate_target(tx, model, target, "IfcMetricValueSelect")?;
            Value::Ref(target)
        }
        Some(MetricValueDraft::Typed { type_name, value }) => {
            if !ifc_schema::ifc4().accepts_type("IfcMetricValueSelect", type_name) {
                return Err(ConstraintError::AuthoringInvalid {
                    entity: METRIC,
                    attribute: "DataValue",
                    value: format!("type {type_name} is outside IfcMetricValueSelect"),
                });
            }
            Value::Typed {
                type_name: Arc::from(type_name.to_ascii_uppercase()),
                value: Box::new(value.clone()),
            }
        }
    };
    Ok(tx.create(Entity::new(
        METRIC,
        vec![
            text(draft.base.name),
            optional_text(draft.base.description),
            enumeration(draft.base.grade.token()),
            optional_text(draft.base.source),
            optional_ref(draft.base.creating_actor),
            optional_text(draft.base.creation_time),
            optional_text(draft.base.user_defined_grade),
            enumeration(draft.benchmark.token()),
            optional_text(draft.value_source),
            data_value,
            optional_ref(draft.reference_path),
        ],
    )))
}

/// Validate and stage one objective.
pub fn create_objective(
    tx: &mut Transaction,
    model: &Model,
    draft: ObjectiveDraft<'_>,
) -> ConstraintResult<EntityId> {
    validate_base(tx, model, OBJECTIVE, draft.base)?;
    if draft.qualifier == ObjectiveQualifier::UserDefined && draft.user_defined_qualifier.is_none()
    {
        return Err(ConstraintError::AuthoringInvalid {
            entity: OBJECTIVE,
            attribute: "WR21",
            value: "USERDEFINED qualifier requires UserDefinedQualifier".into(),
        });
    }
    let benchmarks = match draft.benchmark_values {
        None => Value::Null,
        Some([]) => {
            return Err(ConstraintError::AuthoringInvalid {
                entity: OBJECTIVE,
                attribute: "BenchmarkValues",
                value: "empty LIST [1:?]".into(),
            });
        }
        Some(values) => {
            for &target in values {
                validate_target(tx, model, target, "IfcConstraint")?;
            }
            refs(values)
        }
    };
    Ok(tx.create(Entity::new(
        OBJECTIVE,
        vec![
            text(draft.base.name),
            optional_text(draft.base.description),
            enumeration(draft.base.grade.token()),
            optional_text(draft.base.source),
            optional_ref(draft.base.creating_actor),
            optional_text(draft.base.creation_time),
            optional_text(draft.base.user_defined_grade),
            benchmarks,
            draft
                .logical_aggregator
                .map_or(Value::Null, |value| enumeration(value.token())),
            enumeration(draft.qualifier.token()),
            optional_text(draft.user_defined_qualifier),
        ],
    )))
}

/// Validate and stage one resource-level constraint relationship.
pub fn relate_resource_constraint(
    tx: &mut Transaction,
    model: &Model,
    draft: ResourceConstraintDraft<'_>,
) -> ConstraintResult<EntityId> {
    validate_target(tx, model, draft.relating_constraint, "IfcConstraint")?;
    validate_set(
        tx,
        model,
        RESOURCE_REL,
        "RelatedResourceObjects",
        draft.related_resources,
        "IfcResourceObjectSelect",
    )?;
    Ok(tx.create(Entity::new(
        RESOURCE_REL,
        vec![
            optional_text(draft.name),
            optional_text(draft.description),
            Value::Ref(draft.relating_constraint),
            refs(draft.related_resources),
        ],
    )))
}

/// Validate and stage one rooted constraint association.
pub fn associate_constraint(
    tx: &mut Transaction,
    model: &Model,
    draft: ConstraintAssociationDraft<'_>,
) -> ConstraintResult<EntityId> {
    if Guid::parse(draft.global_id).is_none() {
        return Err(ConstraintError::AuthoringInvalid {
            entity: ASSIGNMENT,
            attribute: "GlobalId",
            value: draft.global_id.into(),
        });
    }
    validate_target(tx, model, draft.relating_constraint, "IfcConstraint")?;
    validate_set(
        tx,
        model,
        ASSIGNMENT,
        "RelatedObjects",
        draft.related_objects,
        "IfcDefinitionSelect",
    )?;
    Ok(tx.create(Entity::new(
        ASSIGNMENT,
        vec![
            text(draft.global_id),
            Value::Null,
            optional_text(draft.name),
            optional_text(draft.description),
            refs(draft.related_objects),
            optional_text(draft.intent),
            Value::Ref(draft.relating_constraint),
        ],
    )))
}

fn validate_base(
    tx: &Transaction,
    model: &Model,
    kind: &'static str,
    draft: ConstraintBaseDraft<'_>,
) -> ConstraintResult<()> {
    if draft.grade == ConstraintGrade::UserDefined && draft.user_defined_grade.is_none() {
        return Err(ConstraintError::AuthoringInvalid {
            entity: kind,
            attribute: "WR11",
            value: "USERDEFINED grade requires UserDefinedGrade".into(),
        });
    }
    if let Some(actor) = draft.creating_actor {
        validate_target(tx, model, actor, "IfcActorSelect")?;
    }
    Ok(())
}

fn validate_set(
    tx: &Transaction,
    model: &Model,
    kind: &'static str,
    attribute: &'static str,
    targets: &[EntityId],
    expected: &'static str,
) -> ConstraintResult<()> {
    if targets.is_empty() {
        return Err(ConstraintError::AuthoringInvalid {
            entity: kind,
            attribute,
            value: "empty SET [1:?]".into(),
        });
    }
    let mut seen = HashSet::new();
    for &target in targets {
        if !seen.insert(target) {
            return Err(ConstraintError::AuthoringInvalid {
                entity: kind,
                attribute,
                value: format!("duplicate {target}"),
            });
        }
        validate_target(tx, model, target, expected)?;
    }
    Ok(())
}

fn validate_target(
    tx: &Transaction,
    model: &Model,
    target: EntityId,
    expected: &'static str,
) -> ConstraintResult<()> {
    let actual =
        final_type(tx, model, target).ok_or(ConstraintError::UnknownEntity { id: target })?;
    if ifc_schema::ifc4().accepts_type(expected, actual) {
        Ok(())
    } else {
        Err(ConstraintError::AuthoringReferenceType {
            target,
            expected,
            actual: actual.into(),
        })
    }
}

fn final_type<'a>(tx: &'a Transaction, model: &'a Model, id: EntityId) -> Option<&'a str> {
    for edit in tx.edits().iter().rev() {
        match edit {
            Edit::Create {
                id: edit_id,
                entity,
            } if *edit_id == id => return Some(&entity.type_name),
            Edit::Remove { id: edit_id } if *edit_id == id => return None,
            Edit::Retype {
                id: edit_id,
                type_name,
            } if *edit_id == id => return Some(type_name),
            _ => {}
        }
    }
    model.get(id).map(|entity| entity.type_name.as_ref())
}

fn text(value: &str) -> Value {
    Value::Text(Arc::from(value))
}
fn optional_text(value: Option<&str>) -> Value {
    value.map_or(Value::Null, text)
}
fn optional_ref(value: Option<EntityId>) -> Value {
    value.map_or(Value::Null, Value::Ref)
}
fn enumeration(value: &str) -> Value {
    Value::Enum(Arc::from(value))
}
fn refs(values: &[EntityId]) -> Value {
    Value::List(values.iter().copied().map(Value::Ref).collect())
}
