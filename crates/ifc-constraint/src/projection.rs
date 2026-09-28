//! Strict borrowed constraint projections and direct relationship queries,
//! read by attribute name in the model's declared release (#212).

use ifc_model::guid::Guid;
use ifc_model::{Entity, EntityId, Value};
use ifc_schema::SchemaVersion;

use crate::release::Layout;
use crate::types::{Benchmark, ConstraintGrade, LogicalOperator, MetricValue, ObjectiveQualifier};
use crate::view::{invalid, wrong, ConstraintView, Record};
use crate::{ConstraintError, ConstraintResult};

const METRIC: &str = "IFCMETRIC";
const OBJECTIVE: &str = "IFCOBJECTIVE";
const RESOURCE_REL: &str = "IFCRESOURCECONSTRAINTRELATIONSHIP";
const ASSIGNMENT: &str = "IFCRELASSOCIATESCONSTRAINT";

macro_rules! projection {
    ($name:ident, $kind:expr) => {
        /// Strict borrowed projection, read against one release.
        #[derive(Debug, Clone, Copy)]
        pub struct $name<'m> {
            record: Record<'m>,
        }
        impl<'m> $name<'m> {
            /// Construct from an entity of the exact expected kind, read
            /// against `release`.
            ///
            /// [`ConstraintView`] binds the model's declared release; use
            /// this when the release is known some other way.
            ///
            /// # Errors
            ///
            /// `WrongEntityType` for another entity, `UnsupportedSchema`
            /// for a release this crate is not verified against (IFC4X1,
            /// IFC4X2), and `EntityNotInSchema` for an entity the release
            /// does not declare.
            pub fn try_new(
                id: EntityId,
                entity: &'m Entity,
                release: SchemaVersion,
            ) -> ConstraintResult<Self> {
                Self::bound(id, entity, Layout::of_version(release)?)
            }
            pub(crate) fn bound(
                id: EntityId,
                entity: &'m Entity,
                layout: Layout,
            ) -> ConstraintResult<Self> {
                if !entity.is_type($kind) {
                    return Err(wrong($kind, entity));
                }
                layout.require_entity($kind)?;
                Ok(Self {
                    record: Record {
                        kind: $kind,
                        id,
                        entity,
                        layout,
                    },
                })
            }
            /// Stable model identifier.
            #[must_use]
            pub const fn id(self) -> EntityId {
                self.record.id
            }
            /// The release this projection reads against.
            #[must_use]
            pub const fn release(self) -> SchemaVersion {
                self.record.layout.version()
            }
        }
    };
}

projection!(Metric, METRIC);
projection!(Objective, OBJECTIVE);
projection!(ResourceConstraintRelationship, RESOURCE_REL);
projection!(ConstraintAssignment, ASSIGNMENT);

/// A required or optional enumeration read by name and parsed.
fn enumeration<T>(
    record: Record<'_>,
    attribute: &'static str,
    parse: fn(&str) -> Option<T>,
) -> ConstraintResult<Option<T>> {
    match record.value(attribute)? {
        None => Ok(None),
        Some(Value::Enum(value)) => parse(value).map(Some).ok_or_else(|| {
            invalid(
                record.kind,
                record.id,
                attribute,
                &Value::Enum(value.clone()),
            )
        }),
        Some(value) => Err(invalid(record.kind, record.id, attribute, value)),
    }
}

fn required<T>(
    record: Record<'_>,
    attribute: &'static str,
    value: Option<T>,
) -> ConstraintResult<T> {
    value.ok_or(ConstraintError::MissingAttribute {
        entity: record.kind,
        id: record.id,
        attribute,
    })
}

fn validate_base(view: ConstraintView<'_>, record: Record<'_>) -> ConstraintResult<()> {
    record.required_text("Name")?;
    let grade = required(
        record,
        "ConstraintGrade",
        enumeration(record, "ConstraintGrade", ConstraintGrade::parse)?,
    )?;
    if grade == ConstraintGrade::UserDefined && record.optional_text("UserDefinedGrade")?.is_none()
    {
        return Err(ConstraintError::Semantic {
            entity: record.kind,
            id: record.id,
            rule: "WR11",
            detail: "USERDEFINED grade requires UserDefinedGrade".into(),
        });
    }
    if let Some(actor) = record.optional_ref("CreatingActor")? {
        record.validate_target(view.model(), "CreatingActor", actor, "IfcActorSelect")?;
    }
    match record.optional_text("CreationTime") {
        Ok(_) => {}
        Err(ConstraintError::StructuredValue { target, .. }) => {
            record.validate_target(view.model(), "CreationTime", target, "IfcDateTimeSelect")?;
        }
        Err(error) => return Err(error),
    }
    Ok(())
}

