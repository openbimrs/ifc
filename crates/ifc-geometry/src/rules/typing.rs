//! Type-membership `WHERE` rules: `IN TYPEOF(slot)`.
//!
//! # Exact type versus subtype
//!
//! EXPRESS `TYPEOF` yields the full supertype set, so
//! `IFCBOUNDEDCURVE IN TYPEOF(x)` is a subtype test, not an exact-name
//! match. These rules therefore ask the declared release's own entity
//! table (`Subject::ref_is_a`) rather than comparing names, or a polyline
//! would fail to count as a bounded curve.

use ifc_model::{EntityId, Value};

use super::release::Subject;
use super::violation::{RuleViolation, ViolationKind};

/// Run the type-membership rules that apply to this entity.
///
/// Every `IN TYPEOF` test is answered in the declared release's own entity
/// table. The texts are the same in every release that declares them
/// (IFC4X3 ADD2 only qualifies `Directrix\IfcIndexedPolyCurve.Segments`),
/// except that IFC2X3 TC1 numbers them and alone declares
/// `IfcSweptSurface.WR1`.
pub(crate) fn check(s: &Subject<'_>, out: &mut Vec<RuleViolation>) {
    // A composite curve segment must carry a bounded parent curve:
    // an unbounded line has no span to contribute to the composite.
    // ParentCurve is slot 2 (Transition, SameSense, ParentCurve).
    if let Some(rule) = s.rule("IFCCOMPOSITECURVESEGMENT", "ParentIsBoundedCurve") {
        require_kind(s, 2, "IFCBOUNDEDCURVE", true, rule, "ParentCurve", out);
    }

    // Trimming an already-bounded curve is meaningless: its bounds are
    // its own, and the trim parameters would contradict them.
    if let Some(rule) = s.rule("IFCTRIMMEDCURVE", "NoTrimOfBoundedCurves") {
        require_kind(s, 0, "IFCBOUNDEDCURVE", false, rule, "BasisCurve", out);
    }

    // A surface curve states its 3D geometry; a p-curve is defined in a
    // parameter domain, so it cannot serve as that 3D curve.
    if let Some(rule) = s.rule("IFCSURFACECURVE", "CurveIsNotPcurve") {
        require_kind(s, 0, "IFCPCURVE", false, rule, "Curve3D", out);
    }

    // A boxed half space bounds an unbounded surface; a curve-bounded
    // plane already carries its own boundary.
    if let Some(rule) = s.rule("IFCBOXEDHALFSPACE", "UnboundedSurface") {
        require_kind(
            s,
            0,
            "IFCCURVEBOUNDEDPLANE",
            false,
            rule,
            "BaseSurface",
            out,
        );
    }

    // IFC2X3 TC1 alone: `IfcSweptSurface.WR1 : NOT('IFC2X3.IFCDERIVEDPROFILEDEF'
    // IN TYPEOF(SweptCurve))`. IFC4 dropped the rule, so a derived profile
    // is swept freely from IFC4 ADD2 TC1 on.
    if let Some(rule) = s.rule("IFCSWEPTSURFACE", "WR1") {
        require_kind(s, 0, "IFCDERIVEDPROFILEDEF", false, rule, "SweptCurve", out);
    }

    // A geometric CURVE set must hold no surfaces.
    if let Some(rule) = s.rule("IFCGEOMETRICCURVESET", "NoSurfaces") {
        if let Some(Value::List(items)) = s.entity.attribute(0).map(|v| v.unwrap_typed()) {
            for item in items {
                let Value::Ref(target) = item.unwrap_typed() else {
                    continue;
                };
                if s.ref_is_a(*target, "IFCSURFACE") {
                    out.push(s.violation(
                        rule,
                        ViolationKind::WrongType,
                        format!("Elements holds surface {target}, which a curve set excludes"),
                    ));
                }
            }
        }
    }

    // A polygonal swept disk needs a polyline directrix, or an indexed
    // poly-curve with no explicit segments (which is the same shape).
    if let Some(rule) = s.rule("IFCSWEPTDISKSOLIDPOLYGONAL", "DirectrixIsPolyline") {
        directrix_is_polyline(s, rule, out);
    }

    // IfcIndexedPolyCurve.Consecutive: Segments is slot 1, and each
    // segment is an IfcLineIndex or IfcArcIndex -- both plain integer
    // lists -- so the join compares last index against first. IFC4 to
    // IFC4X2 guard it with `SIZEOF(Segments) = 0`, IFC4X3 ADD2 with
    // `NOT(EXISTS(Segments))`; both hold for an absent list, and Segments
    // is `LIST [1:?]`, so they agree on every list that can be written.
    if let Some(rule) = s.rule("IFCINDEXEDPOLYCURVE", "Consecutive") {
        if let Some(Value::List(items)) = s.entity.attribute(1).map(|v| v.unwrap_typed()) {
            let segments: Vec<Vec<i64>> = items
                .iter()
                .map(|seg| match seg.unwrap_typed() {
                    Value::List(idx) => idx
                        .iter()
                        .filter_map(|v| match v.unwrap_typed() {
                            Value::Integer(n) => Some(*n),
                            Value::Real(n) => Some(*n as i64),
                            _ => None,
                        })
                        .collect(),
                    _ => Vec::new(),
                })
                .collect();
            // An empty Segments list satisfies the rule outright.
            if !segments.is_empty() && !super::express::consecutive_segments(&segments) {
                out.push(s.violation(
                    rule,
                    ViolationKind::Disagreement,
                    "Segments do not join end-to-start".to_string(),
                ));
            }
        }
    }
}

