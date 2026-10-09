//! Dimensional `WHERE` rules on curves, transformation operators and sets.
//!
//! Every rule here is a statement about `Dim`, which is DERIVED. See
//! [`super::dimension`] for the derivation itself; this module only decides
//! what to do when the derived answer disagrees with the schema.

use ifc_model::{EntityId, Value};

use super::dimension::{dim_of, first_dim_disagreement, list_refs};
use super::release::Subject;
use super::violation::{RuleViolation, ViolationKind};

/// Run the dimensional curve/operator rules that apply to this entity.
///
/// Each rule is looked up by the entity that declares it, so it binds that
/// entity's subtypes in the release, and is reported under the release's
/// own name (`WR41` for `IfcPolyline.SameDim` in IFC2X3 TC1). The text of
/// every rule here is the same in every release that declares it.
pub(crate) fn check(s: &Subject<'_>, out: &mut Vec<RuleViolation>) {
    if let Some(rule) = s.rule("IFCPOLYLINE", "SameDim") {
        same_dim_list(s, 0, rule, "Points", out);
    }
    if let Some(rule) = s.rule("IFCGEOMETRICSET", "ConsistentDim") {
        same_dim_list(s, 0, rule, "Elements", out);
    }
    if let Some(rule) = s.rule("IFCLINE", "SameDim") {
        line_same_dim(s, rule, out);
    }
    // ReferenceCurve is slot 1 on IfcPcurve; BasisCurve slot 0 on the
    // offsets. IfcOffsetCurve3D's rule is labelled DimIs2D in IFC4 on even
    // though it demands 3: the schema's own label, kept verbatim.
    for (declared_on, label, slot, want) in [
        ("IFCPCURVE", "DimIs2D", 1usize, 2usize),
        ("IFCOFFSETCURVE2D", "DimIs2D", 0, 2),
        ("IFCOFFSETCURVE3D", "DimIs2D", 0, 3),
        ("IFCSURFACECURVE", "CurveIs3D", 0, 3),
        ("IFCSWEPTDISKSOLID", "DirectrixDim", 0, 3),
        ("IFCSECTIONEDSPINE", "SpineCurveDim", 0, 3),
        ("IFCPOLYGONALBOUNDEDHALFSPACE", "BoundaryDim", 3, 2),
    ] {
        if let Some(rule) = s.rule(declared_on, label) {
            fixed_dim_ref(s, slot, want, rule, out);
        }
    }
    if let Some(rule) = s.rule("IFCBSPLINECURVE", "SameDim") {
        // ControlPointsList is slot 1.
        same_dim_list(s, 1, rule, "ControlPointsList", out);
    }
    if let Some(rule) = s.rule("IFCCOMPOSITECURVE", "SameDim") {
        same_dim_list(s, 0, rule, "Segments", out);
    }
    composite_curve_continuity(s, out);
    two_d_composite_curve_dim(s, out);
    if let Some(rule) = s.rule("IFCSECTIONEDSPINE", "ConsistentProfileTypes") {
        // CrossSections is slot 1: SpineCurve, CrossSections,
        // CrossSectionPositions.
        profile_types_agree(s, 1, rule, out);
    }
    transformation_operator(s, out);
}

/// Every member of a list-valued slot must share one dimensionality.
fn same_dim_list(
    s: &Subject<'_>,
    slot: usize,
    rule: &'static str,
    label: &str,
    out: &mut Vec<RuleViolation>,
) {
    let ids = list_refs(s.entity, slot);
    if let Some((expected, found, offender)) = first_dim_disagreement(s.release, s.model, &ids) {
        out.push(s.violation(
            rule,
            ViolationKind::Disagreement,
            format!("{label} starts {expected}D but {offender} is {found}D"),
        ));
    }
}

