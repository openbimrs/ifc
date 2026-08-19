//! Tessellated geometry: explicit meshes addressed by index.
//!
//! # THE off-by-one that corrupts every mesh
//!
//! Every index in this schema is **1-BASED**. `CoordIndex`, `PnIndex` and
//! `InnerCoordIndices` all count from 1 into the coordinate list, because
//! EXPRESS aggregates are 1-based and the values are typed `IfcPositiveInteger`
//! -- the schema literally cannot express a 0.
//!
//! Rust slices are 0-based. Feeding a raw `CoordIndex` to a `Vec` either panics
//! at the last vertex or, far worse, shifts every triangle by one vertex and
//! produces a mesh that still renders: recognisable, watertight-looking, and
//! wrong everywhere. This is the single most damaging mistake available in IFC
//! geometry.
//!
//! The views here therefore expose **both** forms explicitly:
//! `*_indices_1based()` returns the file's own numbers, `*_indices_0based()`
//! returns slice-ready indices and errors on a 0 rather than wrapping.
//!
//! # The PnIndex indirection
//!
//! When `PnIndex` is present the face indices do **not** point into
//! `Coordinates` directly. They point into `PnIndex`, and the value found there
//! is the (1-based) position in `Coordinates`. Skipping the hop silently
//! reorders vertices whenever `PnIndex` is not the identity permutation, which
//! is exactly when an exporter bothered to write it.
//!
//! # Coordinates are not resolved here
//!
//! `IfcCartesianPointList3D` belongs to `IfcGeometryResource`, owned by another
//! module. These views return its `EntityId`.

pub mod polygonal;
pub mod triangulated;

pub use polygonal::{IndexedPolygonalFace, IndexedPolygonalFaceWithVoids, PolygonalFaceSet};
pub use triangulated::TriangulatedFaceSet;

use crate::error::{GeometryError, GeometryResult};
use crate::slots::Slots;
use ifc_model::{Entity, EntityId, Value};

/// Tessellated face set slots shared by both concrete face sets.
///
/// EXPRESS (IFC4 ADD2 TC1): `IfcTessellatedFaceSet` declares `Coordinates`;
/// `IfcTessellatedItem` and `IfcGeometricRepresentationItem` declare no
/// explicit attributes, so `Coordinates` is absolute slot 0 in both subtypes.
pub(super) mod face_set_slot {
    /// `Coordinates : IfcCartesianPointList3D`, on `IfcTessellatedFaceSet`.
    pub const COORDINATES: usize = 0;
}

/// `IfcTessellatedItem`: the abstract root of the tessellation family.
///
/// It declares no attributes. The view exists so a caller holding an
/// unclassified tessellated entity can name it in a diagnostic and dispatch on
/// the type, which is the only thing the supertype offers.
#[derive(Debug, Clone, Copy)]
pub struct TessellatedItem<'m> {
    slots: Slots<'m>,
}

impl<'m> TessellatedItem<'m> {
    /// Wrap an entity assumed to be an `IfcTessellatedItem` subtype.
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

    /// Is this one of the two face sets rather than an indexed face?
    ///
    /// `IfcIndexedPolygonalFace` is also an `IfcTessellatedItem`, so "is a
    /// tessellated item" does not imply "has coordinates".
    pub fn is_face_set(&self) -> bool {
        let n = self.type_name();
        n.eq_ignore_ascii_case("IFCTRIANGULATEDFACESET")
            || n.eq_ignore_ascii_case("IFCPOLYGONALFACESET")
    }
}

/// `IfcTessellatedFaceSet`: the abstract face set, giving access to
/// `Coordinates`.
///
/// Usable over either concrete face set, since `Coordinates` is slot 0 in both.
#[derive(Debug, Clone, Copy)]
pub struct TessellatedFaceSet<'m> {
    slots: Slots<'m>,
}

impl<'m> TessellatedFaceSet<'m> {
    /// Wrap an entity assumed to be an `IfcTessellatedFaceSet` subtype.
    pub fn new(id: EntityId, entity: &'m Entity) -> Self {
        Self {
            slots: Slots::new(id, entity),
        }
    }

