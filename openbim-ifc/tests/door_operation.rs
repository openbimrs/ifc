//! Door operation geometry (#148): one test per supported operation type,
//! with hand-computed frames and sectors, plus placement, units, releases
//! and operation-type precedence. Refusals are in
//! `door_operation_refusals.rs`.
//!
//! Conventions under test (IFC4 ADD2 TC1 documentation): the leaf opens to
//! local +y; LEFT is local low x; LEFT/RIGHT panels hinge at the jambs on
//! the x axis; `PanelWidth` is a fraction of `OverallWidth`.

#![cfg(all(feature = "step", feature = "properties", feature = "geometry-select"))]

mod door_support;

use std::f64::consts::{FRAC_PI_2, PI};

use door_support::*;
use ifc::{door_operation, DoorOperationType, LeafMotion, PanelPosition, Side};
use ifc_properties::{ExactSource, SchemaVersion};

const IFC4: SchemaVersion = SchemaVersion::Ifc4;

fn one(operation: &'static str, panel_operation: &str) -> Door {
    Door::new(
        IFC4,
        operation,
        vec![(20, panel(IFC4, 20, panel_operation, "LEFT", None))],
    )
}

fn two(operation: &'static str, left: (&str, &str), right: (&str, &str)) -> Door {
    Door::new(
        IFC4,
        operation,
        vec![
            (20, panel(IFC4, 20, left.0, "LEFT", Some(left.1))),
            (21, panel(IFC4, 21, right.0, "RIGHT", Some(right.1))),
        ],
    )
}

#[test]
fn single_swing_left_hinges_at_the_origin_and_opens_to_positive_y() {
    let mut door = one("SINGLE_SWING_LEFT", "SWINGING");
    door.location = "(2.,3.,0.)";
    let op = door_operation(&door.model(), DOOR).expect("derivable");
    assert_eq!(op.operation, DoorOperationType::SingleSwingLeft);
    assert_eq!(op.operation_source, ExactSource::Type(DOOR_TYPE));
    assert_eq!(op.panel_source, ExactSource::Type(DOOR_TYPE));
    assert_near(op.overall_width, 0.9);
    let [leaf] = &op.leaves[..] else {
        panic!("one leaf: {:?}", op.leaves)
    };
    // `PanelWidth` `$` is the schema's documented 1: the whole opening.
    assert_near(leaf.width, 0.9);
    assert_eq!(leaf.panel_set, EntityId(20));
    assert_eq!(leaf.position, PanelPosition::Left);
    assert_eq!(leaf.motion, LeafMotion::Swing);
    assert_eq!(leaf.hinge_side, Some(Side::Left));
    assert_close(leaf.frame_world.origin, [2.0, 3.0, 0.0]);
    let sector = leaf.swing.expect("a swing");
    assert_close(sector.center, [2.0, 3.0, 0.0]);
    assert_near(sector.radius, 0.9);
    assert_close(sector.start, [1.0, 0.0, 0.0]);
    assert_close(sector.axis, [0.0, 0.0, 1.0]);
    assert_near(sector.sweep, FRAC_PI_2);
    assert_close(sector.end(), [0.0, 1.0, 0.0]);
}

#[test]
fn single_swing_right_hinges_at_the_far_jamb_under_a_rotated_placement() {
    // RefDirection (0,1,0): local x is world +y, local y is world -x.
    let mut door = one("SINGLE_SWING_RIGHT", "SWINGING");
    door.location = "(1.,1.,0.)";
    door.ref_direction = "(0.,1.,0.)";
    door.width = Some("1.");
    let op = door_operation(&door.model(), DOOR).expect("derivable");
    let leaf = &op.leaves[0];
    assert_eq!(leaf.hinge_side, Some(Side::Right));
    let sector = leaf.swing.expect("a swing");
    // Hinge at local (1,0,0) -> world (1,2,0); closed leaf towards local -x
    // (world -y), open towards local +y (world -x).
    assert_close(sector.center, [1.0, 2.0, 0.0]);
    assert_close(sector.start, [0.0, -1.0, 0.0]);
    assert_close(sector.end(), [-1.0, 0.0, 0.0]);
    assert_close(sector.axis, [0.0, 0.0, -1.0]);
    assert_close(leaf.frame_world.origin, [1.0, 1.0, 0.0]);
    assert_close(leaf.frame_world.basis[0], [0.0, 1.0, 0.0]);
}

