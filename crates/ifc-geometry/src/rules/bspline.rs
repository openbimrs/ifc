//! B-spline `WHERE` rules: parametrisation, weights and list agreement.
//!
//! Every rule here delegates to a normative EXPRESS function in
//! [`super::express`]; this module only reads the slots and reports.
//!
//! # Curve and surface differ in shape, not in kind
//!
//! A curve carries one knot vector; a surface carries one per parametric
//! direction and a control point GRID rather than a list. The surface
//! rules therefore run the same constraint check twice, once per
//! direction, with `UUpper`/`VUpper` derived from the grid.

use ifc_model::{Entity, EntityId, Model, Value};

use super::express::constraints_param_bspline;
use super::violation::{RuleViolation, ViolationKind};

/// Run the B-spline rules that apply to this entity.
pub fn check(_model: &Model, id: EntityId, entity: &Entity, out: &mut Vec<RuleViolation>) {
    let name = entity.type_name.to_ascii_uppercase();
    if name == "IFCBSPLINECURVEWITHKNOTS" || name == "IFCRATIONALBSPLINECURVEWITHKNOTS" {
        curve_with_knots(id, entity, &name, out);
    }
    if name == "IFCRATIONALBSPLINECURVEWITHKNOTS" {
        curve_weights(id, entity, &name, out);
    }
    if name == "IFCBSPLINESURFACEWITHKNOTS" || name == "IFCRATIONALBSPLINESURFACEWITHKNOTS" {
        surface_with_knots(id, entity, &name, out);
    }
    if name == "IFCRATIONALBSPLINESURFACEWITHKNOTS" {
        surface_weights(id, entity, &name, out);
    }
}

/// `ConsistentBSpline` on a knotted curve.
///
/// Slots: Degree, ControlPointsList, CurveForm, ClosedCurve,
/// SelfIntersect, KnotMultiplicities, Knots, KnotSpec.
fn curve_with_knots(id: EntityId, entity: &Entity, type_name: &str, out: &mut Vec<RuleViolation>) {
    let Some(degree) = int_at(entity, 0) else {
        return;
    };
    let Some(cps) = list_len(entity, 1) else {
        return;
    };
    let Some(mult) = int_list(entity, 5) else {
        return;
    };
    let Some(knots) = real_list(entity, 6) else {
        return;
    };

    // UpperIndexOnKnots := SIZEOF(Knots);
    // UpperIndexOnControlPoints := SIZEOF(ControlPointsList) - 1.
    let up_knots = knots.len() as i64;
    let up_cp = cps as i64 - 1;

    if !constraints_param_bspline(degree, up_knots, up_cp, &mult, &knots) {
        out.push(RuleViolation::new(
            id,
            type_name.to_string(),
            "ConsistentBSpline",
            ViolationKind::Disagreement,
            format!(
                "degree {degree} with {up_knots} knots and {cps} control points \
                 is not a valid B-spline parametrisation"
            ),
        ));
    }
}

/// `UDirectionConstraints`, `VDirectionConstraints` and the two
/// `CorrespondingULists`/`CorrespondingVLists` rules.
///
/// Slots: UDegree, VDegree, ControlPointsList, SurfaceForm, UClosed,
/// VClosed, SelfIntersect, UMultiplicities, VMultiplicities, UKnots,
/// VKnots, KnotSpec.
fn surface_with_knots(
    id: EntityId,
    entity: &Entity,
    type_name: &str,
    out: &mut Vec<RuleViolation>,
) {
    // UUpper := SIZEOF(ControlPointsList) - 1;
    // VUpper := SIZEOF(ControlPointsList[1]) - 1.
    let Some(rows) = list_len(entity, 2) else {
        return;
    };
    let cols = first_row_len(entity, 2);

    for (degree_slot, mult_slot, knot_slot, upper, rule, corr, label) in [
        (
            0usize,
            7usize,
            9usize,
            rows,
            "UDirectionConstraints",
            "CorrespondingULists",
            "U",
        ),
        (
            1usize,
            8usize,
            10usize,
            cols.unwrap_or(0),
            "VDirectionConstraints",
            "CorrespondingVLists",
            "V",
        ),
    ] {
        let (Some(degree), Some(mult), Some(knots)) = (
            int_at(entity, degree_slot),
            int_list(entity, mult_slot),
            real_list(entity, knot_slot),
        ) else {
            continue;
        };
        // KnotUUpper := SIZEOF(UKnots), so the correspondence rule is
        // simply that the two parallel lists agree in length.
        if mult.len() != knots.len() {
            out.push(RuleViolation::new(
                id,
                type_name.to_string(),
                corr,
                ViolationKind::Disagreement,
                format!(
                    "{label}Multiplicities holds {} but {label}Knots holds {}",
                    mult.len(),
                    knots.len()
                ),
            ));
        }
        if upper == 0 {
            continue;
        }
        if !constraints_param_bspline(degree, knots.len() as i64, upper as i64 - 1, &mult, &knots) {
            out.push(RuleViolation::new(
                id,
                type_name.to_string(),
                rule,
                ViolationKind::Disagreement,
                format!("the {label} direction is not a valid B-spline parametrisation"),
            ));
        }
    }
}

