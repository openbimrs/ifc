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

use ifc_model::{Entity, EntityId, Model, Value};

use super::violation::{RuleViolation, ViolationKind};

/// Run the scalar rules that apply to this entity.
pub fn check(_model: &Model, id: EntityId, entity: &Entity, out: &mut Vec<RuleViolation>) {
    let name = entity.type_name.to_ascii_uppercase();

    if name.starts_with("IFCCARTESIANTRANSFORMATIONOPERATOR") {
        // Scale is slot 3: Axis1, Axis2, LocalOrigin, Scale.
        let scl = real_at(entity, 3).unwrap_or(1.0);
        positive(id, &name, "ScaleGreaterZero", "Scl", scl, false, out);

        // The non-uniform forms add Scale2 (and Scale3) after the slots
        // their own supertype contributes, and default to Scl when unset.
        if name == "IFCCARTESIANTRANSFORMATIONOPERATOR2DNONUNIFORM" {
            let s2 = real_at(entity, 4).unwrap_or(scl);
            positive(id, &name, "Scale2GreaterZero", "Scl2", s2, false, out);
        } else if name == "IFCCARTESIANTRANSFORMATIONOPERATOR3DNONUNIFORM" {
            // Axis3 occupies slot 4 on the 3D operator.
            let s2 = real_at(entity, 5).unwrap_or(scl);
            let s3 = real_at(entity, 6).unwrap_or(scl);
            positive(id, &name, "Scale2GreaterZero", "Scl2", s2, false, out);
            positive(id, &name, "Scale3GreaterZero", "Scl3", s3, false, out);
        }
    }

    // ParamLength is slot 3: Transition, SameSense, ParentCurve, ParamLength.
    if name == "IFCREPARAMETRISEDCOMPOSITECURVESEGMENT" {
        if let Some(v) = real_at(entity, 3) {
            positive(
                id,
                &name,
                "PositiveLengthParameter",
                "ParamLength",
                v,
                false,
                out,
            );
        }
    }

    // Depth is slot 3: SweptCurve, Position, ExtrudedDirection, Depth.
    if name == "IFCSURFACEOFLINEAREXTRUSION" {
        if let Some(v) = real_at(entity, 3) {
            positive(id, &name, "DepthGreaterZero", "Depth", v, false, out);
        }
    }

    // Magnitude is slot 1 and may legitimately be zero.
    if name == "IFCVECTOR" {
        if let Some(v) = real_at(entity, 1) {
            positive(
                id,
                &name,
                "MagGreaterOrEqualZero",
                "Magnitude",
                v,
                true,
                out,
            );
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
    id: EntityId,
    type_name: &str,
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
    out.push(RuleViolation::new(
        id,
        type_name.to_string(),
        rule,
        ViolationKind::Degenerate,
        format!("{label} is {value}, must be {bound}"),
    ));
}
