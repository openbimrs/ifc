//! Shared schema-resolved borrowed view primitives.

use std::collections::HashSet;

use ifc_model::{Entity, EntityId, Model, Value};
use ifc_schema::{ifc2x3, ifc4, ifc4x3, Schema, SchemaVersion, TypeKind};

use crate::error::{StructuralError, StructuralResult};
use crate::{
    AnalysisModel, LoadGroup, Member, ResultGroup, StaticLoad, StructuralAction,
    StructuralConnection,
};

/// Entry point for strict structural-analysis projections.
#[derive(Debug, Clone, Copy)]
pub struct StructuralView<'m, 's> {
    pub(crate) model: &'m Model,
    pub(crate) schema: &'s Schema,
}

impl<'m, 's> StructuralView<'m, 's> {
    #[must_use]
    pub fn new(model: &'m Model, schema: &'s Schema) -> Self {
        Self { model, schema }
    }

    #[must_use]
    pub fn schema(&self) -> &'s Schema {
        self.schema
    }

    pub fn analysis_model(&self, id: EntityId) -> StructuralResult<AnalysisModel<'m, 's>> {
        Ok(AnalysisModel::from_record(
            self.record(id, "IfcStructuralAnalysisModel")?,
        ))
    }

    pub fn load_group(&self, id: EntityId) -> StructuralResult<LoadGroup<'m, 's>> {
        Ok(LoadGroup::from_record(
            self.record(id, "IfcStructuralLoadGroup")?,
        ))
    }

    pub fn result_group(&self, id: EntityId) -> StructuralResult<ResultGroup<'m, 's>> {
        Ok(ResultGroup::from_record(
            self.record(id, "IfcStructuralResultGroup")?,
        ))
    }

    pub fn member(&self, id: EntityId) -> StructuralResult<Member<'m, 's>> {
        Member::from_record(self.record(id, "IfcStructuralMember")?)
    }

    pub fn connection(&self, id: EntityId) -> StructuralResult<StructuralConnection<'m, 's>> {
        StructuralConnection::from_record(self.record(id, "IfcStructuralConnection")?)
    }

    pub fn action(&self, id: EntityId) -> StructuralResult<StructuralAction<'m, 's>> {
        StructuralAction::from_record(self.record(id, "IfcStructuralAction")?)
    }

    pub fn load(&self, id: EntityId) -> StructuralResult<StaticLoad<'m, 's>> {
        StaticLoad::from_record(self.record(id, "IfcStructuralLoad")?)
    }

    pub fn static_load(&self, id: EntityId) -> StructuralResult<StaticLoad<'m, 's>> {
        self.load(id)
    }

    pub(crate) fn record(
        &self,
        id: EntityId,
        expected: &'static str,
    ) -> StructuralResult<Record<'m, 's>> {
        Record::new(self.model, self.schema, id, expected)
    }

    pub(crate) fn ids_of_ancestor(&self, ancestor: &str) -> Vec<EntityId> {
        let matching_types: HashSet<_> = self
            .model
            .type_histogram()
            .into_iter()
            .filter_map(|(type_name, _)| self.schema.is_a(type_name, ancestor).then_some(type_name))
            .collect();
        self.model
            .iter()
            .filter_map(|(id, entity)| matching_types.contains(&*entity.type_name).then_some(id))
            .collect()
    }
}

