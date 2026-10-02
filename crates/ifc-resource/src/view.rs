//! Shared schema-resolved borrowed view primitives.
//!
//! # Supported releases
//!
//! IFC4 ADD2 TC1 and IFC4X3 ADD2 declare identical attribute layouts for
//! every entity this crate projects (checked against the normative `.exp`
//! files); IFC4X3 only adds `IfcQuantityNumber`, which is not one of the six
//! projected simple-quantity kinds. One code path therefore serves both.
//!
//! IFC2X3 TC1 is read through its own bundled table (#237). Every attribute
//! is resolved by name against that table, so a record is never read with
//! IFC4's slots. Where IFC2X3 declares the same concept under another name
//! with the same declared type and position, the shared accessor reads it:
//! `IfcConstructionResource.ResourceIdentifier` and `IfcPerson.Id` /
//! `IfcOrganization.Id` (all `IfcIdentifier`) answer `identification()`,
//! and `IfcInventory.InventoryType` (`IfcInventoryTypeEnum`) answers
//! `predefined_type()`. What IFC2X3 lacks -- resource types,
//! `IfcResourceTime`, `LongDescription`, `Usage`, `BaseCosts`,
//! `PredefinedType` on resources, `IfcPhysicalSimpleQuantity.Formula` -- is
//! [`ResourceError::NotInSchema`], as is an attribute IFC2X3 declares with a
//! different type (`BaseQuantity : IfcMeasureWithUnit`,
//! `IfcInventory.LastUpdateDate : IfcCalendarDate`), which its own
//! IFC2X3-only accessor reads instead. Authoring stays IFC4/IFC4X3:
//! `ResourceEditor` refuses an IFC2X3 model.
//!
//! IFC4X1 and IFC4X2 are bundled by `ifc-schema` but not verified here, so
//! they are refused with `UnsupportedSchema`, never read as a neighbour.

use std::collections::HashSet;

use ifc_model::{Entity, EntityId, Model, Value};
use ifc_schema::{ifc2x3, ifc4, ifc4x3, Schema, SchemaVersion, TypeKind};

use crate::error::{ResourceError, ResourceResult};
use crate::{ConstructionResource, ResourceTime};

#[derive(Debug, Clone, Copy)]
/// Borrowed, schema-resolved entry point for the bounded resource slice
/// (IFC2X3 TC1, IFC4 ADD2 TC1, IFC4X3 ADD2): pairs a model with the schema
/// selected for it and exposes per-entity projection and query methods.
pub struct ResourceView<'m, 's> {
    pub(crate) model: &'m Model,
    pub(crate) schema: &'s Schema,
}

