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
//! operation, not an afterthought.
//!
//! # What is resolved
//!
//! - `IfcSIUnit` with an `IfcSIPrefix` (`MILLI`, `CENTI`, `KILO`, ...)
//! - `IfcConversionBasedUnit` (inch, foot) via its conversion factor
//! - Angle units, because `IfcPlaneAngleMeasure` may be degrees or radians and
//!   every rotation depends on knowing which

use crate::error::{GeometryError, GeometryResult};
use crate::slots::Slots;
use ifc_model::{EntityId, Model};

/// Multipliers converting file units into SI base units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UnitScale {
    /// Multiply a stored length by this to get metres.
    pub length_to_metres: f64,
    /// Multiply a stored plane angle by this to get radians.
    pub angle_to_radians: f64,
}

impl Default for UnitScale {
    /// SI defaults: metres and radians, both factor 1.
    ///
    /// Used only when a file declares no units at all. That is malformed, but
    /// refusing to load such a file is worse than assuming SI and saying so.
    fn default() -> Self {
        Self {
            length_to_metres: 1.0,
            angle_to_radians: 1.0,
        }
    }
}

impl UnitScale {
    /// Convert a stored length to metres.
    pub fn length(&self, value: f64) -> f64 {
        value * self.length_to_metres
    }

    /// Convert a stored plane angle to radians.
    pub fn angle(&self, value: f64) -> f64 {
        value * self.angle_to_radians
    }

    /// Are lengths already in metres?
    pub fn is_metric_identity(&self) -> bool {
        (self.length_to_metres - 1.0).abs() < f64::EPSILON
    }
}

/// Resolve the project's unit assignment.
///
/// Finds `IfcProject.UnitsInContext` and reads the length and angle units from
/// it. A file with no project, or no unit assignment, yields
/// [`UnitScale::default`] rather than an error: geometry is still readable, it
/// is merely unscaled, and the caller can check
/// [`UnitScale::is_metric_identity`].
pub fn resolve(model: &Model) -> UnitScale {
    let Some(assignment) = find_unit_assignment(model) else {
        return UnitScale::default();
    };

    let mut scale = UnitScale::default();
    let Some(entity) = model.get(assignment) else {
        return scale;
    };
    let slots = Slots::new(assignment, entity);

    for unit_id in slots.opt_ref_list(0) {
        let Some(unit) = model.get(unit_id) else {
            continue;
        };
        let unit_slots = Slots::new(unit_id, unit);

        match unit_type(&unit_slots) {
            Some("LENGTHUNIT") => {
                if let Ok(factor) = length_factor(model, unit_id, &unit_slots) {
                    scale.length_to_metres = factor;
                }
            }
            Some("PLANEANGLEUNIT") => {
                if let Ok(factor) = angle_factor(model, unit_id, &unit_slots) {
                    scale.angle_to_radians = factor;
                }
            }
            _ => {}
        }
    }
    scale
}

/// Locate `IfcProject.UnitsInContext` (attribute 8).
fn find_unit_assignment(model: &Model) -> Option<EntityId> {
    let (id, project) = model.of_type("IFCPROJECT").next()?;
    Slots::new(id, project).opt_ref(8)
}

/// The `UnitType` enum, present on both `IfcSIUnit` and `IfcDerivedUnit`.
fn unit_type<'m>(slots: &Slots<'m>) -> Option<&'m str> {
    match slots.type_name() {
        // IfcSIUnit: (Dimensions, UnitType, Prefix, Name)
        "IFCSIUNIT" => slots.opt_enum(1),
        // IfcConversionBasedUnit: (Dimensions, UnitType, Name, ConversionFactor)
        "IFCCONVERSIONBASEDUNIT" | "IFCCONVERSIONBASEDUNITWITHOFFSET" => slots.opt_enum(1),
        _ => None,
    }
}

/// Metres per stored length unit.
fn length_factor(model: &Model, id: EntityId, slots: &Slots<'_>) -> GeometryResult<f64> {
    match slots.type_name() {
        "IFCSIUNIT" => Ok(prefix_factor(slots.opt_enum(2))),
        "IFCCONVERSIONBASEDUNIT" | "IFCCONVERSIONBASEDUNITWITHOFFSET" => {
            conversion_factor(model, id, slots)
        }
        other => Err(GeometryError::Units(format!(
            "{id} is {other}, which is not a length unit this build understands"
        ))),
    }
}

/// Radians per stored angle unit.
///
/// `IfcSIUnit` for an angle is the radian, so the prefix (almost always absent)
/// is the only scaling. Degrees arrive as an `IfcConversionBasedUnit` whose
/// factor is pi/180.
fn angle_factor(model: &Model, id: EntityId, slots: &Slots<'_>) -> GeometryResult<f64> {
    match slots.type_name() {
        "IFCSIUNIT" => Ok(prefix_factor(slots.opt_enum(2))),
        "IFCCONVERSIONBASEDUNIT" | "IFCCONVERSIONBASEDUNITWITHOFFSET" => {
            conversion_factor(model, id, slots)
        }
        other => Err(GeometryError::Units(format!(
            "{id} is {other}, which is not an angle unit this build understands"
        ))),
    }
}

