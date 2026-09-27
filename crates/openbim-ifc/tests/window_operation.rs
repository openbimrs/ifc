//! Window operation geometry (#170): hand-computed frames and sectors for
//! each panel operation and partitioning, plus placement, units, releases
//! and partitioning precedence. Refusals are in
//! `window_operation_refusals.rs`.
//!
//! Conventions under test (IFC4 ADD2 TC1 documentation): panels open to
//! local +y; a left-hand panel is hinged at local low x, as seen looking
//! along +y (IfcWindowPanelOperationEnum, IfcWindow Figure 298); panels lie
//! in the XZ plane, split at the lining's mullion and transom offsets
//! (IfcWindowPanelPositionEnum Figure 321, IfcWindowLiningProperties
//! Figure 326).

#![cfg(all(feature = "step", feature = "properties", feature = "geometry-select"))]

mod door_support;
mod window_support;

use std::f64::consts::FRAC_PI_2;

use door_support::{assert_close, assert_near};
use ifc::{
    window_operation, Side, WindowPanelMotion, WindowPanelOperation, WindowPanelPosition,
    WindowPartitioning,
};
use ifc_properties::{ExactSource, SchemaVersion};
use window_support::*;

const IFC4: SchemaVersion = SchemaVersion::Ifc4;

/// A single-panel IFC4 window whose one panel operates as `operation`.
fn single(operation: &str) -> Window {
    Window::new(
        IFC4,
        "SINGLE_PANEL",
        vec![(20, panel(IFC4, 20, operation, "NOTDEFINED"))],
    )
}

#[test]
fn a_single_casement_hinged_left_turns_on_the_origin_jamb_towards_positive_y() {
    let mut window = single("SIDEHUNGLEFTHAND");
    window.location = "(2.,3.,1.)";
    let op = window_operation(&window.model(), WINDOW).expect("derivable");
    assert_eq!(op.partitioning, WindowPartitioning::SinglePanel);
    assert_eq!(op.partitioning_source, ExactSource::Type(WINDOW_TYPE));
    assert_eq!(op.panel_source, ExactSource::Type(WINDOW_TYPE));
    assert_eq!(op.lining_source, None);
    assert_near(op.overall_width, 1.2);
    assert_near(op.overall_height, 1.5);
    let [panel] = &op.panels[..] else {
        panic!("one panel: {:?}", op.panels)
    };
    assert_eq!(panel.panel_set, EntityId(20));
    assert_eq!(panel.position, WindowPanelPosition::NotDefined);
    assert_eq!(panel.operation, WindowPanelOperation::SideHungLeftHand);
    assert_eq!(panel.motion, WindowPanelMotion::Swing);
    assert_eq!(panel.hinge_side, Some(Side::Left));
    assert_near(panel.width, 1.2);
    assert_near(panel.height, 1.5);
    assert_close(panel.frame_world.origin, [2.0, 3.0, 1.0]);
    let sector = panel.swing.expect("a swing");
    assert_close(sector.center, [2.0, 3.0, 1.0]);
    assert_near(sector.radius, 1.2);
    assert_close(sector.start, [1.0, 0.0, 0.0]);
    assert_close(sector.axis, [0.0, 0.0, 1.0]);
    assert_near(sector.sweep, FRAC_PI_2);
    assert_close(sector.end(), [0.0, 1.0, 0.0]);
    assert_eq!(panel.tilt, None);
    assert_eq!((panel.frame_depth, panel.frame_thickness), (None, None));
}

#[test]
fn a_single_casement_hinged_right_turns_on_the_far_jamb() {
    let mut window = single("SIDEHUNGRIGHTHAND");
    window.location = "(2.,3.,1.)";
    let op = window_operation(&window.model(), WINDOW).expect("derivable");
    let panel = &op.panels[0];
    assert_eq!(panel.hinge_side, Some(Side::Right));
    let sector = panel.swing.expect("a swing");
    assert_close(sector.center, [3.2, 3.0, 1.0]);
    assert_close(sector.start, [-1.0, 0.0, 0.0]);
    assert_close(sector.axis, [0.0, 0.0, -1.0]);
    assert_close(sector.end(), [0.0, 1.0, 0.0]);
    // The frame stays at the panel's low-x corner, not at the hinge.
    assert_close(panel.frame_world.origin, [2.0, 3.0, 1.0]);
}

