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
pub fn check(_model: &Model, id: EntityId, entity: &Entity, out: &mut Vec<RuleViolation>) {
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
