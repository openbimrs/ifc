//! Strict borrowed constraint projections and direct relationship queries.

use ifc_model::guid::Guid;
use ifc_model::{Entity, EntityId, Value};

use crate::types::{Benchmark, ConstraintGrade, LogicalOperator, MetricValue, ObjectiveQualifier};
use crate::view::{
    invalid, optional_ref, optional_text, required_ref, required_refs, required_text,
    validate_target, wrong, ConstraintView,
};
use crate::{ConstraintError, ConstraintResult};

const METRIC: &str = "IFCMETRIC";
const OBJECTIVE: &str = "IFCOBJECTIVE";
const RESOURCE_REL: &str = "IFCRESOURCECONSTRAINTRELATIONSHIP";
const ASSIGNMENT: &str = "IFCRELASSOCIATESCONSTRAINT";

macro_rules! projection {
    ($name:ident, $kind:expr) => {
        /// Strict borrowed IFC4 projection.
        #[derive(Debug, Clone, Copy)]
        pub struct $name<'m> {
            id: EntityId,
            entity: &'m Entity,
        }
        impl<'m> $name<'m> {
            /// Construct from an entity of the exact expected kind.
            pub fn try_new(id: EntityId, entity: &'m Entity) -> ConstraintResult<Self> {
                if entity.is_type($kind) {
                    Ok(Self { id, entity })
                } else {
                    Err(wrong($kind, entity))
                }
            }
            /// Stable model identifier.
            #[must_use]
            pub const fn id(self) -> EntityId {
                self.id
            }
        }
    };
}

projection!(Metric, METRIC);
projection!(Objective, OBJECTIVE);
projection!(ResourceConstraintRelationship, RESOURCE_REL);
projection!(ConstraintAssignment, ASSIGNMENT);

fn grade(kind: &'static str, id: EntityId, entity: &Entity) -> ConstraintResult<ConstraintGrade> {
    match entity.attribute(2) {
        Some(Value::Enum(value)) => ConstraintGrade::parse(value)
            .ok_or_else(|| invalid(kind, id, "ConstraintGrade", &Value::Enum(value.clone()))),
        Some(value) => Err(invalid(kind, id, "ConstraintGrade", value)),
        None => Err(ConstraintError::MissingAttribute {
            entity: kind,
            id,
            attribute: "ConstraintGrade",
        }),
    }
}

fn validate_base(
    view: ConstraintView<'_>,
    kind: &'static str,
    id: EntityId,
    entity: &Entity,
) -> ConstraintResult<()> {
    required_text(kind, id, entity, 0, "Name")?;
    let grade = grade(kind, id, entity)?;
    if grade == ConstraintGrade::UserDefined
        && optional_text(kind, id, entity, 6, "UserDefinedGrade")?.is_none()
    {
        return Err(ConstraintError::Semantic {
            entity: kind,
            id,
            rule: "WR11",
            detail: "USERDEFINED grade requires UserDefinedGrade".into(),
        });
    }
    if let Some(actor) = optional_ref(kind, id, entity, 4, "CreatingActor")? {
        validate_target(
            view.model(),
            kind,
            id,
            "CreatingActor",
            actor,
            "IfcActorSelect",
        )?;
    }
    Ok(())
}

macro_rules! base_accessors {
    ($name:ident, $kind:expr) => {
        impl<'m> $name<'m> {
            /// Constraint name.
            pub fn name(self) -> ConstraintResult<&'m str> {
                required_text($kind, self.id, self.entity, 0, "Name")
            }
            /// Optional description.
            pub fn description(self) -> ConstraintResult<Option<&'m str>> {
                optional_text($kind, self.id, self.entity, 1, "Description")
            }
            /// Typed constraint grade.
            pub fn grade(self) -> ConstraintResult<ConstraintGrade> {
                grade($kind, self.id, self.entity)
            }
            /// Optional source label.
            pub fn source(self) -> ConstraintResult<Option<&'m str>> {
                optional_text($kind, self.id, self.entity, 3, "ConstraintSource")
            }
            /// Optional creating actor.
            pub fn creating_actor(self) -> ConstraintResult<Option<EntityId>> {
                optional_ref($kind, self.id, self.entity, 4, "CreatingActor")
            }
            /// Optional creation time lexical value.
            pub fn creation_time(self) -> ConstraintResult<Option<&'m str>> {
                optional_text($kind, self.id, self.entity, 5, "CreationTime")
            }
            /// Optional user-defined grade.
            pub fn user_defined_grade(self) -> ConstraintResult<Option<&'m str>> {
                optional_text($kind, self.id, self.entity, 6, "UserDefinedGrade")
            }
        }
    };
}

