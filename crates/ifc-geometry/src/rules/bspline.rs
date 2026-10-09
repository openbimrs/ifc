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

use ifc_model::{Entity, Value};

use super::express::{constraints_param_bspline, curve_weights_positive};
use super::release::Subject;
use super::violation::{RuleViolation, ViolationKind};

/// Run the B-spline rules that apply to this entity.
///
/// IFC4 ADD2 TC1 introduced the knotted B-spline entities, and every later
/// bundled release states these rules and their functions identically;
/// IFC2X3 TC1 has none of them, but states `IfcCurveWeightsPositive` on
/// its rational Bezier curve.
pub(crate) fn check(s: &Subject<'_>, out: &mut Vec<RuleViolation>) {
    if let Some(rule) = s.rule("IFCBSPLINECURVEWITHKNOTS", "ConsistentBSpline") {
        curve_with_knots(s, rule, out);
    }
    // WeightsData is slot 8 on the knotted rational curve and slot 5 on
    // IFC2X3 TC1's rational Bezier curve (`WR2 :
    // IfcCurveWeightsPositive(SELF)`); ControlPointsList is slot 1 on both.
    if let Some(rule) = s.rule("IFCRATIONALBSPLINECURVEWITHKNOTS", "WeightsGreaterZero") {
        curve_weights(s, 8, rule, out);
    }
    if let Some(rule) = s.rule("IFCRATIONALBEZIERCURVE", "WR2") {
        curve_weights(s, 5, rule, out);
    }
    const SURFACE: &str = "IFCBSPLINESURFACEWITHKNOTS";
    let directions = [
        s.rule(SURFACE, "UDirectionConstraints")
            .zip(s.rule(SURFACE, "CorrespondingULists")),
        s.rule(SURFACE, "VDirectionConstraints")
            .zip(s.rule(SURFACE, "CorrespondingVLists")),
    ];
    if directions.iter().any(Option::is_some) {
        surface_with_knots(s, directions, out);
    }
    const RATIONAL: &str = "IFCRATIONALBSPLINESURFACEWITHKNOTS";
    if let (Some(lists), Some(values)) = (
        s.rule(RATIONAL, "CorrespondingWeightsDataLists"),
        s.rule(RATIONAL, "WeightValuesGreaterZero"),
    ) {
        surface_weights(s, (lists, values), out);
    }
}

/// `ConsistentBSpline` on a knotted curve.
///
/// Slots: Degree, ControlPointsList, CurveForm, ClosedCurve,
/// SelfIntersect, KnotMultiplicities, Knots, KnotSpec.
fn curve_with_knots(s: &Subject<'_>, rule: &'static str, out: &mut Vec<RuleViolation>) {
    let entity = s.entity;
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
        out.push(s.violation(
            rule,
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
///
/// `directions` holds, for u then v, the release's names for the
/// direction's constraint and correspondence rules.
fn surface_with_knots(
    s: &Subject<'_>,
    directions: [Option<(&'static str, &'static str)>; 2],
    out: &mut Vec<RuleViolation>,
) {
    let entity = s.entity;
    // UUpper := SIZEOF(ControlPointsList) - 1;
    // VUpper := SIZEOF(ControlPointsList[1]) - 1.
    let Some(rows) = list_len(entity, 2) else {
        return;
    };
    let cols = first_row_len(entity, 2);

    let [u_rules, v_rules] = directions;
    for (degree_slot, mult_slot, knot_slot, upper, rules, label) in [
        (0usize, 7usize, 9usize, rows, u_rules, "U"),
        (1usize, 8usize, 10usize, cols.unwrap_or(0), v_rules, "V"),
    ] {
        let Some((rule, corr)) = rules else {
            continue;
        };
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
            out.push(s.violation(
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
            out.push(s.violation(
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
/// span, so this is a degeneracy rather than a stylistic complaint. The
/// function reads the derived `Weights` array, which is undefined when
/// `WeightsData` (at `slot`) and `ControlPointsList` differ in length; the
/// rule then holds ([`curve_weights_positive`]).
fn curve_weights(s: &Subject<'_>, slot: usize, rule: &'static str, out: &mut Vec<RuleViolation>) {
    let (Some(weights), Some(points)) = (real_list(s.entity, slot), list_len(s.entity, 1)) else {
        return;
    };
    if curve_weights_positive(&weights, points) {
        return;
    }
    if let Some((i, w)) = weights.iter().enumerate().find(|(_, w)| **w <= 0.0) {
        out.push(s.violation(
            rule,
            ViolationKind::Degenerate,
            format!("WeightsData[{i}] is {w}, must be greater than 0"),
        ));
    }
}

/// `IfcSurfaceWeightsPositive`: every weight in the grid must be positive.
///
/// Also carries `CorrespondingWeightsDataLists`, which requires the weight
/// grid to match the control point grid in both directions.
///
/// `rules` holds the release's names for the two rules, in that order.
fn surface_weights(
    s: &Subject<'_>,
    (lists, values): (&'static str, &'static str),
    out: &mut Vec<RuleViolation>,
) {
    let entity = s.entity;
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
        out.push(s.violation(
            lists,
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
                out.push(s.violation(
                    values,
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
