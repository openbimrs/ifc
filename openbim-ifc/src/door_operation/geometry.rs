//! Leaf frames and swing sectors from the door's world transform.
//!
//! Everything is computed in the door's local frame and mapped through its
//! world transform, so a mirrored placement mirrors the geometry exactly.
//! Only the reported hinge *side* needs a world reference, "from above":
//! it is the local side when the placement preserves plan handedness
//! (`(x × y) · Z > 0` for the images of the local axes and world up `Z`)
//! and the other side when it reverses it.

use std::f64::consts::{FRAC_PI_2, PI};

use ifc_geometry::Transform;
use ifc_model::EntityId;

use super::layout::{Motion, Placed};
use super::{DoorOperationError, Leaf, LeafMotion, Sector, Side};

/// Tolerance on the unit length and orthogonality of the placement basis,
/// and on the plan handedness product.
const RIGID_TOLERANCE: f64 = 1e-9;

/// A door's world transform, checked rigid.
pub(super) struct DoorFrame {
    door: EntityId,
    world: Transform,
}

impl DoorFrame {
    /// Accept `world` only if its basis is orthonormal: a leaf width is then
    /// the same length in local and world metres.
    pub(super) fn new(door: EntityId, world: Transform) -> Result<Self, DoorOperationError> {
        let [x, y, z] = world.basis;
        let unit = |v| (dot(v, v) - 1.0).abs() <= RIGID_TOLERANCE;
        let orthogonal = |a, b| dot(a, b).abs() <= RIGID_TOLERANCE;
        let finite = world
            .basis
            .iter()
            .chain([&world.origin])
            .flatten()
            .all(|c| c.is_finite());
        if !(finite
            && unit(x)
            && unit(y)
            && unit(z)
            && orthogonal(x, y)
            && orthogonal(y, z)
            && orthogonal(z, x))
        {
            return Err(DoorOperationError::NonRigidPlacement { door });
        }
        Ok(Self { door, world })
    }

    /// The world side of a hinge on local side `local`.
    fn plan_side(&self, local: Side) -> Result<Side, DoorOperationError> {
        let [x, y, _] = self.world.basis;
        let handedness = cross(x, y)[2];
        if handedness.abs() <= RIGID_TOLERANCE {
            return Err(DoorOperationError::NoPlanHandedness { door: self.door });
        }
        Ok(match (local, handedness > 0.0) {
            (side, true) => side,
            (Side::Left, false) => Side::Right,
            (Side::Right, false) => Side::Left,
        })
    }

    /// World image of a local direction.
    fn direction(&self, local: [f64; 3]) -> [f64; 3] {
        self.world.apply_direction(local)
    }

    /// The leaf `placed`, with widths scaled by `overall_width` metres.
    pub(super) fn leaf(
        &self,
        placed: &Placed,
        overall_width: f64,
    ) -> Result<Leaf, DoorOperationError> {
        let start = placed.start * overall_width;
        let width = placed.width * overall_width;
        let frame_world = self
            .world
            .compose(&Transform::translation([start, 0.0, 0.0]));
        let (motion, hinge, swing) = match placed.motion {
            Motion::Swing(side) => (LeafMotion::Swing, Some(side), true),
            Motion::DoubleSwing(side) => (LeafMotion::DoubleSwing, Some(side), true),
            Motion::Slide(side) => {
                let sign = if side == Side::Left { -1.0 } else { 1.0 };
                let direction = self.direction([sign, 0.0, 0.0]);
                (LeafMotion::Slide { direction }, None, false)
            }
            Motion::RollUp => (LeafMotion::RollUp, None, false),
            Motion::Fixed => (LeafMotion::Fixed, None, false),
        };
        let (hinge_side, swing) = match hinge {
            Some(side) if swing => (
                Some(self.plan_side(side)?),
                Some(self.sector(start, width, side, motion == LeafMotion::DoubleSwing)),
            ),
            _ => (None, None),
        };
        Ok(Leaf {
            panel_set: placed.set,
            position: placed.position,
            motion,
            frame_world,
            width,
            hinge_side,
            swing,
        })
    }

    /// The sector a leaf spanning local `x ∈ [start, start + width]`,
    /// hinged on local side `hinge`, sweeps.
    ///
    /// A single swing turns from closed (hinge towards the free edge, local
    /// ±x) to open (local +y); a double-acting leaf from open towards local
    /// -y through closed to open towards +y. The axis is `start × via`,
    /// where `via` is the ray a quarter turn on, so it stays correct under
    /// a mirrored placement.
    fn sector(&self, start: f64, width: f64, hinge: Side, double: bool) -> Sector {
        let (hinge_x, closed) = match hinge {
            Side::Left => (start, [1.0, 0.0, 0.0]),
            Side::Right => (start + width, [-1.0, 0.0, 0.0]),
        };
        let closed = self.direction(closed);
        let open = self.direction([0.0, 1.0, 0.0]);
        let (first, via, sweep) = if double {
            (self.direction([0.0, -1.0, 0.0]), closed, PI)
        } else {
            (closed, open, FRAC_PI_2)
        };
        Sector {
            center: self.world.apply([hinge_x, 0.0, 0.0]),
            radius: width,
            start: first,
            axis: cross(first, via),
            sweep,
        }
    }
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// `v` rotated by `angle` about the unit `axis` perpendicular to it.
pub(super) fn rotate(v: [f64; 3], axis: [f64; 3], angle: f64) -> [f64; 3] {
    let normal = cross(axis, v);
    let (sin, cos) = angle.sin_cos();
    [
        cos * v[0] + sin * normal[0],
        cos * v[1] + sin * normal[1],
        cos * v[2] + sin * normal[2],
    ]
}
