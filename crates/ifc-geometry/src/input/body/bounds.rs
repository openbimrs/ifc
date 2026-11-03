//! Exact bounds of a tessellated item, read from its vertex list.
//!
//! # Why only the referenced vertices
//!
//! A face set's `Coordinates` may hold points no face addresses: authoring
//! tools leave unreferenced vertices behind, and an `IfcIndexedPolygonalFace`
//! may reach the list only through `PnIndex`. Lowering builds geometry from
//! the faces alone, so a box over the whole list would claim extent the
//! lowered geometry does not have. The bounds here are the box of the
//! vertices the faces address, which is the box of the lowered mesh.
//!
//! The box is exact, in the representation frame composed with the item's
//! mappings, in metres. An item lowering refuses (index 0, an index past
//! the end, a dangling point list, a TIN void or hole) is refused by name
//! here too; no partial box is returned. A 2D point list is refused here as
//! `WrongEntityType`, resolved as the `IfcCartesianPointList3D` the schema
//! requires, where lowering fails on the row width.

use ifc_model::{Entity, EntityId, Model};

use super::BodyItemBounds;
use crate::error::{GeometryError, GeometryResult};
use crate::slots::Slots;
use crate::solid::tessellated::{
    irregular_network, IndexedPolygonalFace, IndexedPolygonalFaceWithVoids, PolygonalFaceSet,
    TessellatedFaceSet, TriangulatedFaceSet, TriangulatedIrregularNetwork,
};
use crate::transform::Transform;
use crate::units::UnitScale;

/// The bounds of one tessellated item placed by `local`.
pub(super) fn tessellated(
    model: &Model,
    units: &UnitScale,
    id: EntityId,
    entity: &Entity,
    type_name: &str,
    local: Transform,
) -> GeometryResult<BodyItemBounds> {
    let (base, referenced) = match type_name {
        "IFCPOLYGONALFACESET" => polygonal(model, id, entity, type_name)?,
        "IFCTRIANGULATEDFACESET" => triangulated(id, entity, false)?,
        irregular_network::TYPE => triangulated(id, entity, true)?,
        _ => {
            return Err(Slots::new(id, entity).unsupported("tessellated item family is not bounded"))
        }
    };
    let points = base.coordinate_list(model)?.coordinates()?;
    let mut bounds: Option<BodyItemBounds> = None;
    for index in referenced {
        let raw = points.get(index).ok_or_else(|| {
            degenerate(
                id,
                type_name,
                format!(
                    "CoordIndex addresses vertex {} but Coordinates has {}",
                    index + 1,
                    points.len()
                ),
            )
        })?;
        let scaled = raw.map(|value| units.length(value));
        let point = local.apply(scaled);
        if point.iter().any(|value| !value.is_finite()) {
            return Err(degenerate(
                id,
                type_name,
                format!("vertex {} of Coordinates is not finite", index + 1),
            ));
        }
        match bounds.as_mut() {
            None => {
                bounds = Some(BodyItemBounds {
                    min: point,
                    max: point,
                })
            }
            Some(known) => {
                for (axis, value) in point.into_iter().enumerate() {
                    known.min[axis] = known.min[axis].min(value);
                    known.max[axis] = known.max[axis].max(value);
                }
            }
        }
    }
    bounds.ok_or_else(|| degenerate(id, type_name, "the face set references no vertex".into()))
}

fn degenerate(id: EntityId, type_name: &str, detail: String) -> GeometryError {
    GeometryError::Degenerate {
        entity: id,
        type_name: type_name.to_string(),
        detail,
    }
}

/// Every `Coordinates` index an `IfcPolygonalFaceSet` addresses, 0-based.
fn polygonal<'m>(
    model: &'m Model,
    id: EntityId,
    entity: &'m Entity,
    type_name: &str,
) -> GeometryResult<(TessellatedFaceSet<'m>, Vec<usize>)> {
    let view = PolygonalFaceSet::new(id, entity);
    let pn = view.pn_index_0based()?;
    let slots = Slots::new(id, entity);
    let mut referenced = Vec::new();
    let mut take = |authored: &[usize]| -> GeometryResult<()> {
        for index in authored {
            referenced.push(match &pn {
                Some(map) => *map.get(*index).ok_or_else(|| {
                    degenerate(
                        id,
                        type_name,
                        format!(
                            "CoordIndex {} is past the end of PnIndex ({} entries)",
                            index + 1,
                            map.len()
                        ),
                    )
                })?,
                None => *index,
            });
        }
        Ok(())
    };
    for face_ref in view.faces()? {
        let face_entity = slots.resolve(model, face_ref)?;
        let face = IndexedPolygonalFace::new(face_ref, face_entity);
        take(&face.outer_loop_0based()?)?;
        if face.has_voids() {
            for inner in
                IndexedPolygonalFaceWithVoids::new(face_ref, face_entity).inner_loops_0based()?
            {
                take(&inner)?;
            }
        }
    }
    Ok((view.base(), referenced))
}

/// Every `Coordinates` index an `IfcTriangulatedFaceSet` addresses, 0-based.
fn triangulated(
    id: EntityId,
    entity: &Entity,
    network: bool,
) -> GeometryResult<(TessellatedFaceSet<'_>, Vec<usize>)> {
    let view = TriangulatedFaceSet::new(id, entity);
    let triangles = view.triangles_0based()?;
    if network {
        TriangulatedIrregularNetwork::new(id, entity).admit(triangles.len())?;
    }
    Ok((view.base(), triangles.into_iter().flatten().collect()))
}
