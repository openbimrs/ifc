#![cfg(feature = "lowering")]

use axiolid_core::Point3;
use axiolid_curve::Curve3;
use axiolid_model::GeometryNode;
use axiolid_surface::Surface;
use ifc_geometry::lower::{lower_curve_node, lower_surface_node, LoweringSession, Tolerance};
use ifc_geometry::{Transform, UnitScale};
use ifc_model::{Codec, EntityId};
use std::path::PathBuf;

fn model() -> ifc_model::Model {
    let path = PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../test/fixtures/nurbs/ifc4_rational_bspline_curve_surface.ifc"
    ));
    ifc_step::StepCodec
        .read_path(&path)
        .expect("fixture parses")
}

fn lower_curve(model: &ifc_model::Model, id: u64) -> ifc_geometry::lower::LoweredGeometry {
    let units = UnitScale::default();
    let mut session = LoweringSession::new(model, &units, Tolerance::building_scale());
    let root = lower_curve_node(&mut session, EntityId(id), Transform::identity())
        .expect("explicit-knot B-spline curve lowers");
    session.finish(root).expect("geometry graph closes")
}

fn lower_surface(model: &ifc_model::Model, id: u64) -> ifc_geometry::lower::LoweredGeometry {
    let units = UnitScale::default();
    let mut session = LoweringSession::new(model, &units, Tolerance::building_scale());
    let root = lower_surface_node(&mut session, EntityId(id), Transform::identity())
        .expect("explicit-knot B-spline surface lowers");
    session.finish(root).expect("geometry graph closes")
}

#[test]
fn imported_rational_nurbs_lower_and_evaluate_without_fidelity_loss() {
    let model = model();
    let units = UnitScale::default();

    let mut curve_session = LoweringSession::new(&model, &units, Tolerance::building_scale());
    let curve_root = lower_curve_node(&mut curve_session, EntityId(10), Transform::identity())
        .expect("rational curve lowers");
    let curve_graph = curve_session
        .finish(curve_root)
        .expect("curve graph closes");
    let curve = match curve_graph.graph.get(curve_graph.root).expect("curve root") {
        GeometryNode::Curve3(curve) => curve,
        other => panic!("expected Curve3, got {other:?}"),
    };
    let Curve3::BSpline(spline) = curve else {
        panic!("expected B-spline curve")
    };
    assert_eq!(spline.degree, 2);
    assert_eq!(
        spline.control_points,
        [
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(1.0, 1.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        ]
    );
    assert_eq!(spline.knots, [0.0, 1.0]);
    assert_eq!(spline.multiplicities, [3, 3]);
    assert_eq!(
        spline.weights.as_deref(),
        Some([1.0, std::f64::consts::FRAC_1_SQRT_2, 1.0].as_slice())
    );
    let point = axiolid_scalar::evaluate3(curve, 0.5).expect("curve evaluates");
    assert!((point.x - std::f64::consts::FRAC_1_SQRT_2).abs() < 1e-14);
    assert!((point.y - std::f64::consts::FRAC_1_SQRT_2).abs() < 1e-14);
    assert!(point.z.abs() < 1e-14);

    let mut surface_session = LoweringSession::new(&model, &units, Tolerance::building_scale());
    let surface_root =
        lower_surface_node(&mut surface_session, EntityId(20), Transform::identity())
            .expect("rational surface lowers");
    let surface_graph = surface_session
        .finish(surface_root)
        .expect("surface graph closes");
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
    assert_eq!((spline.u_degree, spline.v_degree), (1, 1));
    assert_eq!(
        spline.control_points,
        [
            vec![Point3::new(0.0, 0.0, 0.0), Point3::new(0.0, 2.0, 0.0)],
            vec![Point3::new(2.0, 0.0, 0.0), Point3::new(2.0, 2.0, 2.0)],
        ]
    );
    assert_eq!(
        (spline.u_knots.as_slice(), spline.v_knots.as_slice()),
        (&[0.0, 1.0][..], &[0.0, 1.0][..])
    );
    assert_eq!(
        (
            spline.u_multiplicities.as_slice(),
            spline.v_multiplicities.as_slice()
        ),
        (&[2u32, 2][..], &[2u32, 2][..])
    );
    assert_eq!(
        spline.weights.as_ref().unwrap(),
        &[vec![1.0, 2.0], vec![1.0, 1.0]]
    );
    let point = axiolid_scalar::surface::evaluate(surface, 0.5, 0.5).expect("surface evaluates");
    assert!((point.x - 0.8).abs() < 1e-14);
    assert!((point.y - 1.2).abs() < 1e-14);
    assert!((point.z - 0.4).abs() < 1e-14);
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

    let lowered = lower_surface(&model, 21);
    let GeometryNode::Surface(Surface::BSpline(surface)) = lowered.graph.get(lowered.root).unwrap()
    else {
        panic!("expected polynomial B-spline surface")
    };
    assert_eq!(surface.weights, None);
}