impl<'m> StructuralView<'m, 'static> {
    pub fn for_model(model: &'m Model) -> StructuralResult<Self> {
        let token = match model.header().schema.as_slice() {
            [] => return Err(StructuralError::MissingSchema),
            [token] => token,
            tokens => {
                return Err(StructuralError::AmbiguousSchema {
                    tokens: tokens.to_vec(),
                })
            }
        };
        let version = SchemaVersion::from_header_token(token).ok_or_else(|| {
            StructuralError::UnsupportedSchema {
                token: token.clone(),
            }
        })?;
        let schema = match version {
            SchemaVersion::Ifc2x3 => ifc2x3(),
            SchemaVersion::Ifc4 => ifc4(),
            SchemaVersion::Ifc4x3 => ifc4x3(),
        };
        Ok(Self::new(model, schema))
    }
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
    ) -> StructuralResult<Self> {
        let entity = model
            .get(id)
            .ok_or(StructuralError::EntityNotFound { id })?;
        if !schema.is_a(&entity.type_name, expected) {
            return Err(StructuralError::WrongType {
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

    pub(crate) fn has_attribute(&self, attribute: &str) -> bool {
        self.schema
            .attribute_names(&self.entity.type_name)
            .iter()
            .any(|name| name.eq_ignore_ascii_case(attribute))
    }

    fn slot(&self, attribute: &'static str) -> StructuralResult<usize> {
        self.schema
            .attribute_names(&self.entity.type_name)
            .iter()
            .position(|name| name.eq_ignore_ascii_case(attribute))
            .ok_or(StructuralError::MissingAttribute {
                entity: self.id,
                attribute,
            })
    }

    pub(crate) fn value(&self, attribute: &'static str) -> StructuralResult<&'m Value> {
        let slot = self.slot(attribute)?;
        self.entity
            .attributes
            .get(slot)
            .ok_or(StructuralError::MissingAttribute {
                entity: self.id,
                attribute,
            })
    }

    pub(crate) fn optional_text(
        &self,
        attribute: &'static str,
    ) -> StructuralResult<Option<&'m str>> {
        match self.value(attribute)?.unwrap_typed() {
            Value::Null | Value::Derived => Ok(None),
            Value::Text(value) => Ok(Some(value)),
            _ => Err(self.invalid(attribute, "text or null")),
        }
    }

    pub(crate) fn require_object_type_if(
        &self,
        condition: bool,
        rule: &'static str,
    ) -> StructuralResult<()> {
        if condition
            && self
                .optional_text("ObjectType")?
                .is_none_or(|value| value.trim().is_empty())
        {
            return Err(StructuralError::SemanticViolation {
                entity: Some(self.id),
                rule,
            });
        }
        Ok(())
    }

    pub(crate) fn required_enum(&self, attribute: &'static str) -> StructuralResult<&'m str> {
        match self.value(attribute)?.unwrap_typed() {
            Value::Enum(value) if self.declares_enum_member(attribute, value) => Ok(value),
            _ => Err(self.invalid(attribute, "enumeration")),
        }
    }

    pub(crate) fn optional_enum(
        &self,
        attribute: &'static str,
    ) -> StructuralResult<Option<&'m str>> {
        match self.value(attribute)?.unwrap_typed() {
            Value::Null | Value::Derived => Ok(None),
            Value::Enum(value) if self.declares_enum_member(attribute, value) => Ok(Some(value)),
            _ => Err(self.invalid(attribute, "enumeration or null")),
        }
    }

    fn declares_enum_member(&self, attribute: &str, value: &str) -> bool {
        let attributes = self.schema.attributes(&self.entity.type_name);
        let Some(declaration) = attributes
            .iter()
            .find(|declaration| declaration.name.eq_ignore_ascii_case(attribute))
        else {
            return false;
        };
        let mut type_name = declaration.type_name.as_str();
        for _ in 0..16 {
            let Some(type_def) = self.schema.type_def(type_name) else {
                return false;
            };
            match &type_def.kind {
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

    pub(crate) fn optional_bool(&self, attribute: &'static str) -> StructuralResult<Option<bool>> {
        match self.value(attribute)?.unwrap_typed() {
            Value::Null | Value::Derived => Ok(None),
            Value::Bool(value) => Ok(Some(*value)),
            _ => Err(self.invalid(attribute, "boolean or null")),
        }
    }

    pub(crate) fn required_bool(&self, attribute: &'static str) -> StructuralResult<bool> {
        self.optional_bool(attribute)?
            .ok_or_else(|| self.invalid(attribute, "boolean"))
    }

    pub(crate) fn optional_number(&self, attribute: &'static str) -> StructuralResult<Option<f64>> {
        let value = match self.value(attribute)?.unwrap_typed() {
            Value::Null | Value::Derived => Ok(None),
            Value::Integer(value) => Ok(Some(*value as f64)),
            Value::Real(value) => Ok(Some(*value)),
            _ => Err(self.invalid(attribute, "number or null")),
        }?;
        if value.is_some_and(|number| !number.is_finite()) {
            return Err(self.invalid(attribute, "finite number or null"));
        }
        Ok(value)
    }

    pub(crate) fn optional_ref(
        &self,
        attribute: &'static str,
        expected: &'static str,
    ) -> StructuralResult<Option<EntityId>> {
        let target = match self.value(attribute)?.unwrap_typed() {
            Value::Null | Value::Derived => return Ok(None),
            Value::Ref(target) => *target,
            _ => return Err(self.invalid(attribute, "entity reference or null")),
        };
        self.check_reference(attribute, target, &[expected], expected)?;
        Ok(Some(target))
    }

    pub(crate) fn required_ref(
        &self,
        attribute: &'static str,
        expected: &'static str,
    ) -> StructuralResult<EntityId> {
        self.optional_ref(attribute, expected)?
            .ok_or_else(|| self.invalid(attribute, "entity reference"))
    }

    pub(crate) fn required_ref_select(
        &self,
        attribute: &'static str,
        expected: &'static str,
        members: &[&str],
    ) -> StructuralResult<EntityId> {
        let target = match self.value(attribute)?.unwrap_typed() {
            Value::Ref(target) => *target,
            _ => return Err(self.invalid(attribute, "entity reference")),
        };
        self.check_reference(attribute, target, members, expected)?;
        Ok(target)
    }

    pub(crate) fn optional_set_refs(
        &self,
        attribute: &'static str,
        expected: &'static str,
        minimum_when_present: usize,
    ) -> StructuralResult<Vec<EntityId>> {
        let values = match self.value(attribute)?.unwrap_typed() {
            Value::Null | Value::Derived => return Ok(Vec::new()),
            Value::List(values) => values,
            _ => return Err(self.invalid(attribute, "aggregate of entity references or null")),
        };
        if values.len() < minimum_when_present {
            return Err(StructuralError::InvalidCardinality {
                entity: self.id,
                attribute,
                minimum: minimum_when_present,
                maximum: None,
                actual: values.len(),
            });
        }
        let mut targets = Vec::with_capacity(values.len());
        let mut unique = HashSet::with_capacity(values.len());
        for value in values {
            let Value::Ref(target) = value.unwrap_typed() else {
                return Err(self.invalid(attribute, "aggregate of entity references"));
            };
            if !unique.insert(*target) {
                return Err(self.invalid(attribute, "SET of unique entity references"));
            }
            self.check_reference(attribute, *target, &[expected], expected)?;
            targets.push(*target);
        }
        Ok(targets)
    }

    pub(crate) fn required_set_refs_select(
        &self,
        attribute: &'static str,
        expected: &'static str,
        members: &[&str],
        minimum: usize,
    ) -> StructuralResult<Vec<EntityId>> {
        let values = match self.value(attribute)?.unwrap_typed() {
            Value::List(values) => values,
            _ => return Err(self.invalid(attribute, "aggregate of entity references")),
        };
        if values.len() < minimum {
            return Err(StructuralError::InvalidCardinality {
                entity: self.id,
                attribute,
                minimum,
                maximum: None,
                actual: values.len(),
            });
        }
        let mut targets = Vec::with_capacity(values.len());
        let mut unique = HashSet::with_capacity(values.len());
        for value in values {
            let Value::Ref(target) = value.unwrap_typed() else {
                return Err(self.invalid(attribute, "aggregate of entity references"));
            };
            if !unique.insert(*target) {
                return Err(self.invalid(attribute, "SET of unique entity references"));
            }
            self.check_reference(attribute, *target, members, expected)?;
            targets.push(*target);
        }
        Ok(targets)
    }

    fn check_reference(
        &self,
        attribute: &'static str,
        target: EntityId,
        members: &[&str],
        expected: &'static str,
    ) -> StructuralResult<()> {
        let target_entity = self
            .model
            .get(target)
            .ok_or(StructuralError::DanglingReference {
                entity: self.id,
                attribute,
                target,
            })?;
        if !members
            .iter()
            .any(|member| self.schema.is_a(&target_entity.type_name, member))
        {
            return Err(StructuralError::WrongReferenceType {
                entity: self.id,
                attribute,
                target,
                expected,
                actual: target_entity.type_name.to_string(),
            });
        }
        Ok(())
    }

    fn invalid(&self, attribute: &'static str, expected: &'static str) -> StructuralError {
        StructuralError::InvalidValue {
            entity: self.id,
            attribute,
            expected,
        }
    }
}
