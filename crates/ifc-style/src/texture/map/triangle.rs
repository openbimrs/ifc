//! Resolved per-corner texture coordinates for an `IfcIndexedTriangleTextureMap`.
//!
//! # What the schema says, and what it does not
//!
//! `TexCoordIndex` is a list of triangles whose position *i* corresponds to
//! triangle *i* of the face set's `CoordIndex`, and whose three entries
//! correspond, in order, to that triangle's three corners. That is the whole
//! pairing rule: positional, per corner. It is not a per-vertex map. A single
//! position may carry a different texture coordinate in each triangle that
//! uses it (a box corner has three faces meeting at it, each with its own
//! UV), so the result is indexed by *corner*, never by position.
//!
//! Two cases the schema leaves open are reported, not guessed:
//!
//! * `TexCoordIndex` is `OPTIONAL` and neither IFC4 nor IFC4X3 defines what
//!   its absence means. [`IndexedTextureMap::triangle_coordinates`] returns
//!   `Ok(None)` rather than assuming the texture vertices parallel the
//!   positions.
//! * `TexCoordIndex` may be *shorter* than `CoordIndex`. Real exports do this
//!   (the IfcTrafficSignLibrary files texture only a sign's front face). The
//!   trailing triangles are uncovered and read as `None`, which is a fact about
//!   the file, not an error. A *longer* list names triangles that do not exist
//!   and is rejected.
//!
//! [`IndexedTextureMap::triangle_coordinates`]: super::IndexedTextureMap::triangle_coordinates

use ifc_model::{EntityId, Value};

use crate::error::{StyleError, StyleResult};

/// Texture coordinates for each triangle corner of one face set, in
/// `CoordIndex` order.
#[derive(Debug, Clone, PartialEq)]
pub struct TriangleTextureCoordinates {
    triangles: Vec<[[f64; 2]; 3]>,
}

impl TriangleTextureCoordinates {
    /// How many leading triangles of the face set carry coordinates.
    #[must_use]
    pub fn covered(&self) -> usize {
        self.triangles.len()
    }

    /// The `(s, t)` coordinates of triangle `index`'s three corners, in the
    /// corner order `CoordIndex` wrote, or `None` if the map does not reach it.
    #[must_use]
    pub fn triangle(&self, index: usize) -> Option<[[f64; 2]; 3]> {
        self.triangles.get(index).copied()
    }

    /// Every covered triangle's corner coordinates, in `CoordIndex` order.
    #[must_use]
    pub fn triangles(&self) -> &[[[f64; 2]; 3]] {
        &self.triangles
    }
}

/// Resolve `TexCoordIndex` rows against `TexCoordsList`.
///
/// `triangle_count` is the face set's `CoordIndex` length; a caller who does
/// not yet know it passes `usize::MAX` to skip the upper-bound check.
pub(super) fn resolve(
    map: EntityId,
    map_type: &str,
    tex_coord_index: &Value,
    tex_coords_list: &Value,
    triangle_count: usize,
) -> StyleResult<TriangleTextureCoordinates> {
    let invalid = |attribute: &'static str, value: String| StyleError::InvalidValue {
        entity: map_type.to_owned(),
        id: map,
        attribute,
        value,
    };

    let coordinates: Vec<[f64; 2]> = tex_coords_list
        .unwrap_typed()
        .as_list()
        .ok_or_else(|| invalid("TexCoordsList", "not a list".to_owned()))?
        .iter()
        .map(|row| {
            let pair = row.unwrap_typed().as_list()?;
            match pair {
                [s, t] => Some([s.unwrap_typed().as_f64()?, t.unwrap_typed().as_f64()?]),
                _ => None,
            }
        })
        .collect::<Option<_>>()
        .ok_or_else(|| invalid("TexCoordsList", "an entry is not an (s, t) pair".to_owned()))?;

    let rows = tex_coord_index
        .unwrap_typed()
        .as_list()
        .ok_or_else(|| invalid("TexCoordIndex", "not a list".to_owned()))?;
    if rows.len() > triangle_count {
        return Err(invalid(
            "TexCoordIndex",
            format!(
                "{} triangle(s) but the face set declares {triangle_count}",
                rows.len()
            ),
        ));
    }

    let mut triangles = Vec::with_capacity(rows.len());
    for (position, row) in rows.iter().enumerate() {
        let corners = row
            .unwrap_typed()
            .as_list()
            .filter(|corners| corners.len() == 3)
            .ok_or_else(|| {
                invalid(
                    "TexCoordIndex",
                    format!("triangle {} does not have exactly 3 indices", position + 1),
                )
            })?;
        let mut resolved = [[0.0; 2]; 3];
        for (slot, corner) in corners.iter().enumerate() {
            let one_based = corner.unwrap_typed().as_i64().unwrap_or(0);
            let coordinate = usize::try_from(one_based)
                .ok()
                .and_then(|index| index.checked_sub(1))
                .and_then(|index| coordinates.get(index))
                .ok_or_else(|| {
                    invalid(
                        "TexCoordIndex",
                        format!(
                            "triangle {} addresses texture vertex {one_based}, \
                             but TexCoordsList has {}",
                            position + 1,
                            coordinates.len()
                        ),
                    )
                })?;
            resolved[slot] = *coordinate;
        }
        triangles.push(resolved);
    }
    Ok(TriangleTextureCoordinates { triangles })
}

#[cfg(test)]
mod tests;
