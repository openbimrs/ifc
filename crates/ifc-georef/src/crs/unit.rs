//! Explicit length- and plane-angle-unit resolution for CRS coordinates.
//!
//! Both kinds reduce the same way: an `IfcSIUnit` of the expected
//! `UnitType` and base name, optionally prefixed, or an
//! `IfcConversionBasedUnit` whose `ConversionFactor` bottoms out in one.
//! Only the expected `UnitType`/base name and the base factor differ.

use ifc_model::value::Value;
use ifc_model::{EntityId, Model};

use crate::error::{GeorefError, GeorefResult};
use crate::slot::{conversion_based_unit, measure_with_unit, si_unit};

/// A length unit reduced to a metre factor.
#[derive(Debug, Clone, PartialEq)]
pub struct LengthUnit {
    /// Unit name, either an SI token such as `MILLIMETRE` or the authored
    /// `IfcConversionBasedUnit.Name` such as `FOOT`.
    pub name: String,
    /// Metres per one unit. Always finite and strictly positive.
    pub metres_per_unit: f64,
}

/// A plane-angle unit reduced to a radian factor, as an IFC4X3
/// `IfcGeographicCRS.AngleUnit` declares it.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct AngleUnit {
    /// Unit name, either an SI token such as `RADIAN` or the authored
    /// `IfcConversionBasedUnit.Name` such as `DEGREE`.
    pub name: String,
    /// Radians per one unit. Always finite and strictly positive.
    pub radians_per_unit: f64,
}

/// The unit kind being reduced: its `IfcUnitEnum` token and SI base name.
#[derive(Clone, Copy)]
struct Kind {
    unit_type: &'static str,
    si_name: &'static str,
}

const LENGTH: Kind = Kind {
    unit_type: "LENGTHUNIT",
    si_name: "METRE",
};

const PLANE_ANGLE: Kind = Kind {
    unit_type: "PLANEANGLEUNIT",
    si_name: "RADIAN",
};

pub(crate) fn resolve_length_unit(model: &Model, id: EntityId) -> GeorefResult<LengthUnit> {
    let (name, metres_per_unit) = resolve(model, id, LENGTH, &mut Vec::new())?;
    Ok(LengthUnit {
        name,
        metres_per_unit,
    })
}

pub(crate) fn resolve_angle_unit(model: &Model, id: EntityId) -> GeorefResult<AngleUnit> {
    let (name, radians_per_unit) = resolve(model, id, PLANE_ANGLE, &mut Vec::new())?;
    Ok(AngleUnit {
        name,
        radians_per_unit,
    })
}

/// Reduce `id` to `(name, SI base units per unit)`.
fn resolve(
    model: &Model,
    id: EntityId,
    kind: Kind,
    chain: &mut Vec<EntityId>,
) -> GeorefResult<(String, f64)> {
    if chain.len() >= 16 || chain.contains(&id) {
        return Err(GeorefError::UnitCycle { entity: id });
    }
    chain.push(id);
    let entity = model.get(id).ok_or(GeorefError::MissingEntity {
        referrer: id,
        missing: id,
    })?;
    let result = match entity.type_name.as_ref() {
        "IFCSIUNIT" => {
            require_enum(entity.attribute(si_unit::UNIT_TYPE), id, kind.unit_type)?;
            let prefix = optional_enum(
                entity.attribute(si_unit::PREFIX),
                id,
                si_unit::PREFIX,
                "Prefix",
            )?;
            require_enum(entity.attribute(si_unit::NAME), id, kind.si_name)?;
            let factor = match prefix.as_deref() {
                None => 1.0,
                Some("EXA") => 1e18,
                Some("PETA") => 1e15,
                Some("TERA") => 1e12,
                Some("GIGA") => 1e9,
                Some("MEGA") => 1e6,
                Some("KILO") => 1e3,
                Some("HECTO") => 1e2,
                Some("DECA") => 1e1,
                Some("DECI") => 1e-1,
                Some("CENTI") => 1e-2,
                Some("MILLI") => 1e-3,
                Some("MICRO") => 1e-6,
                Some("NANO") => 1e-9,
                Some("PICO") => 1e-12,
                Some("FEMTO") => 1e-15,
                Some("ATTO") => 1e-18,
                Some(_) => {
                    return Err(GeorefError::InvalidUnit {
                        entity: id,
                        detail: "unknown SI prefix",
                    })
                }
            };
            (
                prefix.map_or_else(|| kind.si_name.into(), |p| format!("{p}{}", kind.si_name)),
                factor,
            )
        }
        "IFCCONVERSIONBASEDUNIT" => {
            require_enum(
                entity.attribute(conversion_based_unit::UNIT_TYPE),
                id,
                kind.unit_type,
            )?;
            let name = entity
                .text(conversion_based_unit::NAME)
                .ok_or(GeorefError::MissingAttribute {
                    entity: id,
                    index: conversion_based_unit::NAME,
                    name: "Name",
                })?
                .to_owned();
            let factor_ref = entity
                .reference(conversion_based_unit::CONVERSION_FACTOR)
                .ok_or(GeorefError::InvalidAttribute {
                    entity: id,
                    index: conversion_based_unit::CONVERSION_FACTOR,
                    name: "ConversionFactor",
                })?;
            let factor_entity = model.get(factor_ref).ok_or(GeorefError::MissingEntity {
                referrer: id,
                missing: factor_ref,
            })?;
            if !factor_entity.is_type("IFCMEASUREWITHUNIT") {
                return Err(GeorefError::WrongType {
                    entity: factor_ref,
                    expected: "IFCMEASUREWITHUNIT",
                    actual: factor_entity.type_name.to_string(),
                });
            }
            let value = factor_entity
                .attribute(measure_with_unit::VALUE_COMPONENT)
                .and_then(|v| v.unwrap_typed().as_f64())
                .ok_or(GeorefError::InvalidAttribute {
                    entity: factor_ref,
                    index: measure_with_unit::VALUE_COMPONENT,
                    name: "ValueComponent",
                })?;
            let base_ref = factor_entity
                .reference(measure_with_unit::UNIT_COMPONENT)
                .ok_or(GeorefError::InvalidAttribute {
                    entity: factor_ref,
                    index: measure_with_unit::UNIT_COMPONENT,
                    name: "UnitComponent",
                })?;
            let (_, base) = resolve(model, base_ref, kind, chain)?;
            (name, value * base)
        }
        actual => {
            return Err(GeorefError::WrongType {
                entity: id,
                expected: "IFCSIUNIT or IFCCONVERSIONBASEDUNIT",
                actual: actual.to_owned(),
            })
        }
    };
    chain.pop();
    if !result.1.is_finite() || result.1 <= 0.0 {
        return Err(GeorefError::InvalidUnit {
            entity: id,
            detail: "conversion factor must be finite and positive",
        });
    }
    Ok(result)
}

fn require_enum(value: Option<&Value>, id: EntityId, expected: &'static str) -> GeorefResult<()> {
    match value.map(Value::unwrap_typed) {
        Some(Value::Enum(actual)) if actual.eq_ignore_ascii_case(expected) => Ok(()),
        _ => Err(GeorefError::InvalidUnit {
            entity: id,
            detail: expected,
        }),
    }
}

fn optional_enum(
    value: Option<&Value>,
    id: EntityId,
    index: usize,
    name: &'static str,
) -> GeorefResult<Option<String>> {
    match value.map(Value::unwrap_typed) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Enum(value)) => Ok(Some(value.to_ascii_uppercase())),
        _ => Err(GeorefError::InvalidAttribute {
            entity: id,
            index,
            name,
        }),
    }
}
