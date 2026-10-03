//! `IfcCurveSegment` read-back against hand-computed geometry.
//!
//! Lines, arcs and polylines are checked point by point in closed form.
//! Spiral pieces are checked by their law (closed form) and by evaluated
//! positions against reference values: the Fresnel-type integrals of the
//! stated headings, computed independently with mpmath quadrature at 30
//! digits and quoted to 1e-9 m.

use std::f64::consts::PI;

use axiolid_core::{Frame2, Point2, Vec2};
use axiolid_curve::{CurvatureLaw, Curve3, Intrinsic2, Intrinsic3};
use axiolid_model::{CurveRelation, GeometryNode, TrimSelector};
use ifc_model::{EntityId, Model, Value};

use super::super::fixture::{length, parameter, Builder, METRES};
use super::super::lower_curve_node;
use crate::lower::session::LoweringSession;
use crate::lower::LoweredGeometry;
use crate::solid::testkit::{entity, n, r};
use crate::transform::Transform;
use crate::GeometryResult;

fn lower(model: &Model, id: u64) -> GeometryResult<LoweredGeometry> {
    let mut session = LoweringSession::new(model, &METRES);
    let root = lower_curve_node(&mut session, EntityId(id), Transform::identity())?;
    session.finish(root)
}

fn root(lowered: &LoweredGeometry) -> &GeometryNode {
    lowered.graph.get(lowered.root).expect("root node")
}

fn points(lowered: &LoweredGeometry) -> Vec<[f64; 3]> {
    match root(lowered) {
        GeometryNode::Curve3(Curve3::Polyline(polyline)) => {
            polyline.points.iter().map(|p| p.to_array()).collect()
        }
        other => panic!("expected a polyline, got {other:?}"),
    }
}

fn intrinsic(lowered: &LoweredGeometry) -> &Intrinsic3 {
    match root(lowered) {
        GeometryNode::Curve3(Curve3::Intrinsic(curve)) => curve,
        other => panic!("expected an intrinsic curve, got {other:?}"),
    }
}

fn assert_near(actual: [f64; 3], expected: [f64; 3], tolerance: f64) {
    for (a, e) in actual.iter().zip(expected) {
        assert!(
            (a - e).abs() <= tolerance,
            "{actual:?} vs {expected:?} at {tolerance}"
        );
    }
}

/// The planar curve in its XY plane, evaluated by the reference evaluator.
fn point_at(curve: &Intrinsic3, s: f64) -> [f64; 2] {
    assert_eq!(curve.start.z.to_array(), [0.0, 0.0, 1.0], "placed in XY");
    assert!(curve.is_planar());
    let plane = Intrinsic2::new(
        Frame2 {
            origin: Point2::new(curve.start.origin.x, curve.start.origin.y),
            x: Vec2::new(curve.start.x.x, curve.start.x.y),
            y: Vec2::new(curve.start.y.x, curve.start.y.y),
        },
        curve.curvature.clone(),
        curve.length,
    );
    let p = axiolid_evaluate::intrinsic_point(&plane, s).expect("evaluate");
    [p.x, p.y]
}

/// A line piece is its placement plus `|SegmentLength|` along RefDirection.
#[test]
fn a_line_segment_runs_from_its_placement_along_ref_direction() {
    let mut b = Builder::new();
    let line = b.line();
    let place = b.placement([10.0, 5.0], [0.0, 1.0]);
    let segment = b.segment(place, length(7.0), length(20.0), line);
    let lowered = lower(&b.model, segment).expect("line segment");
    assert_eq!(points(&lowered), vec![[10.0, 5.0, 0.0], [10.0, 25.0, 0.0]]);
}

/// A quarter arc of R = 10: forwards it turns left to (10, 10), backwards
/// right to (10, -10). The parent's own Position cancels.
#[test]
fn a_circle_segment_turns_left_forwards_and_right_backwards() {
    for (sign, end) in [(1.0, [10.0, 10.0, 0.0]), (-1.0, [10.0, -10.0, 0.0])] {
        let mut b = Builder::new();
        let circle = b.circle(10.0);
        let place = b.placement([0.0, 0.0], [1.0, 0.0]);
        let quarter = sign * 10.0 * PI / 2.0;
        let segment = b.segment(place, length(4.0), length(quarter), circle);
        let lowered = lower(&b.model, segment).expect("arc segment");
        let GeometryNode::CurveRelation(CurveRelation::Trimmed {
            basis,
            start,
            end: stop,
            ..
        }) = root(&lowered)
        else {
            panic!("expected a trimmed circle");
        };
        assert_eq!(start, &vec![TrimSelector::Parameter(0.0)]);
        let Some(TrimSelector::Parameter(sweep)) = stop.first() else {
            panic!("expected a parameter trim");
        };
        assert!((sweep - PI / 2.0).abs() <= 1e-15);
        let Some(GeometryNode::Curve3(Curve3::Circle(c))) = lowered.graph.get(*basis) else {
            panic!("expected a circle basis");
        };
        let at = |theta: f64| {
            (c.frame.origin + (c.frame.x * theta.cos() + c.frame.y * theta.sin()) * c.radius)
                .to_array()
        };
        assert_near(at(0.0), [0.0, 0.0, 0.0], 1e-12);
        assert_near(at(*sweep), end, 1e-12);
        // The start tangent is RefDirection, in both senses.
        assert_near(c.frame.y.to_array(), [1.0, 0.0, 0.0], 1e-15);
    }
}