macro_rules! base_accessors {
    ($name:ident) => {
        impl<'m> $name<'m> {
            /// Constraint name.
            pub fn name(self) -> ConstraintResult<&'m str> {
                self.record.required_text("Name")
            }
            /// Optional description.
            pub fn description(self) -> ConstraintResult<Option<&'m str>> {
                self.record.optional_text("Description")
            }
            /// Typed constraint grade.
            pub fn grade(self) -> ConstraintResult<ConstraintGrade> {
                required(
                    self.record,
                    "ConstraintGrade",
                    enumeration(self.record, "ConstraintGrade", ConstraintGrade::parse)?,
                )
            }
            /// Optional source label.
            pub fn source(self) -> ConstraintResult<Option<&'m str>> {
                self.record.optional_text("ConstraintSource")
            }
            /// Optional creating actor.
            pub fn creating_actor(self) -> ConstraintResult<Option<EntityId>> {
                self.record.optional_ref("CreatingActor")
            }
            /// Optional creation time, IFC4/IFC4X3 `IfcDateTime` text. An
            /// IFC2X3 `IfcDateTimeSelect` record is `StructuredValue` with
            /// the record's id, never read as text.
            pub fn creation_time(self) -> ConstraintResult<Option<&'m str>> {
                self.record.optional_text("CreationTime")
            }
            /// Optional user-defined grade.
            pub fn user_defined_grade(self) -> ConstraintResult<Option<&'m str>> {
                self.record.optional_text("UserDefinedGrade")
            }
        }
    };
}

base_accessors!(Metric);
base_accessors!(Objective);

impl<'m> Metric<'m> {
    /// Typed comparison benchmark.
    pub fn benchmark(self) -> ConstraintResult<Benchmark> {
        required(
            self.record,
            "Benchmark",
            enumeration(self.record, "Benchmark", Benchmark::parse)?,
        )
    }

    /// Optional source label for the metric value.
    pub fn value_source(self) -> ConstraintResult<Option<&'m str>> {
        self.record.optional_text("ValueSource")
    }

    /// Preserved `IfcMetricValueSelect` (optional from IFC4, required in
    /// IFC2X3), checked against the release's own SELECT.
    pub fn data_value(self) -> ConstraintResult<Option<MetricValue<'m>>> {
        let record = self.record;
        match record.value("DataValue")? {
            None => Ok(None),
            Some(Value::Ref(target)) => Ok(Some(MetricValue::Entity(*target))),
            Some(Value::Typed { type_name, value })
                if record
                    .layout
                    .schema()
                    .accepts_type("IfcMetricValueSelect", type_name) =>
            {
                Ok(Some(MetricValue::Typed {
                    type_name,
                    value: value.as_ref(),
                }))
            }
            Some(value) => Err(invalid(METRIC, record.id, "DataValue", value)),
        }
    }

    /// Optional `IfcReference` path. `NotInSchema` in IFC2X3.
    pub fn reference_path(self) -> ConstraintResult<Option<EntityId>> {
        self.record.optional_ref("ReferencePath")
    }

    fn validate(self, view: ConstraintView<'m>) -> ConstraintResult<Self> {
        let record = self.record;
        validate_base(view, record)?;
        self.benchmark()?;
        match self.data_value()? {
            None if record.requires("DataValue") => {
                return Err(ConstraintError::MissingAttribute {
                    entity: METRIC,
                    id: record.id,
                    attribute: "DataValue",
                })
            }
            Some(MetricValue::Entity(target)) => {
                record.validate_target(
                    view.model(),
                    "DataValue",
                    target,
                    "IfcMetricValueSelect",
                )?;
            }
            _ => {}
        }
        if record.declares("ReferencePath") {
            if let Some(reference) = self.reference_path()? {
                record.validate_target(view.model(), "ReferencePath", reference, "IfcReference")?;
            }
        }
        Ok(self)
    }
}

