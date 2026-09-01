//! Schema-checked, transaction-staged resource authoring.

use std::collections::HashSet;

use ifc_model::{Budget, Entity, EntityId, Model, Transaction, Value};
use ifc_schema::{ifc4, Schema, TypeKind};

use crate::author::draft::{AllocationDraft, NestingDraft, ResourceDraft, ResourceTimeDraft};
use crate::error::{ResourceError, ResourceResult};
use crate::view::validate_object_assignment;
use crate::{ResourceKind, ResourceView};

pub struct ResourceEditor<'m> {
    model: &'m mut Model,
    schema: &'static Schema,
}

impl<'m> ResourceEditor<'m> {
    pub fn for_model(model: &'m mut Model) -> ResourceResult<Self> {
        ResourceView::for_model(model)?;
        Ok(Self {
            model,
            schema: ifc4(),
        })
    }

    fn validate_new_global_id(&self, value: &str) -> ResourceResult<()> {
        validate_global_id(value)?;
        for (id, entity) in self.model.iter() {
            let Some(slot) = self
                .schema
                .attribute_names(&entity.type_name)
                .iter()
                .position(|name| name.eq_ignore_ascii_case("GlobalId"))
            else {
                continue;
            };
            if entity
                .attributes
                .get(slot)
                .is_some_and(|stored| matches!(stored.unwrap_typed(), Value::Text(stored) if stored.as_ref() == value))
            {
                return Err(ResourceError::SemanticViolation {
                    entity: Some(id),
                    rule: "IfcRoot.GlobalId must be unique in the model",
                });
            }
        }
        Ok(())
    }

    pub fn create_resource(&mut self, draft: ResourceDraft<'_>) -> ResourceResult<EntityId> {
        self.validate_new_global_id(draft.global_id)?;
        let entity_type = resource_entity_type(draft.kind);
        if let Some(value) = draft.predefined_type {
            validate_enum(self.schema, entity_type, "PredefinedType", value)?;
            if value == "USERDEFINED"
                && draft
                    .object_type
                    .is_none_or(|value| value.trim().is_empty())
            {
                return Err(ResourceError::SemanticViolation {
                    entity: None,
                    rule: "USERDEFINED_REQUIRES_OBJECT_TYPE",
                });
            }
        }
        if let Some(usage) = draft.usage {
            self.check_reference(usage, "Usage", "IfcResourceTime", usage)?;
        }
        for cost in &draft.base_costs {
            self.check_reference(*cost, "BaseCosts", "IfcAppliedValue", *cost)?;
        }
        if let Some(quantity) = draft.base_quantity {
            self.check_reference(quantity, "BaseQuantity", "IfcPhysicalQuantity", quantity)?;
        }

        let base_costs = (!draft.base_costs.is_empty()).then(|| refs(&draft.base_costs));
        let entity = build_entity(
            self.schema,
            entity_type,
            &[
                ("GlobalId", Some(text(draft.global_id))),
                ("Name", draft.name.map(text)),
                ("ObjectType", draft.object_type.map(text)),
                ("Identification", draft.identification.map(text)),
                ("LongDescription", draft.long_description.map(text)),
                ("Usage", draft.usage.map(Value::Ref)),
                ("BaseCosts", base_costs),
                ("BaseQuantity", draft.base_quantity.map(Value::Ref)),
                (
                    "PredefinedType",
                    draft.predefined_type.map(|value| Value::Enum(value.into())),
                ),
            ],
        )?;
        self.commit_create(entity)
    }

    pub fn create_time(&mut self, draft: ResourceTimeDraft<'_>) -> ResourceResult<EntityId> {
        for (attribute, value) in [
            ("ScheduleUsage", draft.schedule_usage),
            ("ActualUsage", draft.actual_usage),
            ("RemainingUsage", draft.remaining_usage),
            ("Completion", draft.completion),
        ] {
            if value.is_some_and(|value| !value.is_finite() || value <= 0.0) {
                return Err(ResourceError::InvalidDraft {
                    entity_type: "IfcResourceTime",
                    attribute,
                    expected: "finite positive ratio",
                });
            }
        }
        let entity = build_entity(
            self.schema,
            "IfcResourceTime",
            &[
                ("Name", draft.name.map(text)),
                ("ScheduleWork", draft.schedule_work.map(text)),
                ("ScheduleUsage", draft.schedule_usage.map(Value::Real)),
                ("ScheduleStart", draft.schedule_start.map(text)),
                ("ScheduleFinish", draft.schedule_finish.map(text)),
                ("IsOverAllocated", draft.is_over_allocated.map(Value::Bool)),
                ("StatusTime", draft.status_time.map(text)),
                ("ActualWork", draft.actual_work.map(text)),
                ("ActualUsage", draft.actual_usage.map(Value::Real)),
                ("ActualStart", draft.actual_start.map(text)),
                ("ActualFinish", draft.actual_finish.map(text)),
                ("RemainingWork", draft.remaining_work.map(text)),
                ("RemainingUsage", draft.remaining_usage.map(Value::Real)),
                ("Completion", draft.completion.map(Value::Real)),
            ],
        )?;
        self.commit_create(entity)
    }

