//! Shared schema-resolved borrowed view primitives.

use std::collections::HashSet;

use ifc_model::{Entity, EntityId, Model, Value};
use ifc_schema::{ifc4, Schema, SchemaVersion, TypeKind};

use crate::error::{ResourceError, ResourceResult};
use crate::{ConstructionResource, ResourceTime};

#[derive(Debug, Clone, Copy)]
pub struct ResourceView<'m, 's> {
    pub(crate) model: &'m Model,
    pub(crate) schema: &'s Schema,
}

impl<'m, 's> ResourceView<'m, 's> {
    pub fn new(model: &'m Model, schema: &'s Schema) -> ResourceResult<Self> {
        if schema.version() != Some(SchemaVersion::Ifc4) {
            return Err(ResourceError::UnsupportedSchema {
                token: schema.name().to_owned(),
            });
        }
        match model.header().schema.as_slice() {
            [] => {}
            [token] if SchemaVersion::from_header_token(token) == Some(SchemaVersion::Ifc4) => {}
            [token] => {
                return Err(ResourceError::UnsupportedSchema {
                    token: token.clone(),
                });
            }
            tokens => {
                return Err(ResourceError::AmbiguousSchema {
                    tokens: tokens.to_vec(),
                });
            }
        }
        Ok(Self { model, schema })
    }

    #[must_use]
    pub fn schema(&self) -> &'s Schema {
        self.schema
    }

    pub fn resource(&self, id: EntityId) -> ResourceResult<ConstructionResource<'m, 's>> {
        ConstructionResource::from_record(self.record(id, "IfcConstructionResource")?)
    }

    pub fn resource_time(&self, id: EntityId) -> ResourceResult<ResourceTime<'m, 's>> {
        Ok(ResourceTime::from_record(
            self.record(id, "IfcResourceTime")?,
        ))
    }

    pub(crate) fn record(
        &self,
        id: EntityId,
        expected: &'static str,
    ) -> ResourceResult<Record<'m, 's>> {
        Record::new(self.model, self.schema, id, expected)
    }

    pub(crate) fn ids_of_ancestor(&self, ancestor: &str) -> Vec<EntityId> {
        self.model
            .iter()
            .filter_map(|(id, entity)| self.schema.is_a(&entity.type_name, ancestor).then_some(id))
            .collect()
    }
}

impl<'m> ResourceView<'m, 'static> {
    pub fn for_model(model: &'m Model) -> ResourceResult<Self> {
        let token = match model.header().schema.as_slice() {
            [] => return Err(ResourceError::MissingSchema),
            [token] => token,
            tokens => {
                return Err(ResourceError::AmbiguousSchema {
                    tokens: tokens.to_vec(),
                });
            }
        };
        if SchemaVersion::from_header_token(token) != Some(SchemaVersion::Ifc4) {
            return Err(ResourceError::UnsupportedSchema {
                token: token.clone(),
            });
        }
        Self::new(model, ifc4())
    }
}

