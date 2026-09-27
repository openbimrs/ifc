//! Texture coordinates for a lowered `IfcTriangulatedFaceSet` (#30).
//!
//! # Per corner, not per vertex
//!
//! `IfcIndexedTriangleTextureMap.TexCoordIndex[i][k]` is the texture vertex
//! for corner `k` of `CoordIndex` triangle `i`. One position can carry a
//! different coordinate in each triangle that uses it: in the IFC4 ADD2 TC1
//! Figure 415 box, every corner has three. The values therefore become a
//! corner-indexed `AttributeChannel`, so positions stay shared and the mesh
//! stays closed; a per-vertex channel could not hold them.
//!
//! # Coverage
//!
//! - A shorter `TexCoordIndex` maps the leading triangles; the rest are
//!   [`AttributeChannel::UNMAPPED`]. Real files texture only some faces.
//! - A longer one, a 0 index, or an index past `TexCoordsList` is an error:
//!   the map disagrees with the face set it names.
//! - An omitted `TexCoordIndex` adds no channel. The schema does not define
//!   what it means, so guessing a pairing would invent data.
//!
//! Coordinates are not transformed: they live in texture space, not model
//! space, so the product frame never applies to them.
//!
//! Slots are read by position. They are the same in IFC4 ADD2 TC1 and IFC4X3
//! ADD2 (`MappedTo` 1, `TexCoords` 2, `TexCoordIndex` 3; `TexCoordsList` 0).

use axiolid_mesh::{AttributeChannel, Blend};
use ifc_model::{EntityId, Value};

use crate::error::{GeometryError, GeometryResult};
use crate::lower::session::LoweringSession;

/// Name of the texture-coordinate channel on the lowered mesh.
///
/// A face set with several triangle texture maps gets `uv`, `uv1`, `uv2`, ...
/// in file order, so no map is silently dropped.
pub const UV_CHANNEL: &str = "uv";

const TYPE: &str = "IFCINDEXEDTRIANGLETEXTUREMAP";
const TEX_COORDS: usize = 2;
const TEX_COORD_INDEX: usize = 3;

/// One corner-indexed `uv` channel per triangle texture map on `face_set`.
pub(super) fn channels(
    session: &mut LoweringSession<'_>,
    face_set: EntityId,
    triangle_count: usize,
) -> GeometryResult<Vec<AttributeChannel>> {
    let maps = session.triangle_texture_maps(face_set).to_vec();
    let mut out = Vec::with_capacity(maps.len());
    for map in maps {
        if let Some(values) = channel(session, face_set, map, triangle_count)? {
            let name = match out.len() {
                0 => UV_CHANNEL.to_string(),
                n => format!("{UV_CHANNEL}{n}"),
            };
            out.push(AttributeChannel::corner_indexed(
                name,
                values.0,
                2,
                Blend::Linear,
                values.1,
            ));
        }
    }
    Ok(out)
}

/// Values and corner indices for one map, or `None` when `TexCoordIndex` is
/// omitted.
fn channel(
    session: &mut LoweringSession<'_>,
    face_set: EntityId,
    map: EntityId,
    triangle_count: usize,
) -> GeometryResult<Option<(Vec<f64>, Vec<u32>)>> {
    let entity = session.entity(face_set, map)?;
    let index = match entity.attributes.get(TEX_COORD_INDEX) {
        None | Some(Value::Null) => return Ok(None),
        Some(value) => triangles(map, value)?,
    };
    if index.len() > triangle_count {
        return Err(invalid(
            map,
            "TexCoordIndex",
            format!(
                "maps {} triangles, but the face set has {triangle_count}",
                index.len()
            ),
        ));
    }
    let list = entity
        .attributes
        .get(TEX_COORDS)
        .and_then(Value::as_ref_id)
        .ok_or_else(|| missing(map, TYPE, "TexCoords"))?;
    let values = texture_vertices(session, map, list)?;
    let pool = values.len() / 2;

    let mut corners = Vec::with_capacity(triangle_count * 3);
    for triangle in &index {
        for &raw in triangle {
            // 1-based in the file; 0 and past-the-end both name nothing.
            let slot = usize::try_from(raw)
                .ok()
                .filter(|&s| (1..=pool).contains(&s));
            let Some(slot) = slot else {
                return Err(invalid(
                    map,
                    "TexCoordIndex",
                    format!("holds {raw}; TexCoordsList has {pool} entries (1-based)"),
                ));
            };
            corners.push((slot - 1) as u32);
        }
    }
    corners.resize(triangle_count * 3, AttributeChannel::UNMAPPED);
    Ok(Some((values, corners)))
}

/// `TexCoordIndex` as rows of three raw (1-based) integers.
fn triangles(map: EntityId, value: &Value) -> GeometryResult<Vec<[i64; 3]>> {
    let rows = value
        .unwrap_typed()
        .as_list()
        .ok_or_else(|| invalid(map, "TexCoordIndex", "is not a list".to_string()))?;
    rows.iter()
        .map(|row| {
            let row = row.unwrap_typed().as_list();
            let ints: Option<Vec<i64>> = row.map(|r| {
                r.iter()
                    .map(|v| v.unwrap_typed().as_i64())
                    .collect::<Option<Vec<_>>>()
            })?;
            match ints.as_deref() {
                Some(&[a, b, c]) => Some([a, b, c]),
                _ => None,
            }
        })
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| {
            invalid(
                map,
                "TexCoordIndex",
                "every entry must be exactly three integers".to_string(),
            )
        })
}

/// `IfcTextureVertexList.TexCoordsList` flattened to `s, t, s, t, ...`.
fn texture_vertices(
    session: &mut LoweringSession<'_>,
    map: EntityId,
    list: EntityId,
) -> GeometryResult<Vec<f64>> {
    let entity = session.entity(map, list)?;
    let rows = entity
        .attributes
        .first()
        .and_then(|v| v.unwrap_typed().as_list())
        .ok_or_else(|| missing(list, "IFCTEXTUREVERTEXLIST", "TexCoordsList"))?;
    let mut out = Vec::with_capacity(rows.len() * 2);
    for row in rows {
        let pair = row.unwrap_typed().as_list().and_then(|p| match p {
            [s, t] => Some([s.unwrap_typed().as_f64()?, t.unwrap_typed().as_f64()?]),
            _ => None,
        });
        let Some([s, t]) = pair else {
            return Err(GeometryError::Degenerate {
                entity: list,
                type_name: "IFCTEXTUREVERTEXLIST".to_string(),
                detail: "TexCoordsList entries must be (s, t) pairs of numbers".to_string(),
            });
        };
        out.extend([s, t]);
    }
    Ok(out)
}

fn invalid(map: EntityId, attribute: &str, detail: String) -> GeometryError {
    GeometryError::Degenerate {
        entity: map,
        type_name: TYPE.to_string(),
        detail: format!("{attribute} {detail}"),
    }
}

fn missing(entity: EntityId, type_name: &str, attribute: &'static str) -> GeometryError {
    GeometryError::MissingAttribute {
        entity,
        type_name: type_name.to_string(),
        attribute,
    }
}

#[cfg(test)]
mod tests;
