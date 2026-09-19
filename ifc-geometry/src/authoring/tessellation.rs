//! Tessellated geometry: meshes carried as shared points plus indices.
//!
//! # The 1-based index invariant
//!
//! `CoordIndex`, `PnIndex` and `InnerCoordIndices` all count from 1
//! ([`crate::solid`]). Rust counts from 0. Every writer here takes
//! **0-based** indices, the only choice that composes with a caller's
//! own vertex buffers, and converts on the way in. A caller who
//! already holds 1-based data would otherwise have to decrement before
//! calling and this module would have to trust that they did.
//!
//! The conversion is where a mesh silently deforms: an off-by-one
//! shifts every triangle onto its neighbour's vertices, which still
//! parses, still renders, and is wrong. So the bound check happens
//! against the point list the indices refer to, not against the index
//! values alone.
//!
//! Nothing here evaluates: a face set is a description of a mesh, and
//! writing it needs no kernel (ADR 0011).

use ifc_model::{Entity, EntityId, Transaction, Value};

use crate::error::GeometryError;

use super::{invalid, require_finite};

/// `IfcCartesianPointList2D`/`3D`: coordinates shared by index.
const LIST_2D: &str = "IFCCARTESIANPOINTLIST2D";
const LIST_3D: &str = "IFCCARTESIANPOINTLIST3D";

/// Slots for the tessellation entities, from IFC4 ADD2.
mod slot {
    /// `IfcCartesianPointList*.CoordList`.
    pub const COORD_LIST: usize = 0;
    /// `IfcCartesianPointList*.TagList`.
    pub const TAG_LIST: usize = 1;
    /// `IfcTessellatedFaceSet.Coordinates`, inherited by both face sets.
    pub const COORDINATES: usize = 0;
    /// `IfcTriangulatedFaceSet.Normals`.
    pub const TRI_NORMALS: usize = 1;
    /// `IfcTriangulatedFaceSet.Closed`.
    pub const TRI_CLOSED: usize = 2;
    /// `IfcTriangulatedFaceSet.CoordIndex`.
    pub const TRI_COORD_INDEX: usize = 3;
    /// `IfcTriangulatedFaceSet.PnIndex`.
    pub const TRI_PN_INDEX: usize = 4;
    /// `IfcPolygonalFaceSet.Closed`.
    pub const POLY_CLOSED: usize = 1;
    /// `IfcPolygonalFaceSet.Faces`.
    pub const POLY_FACES: usize = 2;
    /// `IfcPolygonalFaceSet.PnIndex`.
    pub const POLY_PN_INDEX: usize = 3;
    /// `IfcIndexedPolygonalFace.CoordIndex`.
    pub const FACE_COORD_INDEX: usize = 0;
    /// `IfcIndexedPolygonalFaceWithVoids.InnerCoordIndices`.
    pub const FACE_INNER: usize = 1;
}

/// Convert one 0-based index, refusing anything outside the point list.
///
/// The upper bound is the reason this is not a bare `+ 1`: an index
/// past the end produces a record that parses and denotes a vertex
/// that does not exist.
fn index_1based(
    type_name: &'static str,
    attribute: &'static str,
    value: usize,
    point_count: usize,
) -> Result<Value, GeometryError> {
    if value >= point_count {
        return Err(invalid(
            type_name,
            attribute,
            format!("index {value} is outside a point list of {point_count}"),
        ));
    }
    // IfcPositiveInteger: the schema counts from 1.
    Ok(Value::Integer(value as i64 + 1))
}

/// Stage an `IfcCartesianPointList3D`.
///
/// Tags are optional but, when given, must match the point count: the
/// schema pairs them positionally, so a short list silently retags the
/// wrong vertices.
///
/// # Errors
///
/// Refuses an empty list, a non-finite coordinate, or a tag list whose
/// length disagrees with the points.
pub fn cartesian_point_list_3d(
    tx: &mut Transaction,
    points: &[[f64; 3]],
    tags: Option<&[&str]>,
) -> Result<EntityId, GeometryError> {
    point_list(
        tx,
        LIST_3D,
        points.iter().map(|p| p.as_slice()),
        points.len(),
        tags,
    )
}

/// Stage an `IfcCartesianPointList2D`.
///
/// # Errors
///
/// As [`cartesian_point_list_3d`].
pub fn cartesian_point_list_2d(
    tx: &mut Transaction,
    points: &[[f64; 2]],
    tags: Option<&[&str]>,
) -> Result<EntityId, GeometryError> {
    point_list(
        tx,
        LIST_2D,
        points.iter().map(|p| p.as_slice()),
        points.len(),
        tags,
    )
}

