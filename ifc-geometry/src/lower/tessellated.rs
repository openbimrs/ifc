//! Tessellated face-set lowering into indexed meshes.
//!
//! # Why a mesh and not topology
//!
//! A tessellated face set is *already* a discretisation. Unlike an
//! `IfcFacetedBrep`, which is an exact solid whose planar faces happen to be
//! polygons, a face set carries no adjacency, no bound nesting, and no claim
//! to exactness. Recovering topology from it means inferring shared edges by
//! comparing float positions, which invents information the file never had.
//! So these lower to `axiolid-mesh` types and stay meshes.
//!
//! # Authored n-gons survive
//!
//! `IfcPolygonalFaceSet` faces are n-gons with optional holes. Triangulating
//! them here would be a lossy decision taken at read time, in the wrong crate:
//! a hole-aware tessellator needs a fill rule and a tolerance, both of which
//! belong to the kernel. `PolygonMesh` stores the authored loops verbatim, so
//! the choice stays with whoever consumes the graph.
//!
//! # Coordinates are shared, indices are not
//!
//! Both families index one `IfcCartesianPointList3D`. The point list is read
//! once and scaled once; faces only carry indices into it. Emitting one vertex
//! per face corner would multiply a wall's vertex count by roughly six and
//! break every consumer that relies on shared positions.

use axiolid_core::{Point3, Vec3};
use axiolid_mesh::{NormalAttribute, PolygonFace, PolygonMesh, TriMesh};
use axiolid_model::{GeometryNode, NodeId};
use ifc_model::EntityId;

use crate::error::GeometryResult;
use crate::lower::session::LoweringSession;
use crate::resource::point::CartesianPointList3D;
use crate::solid::tessellated::{
    IndexedPolygonalFace, IndexedPolygonalFaceWithVoids, PolygonalFaceSet, TriangulatedFaceSet,
};
use crate::transform::Transform;

/// Chain kind reported when a face set nests too deeply or cycles.
const KIND: &str = "tessellated face set";

/// Read `Coordinates`, convert to metres, and place by `frame`.
///
/// One pass over the shared point list. The scale is applied before the frame
/// because the frame is already expressed in metres.
fn positions(
    session: &mut LoweringSession<'_>,
    owner: EntityId,
    coordinates: EntityId,
    frame: Transform,
) -> GeometryResult<Vec<Point3>> {
    let entity = session.entity(owner, coordinates)?;
    let raw = CartesianPointList3D::new(coordinates, entity).coordinates()?;
    Ok(raw
        .into_iter()
        .map(|point| {
            let scaled = point.map(|value| session.units().length(value));
            Point3::from_array(frame.apply(scaled))
        })
        .collect())
}

/// Reject an index that addresses past the end of the coordinate list.
///
/// A face set whose indices dangle is not recoverable: the vertex it names
/// does not exist, and substituting the origin would render a spike to
/// (0,0,0) that looks like geometry. Fail with the entity named instead.
fn checked_index(
    session: &LoweringSession<'_>,
    owner: EntityId,
    index: usize,
    vertex_count: usize,
    field: &str,
) -> GeometryResult<u32> {
    if index >= vertex_count {
        return Err(session.degenerate(
            owner,
            KIND,
            format!(
                "{field} addresses vertex {} but Coordinates has {vertex_count}",
                index + 1
            ),
        ));
    }
    Ok(index as u32)
}

/// Lower an `IfcTriangulatedFaceSet` into [`GeometryNode::TriMesh`].
pub fn lower_triangulated_face_set_node(
    session: &mut LoweringSession<'_>,
    id: EntityId,
    frame: Transform,
) -> GeometryResult<NodeId> {
    if let Some(node) = session.memoized(id, KIND, frame) {
        return Ok(node);
    }
    session.enter(id, KIND)?;
    let result = build_triangulated(session, id, frame);
    session.exit(id);
    let node = result?;
    session.memoize(id, KIND, frame, node);
    Ok(node)
}