impl<'m, 's> ResourceView<'m, 's> {
    /// Builds a view over `model` using an explicit `schema`, failing if
    /// `schema` is not IFC2X3, IFC4 or IFC4X3, or does not match the model's
    /// declared `FILE_SCHEMA` token.
    pub fn new(model: &'m Model, schema: &'s Schema) -> ResourceResult<Self> {
        let Some(version) = schema.version() else {
            return Err(ResourceError::UnsupportedSchema {
                token: schema.name().to_owned(),
            });
        };
        if !matches!(
            version,
            SchemaVersion::Ifc2x3 | SchemaVersion::Ifc4 | SchemaVersion::Ifc4x3
        ) {
            return Err(ResourceError::UnsupportedSchema {
                token: schema.name().to_owned(),
            });
        }
        match model.header().schema.as_slice() {
            [] => {}
            [token] if SchemaVersion::from_header_token(token) == Some(version) => {}
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

    /// The bundled schema this view resolves attributes against.
    #[must_use]
    pub fn schema(&self) -> &'s Schema {
        self.schema
    }

    /// Projects a concrete `IfcConstructionResource` occurrence by entity
    /// id.
    pub fn resource(&self, id: EntityId) -> ResourceResult<ConstructionResource<'m, 's>> {
        ConstructionResource::from_record(self.record(id, "IfcConstructionResource")?)
    }

    /// Projects an `IfcResourceTime` by entity id.
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

    /// Fail with [`ResourceError::NotInSchema`] unless the bound release
    /// declares `entity_type`.
    pub(crate) fn require_entity(&self, entity_type: &'static str) -> ResourceResult<()> {
        require_entity(self.schema, None, entity_type)
    }

    /// Whether the view is bound to IFC2X3 TC1.
    pub(crate) fn is_ifc2x3(&self) -> bool {
        self.schema.version() == Some(SchemaVersion::Ifc2x3)
    }

    pub(crate) fn ids_of_ancestor(&self, ancestor: &str) -> Vec<EntityId> {
        self.model
            .iter()
            .filter_map(|(id, entity)| self.schema.is_a(&entity.type_name, ancestor).then_some(id))
            .collect()
    }
}

impl<'m> ResourceView<'m, 'static> {
    /// Selects the bundled schema matching the model's declared
    /// `FILE_SCHEMA` token (IFC2X3 TC1, IFC4 ADD2 TC1 or IFC4X3 ADD2),
    /// failing if the header names no schema, more than one, or an
    /// unsupported one.
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
        let version = SchemaVersion::from_header_token(token).ok_or_else(|| {
            ResourceError::UnsupportedSchema {
                token: token.clone(),
            }
        })?;
        let schema = match version {
            SchemaVersion::Ifc2x3 => ifc2x3(),
            SchemaVersion::Ifc4 => ifc4(),
            SchemaVersion::Ifc4x3 => ifc4x3(),
            // IFC4X1 and IFC4X2 are bundled by ifc-schema but not verified
            // here. Refused, never aliased.
            _ => {
                return Err(ResourceError::UnsupportedSchema {
                    token: token.clone(),
                });
            }
            // A release added to `SchemaVersion` later has no reviewed
            // resource layout: refused rather than read with another's.
            #[allow(unreachable_patterns)]
            _ => {
                return Err(ResourceError::UnsupportedSchema {
                    token: token.clone(),
                });
            }
        };
        Self::new(model, schema)
    }
}

