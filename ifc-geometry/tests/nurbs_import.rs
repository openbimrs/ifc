#![cfg(feature = "lowering")]

use ifc_geometry::curve::{BSplineCurve, KnotType};
use ifc_geometry::surface::BSplineSurface;
use ifc_model::{Codec, EntityId, Model, Value};
use std::path::PathBuf;

fn fixture() -> PathBuf {
    PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../test/fixtures/nurbs/ifc4_rational_bspline_curve_surface.ifc"
    ))
}

fn load() -> Model {
    ifc_step::StepCodec
        .read_path(&fixture())
        .expect("fixture parses through openbim-step")
}

#[test]
fn part21_parser_preserves_cad_grade_nurbs_parameters() {
    let model = load();
    let curve = BSplineCurve::new(EntityId(10), model.get(EntityId(10)).expect("curve"));
    assert_eq!(curve.degree().unwrap(), 2);
    assert_eq!(
        curve.control_point_refs().unwrap(),
        [EntityId(1), EntityId(2), EntityId(3)]
    );
    let knots = curve.knots().unwrap().expect("explicit knots");
    assert_eq!(knots.values, [0.0, 1.0]);
    assert_eq!(knots.multiplicities, [3, 3]);
    assert_eq!(curve.knot_spec(), KnotType::PiecewiseBezier);
    assert_eq!(curve.closed_curve(), Some(false));
    assert_eq!(
        curve.weights().unwrap().unwrap(),
        [1.0, std::f64::consts::FRAC_1_SQRT_2, 1.0]
    );

    let polynomial = BSplineCurve::new(
        EntityId(11),
        model.get(EntityId(11)).expect("polynomial curve"),
    );
    assert_eq!(polynomial.degree().unwrap(), 1);
    assert_eq!(polynomial.closed_curve(), Some(true));
    assert_eq!(polynomial.knot_spec(), KnotType::QuasiUniform);
    assert_eq!(polynomial.weights().unwrap(), None);

    let surface = BSplineSurface::new(EntityId(20), model.get(EntityId(20)).expect("surface"));
    assert_eq!(
        (surface.u_degree().unwrap(), surface.v_degree().unwrap()),
        (2, 1)
    );
    assert_eq!(
        surface.control_points().unwrap().rows(),
        &[
            vec![EntityId(4), EntityId(5)],
            vec![EntityId(6), EntityId(7)],
            vec![EntityId(4), EntityId(5)]
        ]
    );
    let u = surface.u_knots().unwrap().expect("u knots");
    let v = surface.v_knots().unwrap().expect("v knots");
    assert_eq!((u.values, u.multiplicities), (vec![0.0, 1.0], vec![3, 3]));
    assert_eq!((v.values, v.multiplicities), (vec![0.0, 2.0], vec![2, 2]));
    assert_eq!(surface.knot_spec(), KnotType::PiecewiseBezier);
    assert_eq!(
        (surface.u_closed(), surface.v_closed()),
        (Some(true), Some(false))
    );
    assert_eq!(
        surface.weights().unwrap().unwrap(),
        [vec![1.0, 2.0], vec![1.0, 1.0], vec![1.0, 2.0]]
    );

    let polynomial = BSplineSurface::new(
        EntityId(21),
        model.get(EntityId(21)).expect("polynomial surface"),
    );
    assert_eq!(
        (
            polynomial.u_degree().unwrap(),
            polynomial.v_degree().unwrap()
        ),
        (1, 2)
    );
    assert_eq!(
        (polynomial.u_closed(), polynomial.v_closed()),
        (Some(false), Some(true))
    );
    assert_eq!(polynomial.weights().unwrap(), None);
}

fn reals(values: &[f64]) -> Value {
    Value::List(values.iter().copied().map(Value::Real).collect())
}

#[test]
fn public_surface_view_rejects_non_finite_knots_and_weights() {
    // IFC4 ADD2 TC1 positional attributes for the rational surface fixture.
    const U_KNOTS: usize = 9;
    const V_KNOTS: usize = 10;
    const WEIGHTS_DATA: usize = 12;

    let model = load();
    let source = model.get(EntityId(20)).expect("rational surface");
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let knots = if bad.is_sign_negative() {
            [bad, 1.0]
        } else {
            [0.0, bad]
        };

        let mut u_entity = source.clone();
        u_entity.attributes[U_KNOTS] = reals(&knots);
        let u_error = BSplineSurface::new(EntityId(20), &u_entity)
            .u_knots()
            .unwrap_err();
        assert!(
            u_error.to_string().contains("finite"),
            "u knot {bad}: {u_error}"
        );

        let mut v_entity = source.clone();
        v_entity.attributes[V_KNOTS] = reals(&knots);
        let v_error = BSplineSurface::new(EntityId(20), &v_entity)
            .v_knots()
            .unwrap_err();
        assert!(
            v_error.to_string().contains("finite"),
            "v knot {bad}: {v_error}"
        );

        let mut weight_entity = source.clone();
        weight_entity.attributes[WEIGHTS_DATA] = Value::List(vec![
            reals(&[1.0, 2.0]),
            reals(&[1.0, bad]),
            reals(&[1.0, 2.0]),
        ]);
        let weight_error = BSplineSurface::new(EntityId(20), &weight_entity)
            .weights()
            .unwrap_err();
        assert!(
            weight_error.to_string().contains("finite"),
            "weight {bad}: {weight_error}"
        );
        assert!(
            weight_error.to_string().contains("[1][1]"),
            "weight {bad}: {weight_error}"
        );
    }
}