pub(crate) fn validate_object_assignment(
    model: &Model,
    schema: &Schema,
    relation: Option<EntityId>,
    related_objects_type: Option<&str>,
    related_objects: &[EntityId],
) -> ResourceResult<()> {
    let Some(category) = related_objects_type else {
        return Ok(());
    };
    let expected = if category.eq_ignore_ascii_case("NOTDEFINED") {
        return Ok(());
    } else if category.eq_ignore_ascii_case("PRODUCT") {
        "IfcProduct"
    } else if category.eq_ignore_ascii_case("PROCESS") {
        "IfcProcess"
    } else if category.eq_ignore_ascii_case("CONTROL") {
        "IfcControl"
    } else if category.eq_ignore_ascii_case("RESOURCE") {
        "IfcResource"
    } else if category.eq_ignore_ascii_case("ACTOR") {
        "IfcActor"
    } else if category.eq_ignore_ascii_case("GROUP") {
        "IfcGroup"
    } else if category.eq_ignore_ascii_case("PROJECT") {
        "IfcProject"
    } else {
        return Err(ResourceError::InvalidEnumeration {
            entity: relation,
            attribute: "RelatedObjectsType",
            value: category.to_owned(),
        });
    };
    if related_objects.iter().any(|target| {
        model
            .get(*target)
            .is_none_or(|entity| !schema.is_a(&entity.type_name, expected))
    }) {
        return Err(ResourceError::SemanticViolation {
            entity: relation,
            rule: "IfcRelAssigns.WR1_IfcCorrectObjectAssignment",
        });
    }
    Ok(())
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Record<'m, 's> {
    pub(crate) model: &'m Model,
    pub(crate) schema: &'s Schema,
    pub(crate) id: EntityId,
    pub(crate) entity: &'m Entity,
}

impl<'m, 's> Record<'m, 's> {
    pub(crate) fn new(
        model: &'m Model,
        schema: &'s Schema,
        id: EntityId,
        expected: &'static str,
    ) -> ResourceResult<Self> {
        let entity = model.get(id).ok_or(ResourceError::EntityNotFound { id })?;
        if !schema.is_a(&entity.type_name, expected) {
            return Err(ResourceError::WrongType {
                id,
                expected,
                actual: entity.type_name.to_string(),
            });
        }
        Ok(Self {
            model,
            schema,
            id,
            entity,
        })
    }

    fn slot(&self, attribute: &'static str) -> ResourceResult<usize> {
        self.schema
            .attribute_names(&self.entity.type_name)
            .iter()
            .position(|name| name.eq_ignore_ascii_case(attribute))
            .ok_or(ResourceError::MissingAttribute {
                entity: self.id,
                attribute,
            })
    }

    pub(crate) fn value(&self, attribute: &'static str) -> ResourceResult<&'m Value> {
        self.entity
            .attributes
            .get(self.slot(attribute)?)
            .ok_or(ResourceError::MissingAttribute {
                entity: self.id,
                attribute,
            })
    }

    pub(crate) fn optional_text(&self, attribute: &'static str) -> ResourceResult<Option<&'m str>> {
        match self.value(attribute)?.unwrap_typed() {
            Value::Null | Value::Derived => Ok(None),
            Value::Text(value) => Ok(Some(value)),
            _ => Err(self.invalid(attribute, "text or null")),
        }
    }

    pub(crate) fn optional_bool(&self, attribute: &'static str) -> ResourceResult<Option<bool>> {
        match self.value(attribute)?.unwrap_typed() {
            Value::Null | Value::Derived => Ok(None),
            Value::Bool(value) => Ok(Some(*value)),
            _ => Err(self.invalid(attribute, "boolean or null")),
        }
    }

    pub(crate) fn optional_positive_number(
        &self,
        attribute: &'static str,
    ) -> ResourceResult<Option<f64>> {
        let value = match self.value(attribute)?.unwrap_typed() {
            Value::Null | Value::Derived => return Ok(None),
            Value::Integer(value) => *value as f64,
            Value::Real(value) => *value,
            _ => return Err(self.invalid(attribute, "finite positive number or null")),
        };
        if !value.is_finite() || value <= 0.0 {
            return Err(self.invalid(attribute, "finite positive number or null"));
        }
        Ok(Some(value))
    }

    pub(crate) fn optional_enum(&self, attribute: &'static str) -> ResourceResult<Option<&'m str>> {
        let value = match self.value(attribute)?.unwrap_typed() {
            Value::Null | Value::Derived => return Ok(None),
            Value::Enum(value) => value,
            _ => return Err(self.invalid(attribute, "declared enumeration or null")),
        };
        if !self.declares_enum_member(attribute, value) {
            return Err(ResourceError::InvalidEnumeration {
                entity: Some(self.id),
                attribute,
                value: value.to_string(),
            });
        }
        Ok(Some(value))
    }

    fn declares_enum_member(&self, attribute: &str, value: &str) -> bool {
        let declarations = self.schema.attributes(&self.entity.type_name);
        let Some(declaration) = declarations
            .iter()
            .find(|candidate| candidate.name.eq_ignore_ascii_case(attribute))
        else {
            return false;
        };
        let mut type_name = declaration.type_name.as_str();
        for _ in 0..16 {
            let Some(definition) = self.schema.type_def(type_name) else {
                return false;
            };
            match &definition.kind {
                TypeKind::Enumeration(members) => {
                    return members
                        .iter()
                        .any(|member| member.eq_ignore_ascii_case(value));
                }
                TypeKind::Defined(alias) => type_name = alias,
                TypeKind::Select(_) => return false,
            }
        }
        false
    }

    pub(crate) fn optional_ref(
        &self,
        attribute: &'static str,
        expected: &'static str,
    ) -> ResourceResult<Option<EntityId>> {
        let target = match self.value(attribute)?.unwrap_typed() {
            Value::Null | Value::Derived => return Ok(None),
            Value::Ref(target) => *target,
            _ => return Err(self.invalid(attribute, "entity reference or null")),
        };
        self.check_reference(attribute, target, &[expected], expected)?;
        Ok(Some(target))
    }

    pub(crate) fn required_ref_select(
        &self,
        attribute: &'static str,
        expected: &'static str,
        members: &[&str],
    ) -> ResourceResult<EntityId> {
        let Value::Ref(target) = self.value(attribute)?.unwrap_typed() else {
            return Err(self.invalid(attribute, "entity reference"));
        };
        self.check_reference(attribute, *target, members, expected)?;
        Ok(*target)
    }

    pub(crate) fn refs(
        &self,
        attribute: &'static str,
        expected: &'static str,
        minimum: usize,
        optional: bool,
        unique: bool,
    ) -> ResourceResult<Vec<EntityId>> {
        let values = match self.value(attribute)?.unwrap_typed() {
            Value::Null | Value::Derived if optional => return Ok(Vec::new()),
            Value::List(values) => values,
            _ => return Err(self.invalid(attribute, "aggregate of entity references")),
        };
        if values.len() < minimum {
            return Err(ResourceError::InvalidCardinality {
                entity: self.id,
                attribute,
                minimum,
                actual: values.len(),
            });
        }
        let mut targets = Vec::with_capacity(values.len());
        let mut seen = HashSet::with_capacity(values.len());
        for value in values {
            let Value::Ref(target) = value.unwrap_typed() else {
                return Err(self.invalid(attribute, "aggregate of entity references"));
            };
            if unique && !seen.insert(*target) {
                return Err(ResourceError::DuplicateReference {
                    entity: self.id,
                    attribute,
                    target: *target,
                });
            }
            self.check_reference(attribute, *target, &[expected], expected)?;
            targets.push(*target);
        }
        Ok(targets)
    }

    pub(crate) fn check_reference(
        &self,
        attribute: &'static str,
        target: EntityId,
        members: &[&str],
        expected: &'static str,
    ) -> ResourceResult<()> {
        let entity = self
            .model
            .get(target)
            .ok_or(ResourceError::DanglingReference {
                entity: self.id,
                attribute,
                target,
            })?;
        if !members
            .iter()
            .any(|member| self.schema.is_a(&entity.type_name, member))
        {
            return Err(ResourceError::WrongReferenceType {
                entity: self.id,
                attribute,
                target,
                expected,
                actual: entity.type_name.to_string(),
            });
        }
        Ok(())
    }

    pub(crate) fn require_object_type_if(
        &self,
        condition: bool,
        rule: &'static str,
    ) -> ResourceResult<()> {
        if condition
            && self
                .optional_text("ObjectType")?
                .is_none_or(|value| value.trim().is_empty())
        {
            return Err(ResourceError::SemanticViolation {
                entity: Some(self.id),
                rule,
            });
        }
        Ok(())
    }

    fn invalid(&self, attribute: &'static str, expected: &'static str) -> ResourceError {
        ResourceError::InvalidValue {
            entity: self.id,
            attribute,
            expected,
        }
    }
}
