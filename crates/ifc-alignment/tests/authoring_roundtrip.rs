//! Author alignment records, then read them back (ADR 0011).
//!
//! The point of authoring inside the reading crate is that both directions
//! index one set of slot constants. These tests fail if they ever disagree.

use ifc_alignment::{
    alignment, alignment_segment, cant_layout, cant_segment, horizontal_layout, horizontal_segment,
    read_vertical_segment, vertical_layout, vertical_segment, AlignmentUnits, CantSegmentDraft,
    HorizontalSegmentDraft, VerticalSegmentDraft,
};
use ifc_model::{Entity, Model, Transaction, Value};

fn metres() -> AlignmentUnits {
    AlignmentUnits {
        length_to_metres: 1.0,
        angle_to_radians: 1.0,
    }
}

#[test]
fn an_authored_vertical_segment_reads_back_through_the_reader() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let draft = VerticalSegmentDraft {
        start_dist_along: 100.0,
        horizontal_length: 250.0,
        start_height: 12.5,
        start_gradient: 0.02,
        end_gradient: 0.035,
        radius_of_curvature: None,
        predefined_type: "CONSTANTGRADIENT",
    };
    let id = vertical_segment(&mut tx, &draft).expect("authored");
    tx.commit(&mut model).expect("commit");

    let read = read_vertical_segment(&model, id, metres()).expect("readable");
    assert_eq!(read.start_dist_along, 100.0);
    assert_eq!(read.horizontal_length, 250.0);
    assert_eq!(read.start_height, 12.5);
    assert_eq!(read.start_gradient, 0.02);
    assert_eq!(read.end_gradient, 0.035);
    assert!(read.radius_of_curvature.is_none());
}

/// The strongest form: an authored layout drives the real segment-chain
/// traversal and lowers to an exact neutral curve. Nothing here is a
/// hand-written entity except `IfcRelNests`, which `ifc-author` owns.
#[test]
fn an_authored_horizontal_layout_lowers_to_an_exact_curve() {
    let mut model = Model::default();
    *model.header_mut() = ifc_model::Header {
        schema: vec!["IFC4X3_ADD2".to_owned()],
        ..ifc_model::Header::default()
    };

    let mut tx = Transaction::new(&model);
    let start = tx.create(Entity::new(
        "IFCCARTESIANPOINT",
        vec![Value::List(vec![Value::Real(0.0), Value::Real(0.0)])],
    ));
    let parameters = horizontal_segment(
        &mut tx,
        &HorizontalSegmentDraft {
            start_point: start,
            start_direction: 0.0,
            start_radius: 0.0,
            end_radius: 0.0,
            segment_length: 100.0,
            gravity_center_line_height: None,
            predefined_type: "LINE",
        },
    )
    .expect("authored segment");
    let segment =
        alignment_segment(&mut tx, "0aBcDeFgHiJkLmNoPqRsTu", parameters).expect("authored wrapper");
    let layout =
        horizontal_layout(&mut tx, "1aBcDeFgHiJkLmNoPqRsTu", Some("H")).expect("authored layout");
    tx.create(Entity::new(
        "IFCRELNESTS",
        vec![
            Value::Text("nest".into()),
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Ref(layout),
            Value::List(vec![Value::Ref(segment)]),
        ],
    ));
    tx.commit(&mut model).expect("commit");

    let lowered = ifc_alignment::lower_horizontal_layout(&model, layout, metres(), None)
        .expect("authored layout lowers");
    assert_eq!(lowered.sources.len(), 1, "one segment lowered");
}

#[test]
fn the_three_layout_kinds_write_their_own_types() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let h = horizontal_layout(&mut tx, "04BcDeFgHiJkLmNoPqRsTu", None).expect("h");
    let v = vertical_layout(&mut tx, "05BcDeFgHiJkLmNoPqRsTu", None).expect("v");
    let a = alignment(
        &mut tx,
        "06BcDeFgHiJkLmNoPqRsTu",
        Some("A1"),
        Some("USERDEFINED"),
    )
    .expect("a");
    tx.commit(&mut model).expect("commit");

    for (id, expected) in [
        (h, "IFCALIGNMENTHORIZONTAL"),
        (v, "IFCALIGNMENTVERTICAL"),
        (a, "IFCALIGNMENT"),
    ] {
        let entity = model.get(id).expect("present");
        assert_eq!(entity.type_name.as_ref(), expected);
    }
}

