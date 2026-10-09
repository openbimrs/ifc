//! Scalar `WHERE` rules: magnitudes the schema requires to be positive.
//!
//! # Why `NVL` matters here
//!
//! Every scale rule reads a DERIVED value, not the written attribute:
//! `Scl := NVL(Scale, 1.0)`. An omitted `Scale` therefore means 1.0 and
//! conforms. Reading the raw slot instead would report every file that
//! leaves the optional scale unset, which is most of them.
//!
//! `IfcVector.MagGreaterOrEqualZero` admits zero; the others do not.

use ifc_model::{Entity, Value};

use super::release::Subject;
use super::violation::{RuleViolation, ViolationKind};

/// Run the scalar rules that apply to this entity.
///
/// The text of each is the same in every release that declares it; IFC2X3
/// TC1 numbers them (`WR1`, `WR2`, `WR41`).
pub(crate) fn check(s: &Subject<'_>, out: &mut Vec<RuleViolation>) {
    let entity = s.entity;

    // Scale is slot 3: Axis1, Axis2, LocalOrigin, Scale.
    let scl = real_at(entity, 3).unwrap_or(1.0);
    if let Some(rule) = s.rule("IFCCARTESIANTRANSFORMATIONOPERATOR", "ScaleGreaterZero") {
        positive(s, rule, "Scl", scl, false, out);
    }
    // The non-uniform forms add Scale2 (and Scale3) after the slots their
    // own supertype contributes, and default to Scl when unset.
    if let Some(rule) = s.rule(
        "IFCCARTESIANTRANSFORMATIONOPERATOR2DNONUNIFORM",
        "Scale2GreaterZero",
    ) {
        let s2 = real_at(entity, 4).unwrap_or(scl);
        positive(s, rule, "Scl2", s2, false, out);
    }
    // Axis3 occupies slot 4 on the 3D operator.
    if let Some(rule) = s.rule(
        "IFCCARTESIANTRANSFORMATIONOPERATOR3DNONUNIFORM",
        "Scale2GreaterZero",
    ) {
        let s2 = real_at(entity, 5).unwrap_or(scl);
        positive(s, rule, "Scl2", s2, false, out);
    }
    if let Some(rule) = s.rule(
        "IFCCARTESIANTRANSFORMATIONOPERATOR3DNONUNIFORM",
        "Scale3GreaterZero",
    ) {
        let s3 = real_at(entity, 6).unwrap_or(scl);
        positive(s, rule, "Scl3", s3, false, out);
    }

    // ParamLength is slot 3: Transition, SameSense, ParentCurve, ParamLength.
    let declared = [
        (
            "IFCREPARAMETRISEDCOMPOSITECURVESEGMENT",
            "PositiveLengthParameter",
            3usize,
            "ParamLength",
            false,
        ),
        // Depth is slot 3: SweptCurve, Position, ExtrudedDirection, Depth.
        (
            "IFCSURFACEOFLINEAREXTRUSION",
            "DepthGreaterZero",
            3,
            "Depth",
            false,
        ),
        // Magnitude is slot 1 and may legitimately be zero.
        ("IFCVECTOR", "MagGreaterOrEqualZero", 1, "Magnitude", true),
    ];
    for (declared_on, label, slot, attribute, zero_ok) in declared {
        if let (Some(rule), Some(v)) = (s.rule(declared_on, label), real_at(entity, slot)) {
            positive(s, rule, attribute, v, zero_ok, out);
        }
    }
}

/// A written real at `slot`, unwrapping any defined-type wrapper.
///
/// Returns `None` when the slot is absent, `$`, or `*`: the caller decides
/// what an omitted optional means, because the schema defaults differ.
fn real_at(entity: &Entity, slot: usize) -> Option<f64> {
    match entity.attribute(slot)?.unwrap_typed() {
        Value::Real(v) => Some(*v),
        Value::Integer(v) => Some(*v as f64),
        _ => None,
    }
}

/// Report `value` when it is not positive (or not non-negative).
fn positive(
    s: &Subject<'_>,
    rule: &'static str,
    label: &str,
    value: f64,
    zero_ok: bool,
    out: &mut Vec<RuleViolation>,
) {
    let ok = if zero_ok { value >= 0.0 } else { value > 0.0 };
    if ok {
        return;
    }
    let bound = if zero_ok {
        "at least 0"
    } else {
        "greater than 0"
    };
    out.push(s.violation(
        rule,
        ViolationKind::Degenerate,
        format!("{label} is {value}, must be {bound}"),
    ));
}
