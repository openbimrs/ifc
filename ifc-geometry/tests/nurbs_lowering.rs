#![cfg(feature = "lowering")]

use axiolid_core::Point3;
use axiolid_curve::{Curve3, KnotSpec};
use axiolid_model::GeometryNode;
use axiolid_surface::Surface;
use ifc_geometry::lower::{lower_curve_node, lower_surface_node, LoweringSession};
use ifc_geometry::{Transform, UnitScale};
use ifc_model::{Codec, EntityId};
use std::path::PathBuf;

const UNITS: UnitScale = UnitScale {
    length_to_metres: 0.01,
    angle_to_radians: 1.0,
};

fn frame() -> Transform {
    Transform {
        // Rotate +90 degrees about Z, then translate.
        basis: [[0.0, 1.0, 0.0], [-1.0, 0.0, 0.0], [0.0, 0.0, 1.0]],
        origin: [10.0, 20.0, 30.0],
    }
}

fn model() -> ifc_model::Model {
    let path = PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../test/fixtures/nurbs/ifc4_rational_bspline_curve_surface.ifc"
    ));
    ifc_step::StepCodec
        .read_path(&path)
        .expect("fixture parses")
}

/// The deliberately invalid companion fixture: abstract base spline instances.
fn invalid_base_spline_model() -> ifc_model::Model {
    let path = PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../test/fixtures/nurbs/invalid_abstract_base_splines.ifc"
    ));
    ifc_step::StepCodec
        .read_path(&path)
        .expect("fixture parses")
}

fn lower_curve(model: &ifc_model::Model, id: u64) -> ifc_geometry::lower::LoweredGeometry {
    let mut session = LoweringSession::new(model, &UNITS);
    let root = lower_curve_node(&mut session, EntityId(id), frame())
        .expect("explicit-knot B-spline curve lowers");
    session.finish(root).expect("geometry graph closes")
}

fn lower_surface(model: &ifc_model::Model, id: u64) -> ifc_geometry::lower::LoweredGeometry {
    let mut session = LoweringSession::new(model, &UNITS);
    let root = lower_surface_node(&mut session, EntityId(id), frame())
        .expect("explicit-knot B-spline surface lowers");
    session.finish(root).expect("geometry graph closes")
}

fn assert_point(actual: Point3, expected: [f64; 3]) {
    assert!((actual.x - expected[0]).abs() < 1e-12, "x: {actual:?}");
    assert!((actual.y - expected[1]).abs() < 1e-12, "y: {actual:?}");
    assert!((actual.z - expected[2]).abs() < 1e-12, "z: {actual:?}");
}

#[test]
fn imported_rational_nurbs_curve_lowers_and_evaluates_without_fidelity_loss() {
    let model = model();
    let curve_graph = lower_curve(&model, 10);
    let curve = match curve_graph.graph.get(curve_graph.root).expect("curve root") {
        GeometryNode::Curve3(curve) => curve,
        other => panic!("expected Curve3, got {other:?}"),
    };
    let Curve3::BSpline(spline) = curve else {
        panic!("expected B-spline curve")
    };
    assert_eq!(spline.degree, 2);
    assert_point(spline.control_points[0], [10.0, 20.01, 30.0]);
    assert_point(spline.control_points[1], [9.99, 20.01, 30.0]);
    assert_point(spline.control_points[2], [9.99, 20.0, 30.0]);
    assert_eq!(spline.knots, [0.0, 1.0]);
    assert_eq!(spline.multiplicities, [3, 3]);
    assert_eq!(spline.knot_spec, KnotSpec::PiecewiseBezier);
    assert!(!spline.closed);
    assert_eq!(spline.self_intersect, Some(false));
    assert_eq!(
        spline.weights.as_deref(),
        Some([1.0, std::f64::consts::FRAC_1_SQRT_2, 1.0].as_slice())
    );
    let point = axiolid_reference::evaluate3(curve, 0.5).expect("curve evaluates");
    let offset = 0.01 * std::f64::consts::FRAC_1_SQRT_2;
    assert_point(point, [10.0 - offset, 20.0 + offset, 30.0]);
}