#[test]
fn double_door_single_swing_hinges_each_leaf_at_its_jamb() {
    let mut door = two(
        "DOUBLE_DOOR_SINGLE_SWING",
        ("SWINGING", "0.6"),
        ("SWINGING", "0.4"),
    );
    door.width = Some("1.5");
    let op = door_operation(&door.model(), DOOR).expect("derivable");
    let [left, right] = &op.leaves[..] else {
        panic!("two leaves: {:?}", op.leaves)
    };
    assert_near(left.width, 0.9);
    assert_near(right.width, 0.6);
    assert_eq!(
        (left.position, right.position),
        (PanelPosition::Left, PanelPosition::Right)
    );
    assert_eq!(
        (left.hinge_side, right.hinge_side),
        (Some(Side::Left), Some(Side::Right))
    );
    let (l, r) = (left.swing.expect("swing"), right.swing.expect("swing"));
    assert_close(l.center, [0.0, 0.0, 0.0]);
    assert_close(r.center, [1.5, 0.0, 0.0]);
    assert_close(right.frame_world.origin, [0.9, 0.0, 0.0]);
    assert_near(r.radius, 0.6);
    assert_close(l.end(), [0.0, 1.0, 0.0]);
    assert_close(r.end(), [0.0, 1.0, 0.0]);
}

#[test]
fn double_swing_left_sweeps_both_sides_from_the_left_jamb() {
    let op = door_operation(&one("DOUBLE_SWING_LEFT", "DOUBLE_ACTING").model(), DOOR)
        .expect("derivable");
    let leaf = &op.leaves[0];
    assert_eq!(leaf.motion, LeafMotion::DoubleSwing);
    assert_eq!(leaf.hinge_side, Some(Side::Left));
    let sector = leaf.swing.expect("a swing");
    assert_close(sector.center, [0.0, 0.0, 0.0]);
    assert_near(sector.sweep, PI);
    assert_close(sector.start, [0.0, -1.0, 0.0]);
    assert_close(sector.direction_at(FRAC_PI_2), [1.0, 0.0, 0.0]);
    assert_close(sector.end(), [0.0, 1.0, 0.0]);
}

#[test]
fn double_swing_right_sweeps_both_sides_from_the_right_jamb() {
    let op = door_operation(&one("DOUBLE_SWING_RIGHT", "DOUBLE_ACTING").model(), DOOR)
        .expect("derivable");
    let leaf = &op.leaves[0];
    assert_eq!(leaf.hinge_side, Some(Side::Right));
    let sector = leaf.swing.expect("a swing");
    assert_close(sector.center, [0.9, 0.0, 0.0]);
    assert_close(sector.start, [0.0, -1.0, 0.0]);
    assert_close(sector.direction_at(FRAC_PI_2), [-1.0, 0.0, 0.0]);
    assert_close(sector.end(), [0.0, 1.0, 0.0]);
}

#[test]
fn double_door_double_swing_has_two_half_discs() {
    let door = two(
        "DOUBLE_DOOR_DOUBLE_SWING",
        ("DOUBLE_ACTING", "0.5"),
        ("DOUBLE_ACTING", "0.5"),
    );
    let op = door_operation(&door.model(), DOOR).expect("derivable");
    let [left, right] = &op.leaves[..] else {
        panic!("two leaves")
    };
    let (l, r) = (left.swing.expect("swing"), right.swing.expect("swing"));
    assert_close(l.center, [0.0, 0.0, 0.0]);
    assert_close(r.center, [0.9, 0.0, 0.0]);
    assert_near(l.radius, 0.45);
    assert_near(r.sweep, PI);
    assert_close(r.direction_at(FRAC_PI_2), [-1.0, 0.0, 0.0]);
}

#[test]
fn sliding_leaves_have_a_direction_and_no_sector() {
    for (operation, direction) in [
        ("SLIDING_TO_LEFT", [-1.0, 0.0, 0.0]),
        ("SLIDING_TO_RIGHT", [1.0, 0.0, 0.0]),
    ] {
        let op = door_operation(&one(operation, "SLIDING").model(), DOOR).expect("derivable");
        let leaf = &op.leaves[0];
        assert_eq!(leaf.motion, LeafMotion::Slide { direction }, "{operation}");
        assert_eq!((leaf.hinge_side, leaf.swing), (None, None));
        assert_near(leaf.width, 0.9);
    }
}

