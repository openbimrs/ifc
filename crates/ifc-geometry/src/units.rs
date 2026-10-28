//! Unit resolution: what one coordinate unit actually means.
//!
//! # Why this is not optional
//!
//! IFC coordinates are bare numbers. `IFCCARTESIANPOINT((3000.,0.,0.))` is
//! three metres or three millimetres depending on the project's
//! `IfcUnitAssignment`, and nothing in the geometry says which. A viewer that
//! assumes metres renders a 1000x oversized building; one that assumes
//! millimetres renders a speck.
//!
//! The resolved [`UnitScale`] is therefore an input to every lowering
//! operation, not an afterthought. IFC defines no default length or plane
//! angle unit, and `UnitsInContext` is optional on `IfcContext` in IFC4 and
//! IFC4X3, so a model that does not declare its length unit has no scale to
//! resolve: [`resolve`] refuses it (#585).
//!
//! # What is resolved
//!
//! - `IfcSIUnit` with an `IfcSIPrefix` (`MILLI`, `CENTI`, `KILO`, ...); a
//!   length unit must be the `METRE` and an angle unit the `RADIAN`
//!   (`IfcNamedUnit` WR1)
//! - `IfcConversionBasedUnit` (inch, foot, degree): its `ValueComponent`
//!   stated in its `UnitComponent`, so an inch given as 25.4 millimetres reads
//!   as 0.0254 metres
//! - Angle units, because `IfcPlaneAngleMeasure` may be degrees or radians and
//!   every rotation depends on knowing which. A missing angle unit is refused
//!   only where an angle is read ([`UnitScale::angle`])
//!
//! # What is refused
//!
//! No project, no `UnitsInContext`, no `LENGTHUNIT`; two projects; two units
//! of one type; an unknown prefix; a misnamed SI unit; a context-dependent or
//! offset unit; a conversion factor that is not a positive finite number.
//! A conversion-based unit defined through another one is
//! [`GeometryError::Unsupported`].

use crate::error::{GeometryError, GeometryResult};
use crate::slots::Slots;
use ifc_model::{EntityId, Model, Value};

/// Multipliers converting file units into SI base units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UnitScale {
    /// Multiply a stored length by this to get metres.
    pub length_to_metres: f64,
    /// Multiply a stored plane angle by this to get radians; `None` when the
    /// project declares no `PLANEANGLEUNIT`.
    pub angle_to_radians: Option<f64>,
}

impl UnitScale {
    /// Metres and radians, both factors of one; never a fallback for a
    /// model's missing units.
    pub const SI: Self = Self {
        length_to_metres: 1.0,
        angle_to_radians: Some(1.0),
    };

    /// Convert a stored length to metres.
    pub fn length(&self, value: f64) -> f64 {
        value * self.length_to_metres
    }

    /// Convert a stored plane angle to radians.
    ///
    /// `entity` is the entity holding the angle; it names the refusal when the
    /// project declares no angle unit.
    pub fn angle(&self, entity: EntityId, value: f64) -> GeometryResult<f64> {
        Ok(value * self.radians_per_unit(entity)?)
    }

    /// Radians per stored plane angle unit.
    ///
    /// # Errors
    ///
    /// [`GeometryError::AngleUnitUndeclared`] naming `entity` when the project
    /// declares no angle unit.
    pub fn radians_per_unit(&self, entity: EntityId) -> GeometryResult<f64> {
        self.angle_to_radians
            .ok_or(GeometryError::AngleUnitUndeclared { entity })
    }

    /// Are lengths already in metres?
    pub fn is_metric_identity(&self) -> bool {
        (self.length_to_metres - 1.0).abs() < f64::EPSILON
    }
}

