//! `IfcGrid` placement: positioning against construction grid axes.
//!
//! Structural and architectural drawings lay out elements against a grid
//! ("column at C-4") rather than by coordinate. `IfcGridPlacement` encodes
//! that directly, so the placement is *derived* from two axis curves rather
//! than stated as a transform.
//!
//! # Why this is not just another placement
//!
//! [`super::local::LocalPlacement`] composes stored transforms. A grid
//! placement has no stored transform at all: the position is the intersection
//! of two curves, which must be computed. That needs curve evaluation, which
//! is the geometry kernel's job, so this module exposes the inputs and stops
//! there.

use crate::error::GeometryResult;
use crate::slots::Slots;
use ifc_model::{Entity, EntityId};

/// `IfcGridAxis` attribute slots.
mod axis_slot {
    /// `AxisTag`: the label, e.g. `C` or `4`.
    pub const AXIS_TAG: usize = 0;
    /// `AxisCurve`: the curve the axis follows.
    pub const AXIS_CURVE: usize = 1;
    /// `SameSense`: whether the curve direction agrees with the axis.
    pub const SAME_SENSE: usize = 2;
}

/// `IfcVirtualGridIntersection` attribute slots.
mod intersection_slot {
    /// `IntersectingAxes`: exactly two `IfcGridAxis`.
    pub const INTERSECTING_AXES: usize = 0;
    /// `OffsetDistances`: two or three offsets from the intersection.
    pub const OFFSET_DISTANCES: usize = 1;
}

/// `IfcGridPlacement` attribute slots.
mod placement_slot {
    /// `PlacementLocation`: the grid intersection.
    pub const PLACEMENT_LOCATION: usize = 0;
    /// `PlacementRefDirection`: optional direction reference.
    pub const PLACEMENT_REF_DIRECTION: usize = 1;
}

/// A borrowed view of an `IfcGridAxis`.
#[derive(Debug, Clone, Copy)]
pub struct GridAxis<'m> {
    slots: Slots<'m>,
}

impl<'m> GridAxis<'m> {
    /// Wrap an entity assumed to be an `IfcGridAxis`.
    pub fn new(id: EntityId, entity: &'m Entity) -> Self {
        Self {
            slots: Slots::new(id, entity),
        }
    }

    /// The entity id.
    pub fn id(&self) -> EntityId {
        self.slots.id()
    }

    /// The axis label, e.g. `C` or `4`.
    pub fn tag(&self) -> Option<&'m str> {
        self.slots.opt(axis_slot::AXIS_TAG)?.as_text()
    }

    /// The curve this axis follows.
    pub fn curve(&self) -> GeometryResult<EntityId> {
        self.slots.req_ref(axis_slot::AXIS_CURVE, "AxisCurve")
    }

    /// Whether the axis direction agrees with its curve's direction.
    ///
    /// Required by the schema, but defaulted to `true` when a file omits it:
    /// a missing sense flag should not make a grid unusable.
    pub fn same_sense(&self) -> bool {
        self.slots.opt_bool(axis_slot::SAME_SENSE).unwrap_or(true)
    }
}

/// A borrowed view of an `IfcVirtualGridIntersection`.
///
/// "Virtual" because the axes need not physically cross: the intersection is a
/// computed point, optionally offset.
#[derive(Debug, Clone, Copy)]
pub struct VirtualGridIntersection<'m> {
    slots: Slots<'m>,
}

impl<'m> VirtualGridIntersection<'m> {
    /// Wrap an entity assumed to be an `IfcVirtualGridIntersection`.
    pub fn new(id: EntityId, entity: &'m Entity) -> Self {
        Self {
            slots: Slots::new(id, entity),
        }
    }

    /// The entity id.
    pub fn id(&self) -> EntityId {
        self.slots.id()
    }

    /// The two intersecting axes.
    ///
    /// The schema constrains this to exactly two; a file with a different
    /// count is returned as-is so the caller can report it rather than having
    /// the truth hidden here.
    pub fn axes(&self) -> GeometryResult<Vec<EntityId>> {
        self.slots
            .req_ref_list(intersection_slot::INTERSECTING_AXES, "IntersectingAxes")
    }

    /// Offsets from the computed intersection.
    ///
    /// Two or three values: along the first axis, along the second, and
    /// optionally along Z. In the file's length unit, so unconverted.
    pub fn offsets(&self) -> GeometryResult<Vec<f64>> {
        self.slots
            .req_f64_list(intersection_slot::OFFSET_DISTANCES, "OffsetDistances")
    }
}

/// A borrowed view of an `IfcGridPlacement`.
#[derive(Debug, Clone, Copy)]
pub struct GridPlacement<'m> {
    slots: Slots<'m>,
}

impl<'m> GridPlacement<'m> {
    /// Wrap an entity assumed to be an `IfcGridPlacement`.
    pub fn new(id: EntityId, entity: &'m Entity) -> Self {
        Self {
            slots: Slots::new(id, entity),
        }
    }

    /// The entity id.
    pub fn id(&self) -> EntityId {
        self.slots.id()
    }

    /// The grid intersection this placement sits at.
    pub fn location(&self) -> GeometryResult<EntityId> {
        self.slots
            .req_ref(placement_slot::PLACEMENT_LOCATION, "PlacementLocation")
    }

    /// Optional reference direction.
    ///
    /// `IfcGridPlacementDirectionSelect`: either an `IfcDirection` or a second
    /// `IfcVirtualGridIntersection` pointing at it. Returned as a raw id
    /// because the caller must dispatch on the target's type.
    pub fn ref_direction(&self) -> Option<EntityId> {
        self.slots.opt_ref(placement_slot::PLACEMENT_REF_DIRECTION)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ifc_model::Value;

    #[test]
    fn reads_axis_tag_and_curve() {
        let e = Entity::new(
            "IFCGRIDAXIS",
            vec![
                Value::Text("C".into()),
                Value::Ref(EntityId(5)),
                Value::Bool(true),
            ],
        );
        let axis = GridAxis::new(EntityId(1), &e);
        assert_eq!(axis.tag(), Some("C"));
        assert_eq!(axis.curve().unwrap(), EntityId(5));
        assert!(axis.same_sense());
    }

    /// A missing sense flag must not make the grid unusable.
    #[test]
    fn absent_same_sense_defaults_to_true() {
        let e = Entity::new(
            "IFCGRIDAXIS",
            vec![
                Value::Text("4".into()),
                Value::Ref(EntityId(5)),
                Value::Null,
            ],
        );
        assert!(GridAxis::new(EntityId(1), &e).same_sense());
    }

    #[test]
    fn reads_intersection_axes_and_offsets() {
        let e = Entity::new(
            "IFCVIRTUALGRIDINTERSECTION",
            vec![
                Value::List(vec![Value::Ref(EntityId(1)), Value::Ref(EntityId(2))]),
                Value::List(vec![Value::Real(0.5), Value::Real(-0.25)]),
            ],
        );
        let x = VirtualGridIntersection::new(EntityId(3), &e);
        assert_eq!(x.axes().unwrap().len(), 2);
        assert_eq!(x.offsets().unwrap(), vec![0.5, -0.25]);
    }

    #[test]
    fn ref_direction_is_optional() {
        let e = Entity::new(
            "IFCGRIDPLACEMENT",
            vec![Value::Ref(EntityId(9)), Value::Null],
        );
        let p = GridPlacement::new(EntityId(1), &e);
        assert_eq!(p.location().unwrap(), EntityId(9));
        assert_eq!(p.ref_direction(), None);
    }
}