#[test]
fn double_door_sliding_parts_the_leaves() {
    let door = two(
        "DOUBLE_DOOR_SLIDING",
        ("SLIDING", "0.5"),
        ("SLIDING", "0.5"),
    );
    let op = door_operation(&door.model(), DOOR).expect("derivable");
    assert_eq!(op.operation, DoorOperationType::DoubleDoorSliding);
    assert_eq!(
        op.leaves[0].motion,
        LeafMotion::Slide {
            direction: [-1.0, 0.0, 0.0]
        }
    );
    assert_eq!(
        op.leaves[1].motion,
        LeafMotion::Slide {
            direction: [1.0, 0.0, 0.0]
        }
    );
    assert_close(op.leaves[1].frame_world.origin, [0.45, 0.0, 0.0]);
}

#[test]
fn rolling_up_sweeps_no_floor_area() {
    let op = door_operation(&one("ROLLINGUP", "ROLLINGUP").model(), DOOR).expect("derivable");
    assert_eq!(op.operation, DoorOperationType::RollingUp);
    let leaf = &op.leaves[0];
    assert_eq!(leaf.motion, LeafMotion::RollUp);
    assert_eq!((leaf.hinge_side, leaf.swing), (None, None));
}

#[test]
fn swing_fixed_reads_which_position_swings_from_the_panel_sets() {
    // SWING_FIXED_LEFT: the swinging panel is hinged on its own left. Here
    // it is the RIGHT panel, so the hinge sits in the middle of the door.
    let door = two(
        "SWING_FIXED_LEFT",
        ("FIXEDPANEL", "0.3"),
        ("SWINGING", "0.7"),
    );
    let mut door = door;
    door.width = Some("1.");
    let op = door_operation(&door.model(), DOOR).expect("derivable");
    let [fixed, swinging] = &op.leaves[..] else {
        panic!("two leaves")
    };
    assert_eq!((fixed.motion, fixed.swing), (LeafMotion::Fixed, None));
    assert_eq!(swinging.hinge_side, Some(Side::Left));
    let sector = swinging.swing.expect("swing");
    assert_close(sector.center, [0.3, 0.0, 0.0]);
    assert_near(sector.radius, 0.7);

    // SWING_FIXED_RIGHT with the swinging panel at LEFT: hinged at its right
    // edge, again mid-door.
    let mut door = two(
        "SWING_FIXED_RIGHT",
        ("SWINGING", "0.7"),
        ("FIXEDPANEL", "0.3"),
    );
    door.width = Some("1.");
    let op = door_operation(&door.model(), DOOR).expect("derivable");
    let sector = op.leaves[0].swing.expect("swing");
    assert_eq!(op.leaves[0].hinge_side, Some(Side::Right));
    assert_close(sector.center, [0.7, 0.0, 0.0]);
    assert_close(sector.start, [-1.0, 0.0, 0.0]);
    assert_eq!(op.leaves[1].motion, LeafMotion::Fixed);
}

#[test]
fn a_placement_flipped_upside_down_mirrors_the_plan_and_the_hinge_side() {
    // Axis (0,0,-1), RefDirection (1,0,0): local y = z × x is world -y. The
    // door opens towards world -y, and a left-hung leaf seen from above
    // looking that way has its hinge on the right.
    let mut door = one("SINGLE_SWING_LEFT", "SWINGING");
    door.axis = "(0.,0.,-1.)";
    let op = door_operation(&door.model(), DOOR).expect("derivable");
    let leaf = &op.leaves[0];
    assert_eq!(leaf.hinge_side, Some(Side::Right));
    let sector = leaf.swing.expect("swing");
    assert_close(sector.center, [0.0, 0.0, 0.0]);
    assert_close(sector.start, [1.0, 0.0, 0.0]);
    assert_close(sector.end(), [0.0, -1.0, 0.0]);
    assert_close(sector.axis, [0.0, 0.0, -1.0]);
}

