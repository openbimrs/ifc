#![cfg(feature = "lowering")]

use axiolid_core::Point3;
use axiolid_curve::{Curve3, KnotSpec};
use axiolid_model::GeometryNode;
use axiolid_surface::Surface;
use ifc_geometry::lower::{lower_curve_node, lower_surface_node, LoweringSession, SessionLimits};
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

/// A hostile knot multiplicity is refused before anything reserves.
///
/// The fixture curve is valid; only the budget changes. A caller that sets a
/// small budget must get a typed, locatable refusal naming the entity --
/// never a truncated knot vector and never a panic.
#[test]
fn an_over_budget_knot_vector_is_refused_and_locatable() {
    let model = model();
    let limits = SessionLimits {
        max_aggregate_elements: 4,
        ..SessionLimits::default()
    };
    let mut session = LoweringSession::with_limits(&model, &UNITS, limits);
    let error = lower_curve_node(&mut session, EntityId(11), frame())
        .expect_err("an 8-element knot vector must not fit a budget of 4");
    assert_eq!(error.entity(), Some(EntityId(11)), "got: {error}");
    assert!(
        error.to_string().contains("knot multiplicities"),
        "got: {error}"
    );
    assert!(
        !error.is_unsupported(),
        "a budget refusal is not a capability gap"
    );
}

/// Below the limit, the budget is invisible: byte-identical geometry.
///
/// The project forbids silent approximation, so a budget that perturbed
/// in-range output would be worse than no budget at all.
#[test]
fn a_generous_budget_changes_nothing_about_the_lowered_geometry() {
    let model = model();
    let mut default_session = LoweringSession::new(&model, &UNITS);
    let a = lower_curve_node(&mut default_session, EntityId(11), frame()).expect("lowers");
    let a = default_session.finish(a).expect("finishes");
    let limits = SessionLimits {
        max_aggregate_elements: 1_000_000,
        ..SessionLimits::default()
    };
    let mut budgeted = LoweringSession::with_limits(&model, &UNITS, limits);
    let b = lower_curve_node(&mut budgeted, EntityId(11), frame()).expect("lowers");
    let b = budgeted.finish(b).expect("finishes");
    assert_eq!(a.graph.get(a.root), b.graph.get(b.root));
}

/// The v-knot budget also refuses, naming its own aggregate.
///
/// #21 declares v multiplicities (3,3)=6 with u=(2,2)=4, so a budget of 5
/// passes u and is refused by v -- proving both knot directions are checked
/// independently rather than one standing in for the other.
#[test]
fn an_over_budget_v_knot_vector_is_refused_and_locatable() {
    let model = model();
    let limits = SessionLimits {
        max_aggregate_elements: 5,
        ..SessionLimits::default()
    };
    let mut session = LoweringSession::with_limits(&model, &UNITS, limits);
    let error = lower_surface_node(&mut session, EntityId(21), frame())
        .expect_err("v multiplicities summing to 6 must not fit a budget of 5");
    assert_eq!(error.entity(), Some(EntityId(21)), "got: {error}");
    assert!(
        error.to_string().contains("v knot multiplicities"),
        "got: {error}"
    );
}

/// The knot budget is checked before the control grid is materialized.
///
/// #20 declares u multiplicities (3,3)=6 and a 3x2=6 grid. A budget of 5
/// must be refused by the u-knot check, naming that aggregate -- proving
/// the knot check runs and is not shadowed by the grid check.
#[test]
fn the_knot_budget_is_checked_before_the_control_grid() {
    let model = model();
    let limits = SessionLimits {
        max_aggregate_elements: 5,
        ..SessionLimits::default()
    };
    let mut session = LoweringSession::with_limits(&model, &UNITS, limits);
    let error = lower_surface_node(&mut session, EntityId(20), frame())
        .expect_err("u multiplicities summing to 6 must not fit a budget of 5");
    assert_eq!(error.entity(), Some(EntityId(20)), "got: {error}");
    assert!(
        error.to_string().contains("u knot multiplicities"),
        "got: {error}"
    );
}

/// The control-grid budget refuses independently of the knot budgets.
///
/// The committed fixtures all have grid == max(u_total, v_total), so a knot
/// check always fires first there and the grid check would go unproven.
/// This synthetic surface has a 3x3=9 grid with u=(2,2)=4 and v=(2,2)=4, so
/// a budget of 8 passes both knot checks and only the grid can refuse.
#[test]
fn an_over_budget_control_grid_is_refused_and_locatable() {
    use ifc_model::{Entity, Model, Value};
    let pt = |x: f64| {
        Entity::new(
            "IFCCARTESIANPOINT",
            vec![Value::List(vec![
                Value::Real(x),
                Value::Real(0.0),
                Value::Real(0.0),
            ])],
        )
    };
    let mut model = Model::new();
    for i in 1..=3u64 {
        model.insert(EntityId(i), pt(i as f64));
    }
    let row = || {
        Value::List(vec![
            Value::Ref(EntityId(1)),
            Value::Ref(EntityId(2)),
            Value::Ref(EntityId(3)),
        ])
    };
    let ints = |a: i64, b: i64| Value::List(vec![Value::Integer(a), Value::Integer(b)]);
    let reals = |a: f64, b: f64| Value::List(vec![Value::Real(a), Value::Real(b)]);
    model.insert(
        EntityId(10),
        Entity::new(
            "IFCBSPLINESURFACEWITHKNOTS",
            vec![
                Value::Integer(1),
                Value::Integer(1),
                Value::List(vec![row(), row(), row()]),
                Value::Enum("UNSPECIFIED".into()),
                Value::Bool(false),
                Value::Bool(false),
                Value::Bool(false),
                ints(2, 3),
                ints(3, 2),
                reals(0.0, 1.0),
                reals(0.0, 1.0),
                Value::Enum("UNSPECIFIED".into()),
            ],
        ),
    );
    let limits = SessionLimits {
        max_aggregate_elements: 8,
        ..SessionLimits::default()
    };
    let mut session = LoweringSession::with_limits(&model, &UNITS, limits);
    let error = lower_surface_node(&mut session, EntityId(10), Transform::identity())
        .expect_err("a 3x3 grid must not fit a budget of 8");
    assert_eq!(error.entity(), Some(EntityId(10)), "got: {error}");
    assert!(
        error.to_string().contains("control grid points"),
        "got: {error}"
    );
}
