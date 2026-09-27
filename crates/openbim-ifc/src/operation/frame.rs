//! A product's world transform, checked rigid, and the sector arithmetic in
//! it.
//!
//! Everything is computed in the product's local frame and mapped through
//! its world transform, so a mirrored placement mirrors the geometry
//! exactly. Only a reported hinge *side* needs a world reference, "from
//! above": it is the local side when the placement preserves plan
//! handedness (`(x × y) · Z > 0` for the images of the local axes and world
//! up `Z`) and the other side when it reverses it.

use ifc_geometry::Transform;

use super::{Sector, Side};

/// Tolerance on the unit length and orthogonality of the placement basis,
/// and on the plan handedness product.
const RIGID_TOLERANCE: f64 = 1e-9;

/// Why a placement cannot carry operation geometry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FrameError {
    /// The basis is not orthonormal (or not finite).
    NonRigid,
    /// The local x-y plane is vertical in the world, so a hinge has no left
    /// or right seen from above.
    NoPlanHandedness,
}

/// A world transform whose basis is orthonormal: a length along its local
/// axes is the same length in world metres.
pub(crate) struct RigidFrame {
    world: Transform,
}

impl RigidFrame {
    /// Accept `world` only if its basis is orthonormal and finite.
    pub(crate) fn new(world: Transform) -> Result<Self, FrameError> {
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
            return Err(FrameError::NonRigid);
        }
        Ok(Self { world })
    }

    /// The world side of a hinge on local side `local`.
    pub(crate) fn plan_side(&self, local: Side) -> Result<Side, FrameError> {
        let [x, y, _] = self.world.basis;
        let handedness = cross(x, y)[2];
        if handedness.abs() <= RIGID_TOLERANCE {
            return Err(FrameError::NoPlanHandedness);
        }
        Ok(match (local, handedness > 0.0) {
            (side, true) => side,
            (Side::Left, false) => Side::Right,
            (Side::Right, false) => Side::Left,
        })
    }

    /// World image of a local direction.
    pub(crate) fn direction(&self, local: [f64; 3]) -> [f64; 3] {
        self.world.apply_direction(local)
    }

    /// The frame's own axes, translated to local point `origin`.
    pub(crate) fn at(&self, origin: [f64; 3]) -> Transform {
        self.world.compose(&Transform::translation(origin))
    }

    /// The sector centred on local point `center` whose boundary turns from
    /// local direction `first` through `via` (a quarter turn on) for
    /// `sweep` radians. The axis is `first × via` in world terms, so it stays
    /// correct under a mirrored placement.
    pub(crate) fn sector(
        &self,
        center: [f64; 3],
        first: [f64; 3],
        via: [f64; 3],
        radius: f64,
        sweep: f64,
    ) -> Sector {
        let start = self.direction(first);
        Sector {
            center: self.world.apply(center),
            radius,
            start,
            axis: cross(start, self.direction(via)),
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