#[test]
fn millimetre_files_are_converted_once_to_metres() {
    let panels = vec![(20, panel(IFC4, 20, "SWINGING", "LEFT", Some("1.")))];
    let mut door = Door::new(IFC4, "SINGLE_SWING_RIGHT", panels);
    door.prefix = Some("MILLI");
    door.width = Some("900.");
    door.location = "(1000.,2000.,0.)";
    let op = door_operation(&door.model(), DOOR).expect("derivable");
    assert_near(op.overall_width, 0.9);
    let leaf = &op.leaves[0];
    assert_near(leaf.width, 0.9);
    let sector = leaf.swing.expect("swing");
    assert_close(sector.center, [1.9, 2.0, 0.0]);
    assert_near(sector.radius, 0.9);
}

#[test]
fn ifc2x3_reads_the_operation_from_the_door_style() {
    let v = SchemaVersion::Ifc2x3;
    let door = Door::new(
        v,
        "DOUBLE_DOOR_SINGLE_SWING",
        vec![
            (20, panel(v, 20, "SWINGING", "LEFT", Some("0.5"))),
            (21, panel(v, 21, "SWINGING", "RIGHT", Some("0.5"))),
        ],
    );
    let op = door_operation(&door.model(), DOOR).expect("derivable");
    assert_eq!(op.operation, DoorOperationType::DoubleDoorSingleSwing);
    assert_eq!(op.operation_source, ExactSource::Type(DOOR_TYPE));
    assert_close(op.leaves[1].swing.expect("swing").center, [0.9, 0.0, 0.0]);
}

#[test]
fn ifc4x3_and_the_ifc4_standard_case_are_read_alike() {
    let v = SchemaVersion::Ifc4x3;
    let door = Door::new(
        v,
        "SINGLE_SWING_LEFT",
        vec![(20, panel(v, 20, "SWINGING", "LEFT", None))],
    );
    let op = door_operation(&door.model(), DOOR).expect("derivable");
    assert_eq!(op.operation, DoorOperationType::SingleSwingLeft);

    let mut door = one("SINGLE_SWING_LEFT", "SWINGING");
    door.door_entity = "IfcDoorStandardCase";
    assert!(door_operation(&door.model(), DOOR).is_ok());
}

#[test]
fn operation_type_precedence_between_occurrence_and_type() {
    // Type only.
    let door = one("SINGLE_SWING_LEFT", "SWINGING");
    let op = door_operation(&door.model(), DOOR).expect("derivable");
    assert_eq!(op.operation_source, ExactSource::Type(DOOR_TYPE));

    // Occurrence only: an untyped door with its panels on itself.
    let mut untyped = door.clone();
    untyped.type_operation = None;
    untyped.occurrence_operation = Some("SINGLE_SWING_RIGHT");
    untyped.occurrence_panels = std::mem::take(&mut untyped.type_panels);
    let op = door_operation(&untyped.model(), DOOR).expect("derivable");
    assert_eq!(op.operation, DoorOperationType::SingleSwingRight);
    assert_eq!(op.operation_source, ExactSource::Occurrence);
    assert_eq!(op.panel_source, ExactSource::Occurrence);

    // Both, agreeing: stated on the occurrence.
    let mut both = door.clone();
    both.occurrence_operation = Some("SINGLE_SWING_LEFT");
    let op = door_operation(&both.model(), DOOR).expect("derivable");
    assert_eq!(op.operation_source, ExactSource::Occurrence);
}

#[test]
fn the_occurrence_panel_sets_replace_the_types_never_mix() {
    let mut door = two(
        "DOUBLE_DOOR_SINGLE_SWING",
        ("SWINGING", "0.5"),
        ("SWINGING", "0.5"),
    );
    // The occurrence restates both leaves with a different split.
    door.occurrence_panels = vec![
        (40, panel(IFC4, 40, "SWINGING", "LEFT", Some("0.25"))),
        (41, panel(IFC4, 41, "SWINGING", "RIGHT", Some("0.75"))),
    ];
    let op = door_operation(&door.model(), DOOR).expect("derivable");
    assert_eq!(op.panel_source, ExactSource::Occurrence);
    assert_eq!(
        (op.leaves[0].panel_set, op.leaves[1].panel_set),
        (EntityId(40), EntityId(41))
    );
    assert_near(op.leaves[0].width, 0.225);
}
