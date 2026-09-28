//! Transaction-staged authoring for bounded constraints.
//!
//! Every writer binds the model's declared release and lays its record out
//! by attribute name from that release's table (see `release.rs`, #212).
//! What the release cannot hold is refused, never dropped or written into a
//! slot that means something else. The rooted `IfcRelAssociatesConstraint`
//! lives in `association.rs`.

use std::collections::HashSet;
use std::sync::Arc;

use ifc_model::{Edit, Entity, EntityId, Model, Transaction, Value};
use ifc_schema::{Schema, TypeKind};

use crate::datetime::DateTimeInput;
use crate::release::{bind, Layout};

use crate::types::{
    Benchmark, ConstraintGrade, LogicalOperator, MetricValueDraft, ObjectiveQualifier,
};
use crate::{ConstraintError, ConstraintResult};

const METRIC: &str = "IFCMETRIC";
const OBJECTIVE: &str = "IFCOBJECTIVE";
const RESOURCE_REL: &str = "IFCRESOURCECONSTRAINTRELATIONSHIP";

/// Common inherited `IfcConstraint` fields.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
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
    /// Optional creation time: IFC4/IFC4X3 `IfcDateTime` text, or an
    /// existing or earlier-staged IFC2X3 `IfcDateTimeSelect` record.
    pub creation_time: Option<DateTimeInput<'a>>,
    /// Required when `grade` is user-defined.
    pub user_defined_grade: Option<&'a str>,
}

impl<'a> ConstraintBaseDraft<'a> {
    /// Starts a draft from its required fields; every other field is unset.
    #[must_use]
    pub fn new(name: &'a str, grade: ConstraintGrade) -> Self {
        Self {
            name,
            description: None,
            grade,
            source: None,
            creating_actor: None,
            creation_time: None,
            user_defined_grade: None,
        }
    }

    /// Sets `description`: Optional description.
    #[must_use]
    pub fn description(mut self, value: &'a str) -> Self {
        self.description = Some(value);
        self
    }

    /// Sets `source`: Optional source label.
    #[must_use]
    pub fn source(mut self, value: &'a str) -> Self {
        self.source = Some(value);
        self
    }

    /// Sets `creating_actor`: Optional existing or earlier-staged actor-select
    /// target.
    #[must_use]
    pub fn creating_actor(mut self, value: EntityId) -> Self {
        self.creating_actor = Some(value);
        self
    }

    /// Sets `creation_time`: text, or an IFC2X3 date record.
    #[must_use]
    pub fn creation_time(mut self, value: impl Into<DateTimeInput<'a>>) -> Self {
        self.creation_time = Some(value.into());
        self
    }

    /// Sets `user_defined_grade`: Required when `grade` is user-defined.
    #[must_use]
    pub fn user_defined_grade(mut self, value: &'a str) -> Self {
        self.user_defined_grade = Some(value);
        self
    }
}

/// Draft for one `IfcMetric`.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
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

impl<'a> MetricDraft<'a> {
    /// Starts a draft from its required fields; every other field is unset.
    #[must_use]
    pub fn new(base: ConstraintBaseDraft<'a>, benchmark: Benchmark) -> Self {
        Self {
            base,
            benchmark,
            value_source: None,
            data_value: None,
            reference_path: None,
        }
    }

    /// Sets `value_source`: Optional source label for the data value.
    #[must_use]
    pub fn value_source(mut self, value: &'a str) -> Self {
        self.value_source = Some(value);
        self
    }

    /// Sets `data_value`: Optional preserved metric SELECT value.
    #[must_use]
    pub fn data_value(mut self, value: MetricValueDraft<'a>) -> Self {
        self.data_value = Some(value);
        self
    }

    /// Sets `reference_path`: Optional existing or earlier-staged
    /// `IfcReference`.
    #[must_use]
    pub fn reference_path(mut self, value: EntityId) -> Self {
        self.reference_path = Some(value);
        self
    }
}

/// Draft for one `IfcObjective`.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
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

impl<'a> ObjectiveDraft<'a> {
    /// Starts a draft from its required fields; every other field is unset.
    #[must_use]
    pub fn new(base: ConstraintBaseDraft<'a>, qualifier: ObjectiveQualifier) -> Self {
        Self {
            base,
            benchmark_values: None,
            logical_aggregator: None,
            qualifier,
            user_defined_qualifier: None,
        }
    }