/// Shared body: the two lists differ only in the width of a row.
fn point_list<'p>(
    tx: &mut Transaction,
    type_name: &'static str,
    rows: impl Iterator<Item = &'p [f64]>,
    count: usize,
    tags: Option<&[&str]>,
) -> Result<EntityId, GeometryError> {
    if count == 0 {
        return Err(invalid(
            type_name,
            "CoordList",
            "expected at least one point",
        ));
    }
    let mut coords = Vec::with_capacity(count);
    for row in rows {
        require_finite(type_name, "CoordList", row)?;
        coords.push(Value::List(row.iter().copied().map(Value::Real).collect()));
    }
    // `TagList` is an IFC4X3 addition: IFC4 declares CoordList alone.
    // Writing a trailing `$` would make the record two attributes wide
    // and invalid against IFC4, so the slot only appears when the
    // caller actually supplies tags -- and a caller who does is
    // asking for IFC4X3 by construction.
    let mut attrs = vec![Value::Null; slot::COORD_LIST + 1];
    attrs[slot::COORD_LIST] = Value::List(coords);
    if let Some(tags) = tags {
        if tags.len() != count {
            return Err(invalid(
                type_name,
                "TagList",
                format!("{} tags for {count} points", tags.len()),
            ));
        }
        attrs.resize(slot::TAG_LIST + 1, Value::Null);
        attrs[slot::TAG_LIST] =
            Value::List(tags.iter().map(|t| Value::Text((*t).into())).collect());
    }
    Ok(tx.create(Entity::new(type_name, attrs)))
}

/// The optional parts of an `IfcTriangulatedFaceSet`.
#[derive(Debug, Default, Clone, Copy)]
pub struct TriangulatedExtras<'a> {
    /// `Closed`: whether the triangles bound a solid.
    ///
    /// Left unset when unknown. `false` is a claim that the mesh is
    /// open, which is not the same as declining to say.
    pub closed: Option<bool>,
    /// `Normals`, one per vertex or per face as the schema allows.
    pub normals: Option<&'a [[f64; 3]]>,
    /// `PnIndex`, 0-based here; converted on the way in.
    pub pn_index: Option<&'a [usize]>,
}

/// Stage an `IfcTriangulatedFaceSet` over an existing point list.
///
/// `triangles` are **0-based** indices into `points`, which must be the
/// list `coordinates` refers to: the count is what bounds the indices.
///
/// # Errors
///
/// Refuses an empty triangle list, an index outside the point list, or
/// a non-finite normal.
pub fn triangulated_face_set(
    tx: &mut Transaction,
    coordinates: EntityId,
    point_count: usize,
    triangles: &[[usize; 3]],
    extras: TriangulatedExtras<'_>,
) -> Result<EntityId, GeometryError> {
    const T: &str = "IFCTRIANGULATEDFACESET";
    if triangles.is_empty() {
        return Err(invalid(T, "CoordIndex", "expected at least one triangle"));
    }
    let mut indexed = Vec::with_capacity(triangles.len());
    for triangle in triangles {
        let mut row = Vec::with_capacity(3);
        for value in triangle {
            row.push(index_1based(T, "CoordIndex", *value, point_count)?);
        }
        indexed.push(Value::List(row));
    }

    let mut attrs = vec![Value::Null; 5];
    attrs[slot::COORDINATES] = Value::Ref(coordinates);
    attrs[slot::TRI_COORD_INDEX] = Value::List(indexed);
    if let Some(closed) = extras.closed {
        attrs[slot::TRI_CLOSED] = Value::Bool(closed);
    }
    if let Some(normals) = extras.normals {
        let mut rows = Vec::with_capacity(normals.len());
        for normal in normals {
            require_finite(T, "Normals", normal)?;
            rows.push(Value::List(
                normal.iter().copied().map(Value::Real).collect(),
            ));
        }
        attrs[slot::TRI_NORMALS] = Value::List(rows);
    }
    if let Some(pn) = extras.pn_index {
        let mut rows = Vec::with_capacity(pn.len());
        for value in pn {
            rows.push(index_1based(T, "PnIndex", *value, point_count)?);
        }
        attrs[slot::TRI_PN_INDEX] = Value::List(rows);
    }
    Ok(tx.create(Entity::new(T, attrs)))
}