base_accessors!(Metric, METRIC);
base_accessors!(Objective, OBJECTIVE);

impl<'m> Metric<'m> {
    /// Typed comparison benchmark.
    pub fn benchmark(self) -> ConstraintResult<Benchmark> {
        match self.entity.attribute(7) {
            Some(Value::Enum(value)) => Benchmark::parse(value)
                .ok_or_else(|| invalid(METRIC, self.id, "Benchmark", &Value::Enum(value.clone()))),
            Some(value) => Err(invalid(METRIC, self.id, "Benchmark", value)),
            None => Err(ConstraintError::MissingAttribute {
                entity: METRIC,
                id: self.id,
                attribute: "Benchmark",
            }),
        }
    }

    /// Optional source label for the metric value.
    pub fn value_source(self) -> ConstraintResult<Option<&'m str>> {
        optional_text(METRIC, self.id, self.entity, 8, "ValueSource")
    }

    /// Preserved optional `IfcMetricValueSelect`.
    pub fn data_value(self) -> ConstraintResult<Option<MetricValue<'m>>> {
        match self.entity.attribute(9) {
            None | Some(Value::Null) => Ok(None),
            Some(Value::Ref(target)) => Ok(Some(MetricValue::Entity(*target))),
            Some(Value::Typed { type_name, value })
                if ifc_schema::ifc4().accepts_type("IfcMetricValueSelect", type_name) =>
            {
                Ok(Some(MetricValue::Typed {
                    type_name,
                    value: value.as_ref(),
                }))
            }
            Some(value) => Err(invalid(METRIC, self.id, "DataValue", value)),
        }
    }

    /// Optional `IfcReference` path.
    pub fn reference_path(self) -> ConstraintResult<Option<EntityId>> {
        optional_ref(METRIC, self.id, self.entity, 10, "ReferencePath")
    }

    fn validate(self, view: ConstraintView<'m>) -> ConstraintResult<Self> {
        validate_base(view, METRIC, self.id, self.entity)?;
        self.benchmark()?;
        match self.entity.attribute(9) {
            None | Some(Value::Null) => {}
            Some(Value::Ref(target)) => validate_target(
                view.model(),
                METRIC,
                self.id,
                "DataValue",
                *target,
                "IfcMetricValueSelect",
            )?,
            Some(Value::Typed { type_name, .. })
                if ifc_schema::ifc4().accepts_type("IfcMetricValueSelect", type_name) => {}
            Some(value) => return Err(invalid(METRIC, self.id, "DataValue", value)),
        }
        if let Some(reference) = self.reference_path()? {
            validate_target(
                view.model(),
                METRIC,
                self.id,
                "ReferencePath",
                reference,
                "IfcReference",
            )?;
        }
        Ok(self)
    }
}

impl<'m> Objective<'m> {
    /// Optional ordered benchmark constraints.
    pub fn benchmark_values(self) -> ConstraintResult<Option<Vec<EntityId>>> {
        match self.entity.attribute(7) {
            None | Some(Value::Null) => Ok(None),
            Some(Value::List(values)) if !values.is_empty() => values
                .iter()
                .map(|value| match value {
                    Value::Ref(target) => Ok(*target),
                    other => Err(invalid(OBJECTIVE, self.id, "BenchmarkValues", other)),
                })
                .collect::<ConstraintResult<Vec<_>>>()
                .map(Some),
            Some(value) => Err(invalid(OBJECTIVE, self.id, "BenchmarkValues", value)),
        }
    }