    /// Sets `benchmark_values`: Optional non-empty ordered constraints.
    #[must_use]
    pub fn benchmark_values(mut self, value: &'a [EntityId]) -> Self {
        self.benchmark_values = Some(value);
        self
    }

    /// Sets `logical_aggregator`: Optional logical operator.
    #[must_use]
    pub fn logical_aggregator(mut self, value: LogicalOperator) -> Self {
        self.logical_aggregator = Some(value);
        self
    }

    /// Sets `user_defined_qualifier`: Required when `qualifier` is user-
    /// defined.
    #[must_use]
    pub fn user_defined_qualifier(mut self, value: &'a str) -> Self {
        self.user_defined_qualifier = Some(value);
        self
    }
}

/// Draft for one resource-level constraint relationship.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
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

impl<'a> ResourceConstraintDraft<'a> {
    /// Starts a draft from its required fields; every other field is unset.
    #[must_use]
    pub fn new(relating_constraint: EntityId, related_resources: &'a [EntityId]) -> Self {
        Self {
            name: None,
            description: None,
            relating_constraint,
            related_resources,
        }
    }

    /// Sets `name`: Optional relationship name.
    #[must_use]
    pub fn name(mut self, value: &'a str) -> Self {
        self.name = Some(value);
        self
    }

    /// Sets `description`: Optional relationship description.
    #[must_use]
    pub fn description(mut self, value: &'a str) -> Self {
        self.description = Some(value);
        self
    }
}

/// Validate and stage one metric in the model's declared release.
///
/// # Release
///
/// IFC4 and IFC4X3 declare eleven attributes. IFC2X3 declares ten: no
/// `ReferencePath`, a required `DataValue`, `CreationTime` as an
/// `IfcDateTimeSelect` record, and an `IfcBenchmarkEnum` without IFC4's
/// `INCLUDES`, `NOTINCLUDES`, `INCLUDEDIN` and `NOTINCLUDEDIN`.
///
/// # Errors
///
/// A user-defined grade without `UserDefinedGrade`; a reference outside its
/// declared type in the release; a header binding no single verified
/// release (`MultipleSchemas`, `UnsupportedSchema`); a value the release
/// does not declare (`AuthoringNotInSchema`) or cannot hold
/// (`AuthoringValueType`: a token its enumeration lacks, text where it
/// declares a record); a data value type outside the release's
/// `IfcMetricValueSelect` (`AuthoringInvalid`); and a required value left unset
/// (`AuthoringRequired`). Nothing is staged on an error.
pub fn create_metric(
    tx: &mut Transaction,
    model: &Model,
    draft: MetricDraft<'_>,
) -> ConstraintResult<EntityId> {
    let layout = bind(model)?;
    layout.require_entity(METRIC)?;
    let base = base_values(layout, tx, model, METRIC, draft.base)?;
    if let Some(path) = draft.reference_path {
        if layout.declared(METRIC, "ReferencePath").is_some() {
            validate_target_in(layout.schema(), tx, model, path, "IfcReference")?;
        }
    }
    let data_value = match draft.data_value {
        None => Value::Null,
        Some(MetricValueDraft::Entity(target)) => {
            validate_target_in(layout.schema(), tx, model, target, "IfcMetricValueSelect")?;
            Value::Ref(target)
        }
        Some(MetricValueDraft::Typed { type_name, value }) => {
            if !layout
                .schema()
                .accepts_type("IfcMetricValueSelect", type_name)
            {
                return Err(ConstraintError::AuthoringInvalid {
                    entity: METRIC,
                    attribute: "DataValue",
                    value: format!(
                        "type {type_name} is outside {:?} IfcMetricValueSelect",
                        layout.version()
                    ),
                });
            }
            Value::Typed {
                type_name: Arc::from(type_name.to_ascii_uppercase()),
                value: Box::new(value.clone()),
            }
        }
    };
    let mut values = base;
    values.extend([
        (
            "Benchmark",
            enumerator(layout, METRIC, "Benchmark", draft.benchmark.token())?,
        ),
        ("ValueSource", optional_text(draft.value_source)),
        ("DataValue", data_value),
        ("ReferencePath", optional_ref(draft.reference_path)),
    ]);
    let record = layout.named_record(METRIC, values)?;
    Ok(tx.create(record))
}