/// `IfcCurveWeightsPositive`: every weight must be strictly positive.
///
/// A zero or negative weight makes the rational basis undefined at that
/// span, so this is a degeneracy rather than a stylistic complaint.
fn curve_weights(id: EntityId, entity: &Entity, type_name: &str, out: &mut Vec<RuleViolation>) {
    // WeightsData is slot 8, after the knotted-curve slots.
    let Some(weights) = real_list(entity, 8) else {
        return;
    };
    if let Some((i, w)) = weights.iter().enumerate().find(|(_, w)| **w <= 0.0) {
        out.push(RuleViolation::new(
            id,
            type_name.to_string(),
            "WeightsGreaterZero",
            ViolationKind::Degenerate,
            format!("WeightsData[{i}] is {w}, must be greater than 0"),
        ));
    }
}

/// `IfcSurfaceWeightsPositive`: every weight in the grid must be positive.
///
/// Also carries `CorrespondingWeightsDataLists`, which requires the weight
/// grid to match the control point grid in both directions.
fn surface_weights(id: EntityId, entity: &Entity, type_name: &str, out: &mut Vec<RuleViolation>) {
    // WeightsData is slot 12 on the rational surface.
    let Some(Value::List(rows)) = entity.attribute(12).map(|v| v.unwrap_typed()) else {
        return;
    };
    let cp_rows = list_len(entity, 2);
    let cp_cols = first_row_len(entity, 2);
    let w_cols = rows.first().and_then(|r| match r.unwrap_typed() {
        Value::List(c) => Some(c.len()),
        _ => None,
    });
    if cp_rows != Some(rows.len()) || (cp_cols.is_some() && cp_cols != w_cols) {
        out.push(RuleViolation::new(
            id,
            type_name.to_string(),
            "CorrespondingWeightsDataLists",
            ViolationKind::Disagreement,
            format!(
                "WeightsData is {}x{:?} but ControlPointsList is {:?}x{:?}",
                rows.len(),
                w_cols,
                cp_rows,
                cp_cols
            ),
        ));
    }
    for (i, row) in rows.iter().enumerate() {
        let Value::List(cells) = row.unwrap_typed() else {
            continue;
        };
        for (j, cell) in cells.iter().enumerate() {
            let w = match cell.unwrap_typed() {
                Value::Real(v) => *v,
                Value::Integer(v) => *v as f64,
                _ => continue,
            };
            if w <= 0.0 {
                out.push(RuleViolation::new(
                    id,
                    type_name.to_string(),
                    "WeightValuesGreaterZero",
                    ViolationKind::Degenerate,
                    format!("WeightsData[{i}][{j}] is {w}, must be greater than 0"),
                ));
                return;
            }
        }
    }
}

/// Integer at `slot`.
fn int_at(entity: &Entity, slot: usize) -> Option<i64> {
    match entity.attribute(slot)?.unwrap_typed() {
        Value::Integer(v) => Some(*v),
        Value::Real(v) => Some(*v as i64),
        _ => None,
    }
}

/// Length of a list-valued slot.
fn list_len(entity: &Entity, slot: usize) -> Option<usize> {
    match entity.attribute(slot)?.unwrap_typed() {
        Value::List(items) => Some(items.len()),
        _ => None,
    }
}

/// Length of the first inner list of a grid-valued slot.
fn first_row_len(entity: &Entity, slot: usize) -> Option<usize> {
    match entity.attribute(slot)?.unwrap_typed() {
        Value::List(rows) => rows.first().and_then(|r| match r.unwrap_typed() {
            Value::List(cells) => Some(cells.len()),
            _ => None,
        }),
        _ => None,
    }
}

/// Integers in a list-valued slot.
fn int_list(entity: &Entity, slot: usize) -> Option<Vec<i64>> {
    match entity.attribute(slot)?.unwrap_typed() {
        Value::List(items) => Some(
            items
                .iter()
                .filter_map(|v| match v.unwrap_typed() {
                    Value::Integer(n) => Some(*n),
                    Value::Real(n) => Some(*n as i64),
                    _ => None,
                })
                .collect(),
        ),
        _ => None,
    }
}

/// Reals in a list-valued slot.
fn real_list(entity: &Entity, slot: usize) -> Option<Vec<f64>> {
    match entity.attribute(slot)?.unwrap_typed() {
        Value::List(items) => Some(
            items
                .iter()
                .filter_map(|v| match v.unwrap_typed() {
                    Value::Real(n) => Some(*n),
                    Value::Integer(n) => Some(*n as f64),
                    _ => None,
                })
                .collect(),
        ),
        _ => None,
    }
}
