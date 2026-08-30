#![cfg(feature = "lowering")]

use ifc_geometry::curve::BSplineCurve;
use ifc_geometry::surface::BSplineSurface;
use ifc_model::{Codec, EntityId, Model};
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
    assert_eq!(
        curve.weights().unwrap().unwrap(),
        [1.0, std::f64::consts::FRAC_1_SQRT_2, 1.0]
    );

    let surface = BSplineSurface::new(EntityId(20), model.get(EntityId(20)).expect("surface"));
    assert_eq!(
        (surface.u_degree().unwrap(), surface.v_degree().unwrap()),
        (1, 1)
    );
    assert_eq!(
        surface.control_points().unwrap().rows(),
        &[
            vec![EntityId(4), EntityId(5)],
            vec![EntityId(6), EntityId(7)]
        ]
    );
    let u = surface.u_knots().unwrap().expect("u knots");
    let v = surface.v_knots().unwrap().expect("v knots");
    assert_eq!((u.values, u.multiplicities), (vec![0.0, 1.0], vec![2, 2]));
    assert_eq!((v.values, v.multiplicities), (vec![0.0, 1.0], vec![2, 2]));
    assert_eq!(
        surface.weights().unwrap().unwrap(),
        [vec![1.0, 2.0], vec![1.0, 1.0]]
    );
}
