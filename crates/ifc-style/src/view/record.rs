//! The schema-resolved attribute reader shared by every presentation projection.
//!
//! Split out of `view.rs`: the entry-point struct enumerates projections while
//! this module owns typed attribute access and the EXPRESS bound checks the
//! projections rely on. Both halves grow independently.

use ifc_model::{Entity, EntityId, Model, Value};
use ifc_schema::Schema;

use crate::error::{StyleError, StyleResult};

#[derive(Debug, Clone, Copy)]
pub(crate) struct Record<'m, 's> {
    pub(crate) id: EntityId,
    pub(crate) entity: &'m Entity,
    pub(crate) model: &'m Model,
    pub(crate) schema: &'s Schema,
}

impl<'m, 's> Record<'m, 's> {
    pub(crate) fn has_attribute(&self, name: &str) -> bool {
        self.slot(name).is_some()
    }

    /// The entity's declared IFC type name.
    pub(crate) fn type_name(&self) -> &'m str {
        &self.entity.type_name
    }

    /// Whether `declared` is `expected` or a subtype of it, per the schema.
    pub(crate) fn is_a(&self, declared: &str, expected: &str) -> bool {
        self.schema.is_a(declared, expected)
    }

    /// The entity is not the type (or subtype family) the caller required.
    pub(crate) fn wrong_type(&self, expected: &'static str) -> StyleError {
        StyleError::WrongEntityType {
            id: self.id,
            expected,
            actual: self.entity.type_name.to_string(),
        }
    }

    /// Two attributes that must be index-aligned hold different lengths.
    pub(crate) fn mismatched_lists(
        &self,
        attribute: &'static str,
        len: usize,
        other_attribute: &'static str,
        other_len: usize,
    ) -> StyleError {
        StyleError::InvalidValue {
            entity: self.entity.type_name.to_string(),
            id: self.id,
            attribute,
            value: format!("{len} value(s) cannot pair with {other_len} in {other_attribute}"),
        }
    }

    fn slot(&self, name: &str) -> Option<usize> {
        self.schema
            .attributes(&self.entity.type_name)
            .iter()
            .position(|attribute| attribute.name.eq_ignore_ascii_case(name))
    }

    pub(crate) fn value(&self, attribute: &'static str) -> StyleResult<&'m Value> {
        let Some(slot) = self.slot(attribute) else {
            return Err(StyleError::UnsupportedAttribute {
                schema: self.schema.name().to_owned(),
                entity: "presentation entity",
                attribute,
            });
        };
        self.entity
            .attribute(slot)
            .ok_or_else(|| StyleError::MissingAttribute {
                entity: self.entity.type_name.to_string(),
                id: self.id,
                attribute,
            })
    }

    pub(crate) fn required_text(&self, attribute: &'static str) -> StyleResult<&'m str> {
        let value = self.value(attribute)?;
        match value.unwrap_typed() {
            Value::Text(text) => Ok(text),
            Value::Null | Value::Derived => Err(self.missing(attribute)),
            other => Err(self.invalid(attribute, other)),
        }
    }

    pub(crate) fn optional_text(&self, attribute: &'static str) -> StyleResult<Option<&'m str>> {
        if !self.has_attribute(attribute) {
            return Ok(None);
        }
        match self.value(attribute)?.unwrap_typed() {
            Value::Null | Value::Derived => Ok(None),
            Value::Text(text) => Ok(Some(text)),
            other => Err(self.invalid(attribute, other)),
        }
    }

    pub(crate) fn required_enum(&self, attribute: &'static str) -> StyleResult<&'m str> {
        match self.value(attribute)?.unwrap_typed() {
            Value::Enum(value) => Ok(value),
            Value::Null | Value::Derived => Err(self.missing(attribute)),
            other => Err(self.invalid(attribute, other)),
        }
    }

    pub(crate) fn optional_enum(&self, attribute: &'static str) -> StyleResult<Option<&'m str>> {
        if !self.has_attribute(attribute) {
            return Ok(None);
        }
        match self.value(attribute)?.unwrap_typed() {
            Value::Null | Value::Derived => Ok(None),
            Value::Enum(value) => Ok(Some(value)),
            other => Err(self.invalid(attribute, other)),
        }
    }

    pub(crate) fn required_number(&self, attribute: &'static str) -> StyleResult<f64> {
        match self.value(attribute)?.unwrap_typed() {
            Value::Real(value) => Ok(*value),
            Value::Integer(value) => Ok(*value as f64),
            Value::Null | Value::Derived => Err(self.missing(attribute)),
            value => Err(self.invalid(attribute, value)),
        }
    }

    pub(crate) fn optional_number(&self, attribute: &'static str) -> StyleResult<Option<f64>> {
        if !self.has_attribute(attribute) {
            return Ok(None);
        }
        match self.value(attribute)?.unwrap_typed() {
            Value::Null | Value::Derived => Ok(None),
            Value::Real(value) => Ok(Some(*value)),
            Value::Integer(value) => Ok(Some(*value as f64)),
            value => Err(self.invalid(attribute, value)),
        }
    }

    pub(crate) fn normalized(&self, attribute: &'static str) -> StyleResult<f64> {
        let value = self.required_number(attribute)?;
        if value.is_finite() && (0.0..=1.0).contains(&value) {
            Ok(value)
        } else {
            Err(StyleError::OutOfRange {
                entity: "IfcNormalisedRatioMeasure",
                id: self.id,
                attribute,
                value,
                minimum: 0.0,
                maximum: 1.0,
            })
        }
    }

    pub(crate) fn optional_normalized(&self, attribute: &'static str) -> StyleResult<Option<f64>> {
        let Some(value) = self.optional_number(attribute)? else {
            return Ok(None);
        };
        if value.is_finite() && (0.0..=1.0).contains(&value) {
            Ok(Some(value))
        } else {
            Err(StyleError::OutOfRange {
                entity: "IfcNormalisedRatioMeasure",
                id: self.id,
                attribute,
                value,
                minimum: 0.0,
                maximum: 1.0,
            })
        }
    }

    /// A measure whose EXPRESS type forbids zero and negative values
    /// (`IfcPositiveLengthMeasure`, `IfcPositivePlaneAngleMeasure`).
    ///
    /// The bound is exclusive at the low end; [`StyleError::OutOfRange`] can
    /// only report an inclusive pair, so it names `0.0` and the measure type
    /// and the caller reads exclusivity from the IFC type name.
    pub(crate) fn positive(
        &self,
        attribute: &'static str,
        measure: &'static str,
    ) -> StyleResult<f64> {
        let value = self.required_number(attribute)?;
        if value.is_finite() && value > 0.0 {
            Ok(value)
        } else {
            Err(StyleError::OutOfRange {
                entity: measure,
                id: self.id,
                attribute,
                value,
                minimum: 0.0,
                maximum: f64::INFINITY,
            })
        }
    }

    /// A `LIST OF` real-valued measures, rejecting a shorter list than the
    /// schema's declared lower bound.
    pub(crate) fn required_numbers(
        &self,
        attribute: &'static str,
        minimum: usize,
    ) -> StyleResult<Vec<f64>> {
        let value = self.value(attribute)?;
        let Value::List(items) = value.unwrap_typed() else {
            return if matches!(value, Value::Null | Value::Derived) {
                Err(self.missing(attribute))
            } else {
                Err(self.invalid(attribute, value))
            };
        };
        let mut out = Vec::with_capacity(items.len());
        for item in items {
            match item.unwrap_typed() {
                Value::Real(value) => out.push(*value),
                Value::Integer(value) => out.push(*value as f64),
                other => return Err(self.invalid(attribute, other)),
            }
        }
        if out.len() < minimum {
            return Err(StyleError::InvalidValue {
                entity: self.entity.type_name.to_string(),
                id: self.id,
                attribute,
                value: format!("{} value(s), expected at least {minimum}", out.len()),
            });
        }
        Ok(out)
    }

    pub(crate) fn required_refs_any(&self, attribute: &'static str) -> StyleResult<Vec<EntityId>> {
        let value = self.value(attribute)?;
        let Value::List(items) = value.unwrap_typed() else {
            return if matches!(value, Value::Null | Value::Derived) {
                Err(self.missing(attribute))
            } else {
                Err(self.invalid(attribute, value))
            };
        };
        let mut out = Vec::with_capacity(items.len());
        for item in items {
            let Some(target) = item.unwrap_typed().as_ref_id() else {
                return Err(self.invalid(attribute, item));
            };
            if self.model.get(target).is_none() {
                return Err(StyleError::DanglingReference {
                    source_id: self.id,
                    target,
                });
            }
            out.push(target);
        }
        Ok(out)
    }

    pub(crate) fn required_refs(
        &self,
        attribute: &'static str,
        expected: &'static str,
        minimum: usize,
        maximum: Option<usize>,
    ) -> StyleResult<Vec<EntityId>> {
        let targets = self.required_refs_any(attribute)?;
        self.check_ref_count(attribute, &targets, minimum, maximum)?;
        for target in &targets {
            self.check_reference(*target, expected)?;
        }
        Ok(targets)
    }

    pub(crate) fn required_refs_select(
        &self,
        attribute: &'static str,
        expected: &'static str,
        members: &[&str],
        minimum: usize,
        maximum: Option<usize>,
    ) -> StyleResult<Vec<EntityId>> {
        let targets = self.required_refs_any(attribute)?;
        self.check_ref_count(attribute, &targets, minimum, maximum)?;
        for target in &targets {
            let target_entity = self
                .model
                .get(*target)
                .ok_or(StyleError::DanglingReference {
                    source_id: self.id,
                    target: *target,
                })?;
            if !members
                .iter()
                .any(|member| self.schema.is_a(&target_entity.type_name, member))
            {
                return Err(StyleError::ReferenceType {
                    target: *target,
                    expected,
                    actual: target_entity.type_name.to_string(),
                });
            }
        }
        Ok(targets)
    }

    pub(crate) fn check_ref_count(
        &self,
        attribute: &'static str,
        targets: &[EntityId],
        minimum: usize,
        maximum: Option<usize>,
    ) -> StyleResult<()> {
        if targets.len() < minimum || maximum.is_some_and(|maximum| targets.len() > maximum) {
            let expected = maximum.map_or_else(
                || format!("at least {minimum}"),
                |maximum| format!("{minimum}..={maximum}"),
            );
            return Err(StyleError::InvalidValue {
                entity: self.entity.type_name.to_string(),
                id: self.id,
                attribute,
                value: format!("{} reference(s), expected {expected}", targets.len()),
            });
        }
        Ok(())
    }

    pub(crate) fn optional_raw(&self, attribute: &'static str) -> StyleResult<Option<&'m Value>> {
        if !self.has_attribute(attribute) {
            return Ok(None);
        }
        let value = self.value(attribute)?;
        if matches!(value, Value::Null | Value::Derived) {
            Ok(None)
        } else {
            Ok(Some(value))
        }
    }

    pub(crate) fn required_integer(&self, attribute: &'static str) -> StyleResult<i64> {
        match self.value(attribute)?.unwrap_typed() {
            Value::Integer(value) => Ok(*value),
            Value::Null | Value::Derived => Err(self.missing(attribute)),
            value => Err(self.invalid(attribute, value)),
        }
    }

    pub(crate) fn required_bool(&self, attribute: &'static str) -> StyleResult<bool> {
        match self.value(attribute)?.unwrap_typed() {
            Value::Bool(value) => Ok(*value),
            Value::Null | Value::Derived => Err(self.missing(attribute)),
            value => Err(self.invalid(attribute, value)),
        }
    }

    pub(crate) fn optional_bool(&self, attribute: &'static str) -> StyleResult<Option<bool>> {
        if !self.has_attribute(attribute) {
            return Ok(None);
        }
        match self.value(attribute)?.unwrap_typed() {
            Value::Null | Value::Derived => Ok(None),
            Value::Bool(value) => Ok(Some(*value)),
            value => Err(self.invalid(attribute, value)),
        }
    }

    pub(crate) fn optional_ref_any(
        &self,
        attribute: &'static str,
    ) -> StyleResult<Option<EntityId>> {
        if !self.has_attribute(attribute) {
            return Ok(None);
        }
        match self.value(attribute)?.unwrap_typed() {
            Value::Null | Value::Derived => Ok(None),
            Value::Ref(target) => {
                if self.model.get(*target).is_none() {
                    Err(StyleError::DanglingReference {
                        source_id: self.id,
                        target: *target,
                    })
                } else {
                    Ok(Some(*target))
                }
            }
            value => Err(self.invalid(attribute, value)),
        }
    }

    pub(crate) fn required_ref(
        &self,
        attribute: &'static str,
        expected: &'static str,
    ) -> StyleResult<EntityId> {
        match self.value(attribute)?.unwrap_typed() {
            Value::Ref(id) => {
                self.check_reference(*id, expected)?;
                Ok(*id)
            }
            Value::Null | Value::Derived => Err(self.missing(attribute)),
            other => Err(self.invalid(attribute, other)),
        }
    }

    pub(crate) fn optional_ref(
        &self,
        attribute: &'static str,
        expected: &'static str,
    ) -> StyleResult<Option<EntityId>> {
        let target = self.optional_ref_any(attribute)?;
        if let Some(target) = target {
            self.check_reference(target, expected)?;
        }
        Ok(target)
    }

    pub(crate) fn optional_ref_select(
        &self,
        attribute: &'static str,
        expected: &'static str,
        members: &[&str],
    ) -> StyleResult<Option<EntityId>> {
        let target = self.optional_ref_any(attribute)?;
        if let Some(target) = target {
            self.check_reference_select(target, expected, members)?;
        }
        Ok(target)
    }

    pub(crate) fn required_ref_select(
        &self,
        attribute: &'static str,
        expected: &'static str,
        members: &[&str],
    ) -> StyleResult<EntityId> {
        match self.value(attribute)?.unwrap_typed() {
            Value::Ref(target) => {
                self.check_reference_select(*target, expected, members)?;
                Ok(*target)
            }
            Value::Null | Value::Derived => Err(self.missing(attribute)),
            other => Err(self.invalid(attribute, other)),
        }
    }

    pub(crate) fn optional_refs(
        &self,
        attribute: &'static str,
        expected: &'static str,
    ) -> StyleResult<Vec<EntityId>> {
        match self.value(attribute)?.unwrap_typed() {
            Value::Null | Value::Derived => Ok(Vec::new()),
            Value::List(values) => values
                .iter()
                .map(|value| match value.unwrap_typed() {
                    Value::Ref(id) => {
                        self.check_reference(*id, expected)?;
                        Ok(*id)
                    }
                    other => Err(self.invalid(attribute, other)),
                })
                .collect(),
            other => Err(self.invalid(attribute, other)),
        }
    }

    pub(crate) fn check_reference(
        &self,
        target: EntityId,
        expected: &'static str,
    ) -> StyleResult<()> {
        let target_entity = self
            .model
            .get(target)
            .ok_or(StyleError::DanglingReference {
                source_id: self.id,
                target,
            })?;
        if !self.schema.is_a(&target_entity.type_name, expected) {
            return Err(StyleError::ReferenceType {
                target,
                expected,
                actual: target_entity.type_name.to_string(),
            });
        }
        Ok(())
    }

    pub(crate) fn check_reference_select(
        &self,
        target: EntityId,
        expected: &'static str,
        members: &[&str],
    ) -> StyleResult<()> {
        let target_entity = self
            .model
            .get(target)
            .ok_or(StyleError::DanglingReference {
                source_id: self.id,
                target,
            })?;
        if !members
            .iter()
            .any(|member| self.schema.is_a(&target_entity.type_name, member))
        {
            return Err(StyleError::ReferenceType {
                target,
                expected,
                actual: target_entity.type_name.to_string(),
            });
        }
        Ok(())
    }

    fn missing(&self, attribute: &'static str) -> StyleError {
        StyleError::MissingAttribute {
            entity: self.entity.type_name.to_string(),
            id: self.id,
            attribute,
        }
    }

    pub(crate) fn invalid(&self, attribute: &'static str, value: &Value) -> StyleError {
        StyleError::InvalidValue {
            entity: self.entity.type_name.to_string(),
            id: self.id,
            attribute,
            value: format!("{value:?}"),
        }
    }
}