/// Resolve the project's unit assignment.
///
/// Reads the length and angle units from `IfcProject.UnitsInContext`. Any
/// `Err` means the model has no usable scale; nothing is assumed.
///
/// # Errors
///
/// [`GeometryError::LengthUnitUndeclared`] when there is no project, no
/// assignment or no `LENGTHUNIT`; [`GeometryError::UnitsAmbiguous`] for two
/// projects or two units of one type; [`GeometryError::UnitUnreadable`] for a
/// declared length or angle unit without an exact factor. A missing angle unit
/// is not an error here: see [`UnitScale::angle`].
pub fn resolve(model: &Model) -> GeometryResult<UnitScale> {
    let project = match model.ids_of_type("IFCPROJECT") {
        [] => {
            return Err(GeometryError::LengthUnitUndeclared {
                project: None,
                assignment: None,
            });
        }
        [project] => *project,
        [first, second, ..] => {
            return Err(GeometryError::UnitsAmbiguous {
                first: *first,
                second: *second,
                kind: "IFCPROJECT",
            });
        }
    };
    let project_entity = model
        .get(project)
        .ok_or(GeometryError::LengthUnitUndeclared {
            project: Some(project),
            assignment: None,
        })?;
    let project_slots = Slots::new(project, project_entity);
    let Some(value) = project_slots.opt(8) else {
        return Err(GeometryError::LengthUnitUndeclared {
            project: Some(project),
            assignment: None,
        });
    };
    let assignment = value
        .as_ref_id()
        .ok_or_else(|| project_slots.kind_error("UnitsInContext", "an entity reference", value))?;
    let assignment_entity = project_slots.resolve(model, assignment)?;
    if &*assignment_entity.type_name != "IFCUNITASSIGNMENT" {
        return Err(GeometryError::WrongEntityType {
            entity: assignment,
            actual: assignment_entity.type_name.to_string(),
            expected: "IFCUNITASSIGNMENT",
        });
    }
    let slots = Slots::new(assignment, assignment_entity);

    // Classify every member first: two units of one type are ambiguous even
    // when the first cannot be read.
    let mut length: Option<EntityId> = None;
    let mut angle: Option<EntityId> = None;
    for unit_id in slots.req_ref_list(0, "Units")? {
        let unit = slots.resolve(model, unit_id)?;
        match unit_type(&Slots::new(unit_id, unit)) {
            Some("LENGTHUNIT") => {
                if let Some(first) = length {
                    return Err(ambiguous(first, unit_id, "LENGTHUNIT"));
                }
                length = Some(unit_id);
            }
            Some("PLANEANGLEUNIT") => {
                if let Some(first) = angle {
                    return Err(ambiguous(first, unit_id, "PLANEANGLEUNIT"));
                }
                angle = Some(unit_id);
            }
            _ => {}
        }
    }
    let read = |unit_id: EntityId, kind: &'static str| -> GeometryResult<f64> {
        let unit = slots.resolve(model, unit_id)?;
        factor(model, &Slots::new(unit_id, unit), kind)
    };
    let length = length
        .map(|unit_id| read(unit_id, "LENGTHUNIT"))
        .transpose()?;
    let angle = angle
        .map(|unit_id| read(unit_id, "PLANEANGLEUNIT"))
        .transpose()?;

    let Some(length_to_metres) = length else {
        return Err(GeometryError::LengthUnitUndeclared {
            project: Some(project),
            assignment: Some(assignment),
        });
    };
    Ok(UnitScale {
        length_to_metres,
        angle_to_radians: angle,
    })
}

fn ambiguous(first: EntityId, second: EntityId, kind: &'static str) -> GeometryError {
    GeometryError::UnitsAmbiguous {
        first,
        second,
        kind,
    }
}

/// The `UnitType` enum of an `IfcNamedUnit` (attribute 1).
fn unit_type<'m>(slots: &Slots<'m>) -> Option<&'m str> {
    match slots.type_name() {
        // IfcSIUnit: (Dimensions, UnitType, Prefix, Name)
        // IfcConversionBasedUnit[WithOffset]: (Dimensions, UnitType, Name, ConversionFactor[, ConversionOffset])
        // IfcContextDependentUnit: (Dimensions, UnitType, Name)
        "IFCSIUNIT"
        | "IFCCONVERSIONBASEDUNIT"
        | "IFCCONVERSIONBASEDUNITWITHOFFSET"
        | "IFCCONTEXTDEPENDENTUNIT" => slots.opt_enum(1),
        _ => None,
    }
}

/// Metres per stored length unit, or radians per stored angle unit.
fn factor(model: &Model, slots: &Slots<'_>, kind: &'static str) -> GeometryResult<f64> {
    let unreadable = |detail: String| GeometryError::UnitUnreadable {
        unit: slots.id(),
        unit_type: kind,
        detail,
    };
    let factor = match slots.type_name() {
        "IFCSIUNIT" => si_factor(slots, kind)?,
        "IFCCONVERSIONBASEDUNIT" | "IFCCONVERSIONBASEDUNITWITHOFFSET" => {
            if slots.type_name() == "IFCCONVERSIONBASEDUNITWITHOFFSET" {
                match slots.opt(4) {
                    None => {}
                    Some(offset) if offset.unwrap_typed().as_f64() == Some(0.0) => {}
                    Some(_) => {
                        return Err(unreadable(
                            "an offset unit has no linear factor".to_string(),
                        ));
                    }
                }
            }
            conversion_factor(model, slots, kind)?
        }
        "IFCCONTEXTDEPENDENTUNIT" => {
            return Err(unreadable(
                "a context-dependent unit states no conversion".to_string(),
            ));
        }
        other => {
            return Err(unreadable(format!(
                "{other} is not a unit this build reads"
            )))
        }
    };
    if factor.is_finite() && factor > 0.0 {
        Ok(factor)
    } else {
        Err(unreadable(format!(
            "its factor {factor} is not a positive finite number"
        )))
    }
}

