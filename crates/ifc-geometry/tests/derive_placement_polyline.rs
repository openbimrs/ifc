//! Deriving a placement frame from a straight-segment basis curve (#96).
//!
//! Every expected value is worked out by hand in the test's comment: the
//! point by walking the segment lengths, the axes from the reference-up
//! construction (`right = tangent x up_ref`, `up = right x tangent`) with
//! `up_ref = +Z`. Nothing is read back from the code under test.

#![cfg(feature = "compile")]

use axiolid_evaluate::ReferenceCurveEvaluator;
use ifc_alignment::{CurveMeasure, PointByDistance};
use ifc_geometry::constraint::placement::derive::derive_placement_transform;
use ifc_geometry::transform::Transform;
use ifc_geometry::units::UnitScale;
use ifc_geometry::GeometryError;
use ifc_model::{Entity, EntityId, Model, Value};
use std::sync::Arc;

const TOL: f64 = 1e-9;
const PLACEMENT: EntityId = EntityId(9_001);
const BASIS: EntityId = EntityId(100);

fn metres() -> UnitScale {
    UnitScale {
        length_to_metres: 1.0,
        angle_to_radians: 1.0,
    }
}

fn millimetres() -> UnitScale {
    UnitScale {
        length_to_metres: 0.001,
        angle_to_radians: 1.0,
    }
}

fn reals(values: &[f64]) -> Value {
    Value::List(values.iter().copied().map(Value::Real).collect())
}

/// `#BASIS = IFCPOLYLINE((#1, #2, ...))` over the given coordinates.
fn polyline_model(points: &[&[f64]]) -> Model {
    let mut model = Model::new();
    let mut refs = Vec::new();
    for (i, point) in points.iter().enumerate() {
        let id = EntityId(i as u64 + 1);
        model.insert(id, Entity::new("IFCCARTESIANPOINT", vec![reals(point)]));
        refs.push(Value::Ref(id));
    }
    model.insert(BASIS, Entity::new("IFCPOLYLINE", vec![Value::List(refs)]));
    model
}

/// `#BASIS = IFCINDEXEDPOLYCURVE(#50, segments, $)` over a 2D or 3D list.
fn indexed_model(points: &[&[f64]], segments: Option<Vec<(&str, Vec<i64>)>>) -> Model {
    let mut model = Model::new();
    let list_type = if points[0].len() == 2 {
        "IFCCARTESIANPOINTLIST2D"
    } else {
        "IFCCARTESIANPOINTLIST3D"
    };
    let rows = Value::List(points.iter().map(|p| reals(p)).collect());
    model.insert(EntityId(50), Entity::new(list_type, vec![rows]));
    let segments = match segments {
        None => Value::Null,
        Some(segments) => Value::List(
            segments
                .into_iter()
                .map(|(tag, indices)| Value::Typed {
                    type_name: Arc::from(tag),
                    value: Box::new(Value::List(
                        indices.into_iter().map(Value::Integer).collect(),
                    )),
                })
                .collect(),
        ),
    };
    model.insert(
        BASIS,
        Entity::new(
            "IFCINDEXEDPOLYCURVE",
            vec![Value::Ref(EntityId(50)), segments, Value::Null],
        ),
    );
    model
}

fn expression(measure: CurveMeasure, offsets: [Option<f64>; 3]) -> PointByDistance {
    PointByDistance {
        entity: EntityId(9_000),
        distance_along: measure,
        offset_lateral: offsets[0],
        offset_vertical: offsets[1],
        offset_longitudinal: offsets[2],
        basis_curve: BASIS,
    }
}

fn derive(
    model: &Model,
    units: &UnitScale,
    measure: CurveMeasure,
    offsets: [Option<f64>; 3],
) -> Result<Transform, GeometryError> {
    derive_placement_transform(
        model,
        units,
        PLACEMENT,
        &expression(measure, offsets),
        &ReferenceCurveEvaluator::default(),
    )
}

fn assert_vec(actual: [f64; 3], expected: [f64; 3], what: &str) {
    for axis in 0..3 {
        assert!(
            (actual[axis] - expected[axis]).abs() < TOL,
            "{what}: got {actual:?}, expected {expected:?}"
        );
    }
}

/// Plan L: (0,0) -> (10,0) -> (10,20), 30 m long.
const L_PATH: [&[f64]; 3] = [&[0.0, 0.0], &[10.0, 0.0], &[10.0, 20.0]];

/// The frame at 15 m along the L, offset 2 left, 1 up, 0.5 ahead.
///
/// 15 m is 10 m of the first leg plus 5 m of the second, so the centreline
/// point is (10, 5, 0) heading +Y. IFC4.3: a positive `OffsetLateral` is to
/// the LEFT, left = Z x tangent = (0,0,1) x (0,1,0) = (-1,0,0); up =
/// tangent x left = (0,0,1) (#355). Origin = (10,5,0) + 2*left + 1*up +
/// 0.5*tangent = (8, 5.5, 1). The axes are (tangent, left, up).
fn assert_l_frame(transform: &Transform) {
    assert_vec(transform.basis[0], [0.0, 1.0, 0.0], "tangent");
    assert_vec(transform.basis[1], [-1.0, 0.0, 0.0], "left");
    assert_vec(transform.basis[2], [0.0, 0.0, 1.0], "up");
    assert_vec(transform.origin, [8.0, 5.5, 1.0], "origin");
}