    /// Build from already-wrapped slots, for a subtype delegating upward.
    pub(super) fn from_slots(slots: Slots<'m>) -> Self {
        Self { slots }
    }

    /// The entity id.
    pub fn id(&self) -> EntityId {
        self.slots.id()
    }

    /// The IFC type name, naming the concrete subtype.
    pub fn type_name(&self) -> &'m str {
        self.slots.type_name()
    }

    /// The `IfcCartesianPointList3D` reference holding the vertices.
    ///
    /// TODO: resolve through the point-list module once it exists; this crate
    /// deliberately does not define a competing point-list view.
    pub fn coordinates(&self) -> GeometryResult<EntityId> {
        self.slots
            .req_ref(face_set_slot::COORDINATES, "Coordinates")
    }
}

/// Convert one 1-based STEP index to a 0-based slice index.
///
/// Errors instead of wrapping on 0, because `0usize - 1` wraps to
/// `usize::MAX`, and an index that huge either panics far from the cause or
/// silently reads whatever a growable buffer happens to hold.
pub(super) fn to_zero_based(
    slots: &Slots<'_>,
    attribute: &'static str,
    index: i64,
) -> GeometryResult<usize> {
    if index < 1 {
        return Err(GeometryError::Degenerate {
            entity: slots.id(),
            type_name: slots.type_name().to_string(),
            detail: format!(
                "{attribute} holds {index}, but IFC indices are 1-based and must be at least 1"
            ),
        });
    }
    Ok((index - 1) as usize)
}

/// Read a flat list of integers from a slot, unwrapping typed values.
///
/// Necessary because `IfcPositiveInteger` may arrive either bare or wrapped by
/// an exporter that spells out the defined type.
pub(super) fn int_list(value: &Value) -> Option<Vec<i64>> {
    let items = value.as_list()?;
    items
        .iter()
        .map(|v| match v.unwrap_typed() {
            Value::Integer(i) => Some(*i),
            Value::Real(r) => Some(*r as i64),
            _ => None,
        })
        .collect()
}

/// Read a list of integer lists from a slot, e.g. `CoordIndex`.
pub(super) fn int_grid(value: &Value) -> Option<Vec<Vec<i64>>> {
    let rows = value.as_list()?;
    rows.iter()
        .map(|row| int_list(row.unwrap_typed()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solid::testkit::{entity, ints, r};

    #[test]
    fn coordinates_is_slot_zero_for_both_concrete_face_sets() {
        for name in ["IFCTRIANGULATEDFACESET", "IFCPOLYGONALFACESET"] {
            let e = entity(name, vec![r(50)]);
            assert_eq!(
                TessellatedFaceSet::new(EntityId(1), &e)
                    .coordinates()
                    .unwrap(),
                EntityId(50),
                "{name}"
            );
        }
    }

    /// An indexed face is a tessellated item but carries no coordinates, so
    /// "tessellated" alone must not imply a face set.
    #[test]
    fn an_indexed_polygonal_face_is_a_tessellated_item_but_not_a_face_set() {
        let face = entity("IFCINDEXEDPOLYGONALFACE", vec![ints(&[1, 2, 3])]);
        assert!(!TessellatedItem::new(EntityId(1), &face).is_face_set());

        let set = entity("IFCPOLYGONALFACESET", vec![r(1)]);
        assert!(TessellatedItem::new(EntityId(1), &set).is_face_set());
    }

    /// A 0 index cannot occur in valid IFC, and `0 - 1` on a usize wraps to
    /// usize::MAX, so it must be rejected rather than converted.
    #[test]
    fn a_zero_index_is_rejected_instead_of_wrapping_to_usize_max() {
        let e = entity("IFCTRIANGULATEDFACESET", vec![]);
        let slots = Slots::new(EntityId(3), &e);

        assert_eq!(to_zero_based(&slots, "CoordIndex", 1).unwrap(), 0);
        assert_eq!(to_zero_based(&slots, "CoordIndex", 7).unwrap(), 6);

        for bad in [0, -1] {
            let err = to_zero_based(&slots, "CoordIndex", bad).unwrap_err();
            assert_eq!(err.entity(), Some(EntityId(3)));
            assert!(err.to_string().contains("1-based"));
        }
    }
}
