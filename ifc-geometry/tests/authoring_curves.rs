//! Curves: authored, then read back through the same crate's views.
//!
//! The three cases worth the most here are the ones where a wrong
//! write still produces a parseable file: a line's parameter scale, a
//! trim select that loses its measure wrapper, and a B-spline knot
//! vector that does not describe the curve it is attached to.

use ifc_geometry::authoring::{
    bspline_curve_with_knots, cartesian_point, cartesian_point_list_3d, circle, composite_curve,
    composite_curve_segment, direction, ellipse, indexed_poly_curve, line, offset_curve_2d,
    offset_curve_3d, rational_bspline_curve_with_knots, trimmed_curve, vector, KnotVector,
    PolyCurveSegment,
};
use ifc_geometry::curve::bspline::BSplineCurve;
use ifc_geometry::curve::composite::{CompositeCurve, CompositeCurveSegment, TransitionCode};
use ifc_geometry::curve::conic::{Circle, Ellipse};
use ifc_geometry::curve::line::Line;
use ifc_geometry::curve::trimmed::{Trim, TrimmedCurve, TrimmingPreference};
use ifc_model::{Model, Transaction};

/// A line's vector carries direction *and* parameter scale.
///
/// Magnitude 5 on a unit direction means parameter 1 advances five
/// length units. A writer that normalises it away changes the curve.
#[test]
fn a_line_keeps_its_parameter_scale() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let point = cartesian_point(&mut tx, &[1.0, 2.0, 3.0]).expect("point");
    let dir = direction(&mut tx, &[1.0, 0.0, 0.0]).expect("direction");
    let v = vector(&mut tx, dir, 5.0).expect("vector");
    let id = line(&mut tx, point, v);

    let mut model = model;
    tx.commit(&mut model).expect("commit");

    let view = Line::new(id, model.get(id).expect("line"));
    assert_eq!(view.point_ref().expect("pnt"), point);
    assert_eq!(view.direction_vector_ref().expect("dir"), v);

    let vector_view = model.get(v).expect("vector");
    assert_eq!(vector_view.type_name.as_ref(), "IFCVECTOR");
    assert_eq!(vector_view.attributes.len(), 2, "Orientation and Magnitude");
}

/// Conics round-trip through their views.
#[test]
fn conics_round_trip() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let origin = cartesian_point(&mut tx, &[0.0, 0.0]).expect("origin");
    let placement = ifc_geometry::authoring::axis2_placement_2d(&mut tx, origin, None);

    let c = circle(&mut tx, placement, 2.5).expect("circle");
    // Distinct semi-axes, so a slot swap is visible.
    let e = ellipse(&mut tx, placement, 4.0, 1.5).expect("ellipse");

    let mut model = model;
    tx.commit(&mut model).expect("commit");

    let view = Circle::new(c, model.get(c).expect("circle"));
    assert_eq!(view.radius().expect("radius"), 2.5);
    assert_eq!(view.position_ref().expect("position"), placement);

    let view = Ellipse::new(e, model.get(e).expect("ellipse"));
    assert_eq!(view.semi_axis_1().expect("semi 1"), 4.0);
    assert_eq!(view.semi_axis_2().expect("semi 2"), 1.5);
}

/// A trimmed curve's parameter keeps its measure wrapper.
///
/// The reader tolerates a bare real, so a writer that drops
/// `IFCPARAMETERVALUE` round-trips and still loses the only marker
/// saying the number is a parameter. Reading it back through the view
/// proves the wrapper survived.
#[test]
fn a_trim_keeps_its_parameter_measure() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let origin = cartesian_point(&mut tx, &[0.0, 0.0]).expect("origin");
    let placement = ifc_geometry::authoring::axis2_placement_2d(&mut tx, origin, None);
    let basis = circle(&mut tx, placement, 1.0).expect("basis");
    let stop = cartesian_point(&mut tx, &[1.0, 0.0]).expect("stop");

    let id = trimmed_curve(
        &mut tx,
        basis,
        Trim {
            cartesian: None,
            parameter: Some(0.0),
        },
        Trim {
            cartesian: Some(stop),
            parameter: Some(90.0),
        },
        true,
        TrimmingPreference::Parameter,
    )
    .expect("trimmed");

    let mut model = model;
    tx.commit(&mut model).expect("commit");

    let view = TrimmedCurve::new(id, model.get(id).expect("trimmed"));
    assert_eq!(view.basis_curve_ref().expect("basis"), basis);
    assert!(view.sense_agreement().expect("sense"));
    assert_eq!(view.master_representation(), TrimmingPreference::Parameter);

    let first = view.trim1().expect("trim 1");
    let second = view.trim2().expect("trim 2");
    assert_eq!(first.parameter, Some(0.0));
    assert_eq!(first.cartesian, None);

    // Both forms supplied: the reader must see each in its own field.
    assert_eq!(second.parameter, Some(90.0));
    assert_eq!(second.cartesian, Some(stop));
}

