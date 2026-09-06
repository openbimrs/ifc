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

use crate::resource::direction::Direction;
use crate::resource::point::CartesianPoint;

/// How deep a `Dim` derivation may recurse before it is called malformed.
///
/// Trimmed curves nest, and a cyclic file must not hang the validator.
const MAX_DEPTH: usize = 32;

/// Dimensionality of any geometry resource entity, or `None` when the schema
/// leaves it undefined (`RETURN(?)`) or the file is too malformed to say.
pub fn dim_of(model: &Model, id: EntityId) -> Option<usize> {
    dim_with_depth(model, id, 0)
}

fn dim_with_depth(model: &Model, id: EntityId, depth: usize) -> Option<usize> {
    if depth > MAX_DEPTH {
        return None;
    }
    let entity = model.get(id)?;
    let name = entity.type_name.to_ascii_uppercase();
    let is_a = |super_type: &str| crate::select::is_a(&name, super_type);

    // Leaves: the two entities that carry explicit coordinate lists.
    if name == "IFCCARTESIANPOINT" {
        return CartesianPoint::new(id, entity)
            .coordinates()
            .ok()
            .map(|c| c.len());
    }
    if name == "IFCDIRECTION" {
        return Direction::new(id, entity).ratios().ok().map(|r| r.len());
    }

    // Entities the schema fixes at three dimensions outright.
    if is_a("IFCSURFACE")
        || is_a("IFCSOLIDMODEL")
        || is_a("IFCHALFSPACESOLID")
        || is_a("IFCCSGPRIMITIVE3D")
        || is_a("IFCTESSELLATEDFACESET")
        || is_a("IFCSECTIONEDSPINE")
        || name == "IFCBOUNDINGBOX"
        || name == "IFCFACEBASEDSURFACEMODEL"
        || name == "IFCSHELLBASEDSURFACEMODEL"
    {
        return Some(3);
    }

    // A placement takes its dimensionality from its Location point.
    if is_a("IFCPLACEMENT") {
        return slot_ref(entity, 0).and_then(|p| dim_with_depth(model, p, depth + 1));
    }
    if is_a("IFCCARTESIANTRANSFORMATIONOPERATOR") {
        // LocalOrigin is slot 2: Axis1, Axis2, LocalOrigin, Scale.
        return slot_ref(entity, 2).and_then(|p| dim_with_depth(model, p, depth + 1));
    }
    if name == "IFCGEOMETRICSET" || name == "IFCGEOMETRICCURVESET" {
        return first_of_list(entity, 0).and_then(|e| dim_with_depth(model, e, depth + 1));
    }
    if is_a("IFCBOOLEANRESULT") {
        return slot_ref(entity, 1).and_then(|e| dim_with_depth(model, e, depth + 1));
    }
    if name == "IFCCOMPOSITECURVESEGMENT" {
        return slot_ref(entity, 2).and_then(|c| dim_with_depth(model, c, depth + 1));
    }
    if is_a("IFCCURVE") {
        return curve_dim(model, id, entity, &name, depth);
    }
    None
}

/// `IfcCurveDim`, transcribed from the schema.
///
/// Order matters: `IfcTrimmedCurve` and the offsets must be tested before the
/// generic families they would otherwise fall through to.
fn curve_dim(
    model: &Model,
    id: EntityId,
    entity: &ifc_model::Entity,
    name: &str,
    depth: usize,
) -> Option<usize> {
    let is_a = |super_type: &str| crate::select::is_a(name, super_type);

    if name == "IFCOFFSETCURVE2D" {
        return Some(2);
    }
    if name == "IFCOFFSETCURVE3D" || name == "IFCPCURVE" {
        // A p-curve is a 3D curve lying on a surface; only its reference
        // curve is two-dimensional.
        return Some(3);
    }
    if name == "IFCLINE" {
        // Pnt is slot 0.
        return slot_ref(entity, 0).and_then(|p| dim_with_depth(model, p, depth + 1));
    }
    if is_a("IFCCONIC") {
        // Position is slot 0 and is itself a placement.
        return slot_ref(entity, 0).and_then(|p| dim_with_depth(model, p, depth + 1));
    }
    if name == "IFCPOLYLINE" {
        return first_of_list(entity, 0).and_then(|p| dim_with_depth(model, p, depth + 1));
    }
    if name == "IFCTRIMMEDCURVE" {
        return slot_ref(entity, 0).and_then(|c| dim_with_depth(model, c, depth + 1));
    }
    if is_a("IFCCOMPOSITECURVE") {
        return first_of_list(entity, 0).and_then(|s| dim_with_depth(model, s, depth + 1));
    }
    if is_a("IFCBSPLINECURVE") {
        // ControlPointsList is slot 1: Degree, ControlPointsList, ...
        return first_of_list(entity, 1).and_then(|p| dim_with_depth(model, p, depth + 1));
    }
    if name == "IFCINDEXEDPOLYCURVE" {
        return slot_ref(entity, 0).and_then(|p| point_list_dim(model, p));
    }
    let _ = id;
    None
}

/// `IfcPointListDim`: 2 for a 2D point list, 3 for a 3D one.
fn point_list_dim(model: &Model, id: EntityId) -> Option<usize> {
    let entity = model.get(id)?;
    match entity.type_name.to_ascii_uppercase().as_str() {
        "IFCCARTESIANPOINTLIST2D" => Some(2),
        "IFCCARTESIANPOINTLIST3D" => Some(3),
        _ => None,
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
pub fn first_dim_disagreement(model: &Model, ids: &[EntityId]) -> Option<(usize, usize, EntityId)> {
    let mut expected: Option<usize> = None;
    for id in ids {
        let Some(dim) = dim_of(model, *id) else {
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
        assert_eq!(dim_of(&m, EntityId(3)), Some(2), "polyline is 2D");
        assert_eq!(
            dim_of(&m, EntityId(4)),
            Some(3),
            "IfcCurveDim: p-curve is 3D"
        );
    }
}
