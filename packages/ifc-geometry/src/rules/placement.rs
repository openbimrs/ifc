//! Where-rules on placements and directions.
//!
//! These are the rules whose violation produces the most confusing downstream
//! symptoms: geometry that renders in the wrong orientation, or a transform
//! that silently shears because two axes were not independent.

use super::{RuleViolation, ViolationKind};
use crate::resource::{direction::Direction, point::CartesianPoint};
use ifc_model::{Entity, EntityId, Model};

/// Run the placement rules that apply to this entity.
pub fn check(model: &Model, id: EntityId, entity: &Entity, out: &mut Vec<RuleViolation>) {
    match entity.type_name.to_ascii_uppercase().as_str() {
        "IFCAXIS2PLACEMENT3D" => axis2_placement_3d(model, id, entity, out),
        "IFCAXIS2PLACEMENT2D" => axis2_placement_2d(model, id, entity, out),
        "IFCAXIS1PLACEMENT" => axis1_placement(model, id, entity, out),
        "IFCDIRECTION" => direction(id, entity, out),
        _ => {}
    }
}

/// `IfcAxis2Placement3D`: five rules, four of them implementable here.
///
/// - `LocationIs3D`     - the location must be a 3D point
/// - `AxisIs3D`         - Axis, if present, is 3D
/// - `RefDirIs3D`       - RefDirection, if present, is 3D
/// - `AxisToRefDirPosition` - they must not be parallel
/// - `AxisAndRefDirProvision` - both or neither, never one
fn axis2_placement_3d(model: &Model, id: EntityId, entity: &Entity, out: &mut Vec<RuleViolation>) {
    const TYPE: &str = "IFCAXIS2PLACEMENT3D";
    let attrs = &entity.attributes;

    // LocationIs3D
    if let Some(loc) = attrs.first().and_then(|v| v.as_ref_id()) {
        if let Some(dim) = point_dim(model, loc) {
            if dim != 3 {
                out.push(RuleViolation::new(
                    id,
                    TYPE,
                    "LocationIs3D",
                    ViolationKind::Dimensionality,
                    format!("Location {loc} is {dim}D, must be 3D"),
                ));
            }
        }
    }

    let axis = attrs.get(1).and_then(|v| v.as_ref_id());
    let ref_dir = attrs.get(2).and_then(|v| v.as_ref_id());

    // AxisIs3D / RefDirIs3D
    for (slot, rule) in [(axis, "AxisIs3D"), (ref_dir, "RefDirIs3D")] {
        if let Some(dir_id) = slot {
            if let Some(dim) = direction_dim(model, dir_id) {
                if dim != 3 {
                    out.push(RuleViolation::new(
                        id,
                        TYPE,
                        rule,
                        ViolationKind::Dimensionality,
                        format!("{dir_id} is {dim}D, must be 3D"),
                    ));
                }
            }
        }
    }

    // AxisAndRefDirProvision: NOT (EXISTS(Axis) XOR EXISTS(RefDirection)).
    // One without the other is under-determined: the schema requires both or
    // neither, defaulting to the global axes when absent.
    if axis.is_some() != ref_dir.is_some() {
        out.push(RuleViolation::new(
            id,
            TYPE,
            "AxisAndRefDirProvision",
            ViolationKind::Disagreement,
            "Axis and RefDirection must both be present or both absent",
        ));
    }

    // AxisToRefDirPosition: the cross product must be non-zero, i.e. the two
    // directions must not be parallel. This is the rule that silently produces
    // a degenerate basis when ignored.
    if let (Some(a), Some(r)) = (axis, ref_dir) {
        if let (Some(av), Some(rv)) = (direction_ratios(model, a), direction_ratios(model, r)) {
            if is_parallel(&av, &rv) {
                out.push(RuleViolation::new(
                    id,
                    TYPE,
                    "AxisToRefDirPosition",
                    ViolationKind::Degenerate,
                    format!("Axis {a} is parallel to RefDirection {r}; the basis is degenerate"),
                ));
            }
        }
    }
}