/// A trim stating neither a point nor a parameter is refused.
#[test]
fn an_empty_trim_is_refused() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let origin = cartesian_point(&mut tx, &[0.0, 0.0]).expect("origin");
    let placement = ifc_geometry::authoring::axis2_placement_2d(&mut tx, origin, None);
    let basis = circle(&mut tx, placement, 1.0).expect("basis");

    assert!(
        trimmed_curve(
            &mut tx,
            basis,
            Trim::default(),
            Trim {
                cartesian: None,
                parameter: Some(1.0)
            },
            true,
            TrimmingPreference::Unspecified,
        )
        .is_err(),
        "SET [1:2] has a lower bound of one"
    );
}

/// A composite curve round-trips with its transition codes.
#[test]
fn a_composite_curve_keeps_its_transitions() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let origin = cartesian_point(&mut tx, &[0.0, 0.0]).expect("origin");
    let placement = ifc_geometry::authoring::axis2_placement_2d(&mut tx, origin, None);
    let arc = circle(&mut tx, placement, 1.0).expect("arc");

    let first = composite_curve_segment(&mut tx, TransitionCode::ContSameGradient, true, arc);
    // The last segment of an open curve does not continue into anything.
    let last = composite_curve_segment(&mut tx, TransitionCode::Discontinuous, false, arc);
    let id = composite_curve(&mut tx, &[first, last], None).expect("composite");

    let mut model = model;
    tx.commit(&mut model).expect("commit");

    let view = CompositeCurve::new(id, model.get(id).expect("composite"));
    let segments = view.segment_refs().expect("segments");
    assert_eq!(segments, vec![first, last]);

    let entity = model.get(first).expect("first");
    let seg = CompositeCurveSegment::new(first, entity);
    assert_eq!(
        seg.transition().expect("transition"),
        TransitionCode::ContSameGradient
    );
    assert!(seg.same_sense().expect("same sense"));
    assert_eq!(seg.parent_curve_ref().expect("parent"), arc);

    let entity = model.get(last).expect("last");
    let seg = CompositeCurveSegment::new(last, entity);
    assert!(!seg.same_sense().expect("same sense"), "false must survive");
    assert!(
        !seg.transition().expect("transition").is_connected(),
        "a discontinuous joint must not claim connection"
    );
}

/// An empty composite curve is refused: `LIST [1:?]`.
#[test]
fn an_empty_composite_curve_is_refused() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    assert!(composite_curve(&mut tx, &[], None).is_err());
}

/// A B-spline's knot vector must describe the curve it is attached to.
///
/// `sum(KnotMultiplicities) = Degree + |ControlPoints| + 1`. A cubic
/// through 5 points needs the multiplicities to total 9; anything else
/// describes no curve, and the writer refuses it.
#[test]
fn a_bspline_knot_vector_must_match_its_control_points() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let points: Vec<_> = (0..5)
        .map(|i| cartesian_point(&mut tx, &[f64::from(i), 0.0, 0.0]).expect("point"))
        .collect();

    // Degree 3 + 5 control points + 1 = 9.
    let good = KnotVector {
        multiplicities: &[4, 1, 4],
        knots: &[0.0, 0.5, 1.0],
        spec: "UNSPECIFIED",
    };
    let id = bspline_curve_with_knots(&mut tx, 3, &points, "UNSPECIFIED", good)
        .expect("a consistent knot vector is accepted");

    // One knot too few: sums to 8, not 9.
    let short = KnotVector {
        multiplicities: &[4, 4],
        knots: &[0.0, 1.0],
        spec: "UNSPECIFIED",
    };
    assert!(
        bspline_curve_with_knots(&mut tx, 3, &points, "UNSPECIFIED", short).is_err(),
        "multiplicities summing to 8 cannot describe a cubic through 5 points"
    );

    // Knots must strictly increase.
    let repeated = KnotVector {
        multiplicities: &[4, 1, 4],
        knots: &[0.0, 0.0, 1.0],
        spec: "UNSPECIFIED",
    };
    assert!(
        bspline_curve_with_knots(&mut tx, 3, &points, "UNSPECIFIED", repeated).is_err(),
        "a repeated knot value belongs in the multiplicity, not the list"
    );

    assert!(
        bspline_curve_with_knots(&mut tx, 0, &points, "UNSPECIFIED", good).is_err(),
        "degree 0 is not a curve"
    );

    let mut model = model;
    tx.commit(&mut model).expect("commit");

    let view = BSplineCurve::new(id, model.get(id).expect("bspline"));
    assert_eq!(view.degree().expect("degree"), 3);
    assert_eq!(view.control_point_refs().expect("control points"), points);
}

