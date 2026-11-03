//! SI input to the unit that applies (#487).
//!
//! A writer holding an SI value needs the value in the model's unit for the
//! measure. IFC states no SI default: a measure without an explicit `Unit`
//! takes the project's unit of its unit type, and a model that declares none
//! is refused ([`crate::ExactUnitError::NoProjectUnit`]) rather than written
//! in an assumed unit.

use ifc_model::{EntityId, Model};

use super::{exact_unit, resolve_unit};
use crate::convert::{unit_conversion, ConversionUnit, UnitConversion, UnitConversionError};

/// The conversion from an SI value to the unit that applies to
/// `measure_type`: `explicit_unit` when stated, otherwise the project unit of
/// the measure's unit type. A dimensionless measure converts by the identity.
///
/// SI means the coherent unit of the measure's dimensions (metre, kilogram,
/// kelvin, radian ...). A project unit of `DEGREE_CELSIUS` converts
/// affinely; units with another offset are refused (#110).
///
/// # Errors
///
/// [`UnitConversionError::OffsetUnit`] for an offset unit, and
/// [`UnitConversionError::Resolve`] with the refusal of [`exact_unit`]: a
/// project without a unit of the type, an unmapped or non-measure type, a
/// unit of the wrong type, or an unresolvable unit.
pub fn si_conversion(
    model: &Model,
    measure_type: &str,
    explicit_unit: Option<EntityId>,
) -> Result<UnitConversion, UnitConversionError> {
    let exact = exact_unit(model, measure_type, explicit_unit)?;
    let Some(unit) = exact.unit else {
        return Ok(UnitConversion::IDENTITY);
    };
    let target = resolve_unit(model, unit)?;
    unit_conversion(&ConversionUnit::coherent_si(target.dimensions), &target)
}

/// An SI `value` in the project unit of `measure_type`.
///
/// # Errors
///
/// [`UnitConversionError::NonFinite`] for NaN or infinity, otherwise as
/// [`si_conversion`] without an explicit unit.
pub fn si_to_project(
    model: &Model,
    measure_type: &str,
    value: f64,
) -> Result<f64, UnitConversionError> {
    if !value.is_finite() {
        return Err(UnitConversionError::NonFinite { value });
    }
    Ok(si_conversion(model, measure_type, None)?.apply(value))
}
