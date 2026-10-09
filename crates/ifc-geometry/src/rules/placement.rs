//! Where-rules on placements and directions.
//!
//! These are the rules whose violation produces the most confusing downstream
//! symptoms: geometry that renders in the wrong orientation, or a transform
//! that silently shears because two axes were not independent.

use super::release::Subject;
use super::{RuleViolation, ViolationKind};
use crate::resource::{direction::Direction, point::CartesianPoint};
use ifc_model::{Entity, EntityId, Model, Value};

/// Run the placement rules that apply to this entity.
///
/// The rules are those of the release `model` declares in `FILE_SCHEMA`,
/// named as it names them.
pub fn check(model: &Model, id: EntityId, entity: &Entity, out: &mut Vec<RuleViolation>) {
    super::run_one(model, id, entity, out, run);
}

/// The placement rules for one entity in its release.
pub(crate) fn run(s: &Subject<'_>, out: &mut Vec<RuleViolation>) {
    axis2_placement_3d(s, out);
    axis2_placement_2d(s, out);
    axis1_placement(s, out);
    location_is_cartesian(s, out);
    direction(s, out);
    correct_local_placement(s, out);
}

/// `IfcAxis2Placement3D` (IFC2X3 TC1 `WR1`-`WR5`).
///
/// - `LocationIs3D`     - the location must be a 3D point
/// - `AxisIs3D`         - Axis, if present, is 3D
/// - `RefDirIs3D`       - RefDirection, if present, is 3D
/// - `AxisToRefDirPosition` - they must not be parallel
/// - `AxisAndRefDirProvision` - both or neither, never one
///
/// The text is the same in every bundled release; IFC4X3 ADD2 adds
/// `LocationIsCP`, checked in [`location_is_cartesian`].
fn axis2_placement_3d(s: &Subject<'_>, out: &mut Vec<RuleViolation>) {
    const TYPE: &str = "IFCAXIS2PLACEMENT3D";
    let model = s.model;
    let attrs = &s.entity.attributes;

    if let Some(rule) = s.rule(TYPE, "LocationIs3D") {
        if let Some(loc) = attrs.first().and_then(|v| v.as_ref_id()) {
            if let Some(dim) = point_dim(model, loc) {
                if dim != 3 {
                    out.push(s.violation(
                        rule,
                        ViolationKind::Dimensionality,
                        format!("Location {loc} is {dim}D, must be 3D"),
                    ));
                }
            }
        }
    }

    let axis = attrs.get(1).and_then(|v| v.as_ref_id());
    let ref_dir = attrs.get(2).and_then(|v| v.as_ref_id());

    for (slot, label) in [(axis, "AxisIs3D"), (ref_dir, "RefDirIs3D")] {
        let (Some(rule), Some(dir_id)) = (s.rule(TYPE, label), slot) else {
            continue;
        };
        if let Some(dim) = direction_dim(model, dir_id) {
            if dim != 3 {
                out.push(s.violation(
                    rule,
                    ViolationKind::Dimensionality,
                    format!("{dir_id} is {dim}D, must be 3D"),
                ));
            }
        }
    }

    // AxisAndRefDirProvision: NOT (EXISTS(Axis) XOR EXISTS(RefDirection)).
    // One without the other is under-determined: the schema requires both or
    // neither, defaulting to the global axes when absent.
    if let Some(rule) = s.rule(TYPE, "AxisAndRefDirProvision") {
        if axis.is_some() != ref_dir.is_some() {
            out.push(s.violation(
                rule,
                ViolationKind::Disagreement,
                "Axis and RefDirection must both be present or both absent",
            ));
        }
    }

    // AxisToRefDirPosition: the cross product must be non-zero, i.e. the two
    // directions must not be parallel. This is the rule that silently produces
    // a degenerate basis when ignored.
    if let (Some(rule), Some(a), Some(r)) = (s.rule(TYPE, "AxisToRefDirPosition"), axis, ref_dir) {
        if cross_product_vanishes(model, a, r) {
            out.push(s.violation(
                rule,
                ViolationKind::Degenerate,
                format!("Axis {a} is parallel to RefDirection {r}; the basis is degenerate"),
            ));
        }
    }
}