/// `IfcSIUnit`: the base unit's prefix factor, after checking its name.
fn si_factor(slots: &Slots<'_>, kind: &'static str) -> GeometryResult<f64> {
    let unreadable = |detail: String| GeometryError::UnitUnreadable {
        unit: slots.id(),
        unit_type: kind,
        detail,
    };
    let expected = if kind == "LENGTHUNIT" {
        "METRE"
    } else {
        "RADIAN"
    };
    if slots.opt_enum(3) != Some(expected) {
        return Err(unreadable(format!(
            "a {kind} IfcSIUnit must be named {expected} (IfcNamedUnit WR1)"
        )));
    }
    match slots.opt(2) {
        None => Ok(1.0),
        Some(Value::Enum(prefix)) => prefix_factor(prefix)
            .ok_or_else(|| unreadable(format!("{prefix} is not an IfcSIPrefix"))),
        Some(_) => Err(unreadable("its prefix is not an enumeration".to_string())),
    }
}

/// `IfcConversionBasedUnit`: `ConversionFactor.ValueComponent` stated in its
/// `UnitComponent`.
fn conversion_factor(model: &Model, slots: &Slots<'_>, kind: &'static str) -> GeometryResult<f64> {
    let unreadable = |detail: String| GeometryError::UnitUnreadable {
        unit: slots.id(),
        unit_type: kind,
        detail,
    };
    let measure_id = slots
        .req_ref(3, "ConversionFactor")
        .map_err(|e| unreadable(e.to_string()))?;
    let measure = slots
        .resolve(model, measure_id)
        .map_err(|e| unreadable(e.to_string()))?;
    if &*measure.type_name != "IFCMEASUREWITHUNIT" {
        return Err(unreadable(format!(
            "its ConversionFactor {measure_id} is {}, not IFCMEASUREWITHUNIT",
            measure.type_name
        )));
    }
    let measure_slots = Slots::new(measure_id, measure);
    let value = measure_slots
        .req_f64(0, "ValueComponent")
        .map_err(|e| unreadable(e.to_string()))?;
    let component_id = measure_slots
        .req_ref(1, "UnitComponent")
        .map_err(|e| unreadable(e.to_string()))?;
    let component = measure_slots
        .resolve(model, component_id)
        .map_err(|e| unreadable(e.to_string()))?;
    let component_slots = Slots::new(component_id, component);
    match &*component.type_name {
        "IFCSIUNIT" => {
            if component_slots.opt_enum(1) != Some(kind) {
                return Err(unreadable(format!(
                    "its UnitComponent {component_id} is not a {kind}"
                )));
            }
            Ok(value * si_factor(&component_slots, kind)?)
        }
        "IFCCONVERSIONBASEDUNIT" | "IFCCONVERSIONBASEDUNITWITHOFFSET" => {
            Err(GeometryError::Unsupported {
                entity: component_id,
                type_name: component.type_name.to_string(),
                detail: "a conversion-based unit defined through another conversion-based unit is not resolved",
            })
        }
        other => Err(unreadable(format!(
            "its UnitComponent {component_id} is {other}, not an IfcSIUnit"
        ))),
    }
}

/// Multiplier for an `IfcSIPrefix`, or `None` for an unknown token.
///
/// Values from ISO 80000; `EXA` through `ATTO` are all legal in IFC even where
/// nonsensical for a building, so the whole table is present rather than the
/// three prefixes that occur in practice.
fn prefix_factor(prefix: &str) -> Option<f64> {
    Some(match prefix {
        "EXA" => 1e18,
        "PETA" => 1e15,
        "TERA" => 1e12,
        "GIGA" => 1e9,
        "MEGA" => 1e6,
        "KILO" => 1e3,
        "HECTO" => 1e2,
        "DECA" => 1e1,
        "DECI" => 1e-1,
        "CENTI" => 1e-2,
        "MILLI" => 1e-3,
        "MICRO" => 1e-6,
        "NANO" => 1e-9,
        "PICO" => 1e-12,
        "FEMTO" => 1e-15,
        "ATTO" => 1e-18,
        _ => return None,
    })
}

#[cfg(test)]
mod tests;
