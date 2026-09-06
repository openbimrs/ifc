//! Surface and swept-solid `WHERE` rules.
//!
//! Mostly degeneracy conditions: a trimmed surface whose two parameters
//! coincide has zero extent, a torus whose minor radius exceeds its major
//! self-intersects, a swept disk whose inner radius exceeds its outer has
//! no material. Each is cheap here and expensive in a kernel.

use ifc_model::{Entity, EntityId, Model, Value};

use super::violation::{RuleViolation, ViolationKind};

/// Run the surface and swept-solid rules that apply to this entity.
pub fn check(model: &Model, id: EntityId, entity: &Entity, out: &mut Vec<RuleViolation>) {
    let name = entity.type_name.to_ascii_uppercase();

    // A trimmed surface needs two distinct parameters in each direction,
    // and the sense flag must agree with their order.
    if name == "IFCRECTANGULARTRIMMEDSURFACE" {
        // BasisSurface, U1, V1, U2, V2, Usense, Vsense.
        let (u1, v1, u2, v2) = (
            real_at(entity, 1),
            real_at(entity, 2),
            real_at(entity, 3),
            real_at(entity, 4),
        );
        distinct(id, &name, "U1AndU2Different", "U1", "U2", u1, u2, out);
        distinct(id, &name, "V1AndV2Different", "V1", "V2", v1, v2, out);
        // VsenseCompatible is unconditional; Usense is exempt on closed
        // surfaces, where wrapping makes the comparison meaningless.
        if let (Some(a), Some(b), Some(sense)) = (v1, v2, bool_at(entity, 6)) {
            if sense != (b > a) {
                out.push(RuleViolation::new(
                    id,
                    name.clone(),
                    "VsenseCompatible",
                    ViolationKind::Disagreement,
                    format!("Vsense is {sense} but V2 > V1 is {}", b > a),
                ));
            }
        }
    }

    // A torus whose minor radius reaches its major degenerates: the tube
    // closes through its own axis.
    if name == "IFCTOROIDALSURFACE" {
        if let (Some(major), Some(minor)) = (real_at(entity, 1), real_at(entity, 2)) {
            if minor >= major {
                out.push(RuleViolation::new(
                    id,
                    name.clone(),
                    "MajorLargerMinor",
                    ViolationKind::Degenerate,
                    format!("MinorRadius {minor} is not less than MajorRadius {major}"),
                ));
            }
        }
    }

    // A swept disk with a hollow core needs the core strictly inside.
    if name == "IFCSWEPTDISKSOLID" || name == "IFCSWEPTDISKSOLIDPOLYGONAL" {
        if let (Some(radius), Some(inner)) = (real_at(entity, 1), real_at(entity, 2)) {
            if radius <= inner {
                out.push(RuleViolation::new(
                    id,
                    name.clone(),
                    "InnerRadiusSize",
                    ViolationKind::Degenerate,
                    format!("InnerRadius {inner} is not smaller than Radius {radius}"),
                ));
            }
        }
    }

    // A fillet cannot be tighter than the disk it rounds.
    if name == "IFCSWEPTDISKSOLIDPOLYGONAL" {
        if let (Some(radius), Some(fillet)) = (real_at(entity, 1), real_at(entity, 5)) {
            if fillet < radius {
                out.push(RuleViolation::new(
                    id,
                    name.clone(),
                    "CorrectRadii",
                    ViolationKind::Degenerate,
                    format!("FilletRadius {fillet} is smaller than Radius {radius}"),
                ));
            }
        }
    }

    // A point needs at least two coordinates to place anything.
    if name == "IFCCARTESIANPOINT" {
        if let Some(Value::List(c)) = entity.attribute(0).map(|v| v.unwrap_typed()) {
            if c.len() < 2 {
                out.push(RuleViolation::new(
                    id,
                    name.clone(),
                    "CP2Dor3D",
                    ViolationKind::Dimensionality,
                    format!("Coordinates holds {}, must hold at least 2", c.len()),
                ));
            }
        }
    }

    // A solid sweeps an AREA profile; a surface sweeps a CURVE profile.
    // Sweeping the wrong kind yields a shape of the wrong dimension.
    if crate::select::is_a(&name, "IFCSWEPTAREASOLID") {
        profile_type(
            model,
            id,
            entity,
            &name,
            0,
            "AREA",
            "SweptAreaType",
            "SweptArea",
            out,
        );
    }
    if name == "IFCEXTRUDEDAREASOLIDTAPERED" || name == "IFCREVOLVEDAREASOLIDTAPERED" {
        tapered_profiles(model, id, entity, &name, out);
    }
    if crate::select::is_a(&name, "IFCSWEPTSURFACE") {
        profile_type(
            model,
            id,
            entity,
            &name,
            0,
            "CURVE",
            "SweptCurveType",
            "SweptCurve",
            out,
        );
    }
}