#[test]
fn imported_rational_nurbs_surface_lowers_and_evaluates_without_fidelity_loss() {
    let model = model();
    let surface_graph = lower_surface(&model, 20);
    let surface = match surface_graph
        .graph
        .get(surface_graph.root)
        .expect("surface root")
    {
        GeometryNode::Surface(surface) => surface,
        other => panic!("expected Surface, got {other:?}"),
    };
    let Surface::BSpline(spline) = surface else {
        panic!("expected B-spline surface")
    };
    assert_eq!((spline.u_degree, spline.v_degree), (2, 1));
    assert_eq!(spline.control_points.len(), 3);
    for (actual, expected) in spline.control_points.iter().flatten().zip([
        [10.0, 20.0, 30.0],
        [9.98, 20.0, 30.0],
        [10.0, 20.02, 30.0],
        [9.98, 20.02, 30.02],
        [10.0, 20.0, 30.0],
        [9.98, 20.0, 30.0],
    ]) {
        assert_point(*actual, expected);
    }
    assert_eq!(spline.u_knots, [0.0, 1.0]);
    assert_eq!(spline.v_knots, [0.0, 2.0]);
    assert_eq!(spline.u_multiplicities, [3, 3]);
    assert_eq!(spline.v_multiplicities, [2, 2]);
    assert_eq!(spline.knot_spec, KnotSpec::PiecewiseBezier);
    assert!(spline.u_closed);
    assert!(!spline.v_closed);
    assert_eq!(spline.self_intersect, Some(false));
    assert_eq!(
        spline.weights.as_ref().unwrap(),
        &[vec![1.0, 2.0], vec![1.0, 1.0], vec![1.0, 2.0]]
    );
    let point = axiolid_reference::surface::evaluate(surface, 0.5, 1.0).expect("surface evaluates");
    assert_point(point, [9.988, 20.008, 30.004]);
}

#[test]
fn explicit_knot_polynomial_subtypes_lower_without_inventing_weights() {
    let model = model();
    let lowered = lower_curve(&model, 11);
    let GeometryNode::Curve3(Curve3::BSpline(curve)) = lowered.graph.get(lowered.root).unwrap()
    else {
        panic!("expected polynomial B-spline curve")
    };
    assert_eq!(curve.weights, None);
    assert_eq!(curve.degree, 1);
    assert!(curve.closed);
    assert_eq!(curve.knot_spec, KnotSpec::QuasiUniform);

    let lowered = lower_surface(&model, 21);
    let GeometryNode::Surface(Surface::BSpline(surface)) = lowered.graph.get(lowered.root).unwrap()
    else {
        panic!("expected polynomial B-spline surface")
    };
    assert_eq!(surface.weights, None);
    assert_eq!((surface.u_degree, surface.v_degree), (1, 2));
    assert!(!surface.u_closed);
    assert!(surface.v_closed);
    assert_eq!(surface.knot_spec, KnotSpec::PiecewiseBezier);
}

/// An abstract base spline is refused, not silently completed.
///
/// `IfcBSplineCurve` and `IfcBSplineSurface` are ABSTRACT SUPERTYPE in IFC4,
/// so a conforming file never contains one -- but real exporters emit them,
/// and the lowering path must produce a typed report naming the entity rather
/// than inventing the knots the concrete `*WithKnots` subtypes carry.
///
/// The instances live in their own fixture because they are illegal IFC:
/// keeping them in the valid fixture made it fail schema validation, and a
/// corpus that cannot be validated is a corpus that hides real defects.
#[test]
fn convention_only_base_splines_are_typed_unsupported() {
    let model = invalid_base_spline_model();

    let mut curve_session = LoweringSession::new(&model, &UNITS);
    let curve_error = lower_curve_node(&mut curve_session, EntityId(12), frame())
        .expect_err("convention-only curve must not invent missing knots");
    assert!(curve_error.is_unsupported(), "got: {curve_error}");
    assert_eq!(curve_error.entity(), Some(EntityId(12)));

    let mut surface_session = LoweringSession::new(&model, &UNITS);
    let surface_error = lower_surface_node(&mut surface_session, EntityId(22), frame())
        .expect_err("convention-only surface must not invent missing knots");
    assert!(surface_error.is_unsupported(), "got: {surface_error}");
    assert_eq!(surface_error.entity(), Some(EntityId(22)));
}
