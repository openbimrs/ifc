//! Frame and sector arithmetic on hand-built transforms, including ones no
//! `IfcAxis2Placement3D` can produce (a reflection), which the model-level
//! tests in `tests/door_operation*.rs` therefore cannot reach.

use std::f64::consts::{FRAC_PI_2, PI};

use ifc_geometry::Transform;
use ifc_model::EntityId;

use super::geometry::DoorFrame;
use super::layout::{Motion, Placed};
use super::{DoorOperationError, LeafMotion, PanelPosition, Side};

const DOOR: EntityId = EntityId(1);

fn close(a: [f64; 3], b: [f64; 3]) -> bool {
    a.iter().zip(b).all(|(p, q)| (p - q).abs() < 1e-12)
}

fn placed(motion: Motion, start: f64, width: f64) -> Placed {
    Placed {
        set: EntityId(20),
        position: PanelPosition::Left,
        motion,
        start,
        width,
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
    let frame = DoorFrame::new(DOOR, mirrored()).expect("rigid");
    let leaf = frame
        .leaf(&placed(Motion::Swing(Side::Left), 0.0, 1.0), 0.9)
        .expect("leaf");
    // Local hinge (0,0,0) maps to world (10,0,0); the closed leaf runs to
    // local +x, world -x, and opens to world +y.
    let sector = leaf.swing.expect("swinging");
    assert!(close(sector.center, [10.0, 0.0, 0.0]));
    assert!((sector.radius - 0.9).abs() < 1e-12);
    assert!(close(sector.start, [-1.0, 0.0, 0.0]));
    assert!(close(sector.end(), [0.0, 1.0, 0.0]));
    assert!(close(sector.direction_at(FRAC_PI_2 / 2.0), {
        let h = std::f64::consts::FRAC_1_SQRT_2;
        [-h, h, 0.0]
    }));
    // Seen from above looking along world +y, +x is on the right. The
    // hinge is at x = 10 and the free edge at x = 9.1, so the SINGLE_SWING_LEFT
    // leaf of a reflected placement hangs on the viewer's right.
    assert_eq!(leaf.hinge_side, Some(Side::Right));
    assert!(close(leaf.frame_world.origin, [10.0, 0.0, 0.0]));
}

#[test]
fn an_unmirrored_placement_keeps_the_operation_side() {
    let frame = DoorFrame::new(DOOR, Transform::identity()).expect("rigid");
    let left = frame
        .leaf(&placed(Motion::Swing(Side::Left), 0.0, 1.0), 0.9)
        .expect("leaf");
    assert_eq!(left.hinge_side, Some(Side::Left));
    let right = frame
        .leaf(&placed(Motion::Swing(Side::Right), 0.0, 1.0), 0.9)
        .expect("leaf");
    assert_eq!(right.hinge_side, Some(Side::Right));
    let sector = right.swing.expect("swinging");
    assert!(close(sector.center, [0.9, 0.0, 0.0]));
    assert!(close(sector.start, [-1.0, 0.0, 0.0]));
    assert!(close(sector.axis, [0.0, 0.0, -1.0]));
}

#[test]
fn a_double_acting_leaf_sweeps_a_half_disc_through_the_closed_position() {
    let frame = DoorFrame::new(DOOR, mirrored()).expect("rigid");
    let leaf = frame
        .leaf(&placed(Motion::DoubleSwing(Side::Right), 0.0, 1.0), 1.0)
        .expect("leaf");
    let sector = leaf.swing.expect("swinging");
    assert_eq!(sector.sweep, PI);
    // Hinge at local x = 1, world x = 9; closed leaf points local -x, world +x.
    assert!(close(sector.center, [9.0, 0.0, 0.0]));
    assert!(close(sector.start, [0.0, -1.0, 0.0]));
    assert!(close(sector.direction_at(FRAC_PI_2), [1.0, 0.0, 0.0]));
    assert!(close(sector.end(), [0.0, 1.0, 0.0]));
    assert_eq!(leaf.hinge_side, Some(Side::Left));
}

#[test]
fn a_reflected_slide_direction_follows_the_reflection() {
    let frame = DoorFrame::new(DOOR, mirrored()).expect("rigid");
    let leaf = frame
        .leaf(&placed(Motion::Slide(Side::Right), 0.5, 0.5), 2.0)
        .expect("leaf");
    assert_eq!(
        leaf.motion,
        LeafMotion::Slide {
            direction: [-1.0, 0.0, 0.0]
        }
    );
    assert_eq!((leaf.hinge_side, leaf.swing), (None, None));
    assert!(close(leaf.frame_world.origin, [9.0, 0.0, 0.0]));
    assert!((leaf.width - 1.0).abs() < 1e-12);
}

#[test]
fn a_scaled_placement_is_refused() {
    let mut scaled = Transform::identity();
    scaled.basis[0] = [2.0, 0.0, 0.0];
    assert_eq!(
        DoorFrame::new(DOOR, scaled).err(),
        Some(DoorOperationError::NonRigidPlacement { door: DOOR })
    );
    let mut sheared = Transform::identity();
    sheared.basis[1] = [0.1, 1.0, 0.0];
    assert!(DoorFrame::new(DOOR, sheared).is_err());
}

#[test]
fn a_door_lying_flat_has_no_hinge_side_seen_from_above() {
    // Local y (opening) points up: a trapdoor-like frame.
    let flat = Transform {
        basis: [[1.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, -1.0, 0.0]],
        origin: [0.0; 3],
    };
    let frame = DoorFrame::new(DOOR, flat).expect("rigid");
    assert_eq!(
        frame
            .leaf(&placed(Motion::Swing(Side::Left), 0.0, 1.0), 1.0)
            .err(),
        Some(DoorOperationError::NoPlanHandedness { door: DOOR })
    );
    // A sliding leaf has no hinge and needs no handedness.
    assert!(frame
        .leaf(&placed(Motion::Slide(Side::Left), 0.0, 1.0), 1.0)
        .is_ok());
}