#[test]
fn a_rotated_placement_carries_the_hinge_and_the_opening_direction() {
    // RefDirection (0,1,0): local x is world +y, local y is world -x.
    let mut window = single("SIDEHUNGRIGHTHAND");
    window.location = "(1.,1.,0.)";
    window.ref_direction = "(0.,1.,0.)";
    let op = window_operation(&window.model(), WINDOW).expect("derivable");
    let panel = &op.panels[0];
    assert_eq!(panel.hinge_side, Some(Side::Right));
    let sector = panel.swing.expect("a swing");
    // Hinge at local (1.2,0,0) -> world (1,2.2,0); closed towards local -x
    // (world -y), open towards local +y (world -x).
    assert_close(sector.center, [1.0, 2.2, 0.0]);
    assert_close(sector.start, [0.0, -1.0, 0.0]);
    assert_close(sector.end(), [-1.0, 0.0, 0.0]);
    assert_close(panel.frame_world.basis[0], [0.0, 1.0, 0.0]);
}

#[test]
fn a_window_placed_upside_down_hangs_its_hinge_on_the_other_side() {
    // Axis (0,0,-1), RefDirection (-1,0,0): the window turned half a turn
    // about its opening direction. Local x is world -x, local z world -z,
    // local y = z × x stays world +y. A left-hand panel's hinge at the
    // origin is now right of its free edge, seen looking along +y.
    for (operation, local, seen) in [
        ("SIDEHUNGLEFTHAND", [-1.0, 0.0, 0.0], Side::Right),
        ("SIDEHUNGRIGHTHAND", [1.0, 0.0, 0.0], Side::Left),
    ] {
        let mut window = single(operation);
        window.axis = "(0.,0.,-1.)";
        window.ref_direction = "(-1.,0.,0.)";
        let op = window_operation(&window.model(), WINDOW).expect("derivable");
        let panel = &op.panels[0];
        assert_eq!(panel.hinge_side, Some(seen), "{operation}");
        let sector = panel.swing.expect("a swing");
        assert_close(sector.start, local);
        assert_close(sector.end(), [0.0, 1.0, 0.0]);
        // The panel hangs down from its placement.
        assert_close(panel.frame_world.basis[2], [0.0, 0.0, -1.0]);
    }
    // Axis (0,0,-1) with RefDirection (1,0,0) flips the opening instead:
    // local y is world -y, and the hinge side flips with it.
    let mut window = single("SIDEHUNGLEFTHAND");
    window.axis = "(0.,0.,-1.)";
    let op = window_operation(&window.model(), WINDOW).expect("derivable");
    let panel = &op.panels[0];
    assert_eq!(panel.hinge_side, Some(Side::Right));
    assert_close(panel.swing.expect("a swing").end(), [0.0, -1.0, 0.0]);
}

#[test]
fn a_fixed_panel_sweeps_nothing() {
    let op = window_operation(&single("FIXEDCASEMENT").model(), WINDOW).expect("derivable");
    let panel = &op.panels[0];
    assert_eq!(panel.operation, WindowPanelOperation::FixedCasement);
    assert_eq!(panel.motion, WindowPanelMotion::Fixed);
    assert_eq!(
        (panel.hinge_side, panel.swing, panel.tilt),
        (None, None, None)
    );
    let op = window_operation(&single("REMOVABLECASEMENT").model(), WINDOW).expect("derivable");
    assert_eq!(op.panels[0].motion, WindowPanelMotion::Removable);
    assert_eq!(op.panels[0].swing, None);
}

