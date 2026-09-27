//! Panel frames and swept sectors from the window's world transform.
//!
//! Everything is computed in the window's local frame and mapped through
//! its world transform (the shared [`RigidFrame`]), so a mirrored or
//! upside-down placement carries the geometry with it and the reported
//! hinge side is the plan side.

use std::f64::consts::FRAC_PI_2;

use ifc_geometry::Transform;
use ifc_model::EntityId;

use super::layout::Placed;
use super::{
    Sector, Side, WindowOperationError, WindowPanel, WindowPanelMotion, WindowPanelOperation,
};
use crate::operation::{FrameError, RigidFrame};

/// Local +y: the direction every panel opens towards.
const OPEN: [f64; 3] = [0.0, 1.0, 0.0];

/// A window's world transform, checked rigid.
pub(super) struct WindowFrame {
    window: EntityId,
    frame: RigidFrame,
}

/// Which edge a top- or bottom-hung panel turns on.
#[derive(Clone, Copy)]
enum Edge {
    Top,
    Bottom,
}

impl WindowFrame {
    /// Accept `world` only if its basis is orthonormal: a panel's size is
    /// then the same length in local and world metres.
    pub(super) fn new(window: EntityId, world: Transform) -> Result<Self, WindowOperationError> {
        let frame = RigidFrame::new(world).map_err(|error| error_for(window, error))?;
        Ok(Self { window, frame })
    }

    /// The panel `placed`, scaled by `[OverallWidth, OverallHeight]` metres.
    pub(super) fn panel(
        &self,
        placed: &Placed,
        [overall_width, overall_height]: [f64; 2],
    ) -> Result<WindowPanel, WindowOperationError> {
        use WindowPanelOperation as O;
        let [x0, x1] = placed.x.map(|ratio| ratio * overall_width);
        let [z0, z1] = placed.z.map(|ratio| ratio * overall_height);
        let (width, height) = (x1 - x0, z1 - z0);
        let operation = placed.panel.operation;
        let (motion, side, edge) = match operation {
            O::SideHungLeftHand => (WindowPanelMotion::Swing, Some(Side::Left), None),
            O::SideHungRightHand => (WindowPanelMotion::Swing, Some(Side::Right), None),
            O::TiltAndTurnLeftHand => (
                WindowPanelMotion::TiltAndTurn,
                Some(Side::Left),
                Some(Edge::Bottom),
            ),
            O::TiltAndTurnRightHand => (
                WindowPanelMotion::TiltAndTurn,
                Some(Side::Right),
                Some(Edge::Bottom),
            ),
            O::TopHung => (WindowPanelMotion::TopHung, None, Some(Edge::Top)),
            O::BottomHung => (WindowPanelMotion::BottomHung, None, Some(Edge::Bottom)),
            O::SlidingHorizontal => (self.slide([1.0, 0.0, 0.0]), None, None),
            O::SlidingVertical => (self.slide([0.0, 0.0, 1.0]), None, None),
            O::RemovableCasement => (WindowPanelMotion::Removable, None, None),
            O::FixedCasement => (WindowPanelMotion::Fixed, None, None),
        };
        let (hinge_side, swing) = match side {
            Some(side) => {
                let (hinge_x, closed) = match side {
                    Side::Left => (x0, [1.0, 0.0, 0.0]),
                    Side::Right => (x1, [-1.0, 0.0, 0.0]),
                };
                let plan = self
                    .frame
                    .plan_side(side)
                    .map_err(|error| error_for(self.window, error))?;
                let sector = self.quarter([hinge_x, 0.0, z0], closed, width);
                (Some(plan), Some(sector))
            }
            None => (None, None),
        };
        let tilt = edge.map(|edge| match edge {
            Edge::Bottom => self.quarter([x0, 0.0, z0], [0.0, 0.0, 1.0], height),
            Edge::Top => self.quarter([x0, 0.0, z1], [0.0, 0.0, -1.0], height),
        });
        Ok(WindowPanel {
            panel_set: placed.panel.set,
            position: placed.panel.position,
            operation,
            motion,
            frame_world: self.frame.at([x0, 0.0, z0]),
            width,
            height,
            hinge_side,
            swing,
            tilt,
            frame_depth: placed.panel.frame_depth,
            frame_thickness: placed.panel.frame_thickness,
        })
    }

    /// A slide along the world image of local `axis`.
    fn slide(&self, axis: [f64; 3]) -> WindowPanelMotion {
        WindowPanelMotion::Slide {
            along: self.frame.direction(axis),
        }
    }

    /// The quarter turn about the hinge at local `hinge` from the closed
    /// panel (local direction `closed`, hinge to free edge) to open towards
    /// local +y.
    fn quarter(&self, hinge: [f64; 3], closed: [f64; 3], radius: f64) -> Sector {
        self.frame.sector(hinge, closed, OPEN, radius, FRAC_PI_2)
    }
}

fn error_for(window: EntityId, error: FrameError) -> WindowOperationError {
    match error {
        FrameError::NonRigid => WindowOperationError::NonRigidPlacement { window },
        FrameError::NoPlanHandedness => WindowOperationError::NoPlanHandedness { window },
    }
}
