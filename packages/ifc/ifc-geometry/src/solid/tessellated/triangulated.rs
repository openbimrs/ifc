//! `IfcTriangulatedFaceSet`: an indexed triangle mesh.
//!
//! The commonest mesh form in IFC4 and the one most exchange pipelines emit.
//! Read the parent module's docs on 1-based indexing before using this.

use super::{face_set_slot, int_grid, int_list, to_zero_based, TessellatedFaceSet};
use crate::error::GeometryResult;
use crate::slots::Slots;
use ifc_model::{Entity, EntityId};

/// `IfcTriangulatedFaceSet` attribute slots.
///
/// EXPRESS (IFC4 ADD2 TC1): `Coordinates` is inherited from
/// `IfcTessellatedFaceSet` at slot 0, then `Normals`, `Closed`, `CoordIndex`,
/// `PnIndex`. Note `Normals` sits **before** `Closed` and `CoordIndex`, so
/// `CoordIndex` is slot 3 and not slot 1 as its prominence suggests.
mod slot {
    /// `Normals : OPTIONAL LIST OF LIST [3:3] OF IfcParameterValue`, slot 1.
    pub const NORMALS: usize = 1;
    /// `Closed : OPTIONAL IfcBoolean`, absolute slot 2.
    pub const CLOSED: usize = 2;
    /// `CoordIndex : LIST OF LIST [3:3] OF IfcPositiveInteger`, slot 3.
    pub const COORD_INDEX: usize = 3;
    /// `PnIndex : OPTIONAL LIST OF IfcPositiveInteger`, absolute slot 4.
    pub const PN_INDEX: usize = 4;
}

/// `IfcTriangulatedFaceSet`: triangles indexed into a shared point list.
///
/// # `Closed` decides what this even is
///
/// `Closed = TRUE` means the set is a boundary representation, a solid.
/// `Closed = FALSE` or absent means it is a surface model with no interior.
/// Only a closed set is a legal boolean operand, and only a closed set has a
/// volume; treating an open one as a solid produces nonsense quantities.
///
/// # Normals
///
/// `Normals` is optional and, when present, is indexed **per face** or **per
/// vertex** depending on `Closed` -- the schema ties the two. Rather than guess,
/// this view exposes the raw triples and leaves the choice to a consumer that
/// has read the rule for its case.
#[derive(Debug, Clone, Copy)]
pub struct TriangulatedFaceSet<'m> {
    slots: Slots<'m>,
}

impl<'m> TriangulatedFaceSet<'m> {
    /// Wrap an entity assumed to be an `IfcTriangulatedFaceSet`.
    pub fn new(id: EntityId, entity: &'m Entity) -> Self {
        Self {
            slots: Slots::new(id, entity),
        }
    }

    /// The entity id.
    pub fn id(&self) -> EntityId {
        self.slots.id()
    }