impl<'m> Objective<'m> {
    /// Optional ordered benchmark constraints.
    ///
    /// `BenchmarkValues` is a `LIST` from IFC4 on: the authored order is
    /// significant and returned unchanged, never sorted or deduplicated.
    /// IFC2X3 declares a single `IfcMetric`, returned as a one-element list.
    pub fn benchmark_values(self) -> ConstraintResult<Option<Vec<EntityId>>> {
        let record = self.record;
        match record.value("BenchmarkValues")? {
            None => Ok(None),
            Some(Value::Ref(target)) if !record.aggregate("BenchmarkValues") => {
                Ok(Some(vec![*target]))
            }
            Some(Value::List(values))
                if record.aggregate("BenchmarkValues") && !values.is_empty() =>
            {
                values
                    .iter()
                    .map(|value| match value {
                        Value::Ref(target) => Ok(*target),
                        other => Err(invalid(OBJECTIVE, record.id, "BenchmarkValues", other)),
                    })
                    .collect::<ConstraintResult<Vec<_>>>()
                    .map(Some)
            }
            Some(value) => Err(invalid(OBJECTIVE, record.id, "BenchmarkValues", value)),
        }
    }

    /// Optional typed logical aggregator. `NotInSchema` in IFC2X3, whose
    /// `ResultValues` in that position is an `IfcMetric`, not an operator.
    pub fn logical_aggregator(self) -> ConstraintResult<Option<LogicalOperator>> {
        enumeration(self.record, "LogicalAggregator", LogicalOperator::parse)
    }

    /// Typed objective qualifier.
    pub fn qualifier(self) -> ConstraintResult<ObjectiveQualifier> {
        required(
            self.record,
            "ObjectiveQualifier",
            enumeration(self.record, "ObjectiveQualifier", ObjectiveQualifier::parse)?,
        )
    }

    /// Optional user-defined objective qualifier.
    pub fn user_defined_qualifier(self) -> ConstraintResult<Option<&'m str>> {
        self.record.optional_text("UserDefinedQualifier")
    }

    fn validate(self, view: ConstraintView<'m>) -> ConstraintResult<Self> {
        let record = self.record;
        validate_base(view, record)?;
        if record.declares("LogicalAggregator") {
            self.logical_aggregator()?;
        }
        if self.qualifier()? == ObjectiveQualifier::UserDefined
            && self.user_defined_qualifier()?.is_none()
        {
            return Err(ConstraintError::Semantic {
                entity: OBJECTIVE,
                id: record.id,
                rule: "WR21",
                detail: "USERDEFINED qualifier requires UserDefinedQualifier".into(),
            });
        }
        if let Some(values) = self.benchmark_values()? {
            let expected = record.declared_type("BenchmarkValues");
            for target in values {
                record.validate_target(view.model(), "BenchmarkValues", target, expected)?;
            }
        }
        Ok(self)
    }
}