/// Stage an `IfcIndexedPolygonalFace`.
///
/// `outer` is 0-based. The schema requires at least three vertices: a
/// face through two points bounds no area.
///
/// # Errors
///
/// Refuses fewer than three vertices or an index outside the list.
pub fn indexed_polygonal_face(
    tx: &mut Transaction,
    outer: &[usize],
    point_count: usize,
) -> Result<EntityId, GeometryError> {
    const T: &str = "IFCINDEXEDPOLYGONALFACE";
    let indices = face_loop(T, "CoordIndex", outer, point_count)?;
    let mut attrs = vec![Value::Null];
    attrs[slot::FACE_COORD_INDEX] = indices;
    Ok(tx.create(Entity::new(T, attrs)))
}

/// Stage an `IfcIndexedPolygonalFaceWithVoids`.
///
/// Both the outer loop and every inner loop are 0-based here. The
/// schema does not say the voids lie inside the outer loop -- checking
/// that needs geometry, which this module does not do -- so a caller
/// who passes a disjoint loop gets a face that parses and is wrong in
/// a way only an evaluator can see.
///
/// # Errors
///
/// Refuses an empty void list (that is a plain
/// [`indexed_polygonal_face`]), any loop under three vertices, or an
/// index outside the list.
pub fn indexed_polygonal_face_with_voids(
    tx: &mut Transaction,
    outer: &[usize],
    voids: &[&[usize]],
    point_count: usize,
) -> Result<EntityId, GeometryError> {
    const T: &str = "IFCINDEXEDPOLYGONALFACEWITHVOIDS";
    if voids.is_empty() {
        return Err(invalid(
            T,
            "InnerCoordIndices",
            "expected at least one void; a face without voids is an IfcIndexedPolygonalFace",
        ));
    }
    let outer_indices = face_loop(T, "CoordIndex", outer, point_count)?;
    let mut inner = Vec::with_capacity(voids.len());
    for loop_indices in voids {
        inner.push(face_loop(
            T,
            "InnerCoordIndices",
            loop_indices,
            point_count,
        )?);
    }
    let mut attrs = vec![Value::Null; 2];
    attrs[slot::FACE_COORD_INDEX] = outer_indices;
    attrs[slot::FACE_INNER] = Value::List(inner);
    Ok(tx.create(Entity::new(T, attrs)))
}

/// One closed loop of at least three 0-based indices.
fn face_loop(
    type_name: &'static str,
    attribute: &'static str,
    indices: &[usize],
    point_count: usize,
) -> Result<Value, GeometryError> {
    if indices.len() < 3 {
        return Err(invalid(
            type_name,
            attribute,
            format!("expected at least 3 vertices, got {}", indices.len()),
        ));
    }
    let mut row = Vec::with_capacity(indices.len());
    for value in indices {
        row.push(index_1based(type_name, attribute, *value, point_count)?);
    }
    Ok(Value::List(row))
}

/// Stage an `IfcPolygonalFaceSet` over existing faces.
///
/// The faces must already be staged: they are `IfcIndexedPolygonalFace`
/// records in their own right, and the schema requires the list be
/// UNIQUE, so a face reused twice is refused here rather than left for
/// a validator.
///
/// # Errors
///
/// Refuses an empty or duplicated face list, or a `PnIndex` entry
/// outside the point list.
pub fn polygonal_face_set(
    tx: &mut Transaction,
    coordinates: EntityId,
    point_count: usize,
    faces: &[EntityId],
    closed: Option<bool>,
    pn_index: Option<&[usize]>,
) -> Result<EntityId, GeometryError> {
    const T: &str = "IFCPOLYGONALFACESET";
    if faces.is_empty() {
        return Err(invalid(T, "Faces", "expected at least one face"));
    }
    let mut seen = faces.to_vec();
    seen.sort_unstable();
    seen.dedup();
    if seen.len() != faces.len() {
        return Err(invalid(
            T,
            "Faces",
            "expected a UNIQUE face list, got a repeated face",
        ));
    }

    let mut attrs = vec![Value::Null; 4];
    attrs[slot::COORDINATES] = Value::Ref(coordinates);
    attrs[slot::POLY_FACES] = Value::List(faces.iter().copied().map(Value::Ref).collect());
    if let Some(closed) = closed {
        attrs[slot::POLY_CLOSED] = Value::Bool(closed);
    }
    if let Some(pn) = pn_index {
        let mut rows = Vec::with_capacity(pn.len());
        for value in pn {
            rows.push(index_1based(T, "PnIndex", *value, point_count)?);
        }
        attrs[slot::POLY_PN_INDEX] = Value::List(rows);
    }
    Ok(tx.create(Entity::new(T, attrs)))
}