/// `IfcAxis2Placement2D`: location and RefDirection must be 2D
/// (`LocationIs2D`, `RefDirIs2D`; IFC2X3 TC1 `WR2`, `WR1`).
fn axis2_placement_2d(s: &Subject<'_>, out: &mut Vec<RuleViolation>) {
    const TYPE: &str = "IFCAXIS2PLACEMENT2D";
    if let Some(rule) = s.rule(TYPE, "LocationIs2D") {
        if let Some(loc) = s.entity.attributes.first().and_then(|v| v.as_ref_id()) {
            if let Some(dim) = point_dim(s.model, loc) {
                if dim != 2 {
                    out.push(s.violation(
                        rule,
                        ViolationKind::Dimensionality,
                        format!("Location {loc} is {dim}D, must be 2D"),
                    ));
                }
            }
        }
    }
    if let Some(rule) = s.rule(TYPE, "RefDirIs2D") {
        if let Some(rd) = s.entity.attributes.get(1).and_then(|v| v.as_ref_id()) {
            if let Some(dim) = direction_dim(s.model, rd) {
                if dim != 2 {
                    out.push(s.violation(
                        rule,
                        ViolationKind::Dimensionality,
                        format!("RefDirection {rd} is {dim}D, must be 2D"),
                    ));
                }
            }
        }
    }
}

/// `IfcAxis1Placement`: `LocationIs3D` and `AxisIs3D` (IFC2X3 TC1 `WR2`,
/// `WR1`).
fn axis1_placement(s: &Subject<'_>, out: &mut Vec<RuleViolation>) {
    const TYPE: &str = "IFCAXIS1PLACEMENT";
    if let Some(rule) = s.rule(TYPE, "LocationIs3D") {
        if let Some(loc) = s.entity.attributes.first().and_then(|v| v.as_ref_id()) {
            if let Some(dim) = point_dim(s.model, loc) {
                if dim != 3 {
                    out.push(s.violation(
                        rule,
                        ViolationKind::Dimensionality,
                        format!("Location {loc} is {dim}D, must be 3D"),
                    ));
                }
            }
        }
    }
    if let Some(rule) = s.rule(TYPE, "AxisIs3D") {
        if let Some(axis) = s.entity.attributes.get(1).and_then(|v| v.as_ref_id()) {
            if let Some(dim) = direction_dim(s.model, axis) {
                if dim != 3 {
                    out.push(s.violation(
                        rule,
                        ViolationKind::Dimensionality,
                        format!("Axis {axis} is {dim}D, must be 3D"),
                    ));
                }
            }
        }
    }
}

/// `LocationIsCP` on the three axis placements, IFC4X3 ADD2 only.
///
/// IFC4X3 ADD2 widens `IfcPlacement.Location` from `IfcCartesianPoint` to
/// `IfcPoint` and restores the old restriction on these three with
/// `LocationIsCP : 'IFC4X3_ADD2.IFCCARTESIANPOINT' IN
/// TYPEOF(SELF\IfcPlacement.Location)`. Earlier releases type `Location`
/// as `IfcCartesianPoint` and declare no such rule.
fn location_is_cartesian(s: &Subject<'_>, out: &mut Vec<RuleViolation>) {
    let rule = [
        "IFCAXIS1PLACEMENT",
        "IFCAXIS2PLACEMENT2D",
        "IFCAXIS2PLACEMENT3D",
    ]
    .into_iter()
    .find_map(|declared_on| s.rule(declared_on, "LocationIsCP"));
    let Some(rule) = rule else {
        return;
    };
    // Location is slot 0, inherited from IfcPlacement.
    let Some(Value::Ref(location)) = s.entity.attribute(0).map(|v| v.unwrap_typed()) else {
        return;
    };
    let Some(point) = s.model.get(*location) else {
        return;
    };
    if s.ref_is_none_of(*location, &["IFCCARTESIANPOINT"]) {
        out.push(s.violation(
            rule,
            ViolationKind::WrongType,
            format!(
                "Location {location} is {}, must be an IfcCartesianPoint",
                point.type_name.to_ascii_uppercase()
            ),
        ));
    }
}