const L_OFFSETS: [Option<f64>; 3] = [Some(2.0), Some(1.0), Some(0.5)];

#[test]
fn a_polyline_basis_derives_the_hand_computed_frame() {
    let model = polyline_model(&L_PATH);
    let transform = derive(&model, &metres(), CurveMeasure::Length(15.0), L_OFFSETS)
        .expect("a stated distance on a polyline has an exact parameter");
    assert_l_frame(&transform);
}

/// The same L authored in millimetres lands in the same place, in metres:
/// the curve, the distance and every offset are converted once.
#[test]
fn a_millimetre_polyline_derives_the_same_frame_in_metres() {
    let model = polyline_model(&[&[0.0, 0.0], &[10_000.0, 0.0], &[10_000.0, 20_000.0]]);
    let transform = derive(
        &model,
        &millimetres(),
        CurveMeasure::Length(15_000.0),
        [Some(2_000.0), Some(1_000.0), Some(500.0)],
    )
    .expect("15 000 mm along a 30 000 mm polyline");
    assert_l_frame(&transform);
}

/// A native parameter follows the IFC polyline parameterisation, one unit
/// per segment: u = 1.5 is the middle of the second leg, (10, 10, 0),
/// not 1.5 m along the first.
#[test]
fn a_polyline_parameter_counts_one_per_segment() {
    let model = polyline_model(&L_PATH);
    let transform = derive(&model, &metres(), CurveMeasure::Parameter(1.5), [None; 3])
        .expect("u = 1.5 lies inside [0, 2]");
    assert_vec(transform.origin, [10.0, 10.0, 0.0], "origin");
    assert_vec(transform.basis[0], [0.0, 1.0, 0.0], "tangent");
}

/// A sloped 3D segment: (0,0,0) -> (3,0,4), 5 m long.
///
/// At 2.5 m the point is (1.5, 0, 2), tangent (0.6, 0, 0.8).
/// left = Z x t = (0, 0.6, 0), normalised (0, 1, 0);
/// up = t x left = (-0.8, 0, 0.6): perpendicular to the tangent in its
/// vertical plane, where IFC4.3 puts `OffsetVertical` and the default `Axis`.
#[test]
fn a_sloped_3d_indexed_polycurve_derives_the_hand_computed_frame() {
    let model = indexed_model(&[&[0.0, 0.0, 0.0], &[3.0, 0.0, 4.0]], None);
    let transform = derive(&model, &metres(), CurveMeasure::Length(2.5), [None; 3])
        .expect("2.5 m along a 5 m segment");
    assert_vec(transform.origin, [1.5, 0.0, 2.0], "origin");
    assert_vec(transform.basis[0], [0.6, 0.0, 0.8], "tangent");
    assert_vec(transform.basis[1], [0.0, 1.0, 0.0], "left");
    assert_vec(transform.basis[2], [-0.8, 0.0, 0.6], "up");
}

/// Without `Segments` the indexed curve is its points in order.
#[test]
fn an_implicit_indexed_polycurve_derives_like_the_polyline() {
    let model = indexed_model(&L_PATH, None);
    let transform = derive(&model, &metres(), CurveMeasure::Length(15.0), L_OFFSETS)
        .expect("an all-lines indexed poly-curve is a polyline");
    assert_l_frame(&transform);
}

/// Explicit line segments are chained through their shared index.
#[test]
fn explicit_line_segments_derive_like_the_polyline() {
    let segments = vec![("IFCLINEINDEX", vec![1, 2]), ("IFCLINEINDEX", vec![2, 3])];
    let model = indexed_model(&L_PATH, Some(segments));
    let transform = derive(&model, &metres(), CurveMeasure::Length(15.0), L_OFFSETS)
        .expect("consecutive line segments form one path");
    assert_l_frame(&transform);
}

/// A multi-point `IfcLineIndex` is fine for a distance, but IFC does not
/// state how it divides its parametric length, so a parameter is refused.
#[test]
fn a_multi_point_line_index_takes_a_distance_but_refuses_a_parameter() {
    let segments = || Some(vec![("IFCLINEINDEX", vec![1, 2, 3])]);
    let model = indexed_model(&L_PATH, segments());
    let transform = derive(&model, &metres(), CurveMeasure::Length(15.0), L_OFFSETS)
        .expect("distance does not depend on the parameterisation");
    assert_l_frame(&transform);

    let error = derive(&model, &metres(), CurveMeasure::Parameter(1.5), [None; 3])
        .expect_err("the parameter split of a multi-point line index is not stated");
    assert!(
        matches!(error, GeometryError::Unsupported { entity: BASIS, .. }),
        "got: {error:?}"
    );
    assert!(error.to_string().contains("IfcLineIndex"), "got: {error}");
}

