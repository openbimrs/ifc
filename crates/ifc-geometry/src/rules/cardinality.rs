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

use ifc_model::{Entity, EntityId, Model, Value};

use super::violation::{RuleViolation, ViolationKind};

/// Run the cardinality rules that apply to this entity.
pub fn check(model: &Model, id: EntityId, entity: &Entity, out: &mut Vec<RuleViolation>) {
    let name = entity.type_name.to_ascii_uppercase();

    // IfcBSplineCurveWithKnots: KnotMultiplicities and Knots are slots
    // 5 and 6 (Degree, ControlPointsList, CurveForm, ClosedCurve,
    // SelfIntersect come first).
    if name == "IFCBSPLINECURVEWITHKNOTS" || name == "IFCRATIONALBSPLINECURVEWITHKNOTS" {
        same_len(
            id,
            &name,
            entity,
            (5, 6),
            "CorrespondingKnotLists",
            ("KnotMultiplicities", "Knots"),
            out,
        );
    }
    // The rational form adds WeightsData at slot 8; it must match the
    // inherited ControlPointsList at slot 1.
    if name == "IFCRATIONALBSPLINECURVEWITHKNOTS" {
        same_len(
            id,
            &name,
            entity,
            (8, 1),
            "SameNumOfWeightsAndPoints",
            ("WeightsData", "ControlPointsList"),
            out,
        );
    }
    // IfcSectionedSpine: CrossSections and CrossSectionPositions.
    if name == "IFCSECTIONEDSPINE" {
        same_len(
            id,
            &name,
            entity,
            (1, 2),
            "CorrespondingSectionPositions",
            ("CrossSections", "CrossSectionPositions"),
            out,
        );
    }

    basis_surface_rules(model, id, entity, &name, out);

    // An intersection or seam curve must associate exactly two p-curves:
    // one per surface. AssociatedGeometry is slot 1.
    if name == "IFCINTERSECTIONCURVE" || name == "IFCSEAMCURVE" {
        if let Some(Value::List(items)) = entity.attribute(1).map(|v| v.unwrap_typed()) {
            if items.len() != 2 {
                out.push(RuleViolation::new(
                    id,
                    name.clone(),
                    "TwoPCurves",
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
    id: EntityId,
    type_name: &str,
    entity: &Entity,
    slots: (usize, usize),
    rule: &'static str,
    labels: (&str, &str),
    out: &mut Vec<RuleViolation>,
) {
    let ((slot_a, slot_b), (label_a, label_b)) = (slots, labels);
    let (Some(a), Some(b)) = (list_len(entity, slot_a), list_len(entity, slot_b)) else {
        return;
    };
    if a != b {
        out.push(RuleViolation::new(
            id,
            type_name.to_string(),
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
fn basis_surface_rules(
    model: &Model,
    id: EntityId,
    entity: &Entity,
    name: &str,
    out: &mut Vec<RuleViolation>,
) {
    // IfcCompositeCurveOnSurface.SameSurface: SIZEOF(BasisSurface) > 0.
    // The segments must share at least one surface; the intersection is
    // empty when they do not.
    if crate::select::is_a(name, "IFCCOMPOSITECURVEONSURFACE") {
        if crate::surface::basis::basis_surfaces(model, id).is_empty() {
            out.push(RuleViolation::new(
                id,
                name.to_string(),
                "SameSurface",
                ViolationKind::Disagreement,
                "the segments of this composite curve share no basis surface".to_string(),
            ));
        }
        return;
    }

    // IfcIntersectionCurve and IfcSeamCurve both read AssociatedGeometry
    // (slot 1) pairwise, so resolve each p-curve's surface separately
    // rather than through the deduplicating set.
    let associated = super::dimension::list_refs(entity, 1);
    if associated.len() != 2 {
        // TwoPCurves already reports the cardinality; do not double-report.
        return;
    }
    let first = crate::surface::basis::basis_surfaces(model, associated[0]);
    let second = crate::surface::basis::basis_surfaces(model, associated[1]);
    if first.is_empty() || second.is_empty() {
        return;
    }

    let same = first == second;
    let (rule, want_same, detail) = if crate::select::is_a(name, "IFCSEAMCURVE") {
        (
            "SameSurface",
            true,
            "a seam curve must run twice over one surface, but its two \
             p-curves name different surfaces",
        )
    } else if crate::select::is_a(name, "IFCINTERSECTIONCURVE") {
        (
            "DistinctSurfaces",
            false,
            "an intersection curve must cross two different surfaces, but \
             both its p-curves name the same one",
        )
    } else {
        return;
    };

    if same != want_same {
        out.push(RuleViolation::new(
            id,
            name.to_string(),
            rule,
            ViolationKind::Disagreement,
            detail.to_string(),
        ));
    }
}
