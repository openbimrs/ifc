//! Leaf frames and swing sectors from the door's world transform.
//!
//! Everything is computed in the door's local frame and mapped through its
//! world transform (the shared [`RigidFrame`]), so a mirrored placement
//! mirrors the geometry exactly and the reported hinge side is the plan side.

use std::f64::consts::{FRAC_PI_2, PI};

use ifc_geometry::Transform;
use ifc_model::EntityId;

use super::layout::{Motion, Placed};
use super::{DoorOperationError, Leaf, LeafMotion, Sector, Side};
use crate::operation::{FrameError, RigidFrame};

/// A door's world transform, checked rigid.
pub(super) struct DoorFrame {
    door: EntityId,
    frame: RigidFrame,
}

impl DoorFrame {
    /// Accept `world` only if its basis is orthonormal: a leaf width is then
    /// the same length in local and world metres.
    pub(super) fn new(door: EntityId, world: Transform) -> Result<Self, DoorOperationError> {
        let frame = RigidFrame::new(world).map_err(|error| error_for(door, error))?;
        Ok(Self { door, frame })
    }

    /// The leaf `placed`, with widths scaled by `overall_width` metres.
    pub(super) fn leaf(
        &self,
        placed: &Placed,
        overall_width: f64,
    ) -> Result<Leaf, DoorOperationError> {
        let start = placed.start * overall_width;
        let width = placed.width * overall_width;
        let frame_world = self.frame.at([start, 0.0, 0.0]);
        let (motion, hinge, swing) = match placed.motion {
            Motion::Swing(side) => (LeafMotion::Swing, Some(side), true),
            Motion::DoubleSwing(side) => (LeafMotion::DoubleSwing, Some(side), true),
            Motion::Slide(side) => {
                let sign = if side == Side::Left { -1.0 } else { 1.0 };
                let direction = self.frame.direction([sign, 0.0, 0.0]);
                (LeafMotion::Slide { direction }, None, false)
            }
            Motion::RollUp => (LeafMotion::RollUp, None, false),
            Motion::Fixed => (LeafMotion::Fixed, None, false),
        };
        let (hinge_side, swing) = match hinge {
            Some(side) if swing => (
                Some(
                    self.frame
                        .plan_side(side)
                        .map_err(|error| error_for(self.door, error))?,
                ),
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
    /// -y through closed to open towards +y.
    fn sector(&self, start: f64, width: f64, hinge: Side, double: bool) -> Sector {
        let (hinge_x, closed) = match hinge {
            Side::Left => (start, [1.0, 0.0, 0.0]),
            Side::Right => (start + width, [-1.0, 0.0, 0.0]),
        };
        let center = [hinge_x, 0.0, 0.0];
        if double {
            self.frame
                .sector(center, [0.0, -1.0, 0.0], closed, width, PI)
        } else {
            self.frame
                .sector(center, closed, [0.0, 1.0, 0.0], width, FRAC_PI_2)
        }
    }
}

fn error_for(door: EntityId, error: FrameError) -> DoorOperationError {
    match error {
        FrameError::NonRigid => DoorOperationError::NonRigidPlacement { door },
        FrameError::NoPlanHandedness => DoorOperationError::NoPlanHandedness { door },
    }
}