/// Clothoid A = 100 from its inflection point: k = s / 10000, and the end
/// of a 50 m piece sits at the Fresnel point (49.921931494, 2.081009340).
#[test]
fn a_clothoid_segment_carries_the_law_and_reaches_the_fresnel_point() {
    let mut b = Builder::new();
    let spiral = b.clothoid(100.0);
    let place = b.placement([0.0, 0.0], [1.0, 0.0]);
    let segment = b.segment(place, length(0.0), length(50.0), spiral);
    let lowered = lower(&b.model, segment).expect("clothoid segment");
    let curve = intrinsic(&lowered);
    assert_eq!(
        curve.curvature,
        CurvatureLaw::Polynomial {
            coefficients: vec![0.0, 1e-4]
        }
    );
    assert_eq!(curve.length, 50.0);
    let [x, y] = point_at(curve, 50.0);
    assert!((x - 49.921_931_493_660).abs() <= 1e-9, "{x}");
    assert!((y - 2.081_009_340_177).abs() <= 1e-9, "{y}");
}

/// A piece from s = 50 for 30 m, placed at (100, 0) heading +x: the law is
/// rebased to k = 0.005 + 1e-4 t and the curve starts AT the placement.
/// Backwards (SegmentLength -30) it reads k = -(0.005 - 1e-4 t).
#[test]
fn an_offset_clothoid_piece_is_rebased_to_its_placement() {
    for (run, coefficients, end) in [
        (30.0, vec![0.005, 1e-4], [129.831_110_777, 2.691_929_297]),
        (-30.0, vec![-0.005, 1e-4], [129.932_090_832, -1.798_125_199]),
    ] {
        let mut b = Builder::new();
        let spiral = b.clothoid(100.0);
        let place = b.placement([100.0, 0.0], [1.0, 0.0]);
        let segment = b.segment(place, length(50.0), length(run), spiral);
        let lowered = lower(&b.model, segment).expect("clothoid piece");
        let curve = intrinsic(&lowered);
        let CurvatureLaw::Polynomial { coefficients: c } = &curve.curvature else {
            panic!("expected a polynomial law");
        };
        for (a, e) in c.iter().zip(&coefficients) {
            assert!((a - e).abs() <= 1e-15, "{c:?}");
        }
        assert_near(curve.start.origin.to_array(), [100.0, 0.0, 0.0], 0.0);
        let [x, y] = point_at(curve, 30.0);
        assert!(
            (x - end[0]).abs() <= 1e-8 && (y - end[1]).abs() <= 1e-8,
            "{x}, {y}"
        );
    }
}

/// The segment length is the cosine spiral's L: with IfcOpenShell's terms
/// for a 0 -> 1/200 transition over 50 m, the end heading is the closed-form
/// average-curvature turn 50/400 = 0.125 rad.
#[test]
fn a_cosine_spiral_segment_reads_its_length_as_l() {
    let mut b = Builder::new();
    let position = b.placement([0.0, 0.0], [1.0, 0.0]);
    let spiral = b.add(entity(
        "IFCCOSINESPIRAL",
        vec![r(position), n(-400.0), n(400.0)],
    ));
    let place = b.placement([0.0, 0.0], [1.0, 0.0]);
    let segment = b.segment(place, length(0.0), length(50.0), spiral);
    let lowered = lower(&b.model, segment).expect("cosine segment");
    let curve = intrinsic(&lowered);
    let turn = curve.total_turning().expect("closed-form turning");
    assert!((turn - 0.125).abs() <= 1e-15, "{turn}");
    let [x, y] = point_at(curve, 50.0);
    assert!((x - 49.929_212_169).abs() <= 1e-8, "{x}");
    assert!((y - 1.856_345_484).abs() <= 1e-8, "{y}");
}