#[test]
fn an_authored_cant_layout_and_segment_read_back() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let layout = cant_layout(&mut tx, "2aBcDeFgHiJkLmNoPqRsTu", Some("C"), 1.5)
        .expect("authored cant layout");
    let segment = cant_segment(
        &mut tx,
        &CantSegmentDraft {
            start_dist_along: 0.0,
            horizontal_length: 60.0,
            start_cant_left: 0.05,
            end_cant_left: Some(0.09),
            start_cant_right: -0.05,
            end_cant_right: Some(-0.09),
            predefined_type: "LINEARTRANSITION",
        },
    )
    .expect("authored cant segment");
    tx.commit(&mut model).expect("commit");

    let read = ifc_alignment::read_cant_segment(&model, segment, metres()).expect("cant readable");
    assert_eq!(read.start_cant_left, 0.05);
    assert_eq!(read.end_cant_left, Some(0.09));
    assert_eq!(read.start_cant_right, -0.05);
    assert_eq!(read.end_cant_right, Some(-0.09));

    let entity = model.get(layout).expect("layout present");
    assert_eq!(entity.type_name.as_ref(), "IFCALIGNMENTCANT");
}

/// Each of these would produce a file the reader rejects, so authoring
/// refuses first. Writing them and discovering the problem on read is the
/// failure mode this crate exists to prevent.
#[test]
fn values_the_reader_would_reject_are_refused_before_staging() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let point = tx.create(Entity::new("IFCCARTESIANPOINT", vec![Value::Null]));

    let line_with_radius = HorizontalSegmentDraft {
        start_point: point,
        start_direction: 0.0,
        start_radius: 250.0,
        end_radius: 250.0,
        segment_length: 10.0,
        gravity_center_line_height: None,
        predefined_type: "LINE",
    };
    assert!(horizontal_segment(&mut tx, &line_with_radius).is_err());

    let negative_horizontal = HorizontalSegmentDraft {
        segment_length: -1.0,
        start_radius: 0.0,
        end_radius: 0.0,
        ..line_with_radius
    };
    assert!(horizontal_segment(&mut tx, &negative_horizontal).is_err());

    let arc_without_radius = VerticalSegmentDraft {
        start_dist_along: 0.0,
        horizontal_length: 50.0,
        start_height: 0.0,
        start_gradient: 0.01,
        end_gradient: -0.01,
        radius_of_curvature: None,
        predefined_type: "CIRCULARARC",
    };
    assert!(vertical_segment(&mut tx, &arc_without_radius).is_err());

    let negative_length = VerticalSegmentDraft {
        horizontal_length: -1.0,
        radius_of_curvature: None,
        predefined_type: "CONSTANTGRADIENT",
        ..arc_without_radius
    };
    assert!(vertical_segment(&mut tx, &negative_length).is_err());

    let unpaired_ends = CantSegmentDraft {
        start_dist_along: 0.0,
        horizontal_length: 60.0,
        start_cant_left: 0.05,
        end_cant_left: Some(0.09),
        start_cant_right: -0.05,
        end_cant_right: None,
        predefined_type: "LINEARTRANSITION",
    };
    assert!(cant_segment(&mut tx, &unpaired_ends).is_err());

    assert!(cant_layout(&mut tx, "3aBcDeFgHiJkLmNoPqRsTu", None, 0.0).is_err());
    assert!(alignment(&mut tx, "too-short", None, None).is_err());
    // Only the point staged above exists: every refusal returned before
    // calling `create`, so nothing partial was left behind.
    assert_eq!(tx.len(), 1, "a refusal must not stage a partial entity");
}