/// The referenced profile must declare the given `ProfileType`.
#[allow(clippy::too_many_arguments)]
fn profile_type(
    model: &Model,
    id: EntityId,
    entity: &Entity,
    type_name: &str,
    slot: usize,
    want: &str,
    rule: &'static str,
    label: &str,
    out: &mut Vec<RuleViolation>,
) {
    let Some(Value::Ref(target)) = entity.attribute(slot).map(|v| v.unwrap_typed()) else {
        return;
    };
    let Some(profile) = model.get(*target) else {
        return;
    };
    // ProfileType is slot 0 on every IfcProfileDef.
    let Some(Value::Enum(kind)) = profile.attribute(0).map(|v| v.unwrap_typed()) else {
        return;
    };
    if kind.eq_ignore_ascii_case(want) {
        return;
    }
    out.push(RuleViolation::new(
        id,
        type_name.to_string(),
        rule,
        ViolationKind::WrongType,
        format!("{label} {target} is a {kind} profile, must be {want}"),
    ));
}

/// `IfcTaperedSweptAreaProfiles`: start and end profiles must correspond.
///
/// The schema admits exactly two shapes: the end profile derives from the
/// start one (`IfcDerivedProfileDef.ParentProfile` is the start), or both
/// are the same parameterised type. Anything else -- notably two
/// unrelated arbitrary profiles -- is refused, because the taper has no
/// correspondence to interpolate along.
fn tapered_profiles(
    model: &Model,
    id: EntityId,
    entity: &Entity,
    type_name: &str,
    out: &mut Vec<RuleViolation>,
) {
    // SweptArea is slot 0; EndSweptArea is slot 4 on both tapered forms
    // (Position, then the extrusion/revolution pair, come between).
    let (Some(Value::Ref(start)), Some(Value::Ref(end))) = (
        entity.attribute(0).map(|v| v.unwrap_typed()),
        entity.attribute(4).map(|v| v.unwrap_typed()),
    ) else {
        return;
    };
    let (Some(s), Some(e)) = (model.get(*start), model.get(*end)) else {
        return;
    };
    let (sn, en) = (
        s.type_name.to_ascii_uppercase(),
        e.type_name.to_ascii_uppercase(),
    );
    let ok = if en == "IFCDERIVEDPROFILEDEF" {
        // ParentProfile is slot 2: ProfileType, ProfileName, ParentProfile.
        matches!(e.attribute(2).map(|v| v.unwrap_typed()), Some(Value::Ref(p)) if *p == *start)
    } else if crate::select::is_a(&sn, "IFCPARAMETERIZEDPROFILEDEF") {
        sn == en
    } else {
        false
    };
    if !ok {
        out.push(RuleViolation::new(
            id,
            type_name.to_string(),
            "CorrectProfileAssignment",
            ViolationKind::Disagreement,
            format!("SweptArea {sn} and EndSweptArea {en} do not correspond"),
        ));
    }
}

/// Two parameters that must differ, or the trim has zero extent.
#[allow(clippy::too_many_arguments)]
fn distinct(
    id: EntityId,
    type_name: &str,
    rule: &'static str,
    label_a: &str,
    label_b: &str,
    a: Option<f64>,
    b: Option<f64>,
    out: &mut Vec<RuleViolation>,
) {
    let (Some(a), Some(b)) = (a, b) else { return };
    if a != b {
        return;
    }
    out.push(RuleViolation::new(
        id,
        type_name.to_string(),
        rule,
        ViolationKind::Degenerate,
        format!("{label_a} and {label_b} are both {a}, so the trim is empty"),
    ));
}

/// A written real at `slot`, unwrapping any defined-type wrapper.
fn real_at(entity: &Entity, slot: usize) -> Option<f64> {
    match entity.attribute(slot).map(|v| v.unwrap_typed()) {
        Some(Value::Real(v)) => Some(*v),
        Some(Value::Integer(v)) => Some(*v as f64),
        _ => None,
    }
}

/// A written boolean at `slot`.
fn bool_at(entity: &Entity, slot: usize) -> Option<bool> {
    match entity.attribute(slot).map(|v| v.unwrap_typed()) {
        Some(Value::Bool(v)) => Some(*v),
        _ => None,
    }
}