impl<'m> ResourceConstraintRelationship<'m> {
    /// Optional relationship name.
    pub fn name(self) -> ConstraintResult<Option<&'m str>> {
        self.record.optional_text("Name")
    }
    /// Optional relationship description.
    pub fn description(self) -> ConstraintResult<Option<&'m str>> {
        self.record.optional_text("Description")
    }
    /// Relating metric or objective.
    pub fn relating_constraint(self) -> ConstraintResult<EntityId> {
        self.record.required_ref("RelatingConstraint")
    }
    /// Non-empty unique resource-select targets.
    pub fn related_resources(self) -> ConstraintResult<Vec<EntityId>> {
        self.record.required_refs("RelatedResourceObjects")
    }
    fn validate(self, view: ConstraintView<'m>) -> ConstraintResult<Self> {
        let record = self.record;
        record.validate_target(
            view.model(),
            "RelatingConstraint",
            self.relating_constraint()?,
            "IfcConstraint",
        )?;
        for target in self.related_resources()? {
            record.validate_target(
                view.model(),
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
        self.record.required_text("GlobalId")
    }
    /// Optional association intent (required in IFC2X3).
    pub fn intent(self) -> ConstraintResult<Option<&'m str>> {
        self.record.optional_text("Intent")
    }
    /// Related definition-select targets (`IfcRoot` under WR21 in IFC2X3).
    pub fn related_objects(self) -> ConstraintResult<Vec<EntityId>> {
        self.record.required_refs("RelatedObjects")
    }
    /// Relating metric or objective.
    pub fn relating_constraint(self) -> ConstraintResult<EntityId> {
        self.record.required_ref("RelatingConstraint")
    }
    fn validate(self, view: ConstraintView<'m>) -> ConstraintResult<Self> {
        let record = self.record;
        if Guid::parse(self.global_id()?).is_none() {
            return Err(ConstraintError::InvalidValue {
                entity: ASSIGNMENT,
                id: record.id,
                attribute: "GlobalId",
                value: self.global_id()?.into(),
            });
        }
        record.validate_target(
            view.model(),
            "RelatingConstraint",
            self.relating_constraint()?,
            "IfcConstraint",
        )?;
        for target in self.related_objects()? {
            record.validate_target(
                view.model(),
                "RelatedObjects",
                target,
                "IfcDefinitionSelect",
            )?;
        }
        Ok(self)
    }
}

impl<'m> ConstraintView<'m> {
    fn entity(self, id: EntityId) -> ConstraintResult<&'m Entity> {
        self.model()
            .get(id)
            .ok_or(ConstraintError::UnknownEntity { id })
    }

    /// Strictly project one metric.
    pub fn metric(self, id: EntityId) -> ConstraintResult<Metric<'m>> {
        Metric::bound(id, self.entity(id)?, self.layout()?)?.validate(self)
    }
    /// Strictly project one objective.
    pub fn objective(self, id: EntityId) -> ConstraintResult<Objective<'m>> {
        Objective::bound(id, self.entity(id)?, self.layout()?)?.validate(self)
    }
    /// Strictly project one resource constraint relationship.
    pub fn resource_constraint_relationship(
        self,
        id: EntityId,
    ) -> ConstraintResult<ResourceConstraintRelationship<'m>> {
        ResourceConstraintRelationship::bound(id, self.entity(id)?, self.layout()?)?.validate(self)
    }
    /// Strictly project one rooted constraint association.
    pub fn constraint_assignment(self, id: EntityId) -> ConstraintResult<ConstraintAssignment<'m>> {
        ConstraintAssignment::bound(id, self.entity(id)?, self.layout()?)?.validate(self)
    }
    /// Resource-select IDs directly governed by a constraint.
    ///
    /// Empty in IFC2X3, which declares no `IfcResourceConstraintRelationship`.
    pub fn resources_constrained_by(self, constraint: EntityId) -> ConstraintResult<Vec<EntityId>> {
        self.constraint(constraint)?;
        let layout = self.layout()?;
        let mut out = Vec::new();
        if layout.require_entity(RESOURCE_REL).is_err() {
            return Ok(out);
        }
        for (id, entity) in self.model().of_type(RESOURCE_REL) {
            let relationship = ResourceConstraintRelationship::bound(id, entity, layout)?;
            if relationship.relating_constraint()? == constraint {
                out.extend(relationship.validate(self)?.related_resources()?);
            }
        }
        Ok(out)
    }
    /// Definition-select IDs directly associated with a constraint.
    pub fn objects_constrained_by(self, constraint: EntityId) -> ConstraintResult<Vec<EntityId>> {
        self.constraint(constraint)?;
        let layout = self.layout()?;
        let mut out = Vec::new();
        for (id, entity) in self.model().of_type(ASSIGNMENT) {
            let relationship = ConstraintAssignment::bound(id, entity, layout)?;
            if relationship.relating_constraint()? == constraint {
                out.extend(relationship.validate(self)?.related_objects()?);
            }
        }
        Ok(out)
    }
    fn constraint(self, id: EntityId) -> ConstraintResult<()> {
        let entity = self.entity(id)?;
        if entity.is_type(METRIC) {
            self.metric(id).map(|_| ())
        } else if entity.is_type(OBJECTIVE) {
            self.objective(id).map(|_| ())
        } else {
            Err(wrong("IfcConstraint", entity))
        }
    }
}
