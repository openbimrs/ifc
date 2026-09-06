//! `IfcGetBasisSurface`: the surfaces a curve-on-surface lies on.
//!
//! # A curve-on-surface reaches its surface three different ways
//!
//! - an `IfcPcurve` names its `BasisSurface` directly;
//! - an `IfcSurfaceCurve` collects one surface per associated p-curve
//!   (this is `IfcAssociatedSurface`, which is just that field read);
//! - an `IfcCompositeCurveOnSurface` intersects what its segments carry.
//!
//! # A schema erratum in the composite branch
//!
//! The normative body loops `Segments[1]` rather than `Segments[i]`:
//!
//! ```text
//!   Surfs := IfcGetBasisSurface(C\IfcCompositeCurve.Segments[1].ParentCurve);
//!   IF N > 1 THEN
//!     REPEAT i := 2 TO N;
//!       Surfs := Surfs * IfcGetBasisSurface(C\IfcCompositeCurve.Segments[1].ParentCurve);
//!     END_REPEAT;
//!   END_IF;
//! ```
//!
//! Intersecting segment one with itself is the identity, so read literally
//! the loop can never narrow the set and the rule it feeds (`SameSurface`)
//! becomes unfalsifiable. The prose directly above the loop states the
//! intent: "the BasisSurface is the intersection of the BasisSurface of all
//! the segments". This module implements that stated intent, so
//! `SameSurface` can actually fail on a composite curve whose segments sit
//! on different surfaces.

use std::collections::BTreeSet;

use ifc_model::{Entity, EntityId, Model, Value};

/// Maximum nesting depth, matching the resolver in `rules::dimension`.
const MAX_DEPTH: usize = 16;

/// The surfaces `curve` lies on, per `IfcGetBasisSurface`.
///
/// Returns an empty set when the curve is not a curve-on-surface, when the
/// file omits the surface, or when nesting exceeds the depth cap. The schema
/// types the result `SET[0:2]`, so emptiness is a legal answer, not an error.
pub fn basis_surfaces(model: &Model, curve: EntityId) -> BTreeSet<EntityId> {
    basis_with_depth(model, curve, 0)
}

fn basis_with_depth(model: &Model, curve: EntityId, depth: usize) -> BTreeSet<EntityId> {
    let mut out = BTreeSet::new();
    if depth > MAX_DEPTH {
        return out;
    }
    let Some(entity) = model.get(curve) else {
        return out;
    };
    let name = entity.type_name.to_ascii_uppercase();

    // An IfcPcurve names its surface in slot 0. IfcAssociatedSurface is
    // exactly this read, which is why it needs no separate function.
    if name == "IFCPCURVE" {
        if let Some(surface) = slot_ref(entity, 0) {
            out.insert(surface);
        }
        return out;
    }

    // Checked before the surface-curve branch: the two are disjoint, and a
    // composite curve on surface is an IfcCompositeCurve, not a surface curve.
    if crate::select::is_a(&name, "IFCCOMPOSITECURVEONSURFACE") {
        return composite_intersection(model, entity, depth);
    }

    if crate::select::is_a(&name, "IFCSURFACECURVE") {
        // AssociatedGeometry is slot 1: Curve3D, AssociatedGeometry, ...
        for pcurve in list_refs(entity, 1) {
            out.extend(basis_with_depth(model, pcurve, depth + 1));
        }
    }
    out
}

/// The intersection of every segment's basis surfaces.
///
/// Implements the documented intent rather than the errant `Segments[1]`
/// index; see the module header.
fn composite_intersection(model: &Model, entity: &Entity, depth: usize) -> BTreeSet<EntityId> {
    let mut acc: Option<BTreeSet<EntityId>> = None;
    // Segments is slot 0 on IfcCompositeCurve; ParentCurve is slot 2 on
    // IfcCompositeCurveSegment (Transition, SameSense, ParentCurve).
    for segment in list_refs(entity, 0) {
        let Some(parent) = model.get(segment).and_then(|s| slot_ref(s, 2)) else {
            continue;
        };
        let surfaces = basis_with_depth(model, parent, depth + 1);
        acc = Some(match acc {
            None => surfaces,
            Some(prev) => prev.intersection(&surfaces).copied().collect(),
        });
    }
    acc.unwrap_or_default()
}

/// The entity referenced at `slot`, unwrapping any defined-type wrapper.
fn slot_ref(entity: &Entity, slot: usize) -> Option<EntityId> {
    match entity.attribute(slot).map(|v| v.unwrap_typed()) {
        Some(Value::Ref(id)) => Some(*id),
        _ => None,
    }
}

/// Every entity reference in the list at `slot`.
fn list_refs(entity: &Entity, slot: usize) -> Vec<EntityId> {
    match entity.attribute(slot).map(|v| v.unwrap_typed()) {
        Some(Value::List(items)) => items
            .iter()
            .filter_map(|v| match v.unwrap_typed() {
                Value::Ref(id) => Some(*id),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}
