//! SI input converted into the unit that applies (#487).
//!
//! `InputUnits::Si` reads the numbers of an edit as SI values. Each typed
//! real of a measure with a verified unit is converted by
//! `ifc_properties::si_conversion` from the property's stated `Unit` (else
//! the project unit); the conversions are cached per measure and unit for
//! the batch. Everything the rule does not cover passes through unchanged
//! and is judged by the edit's own checks afterwards.

use std::collections::BTreeMap;

use ifc_model::{EntityId, Model, Value};
use ifc_properties::{
    measure_unit_type, si_conversion, ExactUnitError, MeasureUnitType, SchemaVersion,
    UnitConversion, UnitConversionError,
};

use super::edit::PropertyEditFailure;
use super::plan::Form;

/// Exact integers a quantity accepts, as `value::quantity_number` limits them.
const MAX_EXACT_INTEGER: i64 = 1 << 53;

pub(super) struct SiInput<'m> {
    model: &'m Model,
    version: SchemaVersion,
    /// The measure (upper-case) and explicit unit to their conversion.
    conversions: BTreeMap<(String, Option<EntityId>), UnitConversion>,
}

impl<'m> SiInput<'m> {
    pub(super) fn new(model: &'m Model, version: SchemaVersion) -> Self {
        Self {
            model,
            version,
            conversions: BTreeMap::new(),
        }
    }

    /// `value`, as written to a member of `form` whose stated unit is
    /// `unit`, with its SI numbers converted.
    pub(super) fn convert(
        &mut self,
        form: Form,
        value: &Value,
        unit: Option<EntityId>,
    ) -> Result<Value, PropertyEditFailure> {
        match (form, value) {
            (Form::Enumerated | Form::List, Value::List(items)) => items
                .iter()
                .map(|item| self.item(form, item, unit))
                .collect::<Result<_, _>>()
                .map(Value::List),
            _ => self.item(form, value, unit),
        }
    }

    fn item(
        &mut self,
        form: Form,
        value: &Value,
        unit: Option<EntityId>,
    ) -> Result<Value, PropertyEditFailure> {
        let Value::Typed {
            type_name,
            value: payload,
        } = value
        else {
            return Ok(value.clone());
        };
        if !matches!(payload.as_ref(), Value::Real(_) | Value::Integer(_)) {
            return Ok(value.clone());
        }
        match measure_unit_type(self.version, type_name) {
            Ok(MeasureUnitType::Named(_) | MeasureUnitType::Derived(_)) => {}
            // Not a measure with a unit; the value checks judge the rest.
            Ok(MeasureUnitType::Dimensionless)
            | Err(ExactUnitError::NotAMeasure { .. } | ExactUnitError::MeasureNotInSchema { .. }) => {
                return Ok(value.clone())
            }
            Err(error) => return Err(refused(type_name, UnitConversionError::Resolve(error))),
            // A kind of unit this code does not know: refuse, never guess.
            Ok(_) => {
                let measure_type = type_name.clone();
                let error = ExactUnitError::UnmappedMeasureType { measure_type };
                return Err(refused(type_name, UnitConversionError::Resolve(error)));
            }
        }
        let number = match payload.as_ref() {
            Value::Real(number) => *number,
            Value::Integer(number)
                if matches!(form, Form::Quantity(_)) && number.abs() <= MAX_EXACT_INTEGER =>
            {
                *number as f64
            }
            Value::Integer(number) if matches!(form, Form::Quantity(_)) => {
                return Err(PropertyEditFailure::InvalidValue(format!(
                    "{type_name}({number}) is beyond the integers a quantity holds exactly"
                )))
            }
            Value::Integer(number) => {
                return Err(PropertyEditFailure::InvalidValue(format!(
                    "{type_name}({number}) takes a real in SI input; an integer is not converted"
                )))
            }
            _ => unreachable!("a real or an integer, checked above"),
        };
        if !number.is_finite() {
            return Err(refused(
                type_name,
                UnitConversionError::NonFinite { value: number },
            ));
        }
        let key = (type_name.to_ascii_uppercase(), unit);
        let conversion = match self.conversions.get(&key) {
            Some(conversion) => *conversion,
            None => {
                let conversion = si_conversion(self.model, type_name, unit)
                    .map_err(|error| refused(type_name, error))?;
                self.conversions.insert(key, conversion);
                conversion
            }
        };
        Ok(Value::Typed {
            type_name: type_name.clone(),
            value: Box::new(Value::Real(conversion.apply(number))),
        })
    }
}

fn refused(measure: &str, error: UnitConversionError) -> PropertyEditFailure {
    PropertyEditFailure::UnitConversion {
        measure: measure.to_owned(),
        error,
    }
}
