//! Cardinality `WHERE` rules: parallel lists that must agree in length.
//!
//! # Why these matter
//!
//! A B-spline with more knots than multiplicities is not slightly wrong,
//! it is unbuildable: the basis functions cannot be formed. Catching it
//! here names the two lists; a kernel would report a bounds failure.
//!
//! Several of the schema indices are DERIVED from a list length
//! (`UpperIndexOnKnots := SIZEOF(Knots)`), so the rule reduces to a
//! comparison of two written lists.

use ifc_model::{Entity, Value};

use super::release::Subject;
use super::violation::{RuleViolation, ViolationKind};

/// Run the cardinality rules that apply to this entity.
///
/// The B-spline, curve-on-surface and seam rules exist from IFC4 ADD2 TC1
/// on; `IfcSectionedSpine.CorrespondingSectionPositions` is IFC2X3 TC1
/// `WR1`; the rational Bezier curve is IFC2X3 TC1 only, the sectioned
/// solid IFC4X1 on and the sectioned surface IFC4X3 ADD2. Each reads the
/// same in every release that declares it.
pub(crate) fn check(s: &Subject<'_>, out: &mut Vec<RuleViolation>) {
    let entity = s.entity;

    // IfcBSplineCurveWithKnots: KnotMultiplicities and Knots are slots
    // 5 and 6 (Degree, ControlPointsList, CurveForm, ClosedCurve,
    // SelfIntersect come first). The rational form adds WeightsData at
    // slot 8; it must match the inherited ControlPointsList at slot 1.
    // IfcSectionedSpine: CrossSections and CrossSectionPositions.
    for (declared_on, label, slots, labels) in [
        (
            "IFCBSPLINECURVEWITHKNOTS",
            "CorrespondingKnotLists",
            (5, 6),
            ("KnotMultiplicities", "Knots"),
        ),
        (
            "IFCRATIONALBSPLINECURVEWITHKNOTS",
            "SameNumOfWeightsAndPoints",
            (8, 1),
            ("WeightsData", "ControlPointsList"),
        ),
        (
            "IFCSECTIONEDSPINE",
            "CorrespondingSectionPositions",
            (1, 2),
            ("CrossSections", "CrossSectionPositions"),
        ),
        // IFC2X3 TC1 `IfcRationalBezierCurve.WR1 : SIZEOF(WeightsData) =
        // SIZEOF(SELF\IfcBSplineCurve.ControlPointsList)`: WeightsData is
        // slot 5, after the five IfcBSplineCurve slots.
        (
            "IFCRATIONALBEZIERCURVE",
            "WR1",
            (5, 1),
            ("WeightsData", "ControlPointsList"),
        ),
        // `SIZEOF(CrossSections) = SIZEOF(CrossSectionPositions)`:
        // IfcSectionedSolidHorizontal (IFC4X1 on: Directrix, CrossSections,
        // CrossSectionPositions) and IfcSectionedSurface (IFC4X3 ADD2:
        // Directrix, CrossSectionPositions, CrossSections).
        (
            "IFCSECTIONEDSOLIDHORIZONTAL",
            "CorrespondingSectionPositions",
            (1, 2),
            ("CrossSections", "CrossSectionPositions"),
        ),
        (
            "IFCSECTIONEDSURFACE",
            "CorrespondingSectionPositions",
            (2, 1),
            ("CrossSections", "CrossSectionPositions"),
        ),
    ] {
        if let Some(rule) = s.rule(declared_on, label) {
            same_len(s, slots, rule, labels, out);
        }
    }

    basis_surface_rules(s, out);

    // An intersection or seam curve must associate exactly two p-curves:
    // one per surface. AssociatedGeometry is slot 1.
    let two = s
        .rule("IFCINTERSECTIONCURVE", "TwoPCurves")
        .or_else(|| s.rule("IFCSEAMCURVE", "TwoPCurves"));
    if let Some(rule) = two {
        if let Some(Value::List(items)) = entity.attribute(1).map(|v| v.unwrap_typed()) {
            if items.len() != 2 {
                out.push(s.violation(
                    rule,
                    ViolationKind::Disagreement,
                    format!(
                        "AssociatedGeometry holds {} p-curves, must hold 2",
                        items.len()
                    ),
                ));
            }
        }
    }
}

/// Two list-valued slots must have the same length.
fn same_len(
    s: &Subject<'_>,
    slots: (usize, usize),
    rule: &'static str,
    labels: (&str, &str),
    out: &mut Vec<RuleViolation>,
) {
    let ((slot_a, slot_b), (label_a, label_b)) = (slots, labels);
    let (Some(a), Some(b)) = (list_len(s.entity, slot_a), list_len(s.entity, slot_b)) else {
        return;
    };
    if a != b {
        out.push(s.violation(
            rule,
            ViolationKind::Disagreement,
            format!("{label_a} holds {a} but {label_b} holds {b}"),
        ));
    }
}

/// Length of a list-valued slot, or `None` when it is absent.
fn list_len(entity: &Entity, slot: usize) -> Option<usize> {
    match entity.attribute(slot).map(|v| v.unwrap_typed()) {
        Some(Value::List(items)) => Some(items.len()),
        _ => None,
    }
}

/// The three `IfcGetBasisSurface` rules.
///
/// All three ask the same question -- which surfaces does this curve lie
/// on -- and differ only in the answer they demand. The reader lives in
/// [`crate::surface::basis`]; this function only compares its result.
fn basis_surface_rules(s: &Subject<'_>, out: &mut Vec<RuleViolation>) {
    let (model, id) = (s.model, s.id);
    // IfcCompositeCurveOnSurface.SameSurface: SIZEOF(BasisSurface) > 0.
    // The segments must share at least one surface; the intersection is
    // empty when they do not.
    if let Some(rule) = s.rule("IFCCOMPOSITECURVEONSURFACE", "SameSurface") {
        if crate::surface::basis::basis_surfaces(model, id).is_empty() {
            out.push(s.violation(
                rule,
                ViolationKind::Disagreement,
                "the segments of this composite curve share no basis surface".to_string(),
            ));
        }
        return;
    }

    let (rule, want_same, detail) = if let Some(rule) = s.rule("IFCSEAMCURVE", "SameSurface") {
        (
            rule,
            true,
            "a seam curve must run twice over one surface, but its two \
             p-curves name different surfaces",
        )
    } else if let Some(rule) = s.rule("IFCINTERSECTIONCURVE", "DistinctSurfaces") {
        (
            rule,
            false,
            "an intersection curve must cross two different surfaces, but \
             both its p-curves name the same one",
        )
    } else {
        return;
    };

    // IfcIntersectionCurve and IfcSeamCurve both read AssociatedGeometry
    // (slot 1) pairwise, so resolve each p-curve's surface separately
    // rather than through the deduplicating set.
    let associated = super::dimension::list_refs(s.entity, 1);
    if associated.len() != 2 {
        // TwoPCurves already reports the cardinality; do not double-report.
        return;
    }
    let first = crate::surface::basis::basis_surfaces(model, associated[0]);
    let second = crate::surface::basis::basis_surfaces(model, associated[1]);
    if first.is_empty() || second.is_empty() {
        return;
    }
    if (first == second) != want_same {
        out.push(s.violation(rule, ViolationKind::Disagreement, detail.to_string()));
    }
}