/// Validate and stage one objective in the model's declared release.
///
/// # Release
///
/// IFC4 and IFC4X3 declare `BenchmarkValues` as a `LIST OF IfcConstraint`
/// and a `LogicalAggregator`. IFC2X3 declares a single `IfcMetric` there
/// and `ResultValues`, an `IfcMetric`, in the aggregator's place; its
/// `IfcObjectiveEnum` lacks several IFC4 qualifiers (`CODEWAIVER`,
/// `EXTERNAL`, `MERGECONFLICT`, `MODELVIEW`, `PARAMETER` among them).
///
/// # Errors
///
/// A user-defined grade or qualifier without its label; an empty benchmark
/// list or a benchmark outside its declared type; a header binding no single
/// verified release; in IFC2X3 more than one benchmark
/// (`AuthoringValueType`) or a logical aggregator (`AuthoringNotInSchema`);
/// and a token the release's enumeration lacks (`AuthoringValueType`).
/// Nothing is staged on an error.
pub fn create_objective(
    tx: &mut Transaction,
    model: &Model,
    draft: ObjectiveDraft<'_>,
) -> ConstraintResult<EntityId> {
    let layout = bind(model)?;
    layout.require_entity(OBJECTIVE)?;
    let base = base_values(layout, tx, model, OBJECTIVE, draft.base)?;
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
            let (aggregate, expected) = layout
                .declared(OBJECTIVE, "BenchmarkValues")
                .map_or((true, "IfcConstraint"), |(_, declared)| {
                    (declared.aggregate, declared.type_name.as_str())
                });
            for &target in values {
                validate_target_in(layout.schema(), tx, model, target, expected)?;
            }
            match (aggregate, values) {
                (true, values) => refs(values),
                (false, [one]) => Value::Ref(*one),
                (false, _) => {
                    return Err(ConstraintError::AuthoringValueType {
                        entity: OBJECTIVE,
                        attribute: "BenchmarkValues",
                        declared: expected,
                        schema: layout.version(),
                    })
                }
            }
        }
    };
    let aggregator = match draft.logical_aggregator {
        None => Value::Null,
        Some(value) => enumerator(layout, OBJECTIVE, "LogicalAggregator", value.token())?,
    };
    let mut values = base;
    values.extend([
        ("BenchmarkValues", benchmarks),
        ("LogicalAggregator", aggregator),
        (
            "ObjectiveQualifier",
            enumerator(
                layout,
                OBJECTIVE,
                "ObjectiveQualifier",
                draft.qualifier.token(),
            )?,
        ),
        (
            "UserDefinedQualifier",
            optional_text(draft.user_defined_qualifier),
        ),
    ]);
    let record = layout.named_record(OBJECTIVE, values)?;
    Ok(tx.create(record))
}

/// Validate and stage one resource-level constraint relationship in the
/// model's declared release.
///
/// # Errors
///
/// An empty or duplicated set; a relating target that is not an
/// `IfcConstraint` or a resource outside `IfcResourceObjectSelect`; a header
/// binding no single verified release; and an IFC2X3 model, which declares
/// no `IfcResourceConstraintRelationship` (`EntityNotInSchema`). Nothing is
/// staged on an error.
pub fn relate_resource_constraint(
    tx: &mut Transaction,
    model: &Model,
    draft: ResourceConstraintDraft<'_>,
) -> ConstraintResult<EntityId> {
    let layout = bind(model)?;
    layout.require_entity(RESOURCE_REL)?;
    validate_target_in(
        layout.schema(),
        tx,
        model,
        draft.relating_constraint,
        "IfcConstraint",
    )?;
    validate_set(
        layout.schema(),
        tx,
        model,
        RESOURCE_REL,
        "RelatedResourceObjects",
        draft.related_resources,
        "IfcResourceObjectSelect",
    )?;
    let record = layout.named_record(
        RESOURCE_REL,
        vec![
            ("Name", optional_text(draft.name)),
            ("Description", optional_text(draft.description)),
            ("RelatingConstraint", Value::Ref(draft.relating_constraint)),
            ("RelatedResourceObjects", refs(draft.related_resources)),
        ],
    )?;
    Ok(tx.create(record))
}