#[test]
fn sliding_panels_move_in_the_window_plane_without_a_sector() {
    for (operation, along) in [
        ("SLIDINGHORIZONTAL", [1.0, 0.0, 0.0]),
        ("SLIDINGVERTICAL", [0.0, 0.0, 1.0]),
    ] {
        let op = window_operation(&single(operation).model(), WINDOW).expect("derivable");
        let panel = &op.panels[0];
        assert_eq!(
            panel.motion,
            WindowPanelMotion::Slide { along },
            "{operation}"
        );
        assert_eq!(
            (panel.hinge_side, panel.swing, panel.tilt),
            (None, None, None)
        );
    }
}

#[test]
fn a_tilt_and_turn_panel_turns_on_its_side_and_tilts_on_its_bottom() {
    let op = window_operation(&single("TILTANDTURNRIGHTHAND").model(), WINDOW).expect("derivable");
    let panel = &op.panels[0];
    assert_eq!(panel.motion, WindowPanelMotion::TiltAndTurn);
    assert_eq!(panel.hinge_side, Some(Side::Right));
    let turn = panel.swing.expect("turns");
    assert_close(turn.center, [1.2, 0.0, 0.0]);
    assert_near(turn.radius, 1.2);
    assert_close(turn.end(), [0.0, 1.0, 0.0]);
    let tilt = panel.tilt.expect("tilts");
    assert_close(tilt.center, [0.0, 0.0, 0.0]);
    assert_near(tilt.radius, 1.5);
    assert_close(tilt.start, [0.0, 0.0, 1.0]);
    assert_close(tilt.end(), [0.0, 1.0, 0.0]);
    assert_near(tilt.sweep, FRAC_PI_2);
}

#[test]
fn top_and_bottom_hung_panels_tilt_on_their_hinged_edge() {
    let op = window_operation(&single("TOPHUNG").model(), WINDOW).expect("derivable");
    let panel = &op.panels[0];
    assert_eq!(panel.motion, WindowPanelMotion::TopHung);
    assert_eq!((panel.hinge_side, panel.swing), (None, None));
    let tilt = panel.tilt.expect("tilts");
    assert_close(tilt.center, [0.0, 0.0, 1.5]);
    assert_close(tilt.start, [0.0, 0.0, -1.0]);
    assert_close(tilt.axis, [1.0, 0.0, 0.0]);
    assert_close(tilt.end(), [0.0, 1.0, 0.0]);

    let op = window_operation(&single("BOTTOMHUNG").model(), WINDOW).expect("derivable");
    let tilt = op.panels[0].tilt.expect("tilts");
    assert_close(tilt.center, [0.0, 0.0, 0.0]);
    assert_close(tilt.start, [0.0, 0.0, 1.0]);
    assert_close(tilt.end(), [0.0, 1.0, 0.0]);
}

#[test]
fn a_two_panel_window_splits_at_the_mullion_and_hinges_at_the_jambs() {
    // The type lists RIGHT before LEFT: `HasPropertySets` is a SET, so the
    // panels are matched by position and returned LEFT, RIGHT.
    let mut window = Window::new(
        IFC4,
        "DOUBLE_PANEL_VERTICAL",
        vec![
            (21, panel(IFC4, 21, "SIDEHUNGRIGHTHAND", "RIGHT")),
            (20, panel(IFC4, 20, "SIDEHUNGLEFTHAND", "LEFT")),
            (25, lining(IFC4, 25, &[("FirstMullionOffset", "0.4")])),
        ],
    );
    window.width = Some("1.5");
    let op = window_operation(&window.model(), WINDOW).expect("derivable");
    assert_eq!(op.partitioning, WindowPartitioning::DoublePanelVertical);
    assert_eq!(op.lining_source, Some(ExactSource::Type(WINDOW_TYPE)));
    let [left, right] = &op.panels[..] else {
        panic!("two panels: {:?}", op.panels)
    };
    assert_eq!(
        (left.panel_set, right.panel_set),
        (EntityId(20), EntityId(21))
    );
    assert_eq!(
        (left.position, right.position),
        (WindowPanelPosition::Left, WindowPanelPosition::Right)
    );
    assert_near(left.width, 0.6);
    assert_near(right.width, 0.9);
    assert_near(right.height, 1.5);
    assert_close(right.frame_world.origin, [0.6, 0.0, 0.0]);
    assert_eq!(
        (left.hinge_side, right.hinge_side),
        (Some(Side::Left), Some(Side::Right))
    );
    let (l, r) = (left.swing.expect("swing"), right.swing.expect("swing"));
    assert_close(l.center, [0.0, 0.0, 0.0]);
    assert_near(l.radius, 0.6);
    assert_close(r.center, [1.5, 0.0, 0.0]);
    assert_near(r.radius, 0.9);
}