/// A 2D polyline cut at 5..15 m and placed at (100, 0) heading +y.
#[test]
fn a_polyline_segment_is_cut_by_arc_length_and_placed() {
    for (start, run, origin, direction, expected) in [
        (
            5.0,
            10.0,
            [100.0, 0.0],
            [0.0, 1.0],
            vec![[100.0, 0.0, 0.0], [100.0, 5.0, 0.0], [95.0, 5.0, 0.0]],
        ),
        (
            15.0,
            -10.0,
            [0.0, 0.0],
            [1.0, 0.0],
            vec![[0.0, 0.0, 0.0], [5.0, 0.0, 0.0], [5.0, -5.0, 0.0]],
        ),
    ] {
        let mut b = Builder::new();
        let corners: Vec<u64> = [[0.0, 0.0], [10.0, 0.0], [10.0, 10.0]]
            .iter()
            .map(|p| b.point(p))
            .collect();
        let polyline = b.add(entity(
            "IFCPOLYLINE",
            vec![Value::List(corners.into_iter().map(r).collect())],
        ));
        let place = b.placement(origin, direction);
        let segment = b.segment(place, length(start), length(run), polyline);
        let lowered = lower(&b.model, segment).expect("polyline segment");
        let got = points(&lowered);
        assert_eq!(got.len(), expected.len(), "{got:?}");
        for (g, e) in got.iter().zip(expected) {
            assert_near(*g, e, 1e-12);
        }
    }
}

/// A closing zero-length segment is its placement: a curve of length zero.
#[test]
fn a_zero_length_segment_is_its_placement() {
    let mut b = Builder::new();
    let spiral = b.clothoid(100.0);
    let place = b.placement([4.0, 2.0], [0.0, 1.0]);
    let segment = b.segment(place, length(0.0), length(0.0), spiral);
    let lowered = lower(&b.model, segment).expect("closing segment");
    let curve = intrinsic(&lowered);
    assert_eq!(curve.length, 0.0);
    assert_near(curve.start.origin.to_array(), [4.0, 2.0, 0.0], 0.0);
    assert_near(curve.start.x.to_array(), [0.0, 1.0, 0.0], 1e-15);
}

/// Each refusal is typed and names the entity that causes it.
#[test]
fn unsupported_segment_forms_are_typed_refusals() {
    // IfcParameterValue measures: no parametric space is defined.
    let mut b = Builder::new();
    let line = b.line();
    let place = b.placement([0.0, 0.0], [1.0, 0.0]);
    let segment = b.segment(place, parameter(0.0), parameter(1.0), line);
    let error = lower(&b.model, segment).expect_err("parameter measures");
    assert!(error.is_unsupported(), "{error}");
    assert!(error.to_string().contains("IfcParameterValue"), "{error}");

    // A polynomial parent cut where its placed start is not closed form, or
    // with no closed-form bound for its trim.
    for (x, z, start, run, needle) in [
        (vec![0.0, 1.0], None, 5.0, 10.0, "non-zero SegmentStart"),
        (vec![0.0, 1.0], None, 0.0, -10.0, "backwards"),
        (vec![0.0, 1.0, 0.5], None, 0.0, 10.0, "degree one"),
        (
            vec![0.0, 1.0],
            Some(vec![0.0, 1.0]),
            0.0,
            10.0,
            "3D polynomial",
        ),
    ] {
        let mut b = Builder::new();
        let position = b.placement([0.0, 0.0], [1.0, 0.0]);
        let polynomial = b.add(entity(
            "IFCPOLYNOMIALCURVE",
            vec![
                r(position),
                Value::List(x.into_iter().map(n).collect()),
                Value::List(vec![n(0.0), n(0.0), n(0.0), n(1e-6)]),
                z.map_or(Value::Null, |z| Value::List(z.into_iter().map(n).collect())),
            ],
        ));
        let place = b.placement([0.0, 0.0], [1.0, 0.0]);
        let segment = b.segment(place, length(start), length(run), polynomial);
        let error = lower(&b.model, segment).expect_err(needle);
        assert!(error.is_unsupported(), "{error}");
        assert_eq!(error.entity(), Some(EntityId(polynomial)));
        assert!(error.to_string().contains(needle), "{needle}: {error}");
    }

    // An IfcAxis2PlacementLinear placement stands at a station (#307).
    let mut b = Builder::new();
    let line = b.line();
    let linear = b.add(entity(
        "IFCAXIS2PLACEMENTLINEAR",
        vec![Value::Null, Value::Null, Value::Null],
    ));
    let segment = b.segment(linear, length(0.0), length(1.0), line);
    let error = lower(&b.model, segment).expect_err("linear placement");
    assert!(error.is_unsupported(), "{error}");
    assert!(error.to_string().contains("#307"), "{error}");

    // An untyped measure is malformed, not a gap.
    let mut b = Builder::new();
    let line = b.line();
    let place = b.placement([0.0, 0.0], [1.0, 0.0]);
    let segment = b.segment(place, n(0.0), n(1.0), line);
    let error = lower(&b.model, segment).expect_err("untyped measure");
    assert!(!error.is_unsupported(), "{error}");
}

