//! Explicit length-unit resolution for map coordinates.

use ifc_model::value::Value;
use ifc_model::{EntityId, Model};

use crate::error::{GeorefError, GeorefResult};

#[derive(Debug, Clone, PartialEq)]
pub struct LengthUnit {
    pub name: String,
    pub metres_per_unit: f64,
}

pub(crate) fn resolve_length_unit(model: &Model, id: EntityId) -> GeorefResult<LengthUnit> {
    resolve(model, id, &mut Vec::new())
}

fn resolve(model: &Model, id: EntityId, chain: &mut Vec<EntityId>) -> GeorefResult<LengthUnit> {
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
            require_enum(entity.attribute(1), id, "LENGTHUNIT")?;
            let prefix = optional_enum(entity.attribute(2), id, 2, "Prefix")?;
            require_enum(entity.attribute(3), id, "METRE")?;
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
            LengthUnit {
                name: prefix.map_or_else(|| "METRE".into(), |p| format!("{p}METRE")),
                metres_per_unit: factor,
            }
        }
        "IFCCONVERSIONBASEDUNIT" => {
            require_enum(entity.attribute(1), id, "LENGTHUNIT")?;
            let name = entity
                .text(2)
                .ok_or(GeorefError::MissingAttribute {
                    entity: id,
                    index: 2,
                    name: "Name",
                })?
                .to_owned();
            let factor_ref = entity.reference(3).ok_or(GeorefError::InvalidAttribute {
                entity: id,
                index: 3,
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
                .attribute(0)
                .and_then(|v| v.unwrap_typed().as_f64())
                .ok_or(GeorefError::InvalidAttribute {
                    entity: factor_ref,
                    index: 0,
                    name: "ValueComponent",
                })?;
            let base_ref = factor_entity
                .reference(1)
                .ok_or(GeorefError::InvalidAttribute {
                    entity: factor_ref,
                    index: 1,
                    name: "UnitComponent",
                })?;
            let base = resolve(model, base_ref, chain)?;
            LengthUnit {
                name,
                metres_per_unit: value * base.metres_per_unit,
            }
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
    if !result.metres_per_unit.is_finite() || result.metres_per_unit <= 0.0 {
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