#[test]
fn figure_297_top_bottom_hung_over_a_tilt_and_turn_panel() {
    // IfcWindow Figure 297: DoublePanelHorizontal, TOP BOTTOMHUNG over
    // BOTTOM TILTANDTURNLEFTHAND, split by a transom.
    let window = Window::new(
        IFC4,
        "DOUBLE_PANEL_HORIZONTAL",
        vec![
            (20, panel(IFC4, 20, "BOTTOMHUNG", "TOP")),
            (21, panel(IFC4, 21, "TILTANDTURNLEFTHAND", "BOTTOM")),
            (25, lining(IFC4, 25, &[("FirstTransomOffset", "0.4")])),
        ],
    );
    let op = window_operation(&window.model(), WINDOW).expect("derivable");
    let [top, bottom] = &op.panels[..] else {
        panic!("two panels")
    };
    assert_eq!(top.position, WindowPanelPosition::Top);
    assert_close(top.frame_world.origin, [0.0, 0.0, 0.6]);
    assert_near(top.height, 0.9);
    assert_near(top.width, 1.2);
    assert_eq!(top.swing, None);
    let tilt = top.tilt.expect("tilts");
    assert_close(tilt.center, [0.0, 0.0, 0.6]);
    assert_near(tilt.radius, 0.9);
    assert_eq!(bottom.hinge_side, Some(Side::Left));
    assert_near(bottom.height, 0.6);
    assert_near(bottom.swing.expect("turns").radius, 1.2);
    assert_near(bottom.tilt.expect("tilts").radius, 0.6);
}