/// `IfcSweptDiskSolidPolygonal.DirectrixIsPolyline`:
/// `('IFCPOLYLINE' IN TYPEOF(Directrix)) OR (('IFCINDEXEDPOLYCURVE' IN
/// TYPEOF(Directrix)) AND NOT(EXISTS(Directrix.Segments)))`.
fn directrix_is_polyline(s: &Subject<'_>, rule: &'static str, out: &mut Vec<RuleViolation>) {
    // Directrix is slot 0, inherited from IfcSweptDiskSolid.
    let Some(Value::Ref(target)) = s.entity.attribute(0).map(|v| v.unwrap_typed()) else {
        return;
    };
    let Some(e) = s.model.get(*target) else {
        return;
    };
    let violated = if s.ref_is_a(*target, "IFCPOLYLINE") {
        false
    } else if s.ref_is_a(*target, "IFCINDEXEDPOLYCURVE") {
        // NOT(EXISTS(Directrix.Segments)).
        matches!(e.attribute(1).map(|v| v.unwrap_typed()), Some(Value::List(seg)) if !seg.is_empty())
    } else {
        s.ref_is_none_of(*target, &["IFCPOLYLINE"])
    };
    if violated {
        out.push(s.violation(
            rule,
            ViolationKind::WrongType,
            format!(
                "Directrix {target} is {}, must be a polyline",
                e.type_name.to_ascii_uppercase()
            ),
        ));
    }
}

/// A referenced entity must (or must not) be of a given kind.
///
/// `want` selects the direction: `true` requires the kind, `false`
/// forbids it. Both forms appear in the schema and reading them as one
/// predicate keeps the call sites honest about which way round they are.
fn require_kind(
    s: &Subject<'_>,
    slot: usize,
    kind: &str,
    want: bool,
    rule: &'static str,
    label: &str,
    out: &mut Vec<RuleViolation>,
) {
    let Some(Value::Ref(target)) = s.entity.attribute(slot).map(|v| v.unwrap_typed()) else {
        return;
    };
    let target: EntityId = *target;
    let Some(e) = s.model.get(target) else { return };
    let violated = if want {
        s.ref_is_none_of(target, &[kind])
    } else {
        s.ref_is_a(target, kind)
    };
    if !violated {
        return;
    }
    let actual = e.type_name.to_ascii_uppercase();
    let detail = if want {
        format!("{label} {target} is {actual}, must be a {kind}")
    } else {
        format!("{label} {target} is {actual}, which a {kind} excludes")
    };
    out.push(s.violation(rule, ViolationKind::WrongType, detail));
}