/// A referenced entity must have exactly the dimensionality the schema fixes.
pub(super) fn fixed_dim_ref(
    s: &Subject<'_>,
    slot: usize,
    want: usize,
    rule: &'static str,
    out: &mut Vec<RuleViolation>,
) {
    let Some(Value::Ref(target)) = s.entity.attribute(slot).map(|v| v.unwrap_typed()) else {
        return;
    };
    let Some(found) = dim_of(s.release, s.model, *target) else {
        return;
    };
    if found != want {
        out.push(s.violation(
            rule,
            ViolationKind::Dimensionality,
            format!("{target} is {found}D, must be {want}D"),
        ));
    }
}

/// `IfcLine.SameDim`: the direction and the point must agree.
fn line_same_dim(s: &Subject<'_>, rule: &'static str, out: &mut Vec<RuleViolation>) {
    let (Some(Value::Ref(pnt)), Some(Value::Ref(dir))) = (
        s.entity.attribute(0).map(|v| v.unwrap_typed()),
        s.entity.attribute(1).map(|v| v.unwrap_typed()),
    ) else {
        return;
    };
    let (Some(pd), Some(dd)) = (
        dim_of(s.release, s.model, *pnt),
        dim_of(s.release, s.model, *dir),
    ) else {
        return;
    };
    if pd != dd {
        out.push(s.violation(
            rule,
            ViolationKind::Disagreement,
            format!("Pnt {pnt} is {pd}D but Dir {dir} is {dd}D"),
        ));
    }
}

/// The 2D/3D transformation operators constrain their own `Dim` and each
/// optional axis.
///
/// `Dim` here is `LocalOrigin.Dim`, so `DimEqual2`/`DimIs3D` are really
/// statements about the local origin point. The non-uniform operators are
/// subtypes and are bound by the same rules.
fn transformation_operator(s: &Subject<'_>, out: &mut Vec<RuleViolation>) {
    const OP2: &str = "IFCCARTESIANTRANSFORMATIONOPERATOR2D";
    const OP3: &str = "IFCCARTESIANTRANSFORMATIONOPERATOR3D";
    // LocalOrigin is slot 2 and carries the operator's own Dim; Axis1/Axis2
    // are optional; Axis3 exists only on the 3D operator.
    let rules: [(&str, &str, usize, usize); 7] = [
        (OP2, "DimEqual2", 2, 2),
        (OP2, "Axis1Is2D", 0, 2),
        (OP2, "Axis2Is2D", 1, 2),
        (OP3, "DimIs3D", 2, 3),
        (OP3, "Axis1Is3D", 0, 3),
        (OP3, "Axis2Is3D", 1, 3),
        (OP3, "Axis3Is3D", 4, 3),
    ];
    for (declared_on, label, slot, want) in rules {
        if let Some(rule) = s.rule(declared_on, label) {
            fixed_dim_ref(s, slot, want, rule, out);
        }
    }
}