fn require_entity(
    schema: &Schema,
    entity: Option<EntityId>,
    entity_type: &'static str,
) -> ResourceResult<()> {
    if schema.entity(entity_type).is_none() {
        return Err(ResourceError::NotInSchema {
            schema: schema.name().to_owned(),
            entity,
            entity_type: entity_type.to_owned(),
            attribute: None,
        });
    }
    Ok(())
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
        require_entity(schema, Some(id), expected)?;
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
            .ok_or_else(|| self.not_in_schema(attribute))
    }

    /// Whether this record is read through the IFC2X3 TC1 table.
    pub(crate) fn is_ifc2x3(&self) -> bool {
        self.schema.version() == Some(SchemaVersion::Ifc2x3)
    }

    /// The refusal for an attribute the bound release does not declare on
    /// this record's type in the form the caller reads it.
    pub(crate) fn not_in_schema(&self, attribute: &'static str) -> ResourceError {
        ResourceError::NotInSchema {
            schema: self.schema.name().to_owned(),
            entity: Some(self.id),
            entity_type: self.entity.type_name.to_string(),
            attribute: Some(attribute),
        }
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

    pub(crate) fn required_text(&self, attribute: &'static str) -> ResourceResult<&'m str> {
        match self.value(attribute)?.unwrap_typed() {
            Value::Text(value) => Ok(value),
            _ => Err(self.invalid(attribute, "text")),
        }
    }

    pub(crate) fn optional_text_list(
        &self,
        attribute: &'static str,
        minimum: usize,
    ) -> ResourceResult<Vec<&'m str>> {
        let values = match self.value(attribute)?.unwrap_typed() {
            Value::Null | Value::Derived => return Ok(Vec::new()),
            Value::List(values) => values,
            _ => return Err(self.invalid(attribute, "aggregate of text or null")),
        };
        if values.len() < minimum {
            return Err(ResourceError::InvalidCardinality {
                entity: self.id,
                attribute,
                minimum,
                actual: values.len(),
            });
        }
        values
            .iter()
            .map(|value| match value.unwrap_typed() {
                Value::Text(value) => Ok(value.as_ref()),
                _ => Err(self.invalid(attribute, "aggregate of text or null")),
            })
            .collect()
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

    pub(crate) fn optional_finite_number(
        &self,
        attribute: &'static str,
    ) -> ResourceResult<Option<f64>> {
        let value = match self.value(attribute)?.unwrap_typed() {
            Value::Null | Value::Derived => return Ok(None),
            Value::Integer(value) => *value as f64,
            Value::Real(value) => *value,
            _ => return Err(self.invalid(attribute, "finite number or null")),
        };
        if !value.is_finite() {
            return Err(self.invalid(attribute, "finite number or null"));
        }
        Ok(Some(value))
    }

    pub(crate) fn required_non_negative_number(
        &self,
        attribute: &'static str,
    ) -> ResourceResult<f64> {
        let value = match self.value(attribute)?.unwrap_typed() {
            Value::Integer(value) => *value as f64,
            Value::Real(value) => *value,
            _ => return Err(self.invalid(attribute, "finite non-negative number")),
        };
        if !value.is_finite() || value < 0.0 {
            return Err(self.invalid(attribute, "finite non-negative number"));
        }
        Ok(value)
    }

    pub(crate) fn required_enum(&self, attribute: &'static str) -> ResourceResult<&'m str> {
        let value = match self.value(attribute)?.unwrap_typed() {
            Value::Enum(value) => value,
            _ => return Err(self.invalid(attribute, "declared enumeration")),
        };
        if !self.declares_enum_member(attribute, value) {
            return Err(ResourceError::InvalidEnumeration {
                entity: Some(self.id),
                attribute,
                value: value.to_string(),
            });
        }
        Ok(value)
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
                _ => return false,
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

    pub(crate) fn required_ref(
        &self,
        attribute: &'static str,
        expected: &'static str,
    ) -> ResourceResult<EntityId> {
        let Value::Ref(target) = self.value(attribute)?.unwrap_typed() else {
            return Err(self.invalid(attribute, "entity reference"));
        };
        self.check_reference(attribute, *target, &[expected], expected)?;
        Ok(*target)
    }

    pub(crate) fn optional_ref_select(
        &self,
        attribute: &'static str,
        expected: &'static str,
        members: &[&str],
    ) -> ResourceResult<Option<EntityId>> {
        let target = match self.value(attribute)?.unwrap_typed() {
            Value::Null | Value::Derived => return Ok(None),
            Value::Ref(target) => *target,
            _ => return Err(self.invalid(attribute, "entity reference or null")),
        };
        self.check_reference(attribute, target, members, expected)?;
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
        self.refs_select(attribute, expected, &[expected], minimum, optional, unique)
    }

    pub(crate) fn refs_select(
        &self,
        attribute: &'static str,
        expected: &'static str,
        members: &[&str],
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
            self.check_reference(attribute, *target, members, expected)?;
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

#[cfg(test)]
mod intermediate_release_tests {
    use super::*;

    /// IFC4X1 and IFC4X2 have bundled tables but no verified layout here:
    /// refused with the unsupported-schema error, never read as IFC4/IFC4X3.
    #[test]
    fn ifc4x1_and_ifc4x2_are_refused_not_aliased() {
        for token in ["IFC4X1", "IFC4X2"] {
            let mut model = Model::new();
            model.header_mut().schema = vec![token.to_owned()];
            assert!(
                matches!(
                    ResourceView::for_model(&model),
                    Err(ResourceError::UnsupportedSchema { token: found }) if found == token
                ),
                "{token} must be refused"
            );
        }
    }
}