    pub fn create_allocation(&mut self, draft: AllocationDraft<'_>) -> ResourceResult<EntityId> {
        self.validate_new_global_id(draft.global_id)?;
        self.check_reference_select(
            draft.resource,
            "RelatingResource",
            "IfcResourceSelect",
            &["IfcResource", "IfcTypeResource"],
            draft.resource,
        )?;
        validate_required_unique(draft.resource, "RelatedObjects", &draft.related_objects)?;
        if let Some(category) = draft.related_objects_type {
            validate_enum(
                self.schema,
                "IfcRelAssignsToResource",
                "RelatedObjectsType",
                category,
            )?;
        }
        for target in &draft.related_objects {
            self.check_reference(
                draft.resource,
                "RelatedObjects",
                "IfcObjectDefinition",
                *target,
            )?;
            if *target == draft.resource {
                return Err(ResourceError::SemanticViolation {
                    entity: None,
                    rule: "RESOURCE_ASSIGNMENT_NO_SELF_REFERENCE",
                });
            }
        }
        validate_object_assignment(
            self.model,
            self.schema,
            None,
            draft.related_objects_type,
            &draft.related_objects,
        )?;
        let entity = build_entity(
            self.schema,
            "IfcRelAssignsToResource",
            &[
                ("GlobalId", Some(text(draft.global_id))),
                ("Name", draft.name.map(text)),
                ("Description", draft.description.map(text)),
                ("RelatedObjects", Some(refs(&draft.related_objects))),
                (
                    "RelatedObjectsType",
                    draft
                        .related_objects_type
                        .map(|value| Value::Enum(value.into())),
                ),
                ("RelatingResource", Some(Value::Ref(draft.resource))),
            ],
        )?;
        self.commit_create(entity)
    }

    pub fn create_nesting(&mut self, draft: NestingDraft<'_>) -> ResourceResult<EntityId> {
        self.validate_new_global_id(draft.global_id)?;
        self.check_reference(
            draft.parent,
            "RelatingObject",
            "IfcConstructionResource",
            draft.parent,
        )?;
        validate_required_unique(draft.parent, "RelatedObjects", &draft.children)?;
        for child in &draft.children {
            self.check_reference(
                draft.parent,
                "RelatedObjects",
                "IfcConstructionResource",
                *child,
            )?;
            if *child == draft.parent {
                return Err(ResourceError::SemanticViolation {
                    entity: None,
                    rule: "RESOURCE_NESTING_NO_SELF_REFERENCE",
                });
            }
        }
        {
            let view = ResourceView::new(self.model, self.schema)?;
            for child in &draft.children {
                if view.parent_resource(*child)?.is_some() {
                    return Err(ResourceError::SemanticViolation {
                        entity: Some(*child),
                        rule: "IfcObject.Nests permits at most one resource parent",
                    });
                }
                if view
                    .descendants(*child, Budget::DEFAULT)?
                    .contains(&draft.parent)
                {
                    return Err(ResourceError::SemanticViolation {
                        entity: Some(*child),
                        rule: "resource nesting must remain acyclic",
                    });
                }
            }
        }
        let entity = build_entity(
            self.schema,
            "IfcRelNests",
            &[
                ("GlobalId", Some(text(draft.global_id))),
                ("Name", draft.name.map(text)),
                ("Description", draft.description.map(text)),
                ("RelatingObject", Some(Value::Ref(draft.parent))),
                ("RelatedObjects", Some(refs(&draft.children))),
            ],
        )?;
        self.commit_create(entity)
    }

