//! `IfcPolygonalFaceSet` and the indexed faces it is built from.
//!
//! Polygonal face sets carry n-gons rather than triangles, and their faces are
//! separate entities so that a face with holes can add an attribute. Read the
//! parent module's docs on 1-based indexing before using this.

use super::{face_set_slot, int_grid, int_list, to_zero_based, TessellatedFaceSet};
use crate::error::GeometryResult;
use crate::slots::Slots;
use ifc_model::{Entity, EntityId};

/// `IfcPolygonalFaceSet` attribute slots.
///
/// EXPRESS (IFC4 ADD2 TC1): `Coordinates` is inherited at slot 0, then
/// `Closed`, `Faces`, `PnIndex`. Note the order differs from
/// `IfcTriangulatedFaceSet`, which has `Normals` at slot 1 -- so `Closed` is
/// slot 1 here and slot 2 there. Copying one layout onto the other is a
/// straightforward way to read a boolean as a list.
mod set_slot {
    /// `Closed : OPTIONAL IfcBoolean`, absolute slot 1.
    pub const CLOSED: usize = 1;
    /// `Faces : LIST [1:?] OF IfcIndexedPolygonalFace`, absolute slot 2.
    pub const FACES: usize = 2;
    /// `PnIndex : OPTIONAL LIST [1:?] OF IfcPositiveInteger`, slot 3.
    pub const PN_INDEX: usize = 3;
}

/// `IfcIndexedPolygonalFace` slots.
///
/// EXPRESS: subtypes `IfcTessellatedItem`, which declares no explicit
/// attributes, so `CoordIndex` is absolute slot 0 and the `WithVoids` subtype's
/// `InnerCoordIndices` is slot 1.
mod face_slot {
    /// `CoordIndex : LIST [3:?] OF IfcPositiveInteger`, the outer loop.
    pub const COORD_INDEX: usize = 0;
    /// `InnerCoordIndices` on `IfcIndexedPolygonalFaceWithVoids`, slot 1.
    pub const INNER_COORD_INDICES: usize = 1;
}

/// `IfcPolygonalFaceSet`: n-gon faces indexed into a shared point list.
///
/// # Winding
///
/// When `Closed` is TRUE the schema requires outer loops to wind
/// counter-clockwise seen from outside the shell, and inner loops (holes) to
/// wind clockwise. A consumer computing face normals from winding must respect
/// that, or every hole's normal points into the material.
#[derive(Debug, Clone, Copy)]
pub struct PolygonalFaceSet<'m> {
    slots: Slots<'m>,
}

impl<'m> PolygonalFaceSet<'m> {
    /// Wrap an entity assumed to be an `IfcPolygonalFaceSet`.
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

    /// Whether the face set bounds a solid. `None` means the file did not say.
    ///
    /// Note this is slot 1 here but slot 2 on `IfcTriangulatedFaceSet`.
    pub fn closed(&self) -> Option<bool> {
        self.slots.opt_bool(set_slot::CLOSED)
    }

    /// The `IfcIndexedPolygonalFace` references making up the set.
    pub fn faces(&self) -> GeometryResult<Vec<EntityId>> {
        self.slots.req_ref_list(set_slot::FACES, "Faces")
    }

    /// `PnIndex` exactly as written: **1-based** positions in `Coordinates`.
    pub fn pn_index_1based(&self) -> Option<Vec<i64>> {
        int_list(self.slots.opt(set_slot::PN_INDEX)?)
    }

    /// `PnIndex` converted to 0-based, or `None` when absent.
    ///
    /// The same indirection as on `IfcTriangulatedFaceSet`: with `PnIndex`
    /// present, a face's `CoordIndex` addresses `PnIndex`, not `Coordinates`.
    pub fn pn_index_0based(&self) -> GeometryResult<Option<Vec<usize>>> {
        let Some(raw) = self.pn_index_1based() else {
            return Ok(None);
        };
        raw.into_iter()
            .map(|i| to_zero_based(&self.slots, "PnIndex", i))
            .collect::<GeometryResult<Vec<_>>>()
            .map(Some)
    }
}

/// `IfcIndexedPolygonalFace`: one planar n-gon, as indices.
///
/// A face is a separate entity rather than an inline list precisely so that
/// [`IndexedPolygonalFaceWithVoids`] can extend it with holes.
#[derive(Debug, Clone, Copy)]
pub struct IndexedPolygonalFace<'m> {
    slots: Slots<'m>,
}