/// The inherited `IfcConstraint` values of `draft`, checked against the
/// release, named for [`Layout::named_record`].
fn base_values(
    layout: Layout,
    tx: &Transaction,
    model: &Model,
    kind: &'static str,
    draft: ConstraintBaseDraft<'_>,
) -> ConstraintResult<Vec<(&'static str, Value)>> {
    if draft.grade == ConstraintGrade::UserDefined && draft.user_defined_grade.is_none() {
        return Err(ConstraintError::AuthoringInvalid {
            entity: kind,
            attribute: "WR11",
            value: "USERDEFINED grade requires UserDefinedGrade".into(),
        });
    }
    if let Some(actor) = draft.creating_actor {
        validate_target_in(layout.schema(), tx, model, actor, "IfcActorSelect")?;
    }
    let creation_time = match draft.creation_time {
        None => Value::Null,
        Some(DateTimeInput::Text(text)) => Value::Text(Arc::from(text)),
        Some(DateTimeInput::Record(target)) => {
            if let Some((_, declared)) = layout.declared(kind, "CreationTime") {
                if layout.admits_entity(&declared.type_name, 8) {
                    validate_target_in(
                        layout.schema(),
                        tx,
                        model,
                        target,
                        declared.type_name.as_str(),
                    )?;
                }
            }
            Value::Ref(target)
        }
    };
    Ok(vec![
        ("Name", text(draft.name)),
        ("Description", optional_text(draft.description)),
        (
            "ConstraintGrade",
            enumerator(layout, kind, "ConstraintGrade", draft.grade.token())?,
        ),
        ("ConstraintSource", optional_text(draft.source)),
        ("CreatingActor", optional_ref(draft.creating_actor)),
        ("CreationTime", creation_time),
        ("UserDefinedGrade", optional_text(draft.user_defined_grade)),
    ])
}

/// `token` as an enumerator of the enumeration the release declares for
/// `kind.attribute`: `AuthoringNotInSchema` when the release does not
/// declare the attribute, `AuthoringValueType` when it lacks the token.
fn enumerator(
    layout: Layout,
    kind: &'static str,
    attribute: &'static str,
    token: &str,
) -> ConstraintResult<Value> {
    let Some((_, declared)) = layout.declared(kind, attribute) else {
        return Err(ConstraintError::AuthoringNotInSchema {
            entity: kind,
            attribute,
            schema: layout.version(),
        });
    };
    match layout
        .schema()
        .type_def(&declared.type_name)
        .map(|t| &t.kind)
    {
        Some(TypeKind::Enumeration(members))
            if members
                .iter()
                .any(|member| member.eq_ignore_ascii_case(token)) =>
        {
            Ok(enumeration(token))
        }
        _ => Err(ConstraintError::AuthoringValueType {
            entity: kind,
            attribute,
            declared: declared.type_name.as_str(),
            schema: layout.version(),
        }),
    }
}

fn validate_set(
    schema: &Schema,
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
        validate_target_in(schema, tx, model, target, expected)?;
    }
    Ok(())
}

/// Draft for one `IfcReference`: a path into another entity's attributes.
#[derive(Debug, Clone, Copy, Default)]
#[non_exhaustive]
pub struct ReferenceDraft<'a> {
    /// `TypeIdentifier`, the referenced entity's type name.
    pub type_identifier: Option<&'a str>,
    /// `AttributeIdentifier`, the attribute being addressed.
    pub attribute_identifier: Option<&'a str>,
    /// `InstanceName`, naming the addressed instance.
    pub instance_name: Option<&'a str>,
    /// `ListPositions`, 1-based indices into list-valued attributes.
    pub list_positions: &'a [i64],
    /// `InnerReference`, the next step along the path.
    pub inner_reference: Option<EntityId>,
}

impl<'a> ReferenceDraft<'a> {
    /// Starts an empty draft; every field is unset.
    #[must_use]
    pub fn new() -> Self {
        Self {
            type_identifier: None,
            attribute_identifier: None,
            instance_name: None,
            list_positions: &[],
            inner_reference: None,
        }
    }

    /// Sets `type_identifier`: `TypeIdentifier`, the referenced entity's type
    /// name.
    #[must_use]
    pub fn type_identifier(mut self, value: &'a str) -> Self {
        self.type_identifier = Some(value);
        self
    }

    /// Sets `attribute_identifier`: `AttributeIdentifier`, the attribute being
    /// addressed.
    #[must_use]
    pub fn attribute_identifier(mut self, value: &'a str) -> Self {
        self.attribute_identifier = Some(value);
        self
    }

    /// Sets `instance_name`: `InstanceName`, naming the addressed instance.
    #[must_use]
    pub fn instance_name(mut self, value: &'a str) -> Self {
        self.instance_name = Some(value);
        self
    }