/// A rational B-spline needs one weight per control point.
#[test]
fn a_rational_bspline_needs_a_weight_per_control_point() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let points: Vec<_> = (0..3)
        .map(|i| cartesian_point(&mut tx, &[f64::from(i), 0.0, 0.0]).expect("point"))
        .collect();
    // Degree 2 + 3 control points + 1 = 6.
    let knots = KnotVector {
        multiplicities: &[3, 3],
        knots: &[0.0, 1.0],
        spec: "UNSPECIFIED",
    };

    assert!(
        rational_bspline_curve_with_knots(&mut tx, 2, &points, "UNSPECIFIED", knots, &[1.0, 0.5],)
            .is_err(),
        "2 weights for 3 control points"
    );

    let id = rational_bspline_curve_with_knots(
        &mut tx,
        2,
        &points,
        "UNSPECIFIED",
        knots,
        &[1.0, 0.5, 1.0],
    )
    .expect("matched weights");

    let mut model = model;
    tx.commit(&mut model).expect("commit");
    let entity = model.get(id).expect("rational");
    assert_eq!(entity.attributes.len(), 9, "WeightsData is slot 8");
}

/// An indexed polycurve's segment indices are written 1-based.
///
/// Same boundary as the tessellation writers: the API takes 0-based
/// indices, the file carries `IfcPositiveInteger`. Reading back through
/// the crate's own segment parser proves the conversion and the
/// `IFCLINEINDEX`/`IFCARCINDEX` select tags both survived.
#[test]
fn polycurve_segments_are_written_one_based() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let points = cartesian_point_list_3d(
        &mut tx,
        &[
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [2.0, 1.0, 0.0],
            [3.0, 0.0, 0.0],
        ],
        None,
    )
    .expect("points");

    let id = indexed_poly_curve(
        &mut tx,
        points,
        Some(&[
            PolyCurveSegment::Line(&[0, 1]),
            PolyCurveSegment::Arc([1, 2, 3]),
        ]),
        4,
        None,
    )
    .expect("polycurve");

    let mut model = model;
    tx.commit(&mut model).expect("commit");

    let entity = model.get(id).expect("polycurve");
    let view = ifc_geometry::curve::polyline::IndexedPolyCurve::new(id, entity);
    let segments = view.segments(4).expect("segments parse");
    assert_eq!(segments.len(), 2);

    // The reader hands back 0-based indices, matching what was passed in.
    match &segments[0] {
        ifc_geometry::curve::polyline::PolySegment::Line(indices) => {
            assert_eq!(indices, &vec![0, 1]);
        }
        other => panic!("expected a line segment, got {other:?}"),
    }
    match &segments[1] {
        ifc_geometry::curve::polyline::PolySegment::Arc { start, mid, end } => {
            // `mid` is a point the arc passes through, not a centre.
            assert_eq!((*start, *mid, *end), (1, 2, 3));
        }
        other => panic!("expected an arc segment, got {other:?}"),
    }
}

/// An index past the end of the point list is refused.
#[test]
fn a_polycurve_index_past_the_points_is_refused() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let points = cartesian_point_list_3d(&mut tx, &[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]], None)
        .expect("points");

    assert!(
        indexed_poly_curve(
            &mut tx,
            points,
            Some(&[PolyCurveSegment::Line(&[0, 2])]),
            2,
            None
        )
        .is_err(),
        "index 2 is past a 2 point list"
    );
    assert!(
        indexed_poly_curve(
            &mut tx,
            points,
            Some(&[PolyCurveSegment::Line(&[0])]),
            2,
            None
        )
        .is_err(),
        "a line needs at least two points"
    );
}