/// `IfcAxis2Placement2D`: location and RefDirection must be 2D.
fn axis2_placement_2d(model: &Model, id: EntityId, entity: &Entity, out: &mut Vec<RuleViolation>) {
    const TYPE: &str = "IFCAXIS2PLACEMENT2D";
    if let Some(loc) = entity.attributes.first().and_then(|v| v.as_ref_id()) {
        if let Some(dim) = point_dim(model, loc) {
            if dim != 2 {
                out.push(RuleViolation::new(
                    id,
                    TYPE,
                    "LocationIs2D",
                    ViolationKind::Dimensionality,
                    format!("Location {loc} is {dim}D, must be 2D"),
                ));
            }
        }
    }
    if let Some(rd) = entity.attributes.get(1).and_then(|v| v.as_ref_id()) {
        if let Some(dim) = direction_dim(model, rd) {
            if dim != 2 {
                out.push(RuleViolation::new(
                    id,
                    TYPE,
                    "RefDirIs2D",
                    ViolationKind::Dimensionality,
                    format!("RefDirection {rd} is {dim}D, must be 2D"),
                ));
            }
        }
    }
}

/// `IfcAxis1Placement`: `AxisIs3D`.
fn axis1_placement(model: &Model, id: EntityId, entity: &Entity, out: &mut Vec<RuleViolation>) {
    if let Some(axis) = entity.attributes.get(1).and_then(|v| v.as_ref_id()) {
        if let Some(dim) = direction_dim(model, axis) {
            if dim != 3 {
                out.push(RuleViolation::new(
                    id,
                    "IFCAXIS1PLACEMENT",
                    "AxisIs3D",
                    ViolationKind::Dimensionality,
                    format!("Axis {axis} is {dim}D, must be 3D"),
                ));
            }
        }
    }
}

/// `IfcDirection`: `MagnitudeGreaterZero`.
///
/// A zero-length direction has no orientation; every normalisation downstream
/// divides by zero.
fn direction(id: EntityId, entity: &Entity, out: &mut Vec<RuleViolation>) {
    let view = Direction::new(id, entity);
    if let Ok(ratios) = view.ratios() {
        let magnitude_sq: f64 = ratios.iter().map(|r| r * r).sum();
        if magnitude_sq <= 0.0 {
            out.push(RuleViolation::new(
                id,
                "IFCDIRECTION",
                "MagnitudeGreaterZero",
                ViolationKind::Degenerate,
                "all direction ratios are zero",
            ));
        }
    }
}

/// Dimensionality of a referenced `IfcCartesianPoint`.
fn point_dim(model: &Model, id: EntityId) -> Option<usize> {
    let entity = model.get(id)?;
    CartesianPoint::new(id, entity)
        .coordinates()
        .ok()
        .map(|c| c.len())
}

/// Dimensionality of a referenced `IfcDirection`.
fn direction_dim(model: &Model, id: EntityId) -> Option<usize> {
    direction_ratios(model, id).map(|r| r.len())
}

/// Ratios of a referenced `IfcDirection`.
fn direction_ratios(model: &Model, id: EntityId) -> Option<Vec<f64>> {
    let entity = model.get(id)?;
    Direction::new(id, entity).ratios().ok()
}

/// Are two vectors parallel (cross product effectively zero)?
///
/// Compares against a relative tolerance rather than an absolute one: an
/// absolute epsilon misjudges both millimetre-scale and kilometre-scale
/// models, and IFC files legitimately contain both.
fn is_parallel(a: &[f64], b: &[f64]) -> bool {
    if a.len() < 3 || b.len() < 3 {
        return false;
    }
    let cross = [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ];
    let cross_sq: f64 = cross.iter().map(|c| c * c).sum();
    let scale = (a.iter().map(|x| x * x).sum::<f64>()) * (b.iter().map(|x| x * x).sum::<f64>());
    if scale <= 0.0 {
        return false;
    }
    // sin^2(theta) below 1e-20 means the directions are parallel to any
    // precision a downstream kernel could exploit.
    cross_sq / scale < 1e-20
}
