//! Panel frames and sectors on hand-built transforms, including ones no
//! `IfcAxis2Placement3D` can produce (a reflection), which the model-level
//! tests in `tests/window_operation*.rs` therefore cannot reach.

use std::f64::consts::FRAC_PI_2;

use ifc_geometry::Transform;
use ifc_model::EntityId;

use super::geometry::WindowFrame;
use super::layout::Placed;
use super::read::Panel;
use super::WindowPanelPosition;
use super::{Side, WindowOperationError, WindowPanelMotion, WindowPanelOperation};

const WINDOW: EntityId = EntityId(1);

fn close(a: [f64; 3], b: [f64; 3]) -> bool {
    a.iter().zip(b).all(|(p, q)| (p - q).abs() < 1e-12)
}

fn placed(operation: WindowPanelOperation, x: [f64; 2], z: [f64; 2]) -> Placed {
    Placed {
        panel: Panel {
            set: EntityId(20),
            position: WindowPanelPosition::Left,
            operation,
            frame_depth: None,
            frame_thickness: None,
        },
        x,
        z,
    }
}

/// x' = -x: a pure reflection, determinant -1.
fn mirrored() -> Transform {
    Transform {
        basis: [[-1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        origin: [10.0, 0.0, 0.0],
    }
}

#[test]
fn a_reflected_placement_mirrors_the_sector_and_flips_the_hinge_side() {
    let frame = WindowFrame::new(WINDOW, mirrored()).expect("rigid");
    let panel = frame
        .panel(
            &placed(
                WindowPanelOperation::SideHungLeftHand,
                [0.0, 1.0],
                [0.0, 1.0],
            ),
            [0.8, 1.2],
        )
        .expect("panel");
    // Local hinge (0,0,0) maps to world (10,0,0); the closed panel runs to
    // local +x, world -x, and opens to world +y: seen looking along +y the
    // hinge at x = 10 is right of the free edge at x = 9.2.
    let sector = panel.swing.expect("swinging");
    assert!(close(sector.center, [10.0, 0.0, 0.0]));
    assert!(close(sector.start, [-1.0, 0.0, 0.0]));
    assert!(close(sector.end(), [0.0, 1.0, 0.0]));
    assert_eq!(panel.hinge_side, Some(Side::Right));
    assert_eq!(panel.tilt, None);
}

#[test]
fn a_tilt_and_turn_panel_has_both_sectors() {
    let frame = WindowFrame::new(WINDOW, Transform::identity()).expect("rigid");
    let panel = frame
        .panel(
            &placed(
                WindowPanelOperation::TiltAndTurnRightHand,
                [0.5, 1.0],
                [0.25, 1.0],
            ),
            [2.0, 1.6],
        )
        .expect("panel");
    assert_eq!(panel.motion, WindowPanelMotion::TiltAndTurn);
    assert_eq!(panel.hinge_side, Some(Side::Right));
    let turn = panel.swing.expect("turns");
    assert!(close(turn.center, [2.0, 0.0, 0.4]));
    assert!((turn.radius - 1.0).abs() < 1e-12);
    let tilt = panel.tilt.expect("tilts");
    // Bottom hinge from (1, 0, 0.4); the closed panel rises to +z and the
    // tilted one falls towards +y.
    assert!(close(tilt.center, [1.0, 0.0, 0.4]));
    assert!((tilt.radius - 1.2).abs() < 1e-12);
    assert!(close(tilt.start, [0.0, 0.0, 1.0]));
    assert!(close(tilt.axis, [-1.0, 0.0, 0.0]));
    assert!(close(tilt.end(), [0.0, 1.0, 0.0]));
    assert!((tilt.sweep - FRAC_PI_2).abs() < 1e-12);
}

#[test]
fn a_scaled_placement_is_refused() {
    let mut scaled = Transform::identity();
    scaled.basis[2] = [0.0, 0.0, 2.0];
    assert!(matches!(
        WindowFrame::new(WINDOW, scaled),
        Err(WindowOperationError::NonRigidPlacement { window: WINDOW })
    ));
}

#[test]
fn a_skylight_lying_flat_has_no_side_hinge_seen_from_above() {
    // Local y (opening) points up, local z horizontal.
    let flat = Transform {
        basis: [[1.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, -1.0, 0.0]],
        origin: [0.0; 3],
    };
    let frame = WindowFrame::new(WINDOW, flat).expect("rigid");
    let side_hung = placed(
        WindowPanelOperation::SideHungLeftHand,
        [0.0, 1.0],
        [0.0, 1.0],
    );
    assert!(matches!(
        frame.panel(&side_hung, [1.0, 1.0]),
        Err(WindowOperationError::NoPlanHandedness { window: WINDOW })
    ));
    // A top-hung skylight has no side hinge and needs no handedness.
    let top_hung = placed(WindowPanelOperation::TopHung, [0.0, 1.0], [0.0, 1.0]);
    let panel = frame.panel(&top_hung, [1.0, 1.0]).expect("top hung");
    assert!(close(panel.tilt.expect("tilts").end(), [0.0, 0.0, 1.0]));
}