/// A composite's closing zero-length segment adds nothing; a zero-length
/// segment elsewhere is refused.
#[test]
fn a_composite_drops_only_its_closing_zero_length_segment() {
    let build = |closing_last: bool| {
        let mut b = Builder::new();
        let line = b.line();
        let p0 = b.placement([0.0, 0.0], [1.0, 0.0]);
        let p1 = b.placement([10.0, 0.0], [1.0, 0.0]);
        let body = b.segment(p0, length(0.0), length(10.0), line);
        let closing = b.segment(p1, length(0.0), length(0.0), line);
        let order = if closing_last {
            [body, closing]
        } else {
            [closing, body]
        };
        let composite = b.add(entity(
            "IFCCOMPOSITECURVE",
            vec![
                Value::List(order.into_iter().map(r).collect()),
                Value::Bool(false),
            ],
        ));
        (b.model, composite)
    };
    let (model, composite) = build(true);
    let lowered = lower(&model, composite).expect("composite with closing segment");
    let GeometryNode::CurveRelation(CurveRelation::Composite { segments }) = root(&lowered) else {
        panic!("expected a composite");
    };
    assert_eq!(segments.len(), 1);

    let (model, composite) = build(false);
    let error = lower(&model, composite).expect_err("misplaced zero-length segment");
    assert!(!error.is_unsupported(), "{error}");
}

/// The end of `y = a x^3` by arc length, independently of the kernel:
/// composite Simpson quadrature of `sqrt(1 + (3 a x^2)^2)` and bisection.
fn cubic_end(a: f64, run: f64) -> [f64; 2] {
    let arc = |x: f64| {
        let steps = 2000;
        let h = x / f64::from(steps);
        let speed = |t: f64| (1.0 + (3.0 * a * t * t).powi(2)).sqrt();
        let mut sum = speed(0.0) + speed(x);
        for i in 1..steps {
            sum += speed(f64::from(i) * h) * if i % 2 == 1 { 4.0 } else { 2.0 };
        }
        sum * h / 3.0
    };
    let (mut lo, mut hi) = (0.0, run);
    for _ in 0..200 {
        let mid = 0.5 * (lo + hi);
        if arc(mid) < run {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let x = 0.5 * (lo + hi);
    [x, a * x.powi(3)]
}

/// A CUBIC transition `y = x^3 / (6 R L)` (`R = 300`, `L = 60`) as an
/// `IfcPolynomialCurve` parent (#90): its Bezier, placed at `(10, 20)`
/// heading north, trimmed where its arc length reaches 60. The control
/// points are the power-to-Bernstein conversion over `x in [0, 60]`
/// (`CoefficientsX = (0, 1)` bounds the parameter by the length):
/// `(0, 0)`, `(20, 0)`, `(40, 0)`, `(60, 2)` in the segment's own frame.
#[test]
fn a_polynomial_parent_is_its_bezier_trimmed_by_arc_length() {
    let a = 1.0 / (6.0 * 300.0 * 60.0);
    let mut b = Builder::new();
    let cubic = b.parabola(&[0.0, 0.0, 0.0, a]);
    let segment = b.segment_at([10.0, 20.0], [0.0, 1.0], 60.0, cubic);
    let lowered = lower(&b.model, segment).expect("polynomial parent");
    let GeometryNode::CurveRelation(CurveRelation::Trimmed {
        basis, start, end, ..
    }) = root(&lowered)
    else {
        panic!("expected a trimmed curve");
    };
    assert_eq!(start.as_slice(), [TrimSelector::Parameter(0.0)]);
    assert_eq!(end.as_slice(), [TrimSelector::ArcLength(60.0)]);
    let Some(GeometryNode::Curve3(curve @ Curve3::BSpline(bezier))) = lowered.graph.get(*basis)
    else {
        panic!("expected a B-spline basis");
    };
    assert_eq!(bezier.degree, 3);
    assert_eq!(bezier.knots, vec![0.0, 60.0]);
    assert_eq!(bezier.multiplicities, vec![4, 4]);
    // Heading north: local x is world +y, local y (left) is world -x.
    let expected = [[10.0, 20.0], [10.0, 40.0], [10.0, 60.0], [8.0, 80.0]];
    for (point, [x, y]) in bezier.control_points.iter().zip(expected) {
        assert_near(point.to_array(), [x, y, 0.0], 1e-12);
    }
    let resolved =
        axiolid_evaluate::parameter_at_arc_length3(curve, 0.0, 60.0).expect("arc-length trim");
    let [x, y] = cubic_end(a, 60.0);
    assert!((resolved - x).abs() < 1e-9, "{resolved} != {x}");
    let point = axiolid_evaluate::evaluate3(curve, resolved).expect("end");
    assert_near(point.to_array(), [10.0 - y, 20.0 + x, 0.0], 1e-9);
}