/// `IfcDirection.MagnitudeGreaterZero`, IFC4 ADD2 TC1 on.
///
/// A zero-length direction has no orientation; every normalisation downstream
/// divides by zero. IFC2X3 TC1 declares no rule on `IfcDirection`, so an
/// IFC2X3 file is not checked for it.
fn direction(s: &Subject<'_>, out: &mut Vec<RuleViolation>) {
    let Some(rule) = s.rule("IFCDIRECTION", "MagnitudeGreaterZero") else {
        return;
    };
    let view = Direction::new(s.id, s.entity);
    if let Ok(ratios) = view.ratios() {
        let magnitude_sq: f64 = ratios.iter().map(|r| r * r).sum();
        if magnitude_sq <= 0.0 {
            out.push(s.violation(
                rule,
                ViolationKind::Degenerate,
                "all direction ratios are zero",
            ));
        }
    }
}

/// `IfcLocalPlacement.WR21`, via `IfcCorrectLocalPlacement`.
///
/// The rule and its function test the same three types in every bundled
/// release (IFC4 on qualify `RelPlacement\IfcLocalPlacement`, which changes
/// nothing for a local placement).
///
/// # The function is genuinely three-valued
///
/// It returns UNKNOWN (`?`) for a grid placement and for any case it
/// does not recognise, and EXPRESS treats an UNKNOWN where-rule as
/// satisfied. Only the one explicit FALSE branch is a violation: a 3D
/// relative placement hung off a parent whose own placement is 2D.
/// Reporting the UNKNOWN cases would flag every valid grid placement.
fn correct_local_placement(s: &Subject<'_>, out: &mut Vec<RuleViolation>) {
    let Some(rule) = s.rule("IFCLOCALPLACEMENT", "WR21") else {
        return;
    };
    let (model, entity) = (s.model, s.entity);
    // PlacementRelTo is slot 0, RelativePlacement slot 1.
    let Some(Value::Ref(rel_to)) = entity.attribute(0).map(|v| v.unwrap_typed()) else {
        // No parent: the function returns TRUE.
        return;
    };
    let Some(Value::Ref(axis)) = entity.attribute(1).map(|v| v.unwrap_typed()) else {
        return;
    };
    let Some(parent) = model.get(*rel_to) else {
        return;
    };
    // Only IfcLocalPlacement parents reach a decidable branch.
    if !s.ref_is_a(*rel_to, "IFCLOCALPLACEMENT") {
        return;
    }
    // A 2D axis placement returns TRUE regardless of the parent.
    if !s.ref_is_a(*axis, "IFCAXIS2PLACEMENT3D") {
        return;
    }
    // The FALSE branch: parent RelativePlacement.Dim must be 3.
    let Some(Value::Ref(parent_axis)) = parent.attribute(1).map(|v| v.unwrap_typed()) else {
        return;
    };
    let Some(dim) = super::dimension::dim_of(s.release, model, *parent_axis) else {
        return;
    };
    if dim != 3 {
        out.push(s.violation(
            rule,
            ViolationKind::Dimensionality,
            format!(
                "a 3D RelativePlacement hangs off {rel_to}, whose own \
                 placement is {dim}D"
            ),
        ));
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

/// `IfcCrossProduct(a, b).Magnitude > 0.0` is FALSE for two referenced
/// directions: both resolve, both are 3D, and they are parallel.
///
/// `IfcCrossProduct` returns `?` for a 2D argument, which makes the rule
/// UNKNOWN, so only 3D pairs are judged. Shared by
/// `IfcAxis2Placement3D.AxisToRefDirPosition` and
/// `IfcAxis2PlacementLinear.WR2`, whose texts are the same.
pub(super) fn cross_product_vanishes(model: &Model, a: EntityId, b: EntityId) -> bool {
    match (direction_ratios(model, a), direction_ratios(model, b)) {
        (Some(av), Some(bv)) => is_parallel(&av, &bv),
        _ => false,
    }
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
