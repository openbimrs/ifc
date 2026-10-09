//! Dimensionality of a geometry resource entity.
//!
//! `Dim` is a DERIVED attribute in EXPRESS: it is never written in the file,
//! it is computed from whatever the entity points at. Enforcing any of the
//! schema's dimensional WHERE rules therefore needs the derivation itself,
//! not a slot read.
//!
//! The curve half transcribes `IfcCurveDim` from the schema. Two of its
//! answers are easy to get wrong from intuition:
//!
//! - `IfcPcurve` is **3**, not 2. A p-curve is a curve *on a surface* in
//!   model space; its 2D-ness lives in `ReferenceCurve`, which is what
//!   `IfcPcurve.DimIs2D` constrains.
//! - `IfcOffsetCurve2D`/`3D` are fixed by their own type and do not consult
//!   the basis curve, so a 3D basis under `IfcOffsetCurve2D` still reports 2
//!   and is caught by that entity's own rule instead.

use ifc_model::{EntityId, Model, Value};

use super::release::Release;
use crate::resource::direction::Direction;
use crate::resource::point::CartesianPoint;

/// How deep a `Dim` derivation may recurse before it is called malformed.
///
/// Trimmed curves nest, and a cyclic file must not hang the validator.
const MAX_DEPTH: usize = 32;

/// Dimensionality of any geometry resource entity in `release`, or `None`
/// when the schema leaves it undefined (`RETURN(?)`) or the file is too
/// malformed to say.
///
/// Every type test is `TYPEOF` in the release's own entity table.
pub(crate) fn dim_of(release: &Release, model: &Model, id: EntityId) -> Option<usize> {
    dim_with_depth(release, model, id, 0)
}

fn dim_with_depth(release: &Release, model: &Model, id: EntityId, depth: usize) -> Option<usize> {
    if depth > MAX_DEPTH {
        return None;
    }
    let entity = model.get(id)?;
    let name: &str = &entity.type_name;
    let is_a = |super_type: &str| release.is_a(name, super_type);
    let next = |id: EntityId| dim_with_depth(release, model, id, depth + 1);

    // Leaves: the two entities that carry explicit coordinate lists.
    if is_a("IFCCARTESIANPOINT") {
        return CartesianPoint::new(id, entity)
            .coordinates()
            .ok()
            .map(|c| c.len());
    }
    if is_a("IFCDIRECTION") {
        return Direction::new(id, entity).ratios().ok().map(|r| r.len());
    }

    // Entities the schema fixes at three dimensions outright.
    if is_a("IFCSURFACE")
        || is_a("IFCSOLIDMODEL")
        || is_a("IFCHALFSPACESOLID")
        || is_a("IFCCSGPRIMITIVE3D")
        || is_a("IFCTESSELLATEDFACESET")
        || is_a("IFCSECTIONEDSPINE")
        || is_a("IFCBOUNDINGBOX")
        || is_a("IFCFACEBASEDSURFACEMODEL")
        || is_a("IFCSHELLBASEDSURFACEMODEL")
    {
        return Some(3);
    }

    // A placement takes its dimensionality from its Location point.
    if is_a("IFCPLACEMENT") {
        return slot_ref(entity, 0).and_then(next);
    }
    if is_a("IFCCARTESIANTRANSFORMATIONOPERATOR") {
        // LocalOrigin is slot 2: Axis1, Axis2, LocalOrigin, Scale.
        return slot_ref(entity, 2).and_then(next);
    }
    if is_a("IFCGEOMETRICSET") {
        return first_of_list(entity, 0).and_then(next);
    }
    if is_a("IFCBOOLEANRESULT") {
        return slot_ref(entity, 1).and_then(next);
    }
    // IfcCompositeCurveSegment.Dim := ParentCurve.Dim (IFC2X3 TC1 to
    // IFC4X2); IFC4X3 ADD2 derives both segment kinds through
    // IfcSegmentDim, which reads the same ParentCurve: slot 2 on a composite
    // curve segment (Transition, SameSense, ParentCurve), slot 4 on an
    // IfcCurveSegment (Transition, Placement, SegmentStart, SegmentLength,
    // ParentCurve).
    if is_a("IFCCOMPOSITECURVESEGMENT") {
        return slot_ref(entity, 2).and_then(next);
    }
    if is_a("IFCCURVESEGMENT") {
        return slot_ref(entity, 4).and_then(next);
    }
    if is_a("IFCCURVE") {
        return curve_dim(release, model, entity, depth);
    }
    None
}