    fn check_reference(
        &self,
        owner: EntityId,
        attribute: &'static str,
        expected: &'static str,
        target: EntityId,
    ) -> ResourceResult<()> {
        self.check_reference_select(owner, attribute, expected, &[expected], target)
    }

    fn check_reference_select(
        &self,
        owner: EntityId,
        attribute: &'static str,
        expected: &'static str,
        members: &[&str],
        target: EntityId,
    ) -> ResourceResult<()> {
        let entity = self
            .model
            .get(target)
            .ok_or(ResourceError::DanglingReference {
                entity: owner,
                attribute,
                target,
            })?;
        if !members
            .iter()
            .any(|member| self.schema.is_a(&entity.type_name, member))
        {
            return Err(ResourceError::WrongReferenceType {
                entity: owner,
                attribute,
                target,
                expected,
                actual: entity.type_name.to_string(),
            });
        }
        Ok(())
    }

    fn commit_create(&mut self, entity: Entity) -> ResourceResult<EntityId> {
        let mut transaction = Transaction::new(self.model);
        let expected = transaction.revision();
        let id = transaction.create(entity);
        transaction
            .commit(self.model)
            .map_err(|_| ResourceError::TransactionConflict {
                expected,
                actual: self.model.revision(),
            })?;
        Ok(id)
    }
}

fn resource_entity_type(kind: ResourceKind) -> &'static str {
    match kind {
        ResourceKind::Labor => "IfcLaborResource",
        ResourceKind::Equipment => "IfcConstructionEquipmentResource",
        ResourceKind::Crew => "IfcCrewResource",
        ResourceKind::Material => "IfcConstructionMaterialResource",
        ResourceKind::Product => "IfcConstructionProductResource",
        ResourceKind::Subcontract => "IfcSubContractResource",
    }
}

fn build_entity(
    schema: &Schema,
    entity_type: &'static str,
    values: &[(&'static str, Option<Value>)],
) -> ResourceResult<Entity> {
    if schema.entity(entity_type).is_none() {
        return Err(ResourceError::InvalidDraft {
            entity_type,
            attribute: "<entity>",
            expected: "entity declared by IFC4",
        });
    }
    let attribute_names = schema.attribute_names(entity_type);
    let mut attributes = vec![Value::Null; attribute_names.len()];
    for (name, value) in values {
        if let Some(value) = value {
            let slot = attribute_names
                .iter()
                .position(|candidate| candidate.eq_ignore_ascii_case(name))
                .ok_or(ResourceError::InvalidDraft {
                    entity_type,
                    attribute: name,
                    expected: "attribute declared by IFC4",
                })?;
            attributes[slot] = value.clone();
        }
    }
    Ok(Entity::new(entity_type, attributes))
}

fn validate_enum(
    schema: &Schema,
    entity_type: &'static str,
    attribute: &'static str,
    value: &str,
) -> ResourceResult<()> {
    let declared = schema
        .attributes(entity_type)
        .iter()
        .find(|candidate| candidate.name.eq_ignore_ascii_case(attribute))
        .and_then(|attribute| schema.type_def(&attribute.type_name))
        .is_some_and(|definition| {
            matches!(&definition.kind, TypeKind::Enumeration(values) if values.iter().any(|member| member == value))
        });
    if !declared {
        return Err(ResourceError::InvalidEnumeration {
            entity: None,
            attribute,
            value: value.to_owned(),
        });
    }
    Ok(())
}

fn validate_required_unique(
    owner: EntityId,
    attribute: &'static str,
    values: &[EntityId],
) -> ResourceResult<()> {
    if values.is_empty() {
        return Err(ResourceError::InvalidCardinality {
            entity: owner,
            attribute,
            minimum: 1,
            actual: 0,
        });
    }
    let mut unique = HashSet::with_capacity(values.len());
    for value in values {
        if !unique.insert(*value) {
            return Err(ResourceError::DuplicateReference {
                entity: owner,
                attribute,
                target: *value,
            });
        }
    }
    Ok(())
}

fn validate_global_id(value: &str) -> ResourceResult<()> {
    if value.len() != 22
        || !matches!(value.as_bytes().first(), Some(b'0'..=b'3'))
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'$')
    {
        return Err(ResourceError::InvalidGlobalId);
    }
    Ok(())
}

fn text(value: &str) -> Value {
    Value::Text(value.into())
}

fn refs(values: &[EntityId]) -> Value {
    Value::List(values.iter().copied().map(Value::Ref).collect())
}