    /// Optional typed logical aggregator.
    pub fn logical_aggregator(self) -> ConstraintResult<Option<LogicalOperator>> {
        match self.entity.attribute(8) {
            None | Some(Value::Null) => Ok(None),
            Some(Value::Enum(value)) => LogicalOperator::parse(value).map(Some).ok_or_else(|| {
                invalid(
                    OBJECTIVE,
                    self.id,
                    "LogicalAggregator",
                    &Value::Enum(value.clone()),
                )
            }),
            Some(value) => Err(invalid(OBJECTIVE, self.id, "LogicalAggregator", value)),
        }
    }

    /// Typed objective qualifier.
    pub fn qualifier(self) -> ConstraintResult<ObjectiveQualifier> {
        match self.entity.attribute(9) {
            Some(Value::Enum(value)) => ObjectiveQualifier::parse(value).ok_or_else(|| {
                invalid(
                    OBJECTIVE,
                    self.id,
                    "ObjectiveQualifier",
                    &Value::Enum(value.clone()),
                )
            }),
            Some(value) => Err(invalid(OBJECTIVE, self.id, "ObjectiveQualifier", value)),
            None => Err(ConstraintError::MissingAttribute {
                entity: OBJECTIVE,
                id: self.id,
                attribute: "ObjectiveQualifier",
            }),
        }
    }

    /// Optional user-defined objective qualifier.
    pub fn user_defined_qualifier(self) -> ConstraintResult<Option<&'m str>> {
        optional_text(OBJECTIVE, self.id, self.entity, 10, "UserDefinedQualifier")
    }

    fn validate(self, view: ConstraintView<'m>) -> ConstraintResult<Self> {
        validate_base(view, OBJECTIVE, self.id, self.entity)?;
        self.logical_aggregator()?;
        if self.qualifier()? == ObjectiveQualifier::UserDefined
            && self.user_defined_qualifier()?.is_none()
        {
            return Err(ConstraintError::Semantic {
                entity: OBJECTIVE,
                id: self.id,
                rule: "WR21",
                detail: "USERDEFINED qualifier requires UserDefinedQualifier".into(),
            });
        }
        if let Some(values) = self.benchmark_values()? {
            for target in values {
                validate_target(
                    view.model(),
                    OBJECTIVE,
                    self.id,
                    "BenchmarkValues",
                    target,
                    "IfcConstraint",
                )?;
            }
        }
        Ok(self)
    }
}

impl<'m> ResourceConstraintRelationship<'m> {
    /// Optional relationship name.
    pub fn name(self) -> ConstraintResult<Option<&'m str>> {
        optional_text(RESOURCE_REL, self.id, self.entity, 0, "Name")
    }
    /// Optional relationship description.
    pub fn description(self) -> ConstraintResult<Option<&'m str>> {
        optional_text(RESOURCE_REL, self.id, self.entity, 1, "Description")
    }
    /// Relating metric or objective.
    pub fn relating_constraint(self) -> ConstraintResult<EntityId> {
        required_ref(RESOURCE_REL, self.id, self.entity, 2, "RelatingConstraint")
    }
    /// Non-empty unique resource-select targets.
    pub fn related_resources(self) -> ConstraintResult<Vec<EntityId>> {
        required_refs(
            RESOURCE_REL,
            self.id,
            self.entity,
            3,
            "RelatedResourceObjects",
        )
    }
    fn validate(self, view: ConstraintView<'m>) -> ConstraintResult<Self> {
        validate_target(
            view.model(),
            RESOURCE_REL,
            self.id,
            "RelatingConstraint",
            self.relating_constraint()?,
            "IfcConstraint",
        )?;
        for target in self.related_resources()? {
            validate_target(
                view.model(),
                RESOURCE_REL,
                self.id,
                "RelatedResourceObjects",
                target,
                "IfcResourceObjectSelect",
            )?;
        }
        Ok(self)
    }
}