/// `IfcCurveDim`, transcribed from each release's schema.
///
/// The releases differ only in the families they add, and each family
/// exists only in the releases that list it, so one ordered list serves
/// every release:
///
/// - IFC2X3 TC1: line, conic, polyline, trimmed, composite, B-spline,
///   offset 2D/3D;
/// - IFC4 ADD2 TC1 adds `IfcPcurve` (3) and `IfcIndexedPolyCurve`;
/// - IFC4X1 and IFC4X2 add `IfcOffsetCurveByDistances` (3),
///   `IfcCurveSegment2D` (2) and `IfcAlignmentCurve` (3);
/// - IFC4X3 ADD2 drops the last two and adds `IfcGradientCurve` and
///   `IfcSegmentedReferenceCurve` (3, tested before the composite curve
///   they specialise), `IfcPolynomialCurve` and `IfcSpiral`.
fn curve_dim(
    release: &Release,
    model: &Model,
    entity: &ifc_model::Entity,
    depth: usize,
) -> Option<usize> {
    let is_a = |super_type: &str| release.is_a(&entity.type_name, super_type);
    let next = |id: EntityId| dim_with_depth(release, model, id, depth + 1);

    if is_a("IFCLINE") {
        // Pnt is slot 0.
        return slot_ref(entity, 0).and_then(next);
    }
    if is_a("IFCCONIC") {
        // Position is slot 0 and is itself a placement.
        return slot_ref(entity, 0).and_then(next);
    }
    if is_a("IFCPOLYLINE") {
        return first_of_list(entity, 0).and_then(next);
    }
    if is_a("IFCTRIMMEDCURVE") {
        return slot_ref(entity, 0).and_then(next);
    }
    if is_a("IFCGRADIENTCURVE") || is_a("IFCSEGMENTEDREFERENCECURVE") {
        return Some(3);
    }
    if is_a("IFCCOMPOSITECURVE") {
        return first_of_list(entity, 0).and_then(next);
    }
    if is_a("IFCBSPLINECURVE") {
        // ControlPointsList is slot 1: Degree, ControlPointsList, ...
        return first_of_list(entity, 1).and_then(next);
    }
    if is_a("IFCOFFSETCURVE2D") || is_a("IFCCURVESEGMENT2D") {
        return Some(2);
    }
    if is_a("IFCOFFSETCURVE3D") || is_a("IFCOFFSETCURVEBYDISTANCES") || is_a("IFCALIGNMENTCURVE") {
        return Some(3);
    }
    if is_a("IFCPOLYNOMIALCURVE") {
        // Position, CoefficientsX, CoefficientsY, CoefficientsZ: 2 only
        // for a 2D position without z coefficients.
        let no_z = matches!(
            entity.attribute(3).map(|v| v.unwrap_typed()),
            None | Some(Value::Null)
        );
        let position = slot_ref(entity, 0).and_then(next);
        return Some(if no_z && position == Some(2) { 2 } else { 3 });
    }
    if is_a("IFCPCURVE") {
        // A p-curve is a 3D curve lying on a surface; only its reference
        // curve is two-dimensional.
        return Some(3);
    }
    if is_a("IFCINDEXEDPOLYCURVE") {
        return slot_ref(entity, 0).and_then(|p| point_list_dim(release, model, p));
    }
    if is_a("IFCSPIRAL") {
        // Position is slot 0.
        return slot_ref(entity, 0).and_then(next);
    }
    None
}

/// `IfcPointListDim`: 2 for a 2D point list, 3 for a 3D one.
fn point_list_dim(release: &Release, model: &Model, id: EntityId) -> Option<usize> {
    if release.ref_is_a(model, id, "IFCCARTESIANPOINTLIST2D") {
        Some(2)
    } else if release.ref_is_a(model, id, "IFCCARTESIANPOINTLIST3D") {
        Some(3)
    } else {
        None
    }
}

/// Entity reference held directly in a slot.
fn slot_ref(entity: &ifc_model::Entity, slot: usize) -> Option<EntityId> {
    match entity.attribute(slot)?.unwrap_typed() {
        Value::Ref(id) => Some(*id),
        _ => None,
    }
}

/// First entity reference inside a list-valued slot.
fn first_of_list(entity: &ifc_model::Entity, slot: usize) -> Option<EntityId> {
    match entity.attribute(slot)?.unwrap_typed() {
        Value::List(items) => items.iter().find_map(|v| match v.unwrap_typed() {
            Value::Ref(id) => Some(*id),
            _ => None,
        }),
        _ => None,
    }
}

/// Every entity in a list-valued slot that resolves to a reference.
pub fn list_refs(entity: &ifc_model::Entity, slot: usize) -> Vec<EntityId> {
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

/// The first member of `ids` whose dimensionality differs from the first
/// resolvable one, reported as `(expected, found, offender)`.
///
/// Returns `None` when the list agrees, is empty, or nothing resolves --
/// a rule cannot fire on data it cannot read.
pub(crate) fn first_dim_disagreement(
    release: &Release,
    model: &Model,
    ids: &[EntityId],
) -> Option<(usize, usize, EntityId)> {
    let mut expected: Option<usize> = None;
    for id in ids {
        let Some(dim) = dim_of(release, model, *id) else {
            continue;
        };
        match expected {
            None => expected = Some(dim),
            Some(first) if dim != first => return Some((first, dim, *id)),
            Some(_) => {}
        }
    }
    None
}

#[cfg(test)]
mod probe {
    use super::*;
    use ifc_model::{Entity, Value};

    #[test]
    fn a_pcurve_resolves_to_three_dimensions() {
        let mut m = Model::new();
        m.insert(
            EntityId(1),
            Entity::new(
                "IFCCARTESIANPOINT",
                vec![Value::List(vec![Value::Real(0.0), Value::Real(0.0)])],
            ),
        );
        m.insert(
            EntityId(3),
            Entity::new(
                "IFCPOLYLINE",
                vec![Value::List(vec![Value::Ref(EntityId(1))])],
            ),
        );
        m.insert(
            EntityId(4),
            Entity::new("IFCPCURVE", vec![Value::Null, Value::Ref(EntityId(3))]),
        );
        let release = Release::of(&m);
        assert_eq!(dim_of(&release, &m, EntityId(3)), Some(2), "polyline is 2D");
        assert_eq!(
            dim_of(&release, &m, EntityId(4)),
            Some(3),
            "IfcCurveDim: p-curve is 3D"
        );
    }
}