    /// The inherited `IfcTessellatedFaceSet` attributes.
    pub fn base(&self) -> TessellatedFaceSet<'m> {
        TessellatedFaceSet::from_slots(self.slots)
    }

    /// The `IfcCartesianPointList3D` reference holding the vertices.
    pub fn coordinates(&self) -> GeometryResult<EntityId> {
        self.slots
            .req_ref(face_set_slot::COORDINATES, "Coordinates")
    }

    /// Whether the mesh bounds a solid.
    ///
    /// `None` means the file did not say. That is **not** the same as `false`
    /// for a consumer deciding whether the set may be a boolean operand: an
    /// unstated flag is a file defect worth reporting, not a licence to assume.
    pub fn closed(&self) -> Option<bool> {
        self.slots.opt_bool(slot::CLOSED)
    }

    /// Optional normal triples, exactly as written.
    pub fn normals(&self) -> Option<Vec<[f64; 3]>> {
        let value = self.slots.opt(slot::NORMALS)?;
        let rows = value.as_list()?;
        rows.iter()
            .map(|row| {
                let v = row.unwrap_typed().as_list()?;
                if v.len() != 3 {
                    return None;
                }
                Some([
                    v[0].unwrap_typed().as_f64()?,
                    v[1].unwrap_typed().as_f64()?,
                    v[2].unwrap_typed().as_f64()?,
                ])
            })
            .collect()
    }

    /// Triangles as the file wrote them: **1-based** vertex indices.
    ///
    /// Use this to round-trip or to diff against a file. To index a Rust slice,
    /// use [`Self::triangles_0based`] instead.
    pub fn triangles_1based(&self) -> GeometryResult<Vec<[i64; 3]>> {
        let value = self.slots.req(slot::COORD_INDEX, "CoordIndex")?;
        let rows = int_grid(value).ok_or_else(|| {
            self.slots
                .degenerate("CoordIndex is not a list of integer lists")
        })?;
        rows.into_iter()
            .map(|row| {
                <[i64; 3]>::try_from(row.as_slice()).map_err(|_| {
                    self.slots.degenerate(format!(
                        "CoordIndex entry has {} indices, but a triangle needs exactly 3",
                        row.len()
                    ))
                })
            })
            .collect()
    }

    /// Triangles as 0-based indices, ready to index a vertex `Vec`.
    ///
    /// Applies `PnIndex` when it is present, so the result always addresses
    /// `Coordinates` directly. Rejects a 0 or negative index rather than
    /// wrapping it.
    pub fn triangles_0based(&self) -> GeometryResult<Vec<[usize; 3]>> {
        let pn = self.pn_index_0based()?;
        self.triangles_1based()?
            .into_iter()
            .map(|tri| {
                let mut out = [0usize; 3];
                for (slot_i, raw) in tri.iter().enumerate() {
                    let direct = to_zero_based(&self.slots, "CoordIndex", *raw)?;
                    out[slot_i] = match &pn {
                        Some(map) => *map.get(direct).ok_or_else(|| {
                            self.slots.degenerate(format!(
                                "CoordIndex {raw} is past the end of PnIndex ({} entries)",
                                map.len()
                            ))
                        })?,
                        None => direct,
                    };
                }
                Ok(out)
            })
            .collect()
    }

    /// `PnIndex` exactly as written: **1-based** positions in `Coordinates`.
    pub fn pn_index_1based(&self) -> Option<Vec<i64>> {
        int_list(self.slots.opt(slot::PN_INDEX)?)
    }

    /// `PnIndex` converted to 0-based, or `None` when absent.
    ///
    /// When present, a `CoordIndex` value `i` selects `PnIndex[i - 1]`, and
    /// *that* is the position in `Coordinates`. Skipping the hop silently
    /// permutes the mesh's vertices.
    pub fn pn_index_0based(&self) -> GeometryResult<Option<Vec<usize>>> {
        let Some(raw) = self.pn_index_1based() else {
            return Ok(None);
        };
        raw.into_iter()
            .map(|i| to_zero_based(&self.slots, "PnIndex", i))
            .collect::<GeometryResult<Vec<_>>>()
            .map(Some)
    }

    /// How many triangles the set declares.
    pub fn triangle_count(&self) -> GeometryResult<usize> {
        Ok(self.triangles_1based()?.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solid::testkit::{entity, int_grid as grid, ints, list, n, r};
    use ifc_model::Value;

    fn face_set(coord_index: Value, pn: Value) -> Entity {
        entity(
            "IFCTRIANGULATEDFACESET",
            vec![r(50), Value::Null, Value::Bool(true), coord_index, pn],
        )
    }

    /// CoordIndex sits at slot 3, after the optional Normals and Closed.
    #[test]
    fn coord_index_follows_normals_and_closed_in_the_slot_order() {
        let e = face_set(grid(&[&[1, 2, 3]]), Value::Null);
        let view = TriangulatedFaceSet::new(EntityId(1), &e);
        assert_eq!(view.coordinates().unwrap(), EntityId(50));
        assert_eq!(view.closed(), Some(true));
        assert_eq!(view.triangles_1based().unwrap(), vec![[1, 2, 3]]);
    }

    /// THE critical property: file indices are 1-based, slice indices are
    /// 0-based, and the first vertex of the file is element 0 of the slice.
    #[test]
    fn coord_index_is_one_based_and_converts_to_zero_based_slice_indices() {
        let e = face_set(grid(&[&[1, 2, 3], &[1, 3, 4]]), Value::Null);
        let view = TriangulatedFaceSet::new(EntityId(1), &e);

        assert_eq!(
            view.triangles_1based().unwrap(),
            vec![[1, 2, 3], [1, 3, 4]],
            "the raw form must preserve the file's own numbers"
        );
        assert_eq!(
            view.triangles_0based().unwrap(),
            vec![[0, 1, 2], [0, 2, 3]],
            "index 1 in the file is element 0 of a Rust slice"
        );
    }

    /// A mesh whose highest index equals the vertex count is exactly in range
    /// once converted; off by one here reads past the end.
    #[test]
    fn the_highest_one_based_index_maps_to_the_last_slice_element() {
        let vertex_count = 4usize;
        let e = face_set(grid(&[&[2, 3, 4]]), Value::Null);
        let view = TriangulatedFaceSet::new(EntityId(1), &e);
        let tri = view.triangles_0based().unwrap()[0];
        assert_eq!(tri, [1, 2, 3]);
        assert!(
            tri.iter().all(|i| *i < vertex_count),
            "a valid file's converted indices must all be in range"
        );
    }

    /// A 0 index is impossible in valid IFC and must not wrap to usize::MAX.
    #[test]
    fn a_zero_coord_index_is_rejected_rather_than_wrapping() {
        let e = face_set(grid(&[&[0, 1, 2]]), Value::Null);
        let view = TriangulatedFaceSet::new(EntityId(9), &e);
        assert!(view.triangles_1based().is_ok(), "raw form stays readable");
        let err = view.triangles_0based().unwrap_err();
        assert_eq!(err.entity(), Some(EntityId(9)));
    }

    /// With PnIndex present, CoordIndex points into PnIndex, not straight into
    /// Coordinates; skipping the hop permutes the mesh.
    #[test]
    fn pn_index_indirection_is_applied_not_skipped() {
        // PnIndex reverses a 4-point list: file index 1 selects point 4.
        let e = face_set(grid(&[&[1, 2, 3]]), ints(&[4, 3, 2, 1]));
        let view = TriangulatedFaceSet::new(EntityId(1), &e);

        assert_eq!(view.pn_index_1based().unwrap(), vec![4, 3, 2, 1]);
        assert_eq!(view.pn_index_0based().unwrap().unwrap(), vec![3, 2, 1, 0]);
        assert_eq!(
            view.triangles_0based().unwrap(),
            vec![[3, 2, 1]],
            "CoordIndex 1,2,3 selects PnIndex entries 4,3,2 -> slice 3,2,1"
        );
    }

    #[test]
    fn without_pn_index_the_coord_index_addresses_coordinates_directly() {
        let e = face_set(grid(&[&[1, 2, 3]]), Value::Null);
        let view = TriangulatedFaceSet::new(EntityId(1), &e);
        assert_eq!(view.pn_index_1based(), None);
        assert_eq!(view.pn_index_0based().unwrap(), None);
        assert_eq!(view.triangles_0based().unwrap(), vec![[0, 1, 2]]);
    }

    /// Closed decides whether this is a solid at all, so an unstated flag must
    /// stay distinguishable from an explicit false.
    #[test]
    fn an_absent_closed_flag_is_distinguishable_from_an_explicit_false() {
        let stated = entity(
            "IFCTRIANGULATEDFACESET",
            vec![r(50), Value::Null, Value::Bool(false), grid(&[&[1, 2, 3]])],
        );
        assert_eq!(
            TriangulatedFaceSet::new(EntityId(1), &stated).closed(),
            Some(false)
        );

        let unstated = entity(
            "IFCTRIANGULATEDFACESET",
            vec![r(50), Value::Null, Value::Null, grid(&[&[1, 2, 3]])],
        );
        assert_eq!(
            TriangulatedFaceSet::new(EntityId(1), &unstated).closed(),
            None
        );
    }

    /// A triangle with the wrong arity is a corrupt file, not a polygon.
    #[test]
    fn a_face_without_exactly_three_indices_is_degenerate() {
        let e = face_set(grid(&[&[1, 2, 3, 4]]), Value::Null);
        let err = TriangulatedFaceSet::new(EntityId(4), &e)
            .triangles_1based()
            .unwrap_err();
        assert!(err.to_string().contains("exactly 3"));
    }

    #[test]
    fn normals_are_returned_as_written_without_reinterpretation() {
        let e = entity(
            "IFCTRIANGULATEDFACESET",
            vec![
                r(50),
                list(vec![list(vec![n(0.0), n(0.0), n(1.0)])]),
                Value::Bool(true),
                grid(&[&[1, 2, 3]]),
            ],
        );
        let view = TriangulatedFaceSet::new(EntityId(1), &e);
        assert_eq!(view.normals().unwrap(), vec![[0.0, 0.0, 1.0]]);
        assert_eq!(view.triangle_count().unwrap(), 1);
    }
}