impl<'m> IndexedPolygonalFace<'m> {
    /// Wrap an entity assumed to be an `IfcIndexedPolygonalFace` or subtype.
    pub fn new(id: EntityId, entity: &'m Entity) -> Self {
        Self {
            slots: Slots::new(id, entity),
        }
    }

    /// The entity id.
    pub fn id(&self) -> EntityId {
        self.slots.id()
    }

    /// The IFC type name, naming the concrete subtype.
    pub fn type_name(&self) -> &'m str {
        self.slots.type_name()
    }

    /// Does this face carry inner loops?
    pub fn has_voids(&self) -> bool {
        self.type_name()
            .eq_ignore_ascii_case("IFCINDEXEDPOLYGONALFACEWITHVOIDS")
    }

    /// The outer loop, as the file wrote it: **1-based** indices.
    pub fn outer_loop_1based(&self) -> GeometryResult<Vec<i64>> {
        let value = self.slots.req(face_slot::COORD_INDEX, "CoordIndex")?;
        int_list(value).ok_or_else(|| {
            self.slots
                .degenerate("CoordIndex is not a list of integers")
        })
    }

    /// The outer loop as 0-based indices, ready to index a vertex `Vec`.
    ///
    /// Does **not** apply `PnIndex`: that lives on the containing
    /// `IfcPolygonalFaceSet` and a face has no way to reach it. A caller
    /// holding both must apply the mapping itself.
    pub fn outer_loop_0based(&self) -> GeometryResult<Vec<usize>> {
        self.outer_loop_1based()?
            .into_iter()
            .map(|i| to_zero_based(&self.slots, "CoordIndex", i))
            .collect()
    }
}

/// `IfcIndexedPolygonalFaceWithVoids`: an n-gon with holes.
///
/// # The loops must not be merged
///
/// `InnerCoordIndices` are **holes**. Concatenating them onto the outer loop
/// produces a self-intersecting polygon that most triangulators will happily
/// accept and fill in, so the hole disappears and the face is silently solid.
/// Every accessor here keeps the outer loop and the inner loops separate, and
/// there is deliberately no method that returns them as one list.
#[derive(Debug, Clone, Copy)]
pub struct IndexedPolygonalFaceWithVoids<'m> {
    slots: Slots<'m>,
}

impl<'m> IndexedPolygonalFaceWithVoids<'m> {
    /// Wrap an entity assumed to be an `IfcIndexedPolygonalFaceWithVoids`.
    pub fn new(id: EntityId, entity: &'m Entity) -> Self {
        Self {
            slots: Slots::new(id, entity),
        }
    }

    /// The entity id.
    pub fn id(&self) -> EntityId {
        self.slots.id()
    }

