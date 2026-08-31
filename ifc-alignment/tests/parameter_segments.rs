use axiolid_model::{CurveRelation, GeometryNode};
use ifc_alignment::{
    lower_vertical_segment, read_cant_segment, read_vertical_segment, AlignmentUnits,
    CantSegmentType, VerticalSegmentType,
};
use ifc_model::{Codec, EntityId};
use ifc_step::StepCodec;

fn fixture() -> ifc_model::Model {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../test/fixtures/synthetic-surfaces/synthetic_conic_offset_bounded.ifc");
    StepCodec.read_path(&path).expect("fixture parses")
}

fn units() -> AlignmentUnits {
    AlignmentUnits {
        length_to_metres: 1.0,
        angle_to_radians: 1.0,
    }
}

#[test]
fn resolves_vertical_parameters_and_units_without_curve_approximation() {
    let model = fixture();
    let segment = read_vertical_segment(
        &model,
        EntityId(63),
        AlignmentUnits {
            length_to_metres: 1.0,
            angle_to_radians: 1.0,
        },
    )
    .expect("vertical segment");

    assert_eq!(segment.start_dist_along, 0.0);
    assert_eq!(segment.horizontal_length, 10.0);
    assert_eq!(segment.start_height, 100.0);
    assert_eq!(segment.start_gradient, 0.02);
    assert_eq!(segment.end_gradient, 0.02);
    assert_eq!(segment.radius_of_curvature, None);
    assert_eq!(
        segment.predefined_type,
        VerticalSegmentType::ConstantGradient
    );
}

#[test]
fn resolves_cant_parameters_and_signed_offsets() {
    let model = fixture();
    let segment = read_cant_segment(
        &model,
        EntityId(64),
        AlignmentUnits {
            length_to_metres: 1.0,
            angle_to_radians: 1.0,
        },
    )
    .expect("cant segment");

    assert_eq!(segment.start_dist_along, 0.0);
    assert_eq!(segment.horizontal_length, 10.0);
    assert_eq!(segment.start_cant_left, 0.15);
    assert_eq!(segment.end_cant_left, Some(0.20));
    assert_eq!(segment.start_cant_right, -0.15);
    assert_eq!(segment.end_cant_right, Some(-0.20));
    assert_eq!(segment.predefined_type, CantSegmentType::LinearTransition);
}

#[test]
fn constant_gradient_lowers_to_an_exact_neutral_segment() {
    let lowered = lower_vertical_segment(&fixture(), EntityId(63), units()).expect("vertical line");
    let Some(GeometryNode::CurveRelation(CurveRelation::Trimmed { basis, end, .. })) =
        lowered.graph.get(lowered.root)
    else {
        panic!("expected trimmed vertical line");
    };
    assert_eq!(end, &vec![axiolid_model::TrimSelector::Parameter(10.0)]);
    assert!(matches!(
        lowered.graph.get(*basis),
        Some(GeometryNode::Curve2(_))
    ));
}