/// `CurveContinuous`, `IsClosed`, and IFC2X3 TC1 `Ifc2DCompositeCurve.WR1`.
///
/// # The discontinuous-segment count
///
/// `IfcCompositeCurve.CurveContinuous` reads as: an open curve carries
/// exactly one `DISCONTINUOUS` transition (the final segment, which stops
/// rather than joining), and a closed curve carries none. The count is over
/// `Segments[i].Transition`, slot 0 of `IfcCompositeCurveSegment`.
///
/// The rule is skipped when `ClosedCurve` is not a written boolean: it is
/// `IfcLogical`, so `UNKNOWN` is legal and decides nothing.
///
/// IFC2X3 TC1 `Ifc2DCompositeCurve.WR1 : SELF\IfcCompositeCurve.ClosedCurve`
/// demands the same derived flag `IfcBoundaryCurve.IsClosed` does in IFC4
/// on, and is read the same way.
fn composite_curve_continuity(s: &Subject<'_>, out: &mut Vec<RuleViolation>) {
    let continuous = s.rule("IFCCOMPOSITECURVE", "CurveContinuous");
    let is_closed = s
        .rule("IFCBOUNDARYCURVE", "IsClosed")
        .or_else(|| s.rule("IFC2DCOMPOSITECURVE", "WR1"));
    if continuous.is_none() && is_closed.is_none() {
        return;
    }
    let (model, entity) = (s.model, s.entity);
    // ClosedCurve is DERIVED, never written:
    //   ClosedCurve := Segments[NSegments].Transition <> Discontinuous
    // so it depends on the LAST segment alone, not on the total count.
    let segments = super::dimension::list_refs(entity, 0);
    if segments.is_empty() {
        return;
    }
    let transition_of = |seg: EntityId| -> Option<String> {
        match model.get(seg)?.attribute(0).map(|v| v.unwrap_typed()) {
            Some(Value::Enum(e)) => Some(e.to_ascii_uppercase()),
            _ => None,
        }
    };
    // Skipped when the last transition is unreadable: the derived value is
    // then unknown, and IfcLogical UNKNOWN decides nothing.
    let Some(last) = segments.last().and_then(|s| transition_of(*s)) else {
        return;
    };
    let closed = last != "DISCONTINUOUS";
    let discontinuous = segments
        .iter()
        .filter(|seg| transition_of(**seg).as_deref() == Some("DISCONTINUOUS"))
        .count();

    let want = if closed { 0 } else { 1 };
    if let Some(rule) = continuous.filter(|_| discontinuous != want) {
        out.push(s.violation(
            rule,
            ViolationKind::Disagreement,
            format!(
                "{discontinuous} segments are DISCONTINUOUS; a closed curve \
                 admits 0 and an open curve exactly 1"
            ),
        ));
    }

    // IfcBoundaryCurve.IsClosed (and Ifc2DCompositeCurve.WR1): the same
    // curve, additionally required to close. A boundary that does not
    // close bounds nothing.
    if let Some(rule) = is_closed.filter(|_| !closed) {
        out.push(s.violation(
            rule,
            ViolationKind::Disagreement,
            "the curve must be closed, but its segments end in a discontinuity",
        ));
    }
}

/// IFC2X3 TC1 `Ifc2DCompositeCurve.WR2 : SELF\IfcCurve.Dim = 2`.
///
/// `Dim` is `IfcCurveDim(SELF)`, the first segment's dimensionality; an
/// undecidable `Dim` leaves the rule UNKNOWN.
fn two_d_composite_curve_dim(s: &Subject<'_>, out: &mut Vec<RuleViolation>) {
    let Some(rule) = s.rule("IFC2DCOMPOSITECURVE", "WR2") else {
        return;
    };
    match dim_of(s.release, s.model, s.id) {
        Some(dim) if dim != 2 => out.push(s.violation(
            rule,
            ViolationKind::Dimensionality,
            format!("the curve is {dim}D, must be 2D"),
        )),
        _ => {}
    }
}

/// `ConsistentProfileTypes` on `IfcSectionedSpine` and (IFC4X1 on)
/// `IfcSectionedSolid`: `SIZEOF(QUERY(temp <* CrossSections |
/// CrossSections[1].ProfileType <> temp.ProfileType)) = 0`.
///
/// Every cross-section in the list at `slot` must share the first one's
/// `ProfileType`; a sweep mixing AREA and CURVE profiles has no coherent
/// result.
pub(super) fn profile_types_agree(
    s: &Subject<'_>,
    slot: usize,
    rule: &'static str,
    out: &mut Vec<RuleViolation>,
) {
    let model = s.model;
    let sections = super::dimension::list_refs(s.entity, slot);
    let kind = |id: EntityId| -> Option<String> {
        match model.get(id)?.attribute(0).map(|v| v.unwrap_typed()) {
            Some(Value::Enum(e)) => Some(e.to_ascii_uppercase()),
            _ => None,
        }
    };
    let Some(first) = sections.first().and_then(|s| kind(*s)) else {
        return;
    };
    for section in sections.iter().skip(1) {
        let Some(other) = kind(*section) else {
            continue;
        };
        if other != first {
            out.push(s.violation(
                rule,
                ViolationKind::Disagreement,
                format!("cross-section {section} is {other}, but the first is {first}"),
            ));
            return;
        }
    }
}