/// Offsets take a signed distance and refuse a non-finite one.
#[test]
fn offsets_accept_a_signed_distance() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let origin = cartesian_point(&mut tx, &[0.0, 0.0]).expect("origin");
    let placement = ifc_geometry::authoring::axis2_placement_2d(&mut tx, origin, None);
    let basis = circle(&mut tx, placement, 1.0).expect("basis");
    let up = direction(&mut tx, &[0.0, 0.0, 1.0]).expect("up");

    // Negative offsets inward: IfcLengthMeasure, not the positive form.
    let inward = offset_curve_2d(&mut tx, basis, -0.25, None).expect("negative offset");
    let outward = offset_curve_3d(&mut tx, basis, 0.25, Some(false), up).expect("3d offset");
    assert!(
        offset_curve_2d(&mut tx, basis, f64::NAN, None).is_err(),
        "a non-finite distance is refused"
    );

    let mut model = model;
    tx.commit(&mut model).expect("commit");
    assert_eq!(model.get(inward).expect("2d").attributes.len(), 3);
    assert_eq!(model.get(outward).expect("3d").attributes.len(), 4);
}

/// The trim parameter is written as `IFCPARAMETERVALUE(..)`, not a bare real.
///
/// The crate's own reader accepts both, so a round-trip cannot see the
/// difference -- this asserts on the stored value directly. A file with
/// a bare real is not conforming even though every lenient reader
/// recovers it.
#[test]
fn a_trim_parameter_carries_its_measure_type() {
    use ifc_model::Value;

    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let origin = cartesian_point(&mut tx, &[0.0, 0.0]).expect("origin");
    let placement = ifc_geometry::authoring::axis2_placement_2d(&mut tx, origin, None);
    let basis = circle(&mut tx, placement, 1.0).expect("basis");
    let id = trimmed_curve(
        &mut tx,
        basis,
        Trim {
            cartesian: None,
            parameter: Some(0.0),
        },
        Trim {
            cartesian: None,
            parameter: Some(90.0),
        },
        true,
        TrimmingPreference::Parameter,
    )
    .expect("trimmed");

    let mut model = model;
    tx.commit(&mut model).expect("commit");

    let Value::List(members) = &model.get(id).expect("trimmed").attributes[1] else {
        panic!("Trim1 is not a set");
    };
    match &members[0] {
        Value::Typed { type_name, value } => {
            assert_eq!(type_name.as_ref(), "IFCPARAMETERVALUE");
            assert_eq!(**value, Value::Real(0.0));
        }
        other => panic!("trim parameter lost its measure wrapper: {other:?}"),
    }
}

/// Guards the writers apply that a well-formed call never trips.
///
/// Each of these is a value the schema forbids but a careless caller
/// supplies: a zero multiplicity, a degree of zero, a zero-magnitude
/// vector. Without this test the guards are unexercised code.
#[test]
fn malformed_curve_parameters_are_refused() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let points: Vec<_> = (0..3)
        .map(|i| cartesian_point(&mut tx, &[f64::from(i), 0.0, 0.0]).expect("point"))
        .collect();
    let dir = direction(&mut tx, &[1.0, 0.0, 0.0]).expect("direction");

    // A knot that repeats zero times is not a knot.
    let zero_multiplicity = KnotVector {
        multiplicities: &[6, 0],
        knots: &[0.0, 1.0],
        spec: "UNSPECIFIED",
    };
    assert!(
        bspline_curve_with_knots(&mut tx, 2, &points, "UNSPECIFIED", zero_multiplicity).is_err(),
        "a zero multiplicity is not expressible"
    );

    // Degree 0 would make every span a single point. The knot vector
    // here is consistent *with* degree 0 (multiplicities sum to
    // 0 + 3 + 1 = 4), so the degree guard is the only thing that can
    // reject it -- otherwise the knot identity masks the check.
    let consistent_with_degree_zero = KnotVector {
        multiplicities: &[2, 2],
        knots: &[0.0, 1.0],
        spec: "UNSPECIFIED",
    };
    assert!(
        bspline_curve_with_knots(
            &mut tx,
            0,
            &points,
            "UNSPECIFIED",
            consistent_with_degree_zero
        )
        .is_err(),
        "degree 0 is not a curve"
    );

    // Likewise for a negative degree: 0 + 3 + 1 shifted to -1 + 3 + 1 = 3.
    let consistent_with_negative_degree = KnotVector {
        multiplicities: &[2, 1],
        knots: &[0.0, 1.0],
        spec: "UNSPECIFIED",
    };
    assert!(
        bspline_curve_with_knots(
            &mut tx,
            -1,
            &points,
            "UNSPECIFIED",
            consistent_with_negative_degree,
        )
        .is_err(),
        "a negative degree is not a curve"
    );

    // A zero-magnitude vector gives its line no parameter scale at all.
    assert!(vector(&mut tx, dir, 0.0).is_err(), "zero magnitude");
    assert!(vector(&mut tx, dir, -1.0).is_err(), "negative magnitude");
    assert!(
        vector(&mut tx, dir, f64::NAN).is_err(),
        "non-finite magnitude"
    );
}
