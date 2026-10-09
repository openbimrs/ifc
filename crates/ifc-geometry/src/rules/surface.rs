//! Surface and swept-solid `WHERE` rules.
//!
//! Mostly degeneracy conditions: a trimmed surface whose two parameters
//! coincide has zero extent, a torus whose minor radius exceeds its major
//! self-intersects, a swept disk whose inner radius exceeds its outer has
//! no material. Each is cheap here and expensive in a kernel.

use ifc_model::{Entity, Value};

use super::release::Subject;
use super::violation::{RuleViolation, ViolationKind};

/// Run the surface and swept-solid rules that apply to this entity.
///
/// Each rule reads the same in every release that declares it, except
/// `DirectrixBounded`, which IFC4X3 ADD2 moves (see [`directrix_bounded`]).
pub(crate) fn check(s: &Subject<'_>, out: &mut Vec<RuleViolation>) {
    let entity = s.entity;

    // A trimmed surface needs two distinct parameters in each direction,
    // and the sense flag must agree with their order.
    // BasisSurface, U1, V1, U2, V2, Usense, Vsense.
    const TRIMMED: &str = "IFCRECTANGULARTRIMMEDSURFACE";
    let (u1, v1, u2, v2) = (
        real_at(entity, 1),
        real_at(entity, 2),
        real_at(entity, 3),
        real_at(entity, 4),
    );
    if let Some(rule) = s.rule(TRIMMED, "U1AndU2Different") {
        distinct(s, rule, ("U1", "U2"), (u1, u2), out);
    }
    if let Some(rule) = s.rule(TRIMMED, "V1AndV2Different") {
        distinct(s, rule, ("V1", "V2"), (v1, v2), out);
    }
    // VsenseCompatible is unconditional; Usense is exempt on closed
    // surfaces, where wrapping makes the comparison meaningless.
    if let Some(rule) = s.rule(TRIMMED, "VsenseCompatible") {
        if let (Some(a), Some(b), Some(sense)) = (v1, v2, bool_at(entity, 6)) {
            if sense != (b > a) {
                out.push(s.violation(
                    rule,
                    ViolationKind::Disagreement,
                    format!("Vsense is {sense} but V2 > V1 is {}", b > a),
                ));
            }
        }
    }

    directrix_bounded(s, out);
    trim_values_consistent(s, out);
    usense_compatible(s, out);
    applicable_mapped_repr(s, out);

    // A torus whose minor radius reaches its major degenerates: the tube
    // closes through its own axis.
    if let Some(rule) = s.rule("IFCTOROIDALSURFACE", "MajorLargerMinor") {
        if let (Some(major), Some(minor)) = (real_at(entity, 1), real_at(entity, 2)) {
            if minor >= major {
                out.push(s.violation(
                    rule,
                    ViolationKind::Degenerate,
                    format!("MinorRadius {minor} is not less than MajorRadius {major}"),
                ));
            }
        }
    }

    // A swept disk with a hollow core needs the core strictly inside.
    if let Some(rule) = s.rule("IFCSWEPTDISKSOLID", "InnerRadiusSize") {
        if let (Some(radius), Some(inner)) = (real_at(entity, 1), real_at(entity, 2)) {
            if radius <= inner {
                out.push(s.violation(
                    rule,
                    ViolationKind::Degenerate,
                    format!("InnerRadius {inner} is not smaller than Radius {radius}"),
                ));
            }
        }
    }

    // A fillet cannot be tighter than the disk it rounds.
    if let Some(rule) = s.rule("IFCSWEPTDISKSOLIDPOLYGONAL", "CorrectRadii") {
        if let (Some(radius), Some(fillet)) = (real_at(entity, 1), real_at(entity, 5)) {
            if fillet < radius {
                out.push(s.violation(
                    rule,
                    ViolationKind::Degenerate,
                    format!("FilletRadius {fillet} is smaller than Radius {radius}"),
                ));
            }
        }
    }

    // A point needs at least two coordinates to place anything.
    if let Some(rule) = s.rule("IFCCARTESIANPOINT", "CP2Dor3D") {
        if let Some(Value::List(c)) = entity.attribute(0).map(|v| v.unwrap_typed()) {
            if c.len() < 2 {
                out.push(s.violation(
                    rule,
                    ViolationKind::Dimensionality,
                    format!("Coordinates holds {}, must hold at least 2", c.len()),
                ));
            }
        }
    }

    // A solid sweeps an AREA profile; a surface sweeps a CURVE profile.
    // Sweeping the wrong kind yields a shape of the wrong dimension.
    if let Some(rule) = s.rule("IFCSWEPTAREASOLID", "SweptAreaType") {
        profile_type(s, 0, "AREA", rule, "SweptArea", out);
    }
    let tapered = s
        .rule("IFCEXTRUDEDAREASOLIDTAPERED", "CorrectProfileAssignment")
        .or_else(|| s.rule("IFCREVOLVEDAREASOLIDTAPERED", "CorrectProfileAssignment"));
    if let Some(rule) = tapered {
        tapered_profiles(s, rule, out);
    }
    if let Some(rule) = s.rule("IFCSWEPTSURFACE", "SweptCurveType") {
        profile_type(s, 0, "CURVE", rule, "SweptCurve", out);
    }
}