#[test]
fn triple_partitionings_follow_figure_321() {
    /// (set, [x origin, width], [z origin, height]) in Figure 321's order.
    type Expected = [(u64, [f64; 2], [f64; 2]); 3];
    /// (partitioning, positions of sets 20..22, lining offsets, expected).
    type Case = (
        &'static str,
        [&'static str; 3],
        &'static [(&'static str, &'static str)],
        Expected,
    );
    let fixed = "FIXEDCASEMENT";
    let cases: [Case; 6] = [
        (
            "TRIPLE_PANEL_VERTICAL",
            ["RIGHT", "LEFT", "MIDDLE"],
            &[
                ("FirstMullionOffset", "0.25"),
                ("SecondMullionOffset", "0.75"),
            ],
            [
                (21, [0.0, 0.3], [0.0, 1.5]),
                (22, [0.3, 0.6], [0.0, 1.5]),
                (20, [0.9, 0.3], [0.0, 1.5]),
            ],
        ),
        (
            "TRIPLE_PANEL_HORIZONTAL",
            ["BOTTOM", "MIDDLE", "TOP"],
            &[
                ("FirstTransomOffset", "0.2"),
                ("SecondTransomOffset", "0.6"),
            ],
            [
                (22, [0.0, 1.2], [0.9, 0.6]),
                (21, [0.0, 1.2], [0.3, 0.6]),
                (20, [0.0, 1.2], [0.0, 0.3]),
            ],
        ),
        (
            "TRIPLE_PANEL_BOTTOM",
            ["BOTTOM", "RIGHT", "LEFT"],
            &[("FirstMullionOffset", "0.5"), ("FirstTransomOffset", "0.2")],
            [
                (22, [0.0, 0.6], [0.3, 1.2]),
                (21, [0.6, 0.6], [0.3, 1.2]),
                (20, [0.0, 1.2], [0.0, 0.3]),
            ],
        ),
        (
            "TRIPLE_PANEL_TOP",
            ["LEFT", "TOP", "RIGHT"],
            &[("FirstMullionOffset", "0.5"), ("FirstTransomOffset", "0.8")],
            [
                (21, [0.0, 1.2], [1.2, 0.3]),
                (20, [0.0, 0.6], [0.0, 1.2]),
                (22, [0.6, 0.6], [0.0, 1.2]),
            ],
        ),
        (
            "TRIPLE_PANEL_LEFT",
            ["BOTTOM", "LEFT", "TOP"],
            &[
                ("FirstMullionOffset", "0.25"),
                ("FirstTransomOffset", "0.4"),
            ],
            [
                (21, [0.0, 0.3], [0.0, 1.5]),
                (22, [0.3, 0.9], [0.6, 0.9]),
                (20, [0.3, 0.9], [0.0, 0.6]),
            ],
        ),
        (
            "TRIPLE_PANEL_RIGHT",
            ["RIGHT", "BOTTOM", "TOP"],
            &[
                ("FirstMullionOffset", "0.75"),
                ("FirstTransomOffset", "0.4"),
            ],
            [
                (22, [0.0, 0.9], [0.6, 0.9]),
                (21, [0.0, 0.9], [0.0, 0.6]),
                (20, [0.9, 0.3], [0.0, 1.5]),
            ],
        ),
    ];
    for (partitioning, positions, offsets, expected) in cases {
        let mut sets: Vec<(u64, String)> = positions
            .iter()
            .enumerate()
            .map(|(i, position)| {
                let id = 20 + i as u64;
                (id, panel(IFC4, id, fixed, position))
            })
            .collect();
        sets.push((25, lining(IFC4, 25, offsets)));
        let op = window_operation(&Window::new(IFC4, partitioning, sets).model(), WINDOW)
            .unwrap_or_else(|e| panic!("{partitioning}: {e:?}"));
        assert_eq!(op.panels.len(), 3, "{partitioning}");
        for (panel, (set, [x, width], [z, height])) in op.panels.iter().zip(expected) {
            assert_eq!(panel.panel_set, EntityId(set), "{partitioning}");
            assert_close(panel.frame_world.origin, [x, 0.0, z]);
            assert_near(panel.width, width);
            assert_near(panel.height, height);
        }
    }
}

#[test]
fn millimetre_files_are_converted_once_to_metres() {
    let mut window = Window::new(
        IFC4,
        "SINGLE_PANEL",
        vec![(
            20,
            framed_panel(IFC4, 20, "SIDEHUNGRIGHTHAND", "LEFT", Some(("68.", "80."))),
        )],
    );
    window.prefix = Some("MILLI");
    window.width = Some("1200.");
    window.height = Some("1500.");
    window.location = "(1000.,2000.,500.)";
    let op = window_operation(&window.model(), WINDOW).expect("derivable");
    assert_near(op.overall_width, 1.2);
    assert_near(op.overall_height, 1.5);
    let panel = &op.panels[0];
    assert_near(panel.width, 1.2);
    assert_near(panel.height, 1.5);
    assert_near(panel.frame_depth.expect("stated"), 0.068);
    assert_near(panel.frame_thickness.expect("stated"), 0.08);
    let sector = panel.swing.expect("swing");
    assert_close(sector.center, [2.2, 2.0, 0.5]);
    assert_near(sector.radius, 1.2);
}

#[test]
fn ifc2x3_reads_the_partitioning_and_sets_from_the_window_style() {
    let v = SchemaVersion::Ifc2x3;
    let window = Window::new(
        v,
        "DOUBLE_PANEL_VERTICAL",
        vec![
            (20, panel(v, 20, "SIDEHUNGLEFTHAND", "LEFT")),
            (21, panel(v, 21, "SIDEHUNGRIGHTHAND", "RIGHT")),
            (25, lining(v, 25, &[("FirstMullionOffset", "0.5")])),
        ],
    );
    let op = window_operation(&window.model(), WINDOW).expect("derivable");
    assert_eq!(op.partitioning, WindowPartitioning::DoublePanelVertical);
    assert_eq!(op.partitioning_source, ExactSource::Type(WINDOW_TYPE));
    assert_eq!(op.lining_source, Some(ExactSource::Type(WINDOW_TYPE)));
    let right = op.panels[1].swing.expect("swing");
    assert_close(right.center, [1.2, 0.0, 0.0]);
    assert_near(right.radius, 0.6);
}

#[test]
fn ifc4x3_and_the_ifc4_standard_case_are_read_alike() {
    let v = SchemaVersion::Ifc4x3;
    let window = Window::new(
        v,
        "SINGLE_PANEL",
        vec![(20, panel(v, 20, "SIDEHUNGLEFTHAND", "LEFT"))],
    );
    let op = window_operation(&window.model(), WINDOW).expect("derivable");
    assert_eq!(op.panels[0].hinge_side, Some(Side::Left));

    let mut window = single("SIDEHUNGLEFTHAND");
    window.window_entity = "IfcWindowStandardCase";
    assert!(window_operation(&window.model(), WINDOW).is_ok());
}

#[test]
fn partitioning_precedence_between_occurrence_and_type() {
    let window = single("SIDEHUNGLEFTHAND");

    // Occurrence only: an untyped window with its panels on itself.
    let mut untyped = window.clone();
    untyped.type_partitioning = None;
    untyped.occurrence_partitioning = Some("SINGLE_PANEL");
    untyped.occurrence_sets = std::mem::take(&mut untyped.type_sets);
    let op = window_operation(&untyped.model(), WINDOW).expect("derivable");
    assert_eq!(op.partitioning_source, ExactSource::Occurrence);
    assert_eq!(op.panel_source, ExactSource::Occurrence);

    // Both, agreeing: stated on the occurrence.
    let mut both = window.clone();
    both.occurrence_partitioning = Some("SINGLE_PANEL");
    let op = window_operation(&both.model(), WINDOW).expect("derivable");
    assert_eq!(op.partitioning_source, ExactSource::Occurrence);
}

#[test]
fn occurrence_panel_sets_replace_the_types_and_the_lining_is_chosen_alike() {
    let mut window = Window::new(
        IFC4,
        "DOUBLE_PANEL_VERTICAL",
        vec![
            (20, panel(IFC4, 20, "SIDEHUNGLEFTHAND", "LEFT")),
            (21, panel(IFC4, 21, "SIDEHUNGRIGHTHAND", "RIGHT")),
            (25, lining(IFC4, 25, &[("FirstMullionOffset", "0.5")])),
        ],
    );
    // The occurrence restates both panels, not the lining.
    window.occurrence_sets = vec![
        (40, panel(IFC4, 40, "FIXEDCASEMENT", "LEFT")),
        (41, panel(IFC4, 41, "TILTANDTURNRIGHTHAND", "RIGHT")),
    ];
    let op = window_operation(&window.model(), WINDOW).expect("derivable");
    assert_eq!(op.panel_source, ExactSource::Occurrence);
    assert_eq!(op.lining_source, Some(ExactSource::Type(WINDOW_TYPE)));
    assert_eq!(
        (op.panels[0].panel_set, op.panels[1].panel_set),
        (EntityId(40), EntityId(41))
    );
    assert_eq!(op.panels[0].motion, WindowPanelMotion::Fixed);
}
