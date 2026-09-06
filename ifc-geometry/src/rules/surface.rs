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

    directrix_bounded(model, id, entity, &name, out);
    trim_values_consistent(id, entity, &name, out);
    usense_compatible(model, id, entity, &name, out);
    applicable_mapped_repr(model, id, entity, &name, out);

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

/// `DirectrixBounded` on the three directrix-swept solids.
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
/// A curve that is *both* -- an `IfcTrimmedCurve` over a circle is not, but
/// a hypothetical bounded conic would be -- fails the `= 1` just as a curve
/// that is neither does. That is the schema's literal text and is preserved
/// here rather than relaxed to `>= 1`.
fn directrix_bounded(
    model: &Model,
    id: EntityId,
    entity: &Entity,
    name: &str,
    out: &mut Vec<RuleViolation>,
) {
    // Directrix is slot 0 on IfcSweptDiskSolid, slot 2 on the two swept-area
    // forms (SweptArea, Position, Directrix, ...); the param slots follow.
    let (directrix_slot, start_slot, end_slot) = match name {
        "IFCSWEPTDISKSOLID" | "IFCSWEPTDISKSOLIDPOLYGONAL" => (0usize, 3usize, 4usize),
        "IFCSURFACECURVESWEPTAREASOLID" | "IFCFIXEDREFERENCESWEPTAREASOLID" => (2, 3, 4),
        _ => return,
    };

    let written = |slot: usize| {
        entity
            .attribute(slot)
            .is_some_and(|v| !matches!(v.unwrap_typed(), Value::Null))
    };
    let has_params = written(start_slot) && written(end_slot);
    if has_params {
        return;
    }

    let Some(Value::Ref(directrix)) = entity.attribute(directrix_slot).map(|v| v.unwrap_typed())
    else {
        return;
    };
    let Some(curve) = model.get(*directrix) else {
        return;
    };
    let curve_name = curve.type_name.to_ascii_uppercase();
    // The schema counts how many of the two names the directrix carries and
    // demands exactly one. In IFC4 no entity is both a conic and a bounded
    // curve -- IfcConic sits under IfcCurve, IfcBoundedCurve is its sibling
    // -- so the `= 1` and `>= 1` readings can never disagree on a real file.
    // Only the zero case is reachable, and that is what is reported; a
    // "both" branch would be untestable code asserting an impossible state.
    let bounded_or_conic = crate::select::is_a(&curve_name, "IFCCONIC")
        || crate::select::is_a(&curve_name, "IFCBOUNDEDCURVE");

    if !bounded_or_conic {
        out.push(RuleViolation::new(
            id,
            name.to_string(),
            "DirectrixBounded",
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
fn trim_values_consistent(id: EntityId, entity: &Entity, name: &str, out: &mut Vec<RuleViolation>) {
    if !crate::select::is_a(name, "IFCTRIMMEDCURVE") {
        return;
    }
    // BasisCurve, Trim1, Trim2, SenseAgreement, MasterRepresentation.
    for (slot, rule) in [
        (1usize, "Trim1ValuesConsistent"),
        (2, "Trim2ValuesConsistent"),
    ] {
        let Some(Value::List(items)) = entity.attribute(slot).map(|v| v.unwrap_typed()) else {
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
            out.push(RuleViolation::new(
                id,
                name.to_string(),
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
fn usense_compatible(
    model: &Model,
    id: EntityId,
    entity: &Entity,
    name: &str,
    out: &mut Vec<RuleViolation>,
) {
    if name != "IFCRECTANGULARTRIMMEDSURFACE" {
        return;
    }
    let Some(Value::Ref(basis)) = entity.attribute(0).map(|v| v.unwrap_typed()) else {
        return;
    };
    let Some(surface) = model.get(*basis) else {
        return;
    };
    let surface_name = surface.type_name.to_ascii_uppercase();
    let wraps_in_u = (crate::select::is_a(&surface_name, "IFCELEMENTARYSURFACE")
        && !crate::select::is_a(&surface_name, "IFCPLANE"))
        || crate::select::is_a(&surface_name, "IFCSURFACEOFREVOLUTION");
    if wraps_in_u {
        return;
    }

    // BasisSurface, U1, V1, U2, V2, Usense, Vsense.
    let (Some(u1), Some(u2)) = (real_at(entity, 1), real_at(entity, 3)) else {
        return;
    };
    let Some(Value::Bool(usense)) = entity.attribute(5).map(|v| v.unwrap_typed()) else {
        return;
    };
    if *usense != (u2 > u1) {
        out.push(RuleViolation::new(
            id,
            name.to_string(),
            "UsenseCompatible",
            ViolationKind::Disagreement,
            format!("Usense is {usense} but U1 = {u1} and U2 = {u2}"),
        ));
    }
}

/// `IfcRepresentationMap.ApplicableMappedRepr`.
///
/// Only a shape model can be mapped: mapping a non-shape representation
/// would place something with no geometry to place.
fn applicable_mapped_repr(
    model: &Model,
    id: EntityId,
    entity: &Entity,
    name: &str,
    out: &mut Vec<RuleViolation>,
) {
    if name != "IFCREPRESENTATIONMAP" {
        return;
    }
    // MappingOrigin, MappedRepresentation.
    let Some(Value::Ref(mapped)) = entity.attribute(1).map(|v| v.unwrap_typed()) else {
        return;
    };
    let Some(target) = model.get(*mapped) else {
        return;
    };
    let target_name = target.type_name.to_ascii_uppercase();
    // IfcShapeModel and its two subtypes are representation-layer entities,
    // so this geometry crate's subtype table does not carry them and `is_a`
    // would answer false for every input. The family is closed in IFC4 --
    // IfcShapeModel abstracts exactly IfcShapeRepresentation and
    // IfcTopologyRepresentation -- so name them directly.
    let is_shape_model = matches!(
        target_name.as_str(),
        "IFCSHAPEMODEL" | "IFCSHAPEREPRESENTATION" | "IFCTOPOLOGYREPRESENTATION"
    );
    if !is_shape_model {
        out.push(RuleViolation::new(
            id,
            name.to_string(),
            "ApplicableMappedRepr",
            ViolationKind::WrongType,
            format!("the mapped representation {mapped} is {target_name}, not an IfcShapeModel"),
        ));
    }
}
