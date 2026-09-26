//! Authored records survive serialisation to STEP text and back.
//!
//! The existing authoring round-trips stay in memory: they author onto a
//! `Model` and read the same `Model` back. That never exercises the
//! writer or the parser, so a value that is authored correctly but
//! serialised wrongly, or serialised correctly but reparsed wrongly,
//! passes them. A file is what a consumer actually receives, so this
//! test drives the full path: author, write STEP text, reparse from
//! those bytes, then lower the reparsed model.

use ifc_alignment::authoring::{
    alignment_segment, horizontal_layout, horizontal_segment, vertical_layout, vertical_segment,
    HorizontalSegmentDraft, VerticalSegmentDraft,
};
use ifc_alignment::{read_vertical_segment, AlignmentUnits};
use ifc_model::codec::Codec;
use ifc_model::{Entity, Model, Transaction, Value};
use ifc_step::StepCodec;

fn metres() -> AlignmentUnits {
    AlignmentUnits {
        length_to_metres: 1.0,
        angle_to_radians: 1.0,
    }
}

/// Serialise a model to STEP text, then parse it back from those bytes.
fn through_step_text(model: &Model) -> Model {
    let mut bytes = Vec::new();
    StepCodec
        .write(model, &mut bytes)
        .expect("authored model serialises");
    StepCodec
        .read_bytes(&bytes)
        .expect("serialised text reparses")
}

/// A vertical segment authored, written to text, reparsed, and read.
///
/// Gradients and heights are the values a profile is built from. A writer
/// that drops precision, or a parser that mis-slots an attribute, changes
/// the road surface while leaving the file valid.
#[test]
fn an_authored_vertical_segment_survives_step_text() {
    let mut model = Model::default();
    *model.header_mut() = ifc_model::Header {
        schema: vec!["IFC4X3_ADD2".to_owned()],
        ..ifc_model::Header::default()
    };

    let mut tx = Transaction::new(&model);
    let parameters = vertical_segment(
        &mut tx,
        &VerticalSegmentDraft {
            start_dist_along: 100.0,
            horizontal_length: 250.0,
            start_height: 12.5,
            start_gradient: 0.02,
            end_gradient: 0.035,
            radius_of_curvature: None,
            predefined_type: "CONSTANTGRADIENT",
        },
    )
    .expect("authored segment");
    tx.commit(&mut model).expect("commit");

    let reparsed = through_step_text(&model);
    let read = read_vertical_segment(&reparsed, parameters, metres())
        .expect("reparsed segment reads back");

    assert_eq!(read.start_dist_along, 100.0);
    assert_eq!(read.horizontal_length, 250.0);
    assert_eq!(read.start_height, 12.5);
    assert_eq!(read.start_gradient, 0.02);
    assert_eq!(read.end_gradient, 0.035);
    assert!(read.radius_of_curvature.is_none());
}

/// The strongest form: author a full alignment, serialise it, reparse
/// it, and lower the REPARSED model to an exact centreline.
///
/// This is the path a consumer actually walks -- they receive a file, not
/// a `Model`. Lowering asserts on geometry, so a value corrupted anywhere
/// in write/read shows up as a wrong coordinate rather than as a silently
/// different file.
#[test]
fn an_authored_alignment_lowers_after_a_text_round_trip() {
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
    // A 100 m straight run on a +2% grade from 10 m: every expected value
    // below is an exact decimal, so a precision loss in the writer is
    // visible rather than absorbed by a tolerance.
    let h_params = horizontal_segment(
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
    .expect("authored horizontal");
    let h_segment = alignment_segment(&mut tx, "0aBcDeFgHiJkLmNoPqRsTu", h_params)
        .expect("authored horizontal wrapper");
    let h_layout =
        horizontal_layout(&mut tx, "1aBcDeFgHiJkLmNoPqRsTu", Some("H")).expect("authored h layout");

    let v_params = vertical_segment(
        &mut tx,
        &VerticalSegmentDraft {
            start_dist_along: 0.0,
            horizontal_length: 100.0,
            start_height: 10.0,
            start_gradient: 0.02,
            end_gradient: 0.02,
            radius_of_curvature: None,
            predefined_type: "CONSTANTGRADIENT",
        },
    )
    .expect("authored vertical");
    let v_segment = alignment_segment(&mut tx, "2aBcDeFgHiJkLmNoPqRsTu", v_params)
        .expect("authored vertical wrapper");
    let v_layout =
        vertical_layout(&mut tx, "3aBcDeFgHiJkLmNoPqRsTu", Some("V")).expect("authored v layout");

    let alignment = tx.create(Entity::new(
        "IFCALIGNMENT",
        vec![
            Value::Text("04BcDeFgHiJkLmNoPqRsTu".into()),
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Null,
        ],
    ));
    for (parent, child) in [
        (alignment, h_layout),
        (alignment, v_layout),
        (h_layout, h_segment),
        (v_layout, v_segment),
    ] {
        tx.create(Entity::new(
            "IFCRELNESTS",
            vec![
                Value::Text("nest".into()),
                Value::Null,
                Value::Null,
                Value::Null,
                Value::Ref(parent),
                Value::List(vec![Value::Ref(child)]),
            ],
        ));
    }
    tx.commit(&mut model).expect("commit");

    let reparsed = through_step_text(&model);
    let curve = ifc_alignment::gradient_curve3(&reparsed, alignment, metres())
        .expect("reparsed alignment composes a centreline");

    let axiolid_curve::Curve3::Elevated(elevated) = &curve else {
        panic!("a plan paired with a profile is an elevated curve, got {curve:?}");
    };
    // Authored +2% from 10 m: the profile survived write and reparse.
    assert_eq!(elevated.elevation.height_at(0.0), Some(10.0));
    assert_eq!(elevated.elevation.height_at(100.0), Some(12.0));
    assert_eq!(elevated.elevation.grade_at(50.0), Some(0.02));
}