impl<'m> ConstraintAssignment<'m> {
    /// Root GlobalId.
    pub fn global_id(self) -> ConstraintResult<&'m str> {
        required_text(ASSIGNMENT, self.id, self.entity, 0, "GlobalId")
    }
    /// Optional association intent.
    pub fn intent(self) -> ConstraintResult<Option<&'m str>> {
        optional_text(ASSIGNMENT, self.id, self.entity, 5, "Intent")
    }
    /// Related definition-select targets.
    pub fn related_objects(self) -> ConstraintResult<Vec<EntityId>> {
        required_refs(ASSIGNMENT, self.id, self.entity, 4, "RelatedObjects")
    }
    /// Relating metric or objective.
    pub fn relating_constraint(self) -> ConstraintResult<EntityId> {
        required_ref(ASSIGNMENT, self.id, self.entity, 6, "RelatingConstraint")
    }
    fn validate(self, view: ConstraintView<'m>) -> ConstraintResult<Self> {
        if Guid::parse(self.global_id()?).is_none() {
            return Err(ConstraintError::InvalidValue {
                entity: ASSIGNMENT,
                id: self.id,
                attribute: "GlobalId",
                value: self.global_id()?.into(),
            });
        }
        validate_target(
            view.model(),
            ASSIGNMENT,
            self.id,
            "RelatingConstraint",
            self.relating_constraint()?,
            "IfcConstraint",
        )?;
        for target in self.related_objects()? {
            validate_target(
                view.model(),
                ASSIGNMENT,
                self.id,
                "RelatedObjects",
                target,
                "IfcDefinitionSelect",
            )?;
        }
        Ok(self)
    }
}

impl<'m> ConstraintView<'m> {
    /// Strictly project one metric.
    pub fn metric(self, id: EntityId) -> ConstraintResult<Metric<'m>> {
        let entity = self
            .model()
            .get(id)
            .ok_or(ConstraintError::UnknownEntity { id })?;
        Metric::try_new(id, entity)?.validate(self)
    }
    /// Strictly project one objective.
    pub fn objective(self, id: EntityId) -> ConstraintResult<Objective<'m>> {
        let entity = self
            .model()
            .get(id)
            .ok_or(ConstraintError::UnknownEntity { id })?;
        Objective::try_new(id, entity)?.validate(self)
    }
    /// Strictly project one resource constraint relationship.
    pub fn resource_constraint_relationship(
        self,
        id: EntityId,
    ) -> ConstraintResult<ResourceConstraintRelationship<'m>> {
        let entity = self
            .model()
            .get(id)
            .ok_or(ConstraintError::UnknownEntity { id })?;
        ResourceConstraintRelationship::try_new(id, entity)?.validate(self)
    }
    /// Strictly project one rooted constraint association.
    pub fn constraint_assignment(self, id: EntityId) -> ConstraintResult<ConstraintAssignment<'m>> {
        let entity = self
            .model()
            .get(id)
            .ok_or(ConstraintError::UnknownEntity { id })?;
        ConstraintAssignment::try_new(id, entity)?.validate(self)
    }
    /// Resource-select IDs directly governed by a constraint.
    pub fn resources_constrained_by(self, constraint: EntityId) -> ConstraintResult<Vec<EntityId>> {
        self.constraint(constraint)?;
        let mut out = Vec::new();
        for (id, entity) in self.model().of_type(RESOURCE_REL) {
            let relationship = ResourceConstraintRelationship { id, entity };
            if relationship.relating_constraint()? == constraint {
                out.extend(relationship.validate(self)?.related_resources()?);
            }
        }
        Ok(out)
    }
    /// Definition-select IDs directly associated with a constraint.
    pub fn objects_constrained_by(self, constraint: EntityId) -> ConstraintResult<Vec<EntityId>> {
        self.constraint(constraint)?;
        let mut out = Vec::new();
        for (id, entity) in self.model().of_type(ASSIGNMENT) {
            let relationship = ConstraintAssignment { id, entity };
            if relationship.relating_constraint()? == constraint {
                out.extend(relationship.validate(self)?.related_objects()?);
            }
        }
        Ok(out)
    }
    fn constraint(self, id: EntityId) -> ConstraintResult<()> {
        let entity = self
            .model()
            .get(id)
            .ok_or(ConstraintError::UnknownEntity { id })?;
        if entity.is_type(METRIC) {
            self.metric(id).map(|_| ())
        } else if entity.is_type(OBJECTIVE) {
            self.objective(id).map(|_| ())
        } else {
            Err(wrong("IfcConstraint", entity))
        }
    }
}