/// Read `IfcConversionBasedUnit.ConversionFactor` (attribute 3).
///
/// The factor is an `IfcMeasureWithUnit`, whose attribute 0 is the value.
fn conversion_factor(model: &Model, id: EntityId, slots: &Slots<'_>) -> GeometryResult<f64> {
    let measure_id = slots.req_ref(3, "ConversionFactor")?;
    let measure = slots.resolve(model, measure_id)?;
    let measure_slots = Slots::new(measure_id, measure);
    measure_slots
        .req_f64(0, "ValueComponent")
        .map_err(|_| GeometryError::Units(format!("{id} has an unreadable conversion factor")))
}

/// Multiplier for an `IfcSIPrefix`.
///
/// Values from ISO 80000; `EXA` through `ATTO` are all legal in IFC even where
/// nonsensical for a building, so the whole table is present rather than the
/// three prefixes that occur in practice.
fn prefix_factor(prefix: Option<&str>) -> f64 {
    match prefix {
        None => 1.0,
        Some(p) => match p {
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
            _ => 1.0,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ifc_model::{Entity, Value};

    fn model_with_length_unit(prefix: Option<&str>) -> Model {
        let mut model = Model::new();
        let prefix_value = match prefix {
            Some(p) => Value::Enum(p.into()),
            None => Value::Null,
        };
        model.insert(
            EntityId(1),
            Entity::new(
                "IFCSIUNIT",
                vec![
                    Value::Derived,
                    Value::Enum("LENGTHUNIT".into()),
                    prefix_value,
                    Value::Enum("METRE".into()),
                ],
            ),
        );
        model.insert(
            EntityId(2),
            Entity::new(
                "IFCUNITASSIGNMENT",
                vec![Value::List(vec![Value::Ref(EntityId(1))])],
            ),
        );
        let mut project = vec![Value::Null; 9];
        project[8] = Value::Ref(EntityId(2));
        model.insert(EntityId(3), Entity::new("IFCPROJECT", project));
        model
    }

    #[test]
    fn metres_are_the_identity() {
        let scale = resolve(&model_with_length_unit(None));
        assert_eq!(scale.length_to_metres, 1.0);
        assert!(scale.is_metric_identity());
    }

    /// The case that silently breaks viewers: a 3000 mm wall is 3 m.
    #[test]
    fn millimetres_scale_coordinates_down_by_a_thousand() {
        let scale = resolve(&model_with_length_unit(Some("MILLI")));
        assert_eq!(scale.length_to_metres, 1e-3);
        assert_eq!(scale.length(3000.0), 3.0);
        assert!(!scale.is_metric_identity());
    }

    #[test]
    fn handles_the_whole_si_prefix_table() {
        assert_eq!(prefix_factor(Some("KILO")), 1e3);
        assert_eq!(prefix_factor(Some("CENTI")), 1e-2);
        assert_eq!(prefix_factor(Some("MICRO")), 1e-6);
        assert_eq!(prefix_factor(Some("UNRECOGNIZED")), 1.0);
    }

    /// A file without units still loads; it is merely unscaled.
    #[test]
    fn missing_unit_assignment_falls_back_to_si_rather_than_failing() {
        let scale = resolve(&Model::new());
        assert_eq!(scale, UnitScale::default());
    }

    /// Degrees arrive as a conversion-based unit of pi/180.
    #[test]
    fn resolves_conversion_based_angle_units() {
        let mut model = Model::new();
        model.insert(
            EntityId(1),
            Entity::new(
                "IFCMEASUREWITHUNIT",
                vec![
                    Value::Typed {
                        type_name: "IFCPLANEANGLEMEASURE".into(),
                        value: Box::new(Value::Real(0.017453292519943295)),
                    },
                    Value::Null,
                ],
            ),
        );
        model.insert(
            EntityId(2),
            Entity::new(
                "IFCCONVERSIONBASEDUNIT",
                vec![
                    Value::Null,
                    Value::Enum("PLANEANGLEUNIT".into()),
                    Value::Text("DEGREE".into()),
                    Value::Ref(EntityId(1)),
                ],
            ),
        );
        model.insert(
            EntityId(3),
            Entity::new(
                "IFCUNITASSIGNMENT",
                vec![Value::List(vec![Value::Ref(EntityId(2))])],
            ),
        );
        let mut project = vec![Value::Null; 9];
        project[8] = Value::Ref(EntityId(3));
        model.insert(EntityId(4), Entity::new("IFCPROJECT", project));

        let scale = resolve(&model);
        assert!((scale.angle(90.0) - std::f64::consts::FRAC_PI_2).abs() < 1e-12);
    }
}