    /// The inherited `IfcIndexedPolygonalFace` attributes, giving the outer
    /// loop.
    pub fn base(&self) -> IndexedPolygonalFace<'m> {
        IndexedPolygonalFace { slots: self.slots }
    }

    /// The inner loops, as the file wrote them: **1-based** indices.
    ///
    /// Each entry is one hole. They are never to be appended to the outer loop.
    pub fn inner_loops_1based(&self) -> GeometryResult<Vec<Vec<i64>>> {
        let value = self
            .slots
            .req(face_slot::INNER_COORD_INDICES, "InnerCoordIndices")?;
        int_grid(value).ok_or_else(|| {
            self.slots
                .degenerate("InnerCoordIndices is not a list of integer lists")
        })
    }

    /// The inner loops as 0-based indices, one `Vec` per hole.
    pub fn inner_loops_0based(&self) -> GeometryResult<Vec<Vec<usize>>> {
        self.inner_loops_1based()?
            .into_iter()
            .map(|loop_| {
                loop_
                    .into_iter()
                    .map(|i| to_zero_based(&self.slots, "InnerCoordIndices", i))
                    .collect()
            })
            .collect()
    }

    /// How many holes the face has.
    pub fn void_count(&self) -> GeometryResult<usize> {
        Ok(self.inner_loops_1based()?.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solid::testkit::{entity, int_grid as grid, ints, r, refs};
    use ifc_model::Value;

    /// Closed is slot 1 here but slot 2 on IfcTriangulatedFaceSet; copying the
    /// other layout reads Faces as the boolean.
    #[test]
    fn closed_precedes_faces_unlike_the_triangulated_layout() {
        let e = entity(
            "IFCPOLYGONALFACESET",
            vec![r(50), Value::Bool(true), refs(&[10, 11]), Value::Null],
        );
        let view = PolygonalFaceSet::new(EntityId(1), &e);
        assert_eq!(view.coordinates().unwrap(), EntityId(50));
        assert_eq!(view.closed(), Some(true));
        assert_eq!(view.faces().unwrap(), vec![EntityId(10), EntityId(11)]);
    }

    #[test]
    fn face_set_pn_index_applies_the_same_one_based_rule() {
        let e = entity(
            "IFCPOLYGONALFACESET",
            vec![r(50), Value::Bool(true), refs(&[10]), ints(&[3, 1, 2])],
        );
        let view = PolygonalFaceSet::new(EntityId(1), &e);
        assert_eq!(view.pn_index_1based().unwrap(), vec![3, 1, 2]);
        assert_eq!(view.pn_index_0based().unwrap().unwrap(), vec![2, 0, 1]);
    }

    #[test]
    fn a_face_set_without_pn_index_reports_none() {
        let e = entity(
            "IFCPOLYGONALFACESET",
            vec![r(50), Value::Bool(true), refs(&[10])],
        );
        let view = PolygonalFaceSet::new(EntityId(1), &e);
        assert_eq!(view.pn_index_0based().unwrap(), None);
    }

    #[test]
    fn a_face_outer_loop_converts_from_one_based_to_zero_based() {
        let e = entity("IFCINDEXEDPOLYGONALFACE", vec![ints(&[1, 2, 3, 4])]);
        let view = IndexedPolygonalFace::new(EntityId(1), &e);
        assert_eq!(view.outer_loop_1based().unwrap(), vec![1, 2, 3, 4]);
        assert_eq!(view.outer_loop_0based().unwrap(), vec![0, 1, 2, 3]);
        assert!(!view.has_voids());
    }

    #[test]
    fn a_zero_index_in_a_face_loop_is_rejected() {
        let e = entity("IFCINDEXEDPOLYGONALFACE", vec![ints(&[0, 1, 2])]);
        let err = IndexedPolygonalFace::new(EntityId(8), &e)
            .outer_loop_0based()
            .unwrap_err();
        assert_eq!(err.entity(), Some(EntityId(8)));
    }

    /// THE property: holes stay separate from the outer loop. Merging them
    /// yields a self-intersecting polygon that triangulates into a solid face.
    #[test]
    fn inner_loops_are_never_merged_into_the_outer_loop() {
        let e = entity(
            "IFCINDEXEDPOLYGONALFACEWITHVOIDS",
            vec![
                ints(&[1, 2, 3, 4]),
                grid(&[&[5, 6, 7, 8], &[9, 10, 11, 12]]),
            ],
        );
        let view = IndexedPolygonalFaceWithVoids::new(EntityId(1), &e);

        let outer = view.base().outer_loop_0based().unwrap();
        let inners = view.inner_loops_0based().unwrap();

        assert_eq!(outer, vec![0, 1, 2, 3]);
        assert_eq!(inners, vec![vec![4, 5, 6, 7], vec![8, 9, 10, 11]]);
        assert_eq!(view.void_count().unwrap(), 2);

        // No index of a hole may appear in the outer boundary.
        for inner in &inners {
            for i in inner {
                assert!(
                    !outer.contains(i),
                    "hole index {i} leaked into the outer loop"
                );
            }
        }
    }

    #[test]
    fn a_face_with_voids_is_distinguishable_from_a_plain_face() {
        let plain = entity("IFCINDEXEDPOLYGONALFACE", vec![ints(&[1, 2, 3])]);
        let voided = entity(
            "IFCINDEXEDPOLYGONALFACEWITHVOIDS",
            vec![ints(&[1, 2, 3, 4]), grid(&[&[5, 6, 7]])],
        );
        assert!(!IndexedPolygonalFace::new(EntityId(1), &plain).has_voids());
        assert!(IndexedPolygonalFace::new(EntityId(1), &voided).has_voids());
    }

    /// A plain face read through the WithVoids view has nothing at slot 1;
    /// that must be an error rather than an empty hole list, since silently
    /// dropping holes is exactly the failure this module guards against.
    #[test]
    fn missing_inner_coord_indices_is_an_error_not_an_empty_hole_list() {
        let e = entity("IFCINDEXEDPOLYGONALFACEWITHVOIDS", vec![ints(&[1, 2, 3])]);
        let view = IndexedPolygonalFaceWithVoids::new(EntityId(2), &e);
        assert!(view.inner_loops_1based().is_err());
        assert!(view.base().outer_loop_1based().is_ok());
    }
}