fn build_triangulated(
    session: &mut LoweringSession<'_>,
    id: EntityId,
    frame: Transform,
) -> GeometryResult<NodeId> {
    let entity = session.entity(id, id)?;
    let view = TriangulatedFaceSet::new(id, entity);
    let coordinates = view.coordinates()?;
    let triangles = view.triangles_0based()?;
    let normals = view.normals();
    let positions = positions(session, id, coordinates, frame)?;

    let mut indices = Vec::with_capacity(triangles.len() * 3);
    for triangle in &triangles {
        for corner in triangle {
            indices.push(checked_index(
                session,
                id,
                *corner,
                positions.len(),
                "CoordIndex",
            )?);
        }
    }

    let mut mesh = TriMesh::new(positions, indices);
    // Normals are directions: the linear part applies, the translation does
    // not. Sending them through `apply` would drag every normal to the frame's
    // origin and invert lighting on any placed product.
    if let Some(values) = normals {
        mesh.normals = Some(NormalAttribute {
            values: values
                .into_iter()
                .map(|normal| Vec3::from_array(frame.apply_direction(normal)))
                .collect(),
            indices: None,
        });
    }
    session.node_for(id, GeometryNode::TriMesh(mesh))
}

/// Lower an `IfcPolygonalFaceSet` into [`GeometryNode::PolygonMesh`].
pub fn lower_polygonal_face_set_node(
    session: &mut LoweringSession<'_>,
    id: EntityId,
    frame: Transform,
) -> GeometryResult<NodeId> {
    if let Some(node) = session.memoized(id, KIND, frame) {
        return Ok(node);
    }
    session.enter(id, KIND)?;
    let result = build_polygonal(session, id, frame);
    session.exit(id);
    let node = result?;
    session.memoize(id, KIND, frame, node);
    Ok(node)
}

fn build_polygonal(
    session: &mut LoweringSession<'_>,
    id: EntityId,
    frame: Transform,
) -> GeometryResult<NodeId> {
    let entity = session.entity(id, id)?;
    let view = PolygonalFaceSet::new(id, entity);
    let coordinates = view.coordinates()?;
    let face_refs = view.faces()?;
    let pn = view.pn_index_0based()?;
    let positions = positions(session, id, coordinates, frame)?;

    let mut faces = Vec::with_capacity(face_refs.len());
    for face_ref in &face_refs {
        let face_entity = session.entity(id, *face_ref)?;
        let face = IndexedPolygonalFace::new(*face_ref, face_entity);
        let outer = resolve_loop(session, id, &face.outer_loop_0based()?, &pn, &positions)?;

        // `IfcIndexedPolygonalFaceWithVoids` is a subtype, so the outer loop
        // reads identically; only the inner loops are extra.
        let holes = if face.has_voids() {
            IndexedPolygonalFaceWithVoids::new(*face_ref, face_entity)
                .inner_loops_0based()?
                .iter()
                .map(|inner| resolve_loop(session, id, inner, &pn, &positions))
                .collect::<GeometryResult<Vec<_>>>()?
        } else {
            Vec::new()
        };

        faces.push(PolygonFace { outer, holes });
    }

    session.node_for(
        id,
        GeometryNode::PolygonMesh(PolygonMesh { positions, faces }),
    )
}

/// Map one authored loop through `PnIndex` and bounds-check every entry.
///
/// The face readers already converted from 1-based and applied `PnIndex` where
/// the face itself declares it; the set-level `PnIndex` hop is applied here so
/// both families resolve to direct positions in `Coordinates`.
fn resolve_loop(
    session: &LoweringSession<'_>,
    owner: EntityId,
    authored: &[usize],
    pn: &Option<Vec<usize>>,
    positions: &[Point3],
) -> GeometryResult<Vec<u32>> {
    authored
        .iter()
        .map(|index| {
            let direct = match pn {
                Some(map) => *map.get(*index).ok_or_else(|| {
                    session.degenerate(
                        owner,
                        KIND,
                        format!(
                            "CoordIndex {} is past the end of PnIndex ({} entries)",
                            index + 1,
                            map.len()
                        ),
                    )
                })?,
                None => *index,
            };
            checked_index(session, owner, direct, positions.len(), "CoordIndex")
        })
        .collect()
}

#[cfg(test)]
mod tests;