/// The referenced profile must declare the given `ProfileType`.
fn profile_type(
    s: &Subject<'_>,
    slot: usize,
    want: &str,
    rule: &'static str,
    label: &str,
    out: &mut Vec<RuleViolation>,
) {
    let Some(Value::Ref(target)) = s.entity.attribute(slot).map(|v| v.unwrap_typed()) else {
        return;
    };
    let Some(profile) = s.model.get(*target) else {
        return;
    };
    // ProfileType is slot 0 on every IfcProfileDef.
    let Some(Value::Enum(kind)) = profile.attribute(0).map(|v| v.unwrap_typed()) else {
        return;
    };
    if kind.eq_ignore_ascii_case(want) {
        return;
    }
    out.push(s.violation(
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
fn tapered_profiles(s: &Subject<'_>, rule: &'static str, out: &mut Vec<RuleViolation>) {
    // SweptArea is slot 0; EndSweptArea is slot 4 on both tapered forms
    // (Position, then the extrusion/revolution pair, come between).
    let (Some(Value::Ref(start)), Some(Value::Ref(end))) = (
        s.entity.attribute(0).map(|v| v.unwrap_typed()),
        s.entity.attribute(4).map(|v| v.unwrap_typed()),
    ) else {
        return;
    };
    let (Some(start_def), Some(end_def)) = (s.model.get(*start), s.model.get(*end)) else {
        return;
    };
    let (sn, en) = (
        start_def.type_name.to_ascii_uppercase(),
        end_def.type_name.to_ascii_uppercase(),
    );
    // IfcTaperedSweptAreaProfiles: a derived end profile must derive from
    // the start; otherwise a parameterised start needs an end of the same
    // TYPEOF set, i.e. the same type.
    let ok = if s.ref_is_a(*end, "IFCDERIVEDPROFILEDEF") {
        // ParentProfile is slot 2: ProfileType, ProfileName, ParentProfile.
        matches!(end_def.attribute(2).map(|v| v.unwrap_typed()), Some(Value::Ref(p)) if *p == *start)
    } else if s.ref_is_a(*start, "IFCPARAMETERIZEDPROFILEDEF") {
        sn == en
    } else {
        false
    };
    if !ok {
        out.push(s.violation(
            rule,
            ViolationKind::Disagreement,
            format!("SweptArea {sn} and EndSweptArea {en} do not correspond"),
        ));
    }
}

/// Two parameters that must differ, or the trim has zero extent.
fn distinct(
    s: &Subject<'_>,
    rule: &'static str,
    (label_a, label_b): (&str, &str),
    values: (Option<f64>, Option<f64>),
    out: &mut Vec<RuleViolation>,
) {
    let (Some(a), Some(b)) = values else { return };
    if a != b {
        return;
    }
    out.push(s.violation(
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

/// `DirectrixBounded` on the directrix-swept solids, per release.
///
/// IFC4 ADD2 TC1, IFC4X1 and IFC4X2 declare it on `IfcSweptDiskSolid`,
/// `IfcFixedReferenceSweptAreaSolid` and `IfcSurfaceCurveSweptAreaSolid`.
/// IFC4X3 ADD2 keeps it on `IfcSweptDiskSolid` and moves the other two up to
/// their new common supertype `IfcDirectrixCurveSweptAreaSolid`, so it binds
/// every subtype of that, including the new
/// `IfcDirectrixDerivedReferenceSweptAreaSolid`. IFC2X3 TC1 declares it
/// nowhere. The text is the same wherever it is declared.
///
/// # Reading the set intersection
///
/// The schema writes the alternative as
/// `SIZEOF(['IFC4.IFCCONIC', 'IFC4.IFCBOUNDEDCURVE'] * TYPEOF(Directrix)) = 1`.
/// `TYPEOF` yields the full supertype set of the directrix, so the
/// intersection counts how many of those two names it carries. Exactly one
/// is required: a bounded curve carries its own extent, and a conic is
/// parameterised over a known range. A curve that is neither (an unbounded
/// line, say) has no extent, so the file must supply `StartParam` and
/// `EndParam` instead.
///
/// A curve that is *both* fails the `= 1` just as a curve that is neither
/// does. In no bundled release is an entity both -- `IfcConic` and
/// `IfcBoundedCurve` are siblings under `IfcCurve` -- so only the zero case
/// is reachable, and that is what is reported.
fn directrix_bounded(s: &Subject<'_>, out: &mut Vec<RuleViolation>) {
    // Directrix is slot 0 on IfcSweptDiskSolid, slot 2 on the swept-area
    // forms (SweptArea, Position, Directrix, ...); the param slots follow.
    let (rule, directrix_slot) = if let Some(rule) = s.rule("IFCSWEPTDISKSOLID", "DirectrixBounded")
    {
        (rule, 0usize)
    } else if let Some(rule) = [
        "IFCDIRECTRIXCURVESWEPTAREASOLID",
        "IFCSURFACECURVESWEPTAREASOLID",
        "IFCFIXEDREFERENCESWEPTAREASOLID",
    ]
    .into_iter()
    .find_map(|declared_on| s.rule(declared_on, "DirectrixBounded"))
    {
        (rule, 2)
    } else {
        return;
    };
    let (start_slot, end_slot) = (3usize, 4usize);

    let written = |slot: usize| {
        s.entity
            .attribute(slot)
            .is_some_and(|v| !matches!(v.unwrap_typed(), Value::Null))
    };
    if written(start_slot) && written(end_slot) {
        return;
    }

    let Some(Value::Ref(directrix)) = s.entity.attribute(directrix_slot).map(|v| v.unwrap_typed())
    else {
        return;
    };
    if s.model.get(*directrix).is_none() {
        return;
    }
    let bounded_or_conic =
        s.ref_is_a(*directrix, "IFCCONIC") || s.ref_is_a(*directrix, "IFCBOUNDEDCURVE");
    if !bounded_or_conic {
        out.push(s.violation(
            rule,
            ViolationKind::Disagreement,
            format!(
                "the directrix {directrix} is neither a conic nor a bounded \
                 curve, so StartParam and EndParam are required to bound the \
                 sweep"
            ),
        ));
    }
}

/// `Trim1ValuesConsistent` and `Trim2ValuesConsistent`.
///
/// A trim may be given as a parameter value, as a cartesian point, or as
/// both. When both are present they must be of *different* kinds -- giving
/// two parameters or two points for one end says nothing extra and is
/// almost always a writer bug.
fn trim_values_consistent(s: &Subject<'_>, out: &mut Vec<RuleViolation>) {
    // BasisCurve, Trim1, Trim2, SenseAgreement, MasterRepresentation.
    for (slot, label) in [
        (1usize, "Trim1ValuesConsistent"),
        (2, "Trim2ValuesConsistent"),
    ] {
        let Some(rule) = s.rule("IFCTRIMMEDCURVE", label) else {
            continue;
        };
        let Some(Value::List(items)) = s.entity.attribute(slot).map(|v| v.unwrap_typed()) else {
            continue;
        };
        if items.len() < 2 {
            continue;
        }
        // IfcTrimmingSelect is (IfcCartesianPoint, IfcParameterValue): a
        // reference is the point arm, a number the parameter arm.
        let kind = |v: &Value| match v.unwrap_typed() {
            Value::Ref(_) => Some("a cartesian point"),
            Value::Real(_) | Value::Integer(_) => Some("a parameter value"),
            _ => None,
        };
        let (Some(first), Some(second)) = (kind(&items[0]), kind(&items[1])) else {
            continue;
        };
        if first == second {
            out.push(s.violation(
                rule,
                ViolationKind::Disagreement,
                format!("both trim values are {first}; the two must differ in kind"),
            ));
        }
    }
}

/// `IfcRectangularTrimmedSurface.UsenseCompatible`.
///
/// `Usense` must agree with the direction of travel from `U1` to `U2`,
/// except on the surfaces whose u parameter is an angle and therefore wraps:
/// a cylinder, cone, sphere or torus can legitimately trim "backwards"
/// across the seam. `IfcPlane` is excluded from that exemption because its
/// u is a length, and a surface of revolution is exempt for the same
/// wrapping reason.
fn usense_compatible(s: &Subject<'_>, out: &mut Vec<RuleViolation>) {
    let Some(rule) = s.rule("IFCRECTANGULARTRIMMEDSURFACE", "UsenseCompatible") else {
        return;
    };
    let Some(Value::Ref(basis)) = s.entity.attribute(0).map(|v| v.unwrap_typed()) else {
        return;
    };
    let basis = *basis;
    if s.model.get(basis).is_none() {
        return;
    }
    let wraps_in_u = (s.ref_is_a(basis, "IFCELEMENTARYSURFACE") && !s.ref_is_a(basis, "IFCPLANE"))
        || s.ref_is_a(basis, "IFCSURFACEOFREVOLUTION");
    if wraps_in_u {
        return;
    }

    // BasisSurface, U1, V1, U2, V2, Usense, Vsense.
    let (Some(u1), Some(u2)) = (real_at(s.entity, 1), real_at(s.entity, 3)) else {
        return;
    };
    let Some(Value::Bool(usense)) = s.entity.attribute(5).map(|v| v.unwrap_typed()) else {
        return;
    };
    if *usense != (u2 > u1) {
        out.push(s.violation(
            rule,
            ViolationKind::Disagreement,
            format!("Usense is {usense} but U1 = {u1} and U2 = {u2}"),
        ));
    }
}

/// `IfcRepresentationMap.ApplicableMappedRepr`, IFC4 ADD2 TC1 on.
///
/// Only a shape model can be mapped: mapping a non-shape representation
/// would place something with no geometry to place. IFC2X3 TC1 declares no
/// rule on `IfcRepresentationMap`. `'IFCSHAPEMODEL' IN TYPEOF(...)` is
/// answered in the release's own entity table, which carries the
/// representation family.
fn applicable_mapped_repr(s: &Subject<'_>, out: &mut Vec<RuleViolation>) {
    let Some(rule) = s.rule("IFCREPRESENTATIONMAP", "ApplicableMappedRepr") else {
        return;
    };
    // MappingOrigin, MappedRepresentation.
    let Some(Value::Ref(mapped)) = s.entity.attribute(1).map(|v| v.unwrap_typed()) else {
        return;
    };
    let Some(target) = s.model.get(*mapped) else {
        return;
    };
    if s.ref_is_none_of(*mapped, &["IFCSHAPEMODEL"]) {
        let target_name = target.type_name.to_ascii_uppercase();
        out.push(s.violation(
            rule,
            ViolationKind::WrongType,
            format!("the mapped representation {mapped} is {target_name}, not an IfcShapeModel"),
        ));
    }
}