    /// Sets `list_positions`: `ListPositions`, 1-based indices into list-valued
    /// attributes.
    #[must_use]
    pub fn list_positions(mut self, value: &'a [i64]) -> Self {
        self.list_positions = value;
        self
    }

    /// Sets `inner_reference`: `InnerReference`, the next step along the path.
    #[must_use]
    pub fn inner_reference(mut self, value: EntityId) -> Self {
        self.inner_reference = Some(value);
        self
    }
}

/// Stage an `IfcReference`.
///
/// A reference is a path expression: it names a type, an attribute, and
/// optionally positions within a list, chaining through `InnerReference`
/// to address something nested. Every slot is OPTIONAL, so the schema
/// permits a reference that addresses nothing; that is a silently useless
/// record, so at least one slot must be set here.
///
/// `ListPositions` is `LIST [1:?] OF IfcInteger` and the positions are
/// 1-based: an empty list fails the bound, and a zero or negative index
/// addresses no element. Both are refused rather than written.
///
/// # Errors
///
/// Refuses a fully empty draft, an empty or non-positive `ListPositions`,
/// an `inner_reference` that is not itself an `IfcReference`, a header
/// binding no single verified release, and an IFC2X3 model, which declares
/// no `IfcReference` (`EntityNotInSchema`).
pub fn create_reference(
    tx: &mut Transaction,
    model: &Model,
    draft: ReferenceDraft<'_>,
) -> ConstraintResult<EntityId> {
    const ENTITY: &str = "IfcReference";
    let layout = bind(model)?;
    layout.require_entity("IFCREFERENCE")?;
    let empty = draft.type_identifier.is_none()
        && draft.attribute_identifier.is_none()
        && draft.instance_name.is_none()
        && draft.list_positions.is_empty()
        && draft.inner_reference.is_none();
    if empty {
        return Err(ConstraintError::AuthoringInvalid {
            entity: ENTITY,
            attribute: "TypeIdentifier",
            value: "a reference with every slot unset addresses nothing".to_owned(),
        });
    }
    for position in draft.list_positions {
        if *position < 1 {
            return Err(ConstraintError::AuthoringInvalid {
                entity: ENTITY,
                attribute: "ListPositions",
                value: format!("{position} is not a 1-based list index"),
            });
        }
    }
    if let Some(inner) = draft.inner_reference {
        validate_target_in(layout.schema(), tx, model, inner, ENTITY)?;
    }

    let positions = if draft.list_positions.is_empty() {
        // LIST [1:?]: absent stays null rather than becoming an
        // empty list, which would satisfy the type and break the bound.
        Value::Null
    } else {
        Value::List(
            draft
                .list_positions
                .iter()
                .copied()
                .map(Value::Integer)
                .collect(),
        )
    };
    Ok(tx.create(Entity::new(
        "IFCREFERENCE",
        vec![
            optional_text(draft.type_identifier),
            optional_text(draft.attribute_identifier),
            optional_text(draft.instance_name),
            positions,
            draft.inner_reference.map_or(Value::Null, Value::Ref),
        ],
    )))
}

/// Fail unless `target` resolves, in the model or staged on `tx`, to a type
/// `schema` accepts as `expected`.
pub(crate) fn validate_target_in(
    schema: &Schema,
    tx: &Transaction,
    model: &Model,
    target: EntityId,
    expected: &'static str,
) -> ConstraintResult<()> {
    let actual =
        final_type(tx, model, target).ok_or(ConstraintError::UnknownEntity { id: target })?;
    if schema.accepts_type(expected, actual) {
        Ok(())
    } else {
        Err(ConstraintError::AuthoringReferenceType {
            target,
            expected,
            actual: actual.into(),
        })
    }
}

pub(crate) fn final_type<'a>(
    tx: &'a Transaction,
    model: &'a Model,
    id: EntityId,
) -> Option<&'a str> {
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

pub(crate) fn text(value: &str) -> Value {
    Value::Text(Arc::from(value))
}
pub(crate) fn optional_text(value: Option<&str>) -> Value {
    value.map_or(Value::Null, text)
}
pub(crate) fn optional_ref(value: Option<EntityId>) -> Value {
    value.map_or(Value::Null, Value::Ref)
}
fn enumeration(value: &str) -> Value {
    Value::Enum(Arc::from(value))
}
pub(crate) fn refs(values: &[EntityId]) -> Value {
    Value::List(values.iter().copied().map(Value::Ref).collect())
}
