//! Type-membership `WHERE` rules: `IN TYPEOF(slot)`.
//!
//! # Exact type versus subtype
//!
//! EXPRESS `TYPEOF` yields the full supertype set, so
//! `IFCBOUNDEDCURVE IN TYPEOF(x)` is a subtype test, not an exact-name
//! match. These rules therefore go through `select::is_a` rather than
//! comparing names, or a polyline would fail to count as a bounded curve.

use ifc_model::{Entity, EntityId, Model, Value};

use super::violation::{RuleViolation, ViolationKind};

/// Run the type-membership rules that apply to this entity.
pub fn check(model: &Model, id: EntityId, entity: &Entity, out: &mut Vec<RuleViolation>) {
    let name = entity.type_name.to_ascii_uppercase();

    // A composite curve segment must carry a bounded parent curve:
    // an unbounded line has no span to contribute to the composite.
    if name == "IFCCOMPOSITECURVESEGMENT" || name == "IFCREPARAMETRISEDCOMPOSITECURVESEGMENT" {
        require_kind(
            model,
            id,
            entity,
            &name,
            2,
            "IFCBOUNDEDCURVE",
            true,
            "ParentIsBoundedCurve",
            "ParentCurve",
            out,
        );
    }

    // Trimming an already-bounded curve is meaningless: its bounds are
    // its own, and the trim parameters would contradict them.
    if name == "IFCTRIMMEDCURVE" {
        require_kind(
            model,
            id,
            entity,
            &name,
            0,
            "IFCBOUNDEDCURVE",
            false,
            "NoTrimOfBoundedCurves",
            "BasisCurve",
            out,
        );
    }

    // A surface curve states its 3D geometry; a p-curve is defined in a
    // parameter domain, so it cannot serve as that 3D curve.
    if crate::select::is_a(&name, "IFCSURFACECURVE") {
        require_kind(
            model,
            id,
            entity,
            &name,
            0,
            "IFCPCURVE",
            false,
            "CurveIsNotPcurve",
            "Curve3D",
            out,
        );
    }

    // A boxed half space bounds an unbounded surface; a curve-bounded
    // plane already carries its own boundary.
    if name == "IFCBOXEDHALFSPACE" {
        require_kind(
            model,
            id,
            entity,
            &name,
            0,
            "IFCCURVEBOUNDEDPLANE",
            false,
            "UnboundedSurface",
            "BaseSurface",
            out,
        );
    }

    // A geometric CURVE set must hold no surfaces.
    if name == "IFCGEOMETRICCURVESET" {
        if let Some(Value::List(items)) = entity.attribute(0).map(|v| v.unwrap_typed()) {
            for item in items {
                let Value::Ref(target) = item.unwrap_typed() else {
                    continue;
                };
                let Some(e) = model.get(*target) else {
                    continue;
                };
                if crate::select::is_a(&e.type_name.to_ascii_uppercase(), "IFCSURFACE") {
                    out.push(RuleViolation::new(
                        id,
                        name.clone(),
                        "NoSurfaces",
                        ViolationKind::WrongType,
                        format!("Elements holds surface {target}, which a curve set excludes"),
                    ));
                }
            }
        }
    }

    // A polygonal swept disk needs a polyline directrix, or an indexed
    // poly-curve with no explicit segments (which is the same shape).
    if name == "IFCSWEPTDISKSOLIDPOLYGONAL" {
        if let Some(Value::Ref(target)) = entity.attribute(0).map(|v| v.unwrap_typed()) {
            if let Some(e) = model.get(*target) {
                let n = e.type_name.to_ascii_uppercase();
                let plain_indexed = n == "IFCINDEXEDPOLYCURVE"
                    && !matches!(e.attribute(1).map(|v| v.unwrap_typed()), Some(Value::List(s)) if !s.is_empty());
                if n != "IFCPOLYLINE" && !plain_indexed {
                    out.push(RuleViolation::new(
                        id,
                        name.clone(),
                        "DirectrixIsPolyline",
                        ViolationKind::WrongType,
                        format!("Directrix {target} is {n}, must be a polyline"),
                    ));
                }
            }
        }
    }
}

/// A referenced entity must (or must not) be of a given kind.
///
/// `want` selects the direction: `true` requires the kind, `false`
/// forbids it. Both forms appear in the schema and reading them as one
/// predicate keeps the call sites honest about which way round they are.
#[allow(clippy::too_many_arguments)]
fn require_kind(
    model: &Model,
    id: EntityId,
    entity: &Entity,
    type_name: &str,
    slot: usize,
    kind: &str,
    want: bool,
    rule: &'static str,
    label: &str,
    out: &mut Vec<RuleViolation>,
) {
    let Some(Value::Ref(target)) = entity.attribute(slot).map(|v| v.unwrap_typed()) else {
        return;
    };
    let Some(e) = model.get(*target) else { return };
    let actual = e.type_name.to_ascii_uppercase();
    if crate::select::is_a(&actual, kind) == want {
        return;
    }
    let detail = if want {
        format!("{label} {target} is {actual}, must be a {kind}")
    } else {
        format!("{label} {target} is {actual}, which a {kind} excludes")
    };
    out.push(RuleViolation::new(
        id,
        type_name.to_string(),
        rule,
        ViolationKind::WrongType,
        detail,
    ));
}