/// An arc segment is refused by name, not flattened.
#[test]
fn an_indexed_polycurve_with_an_arc_is_refused_by_name() {
    let segments = vec![("IFCARCINDEX", vec![1, 2, 3])];
    let model = indexed_model(&[&[0.0, 0.0], &[1.0, 1.0], &[2.0, 0.0]], Some(segments));
    let error = derive(&model, &metres(), CurveMeasure::Length(0.5), [None; 3])
        .expect_err("an arc segment is not a polyline");
    assert!(
        matches!(error, GeometryError::Unsupported { entity: BASIS, .. }),
        "got: {error:?}"
    );
    let text = error.to_string();
    assert!(text.contains("IFCINDEXEDPOLYCURVE"), "got: {text}");
    assert!(text.contains("IfcArcIndex"), "got: {text}");
}

/// Segments that do not chain have no single path to measure along.
#[test]
fn non_consecutive_line_segments_are_refused() {
    let segments = vec![("IFCLINEINDEX", vec![1, 2]), ("IFCLINEINDEX", vec![3, 4])];
    let model = indexed_model(
        &[&[0.0, 0.0], &[1.0, 0.0], &[5.0, 0.0], &[6.0, 0.0]],
        Some(segments),
    );
    let error = derive(&model, &metres(), CurveMeasure::Length(0.5), [None; 3])
        .expect_err("a gap between segments is not one curve");
    assert!(
        matches!(error, GeometryError::Degenerate { entity: BASIS, .. }),
        "got: {error:?}"
    );
    assert!(error.to_string().contains("Consecutive"), "got: {error}");
}

/// A repeated vertex has no direction. It is refused naming the curve and
/// the segment, even when the requested distance stops before it.
#[test]
fn a_zero_length_segment_is_refused_by_name() {
    let model = polyline_model(&[&[0.0, 0.0], &[10.0, 0.0], &[10.0, 0.0], &[10.0, 5.0]]);
    let error = derive(&model, &metres(), CurveMeasure::Length(2.0), [None; 3])
        .expect_err("segment 2 has zero length");
    assert!(
        matches!(error, GeometryError::Degenerate { entity: BASIS, .. }),
        "got: {error:?}"
    );
    let text = error.to_string();
    assert!(text.contains("IFCPOLYLINE"), "got: {text}");
    assert!(text.contains("segment 2"), "got: {text}");
}

/// A path with fewer than two points has no segment at all.
#[test]
fn an_indexed_polycurve_without_a_segment_is_refused_by_name() {
    for model in [
        indexed_model(&[&[0.0, 0.0]], None),
        indexed_model(&L_PATH, Some(vec![])),
    ] {
        let error = derive(&model, &metres(), CurveMeasure::Length(0.0), [None; 3])
            .expect_err("nothing to measure along");
        assert!(
            matches!(error, GeometryError::Degenerate { entity: BASIS, .. }),
            "got: {error:?}"
        );
        assert!(error.to_string().contains("no segment"), "got: {error}");
    }
}

/// A vertical segment leaves roll undefined; the refusal says so.
#[test]
fn a_vertical_polyline_reports_undefined_roll() {
    let model = polyline_model(&[&[0.0, 0.0, 0.0], &[0.0, 0.0, 5.0]]);
    let error = derive(&model, &metres(), CurveMeasure::Length(1.0), [None; 3])
        .expect_err("a tangent parallel to up has no roll");
    assert!(
        matches!(
            error,
            GeometryError::Unsupported {
                entity: PLACEMENT,
                ..
            }
        ),
        "got: {error:?}"
    );
    assert!(error.to_string().contains("roll"), "got: {error}");
}

/// Families without a closed-form arc length stay refused by name.
#[test]
fn ellipse_and_bspline_bases_stay_refused_by_name() {
    for (type_name, attributes) in [
        (
            "IFCELLIPSE",
            vec![Value::Null, Value::Real(5.0), Value::Real(3.0)],
        ),
        (
            "IFCBSPLINECURVEWITHKNOTS",
            vec![Value::Integer(1), Value::List(vec![]), Value::Null],
        ),
    ] {
        let mut model = Model::new();
        model.insert(BASIS, Entity::new(type_name, attributes));
        let error = derive(&model, &metres(), CurveMeasure::Length(1.0), [None; 3])
            .expect_err("no exact distance on this family");
        assert!(
            matches!(
                error,
                GeometryError::Unsupported {
                    entity: PLACEMENT,
                    ..
                }
            ),
            "got: {error:?}"
        );
        assert!(error.to_string().contains(type_name), "got: {error}");
    }
}
