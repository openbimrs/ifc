//! `IfcBoundingBox`: an axis-aligned box, and the trap in what it is aligned
//! to.
//!
//! # It is NOT a world-axis-aligned box
//!
//! The box is axis-aligned in the coordinate system of the representation that
//! contains it -- typically the element's own local placement, which is
//! routinely rotated relative to the world. Using the corner and the three
//! extents directly as world AABB min/max gives a wrong box for every element
//! that is not axis-parallel, which in a real building is most of them. The
//! corner must be transformed and the extents rotated, then a world AABB
//! recomputed from the eight corners.
//!
//! # Corner is the minimum, extents are positive
//!
//! `Corner` is the low corner and the box extends along **+X, +Y, +Z** by the
//! three dims. All three are `IfcPositiveLengthMeasure`, so a zero or negative
//! extent is a malformed file, not a degenerate-but-usable box.
//!
//! # Two roles
//!
//! It appears both as a standalone `Box` representation (a cheap proxy for an
//! element's extent) and as `IfcBoxedHalfSpace.Enclosure`, where it bounds an
//! otherwise infinite half space. Same entity, different meaning; see
//! [`crate::solid::halfspace::BoxedHalfSpace`].

use crate::error::GeometryResult;
use crate::slots::Slots;
use ifc_model::{Entity, EntityId};

/// `IfcBoundingBox` attribute slots.
///
/// EXPRESS (IFC4 ADD2 TC1): subtypes `IfcGeometricRepresentationItem`, which
/// declares no explicit attributes, so all four are absolute slots 0-3.
mod slot {
    /// `Corner : IfcCartesianPoint`, the minimum corner.
    pub const CORNER: usize = 0;
    /// `XDim : IfcPositiveLengthMeasure`.
    pub const X_DIM: usize = 1;
    /// `YDim : IfcPositiveLengthMeasure`.
    pub const Y_DIM: usize = 2;
    /// `ZDim : IfcPositiveLengthMeasure`.
    pub const Z_DIM: usize = 3;
}

/// `IfcBoundingBox`: a box given by a corner and three extents.
#[derive(Debug, Clone, Copy)]
pub struct BoundingBox<'m> {
    slots: Slots<'m>,
}

impl<'m> BoundingBox<'m> {
    /// Wrap an entity assumed to be an `IfcBoundingBox`.
    pub fn new(id: EntityId, entity: &'m Entity) -> Self {
        Self {
            slots: Slots::new(id, entity),
        }
    }

    /// The entity id.
    pub fn id(&self) -> EntityId {
        self.slots.id()
    }

    /// The `IfcCartesianPoint` reference at the box's minimum corner.
    ///
    /// TODO: resolve through the point module once it exists; this crate
    /// deliberately does not define a competing point view.
    pub fn corner(&self) -> GeometryResult<EntityId> {
        self.slots.req_ref(slot::CORNER, "Corner")
    }

    /// Extent along the local X axis, in file length units.
    pub fn x_dim(&self) -> GeometryResult<f64> {
        self.slots.req_f64(slot::X_DIM, "XDim")
    }

    /// Extent along the local Y axis, in file length units.
    pub fn y_dim(&self) -> GeometryResult<f64> {
        self.slots.req_f64(slot::Y_DIM, "YDim")
    }

    /// Extent along the local Z axis, in file length units.
    pub fn z_dim(&self) -> GeometryResult<f64> {
        self.slots.req_f64(slot::Z_DIM, "ZDim")
    }

    /// All three extents, in local X, Y, Z order.
    pub fn dimensions(&self) -> GeometryResult<[f64; 3]> {
        Ok([self.x_dim()?, self.y_dim()?, self.z_dim()?])
    }

    /// The extents, rejecting the non-positive values the schema forbids.
    ///
    /// A zero extent is not a flat-but-usable box: `IfcPositiveLengthMeasure`
    /// excludes it, and a consumer that accepts it will divide by it somewhere.
    pub fn checked_dimensions(&self) -> GeometryResult<[f64; 3]> {
        let dims = self.dimensions()?;
        for (axis, value) in ["XDim", "YDim", "ZDim"].iter().zip(dims) {
            // Written as an explicit positive test rather than a negated
            // comparison so that NaN, which compares false against everything,
            // lands in the rejection branch instead of slipping through.
            if !matches!(value.partial_cmp(&0.0), Some(std::cmp::Ordering::Greater)) {
                return Err(self
                    .slots
                    .degenerate(format!("{axis} must be positive, found {value}")));
            }
        }
        Ok(dims)
    }

    /// The local-space maximum corner, given the resolved minimum corner.
    ///
    /// Takes the corner coordinates rather than resolving them, because
    /// `IfcCartesianPoint` belongs to another module. The result is in the
    /// **same local system** as the input; see the module docs before treating
    /// it as a world AABB bound.
    pub fn max_corner_local(&self, corner: [f64; 3]) -> GeometryResult<[f64; 3]> {
        let [x, y, z] = self.dimensions()?;
        Ok([corner[0] + x, corner[1] + y, corner[2] + z])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solid::testkit::{entity, n, r};

    fn bbox(x: f64, y: f64, z: f64) -> Entity {
        entity("IFCBOUNDINGBOX", vec![r(10), n(x), n(y), n(z)])
    }

    #[test]
    fn corner_precedes_the_three_extents_in_slot_order() {
        let e = bbox(1.0, 2.0, 3.0);
        let view = BoundingBox::new(EntityId(1), &e);
        assert_eq!(view.corner().unwrap(), EntityId(10));
        assert_eq!(view.dimensions().unwrap(), [1.0, 2.0, 3.0]);
    }

    /// Corner is the MINIMUM and the box grows along +X/+Y/+Z; treating it as
    /// a centre halves the box and offsets it.
    #[test]
    fn the_corner_is_the_minimum_and_extents_grow_positively() {
        let e = bbox(2.0, 4.0, 6.0);
        let view = BoundingBox::new(EntityId(1), &e);
        assert_eq!(
            view.max_corner_local([10.0, 20.0, 30.0]).unwrap(),
            [12.0, 24.0, 36.0]
        );
        // A negative-coordinate corner still grows positively.
        assert_eq!(
            view.max_corner_local([-1.0, -1.0, -1.0]).unwrap(),
            [1.0, 3.0, 5.0]
        );
    }

    #[test]
    fn a_non_positive_extent_is_rejected_as_degenerate() {
        for e in [
            bbox(0.0, 1.0, 1.0),
            bbox(1.0, -2.0, 1.0),
            bbox(1.0, 1.0, 0.0),
        ] {
            let view = BoundingBox::new(EntityId(6), &e);
            let err = view.checked_dimensions().unwrap_err();
            assert_eq!(err.entity(), Some(EntityId(6)));
            assert!(
                view.dimensions().is_ok(),
                "raw dimensions stay readable for inspection"
            );
        }
        assert_eq!(
            BoundingBox::new(EntityId(6), &bbox(1.0, 2.0, 3.0))
                .checked_dimensions()
                .unwrap(),
            [1.0, 2.0, 3.0]
        );
    }

    #[test]
    fn a_box_missing_an_extent_names_the_attribute() {
        let e = entity("IFCBOUNDINGBOX", vec![r(10), n(1.0), n(2.0)]);
        let err = BoundingBox::new(EntityId(3), &e).z_dim().unwrap_err();
        assert!(err.to_string().contains("ZDim"));
    }
}
