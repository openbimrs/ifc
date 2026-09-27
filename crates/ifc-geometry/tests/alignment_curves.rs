//! Authoring the alignment geometry: spirals and derived curves.

use ifc_geometry::authoring::{
    axis2_placement_2d, cartesian_point, direction, line, offset_curve_by_distances,
    polynomial_curve, segmented_reference_curve, spiral, PolynomialCoefficients, SpiralKind,
};
use ifc_model::{Entity, EntityId, Model, Transaction, Value};

fn placement(tx: &mut Transaction) -> EntityId {
    let origin = cartesian_point(tx, &[0.0, 0.0]).expect("origin");
    axis2_placement_2d(tx, origin, None)
}

/// Every spiral stages with its own arity and leading term.
///
/// The terms are trailing OPTIONALs: an absent one occupies its slot as
/// Null rather than shortening the record, because a shorter list would
/// shift each remaining term up a degree.
#[test]
fn every_spiral_kind_stages() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let position = placement(&mut tx);

    let cases: Vec<(SpiralKind, &str, usize)> = vec![
        (SpiralKind::Clothoid { constant: 150.0 }, "IFCCLOTHOID", 2),
        (
            SpiralKind::Cosine {
                cosine: 2.0,
                constant: None,
            },
            "IFCCOSINESPIRAL",
            3,
        ),
        (
            SpiralKind::Sine {
                sine: 2.0,
                linear: Some(1.0),
                constant: None,
            },
            "IFCSINESPIRAL",
            4,
        ),
    ];
    for (kind, expected, arity) in cases {
        let id = spiral(&mut tx, position, kind).expect("spiral");
        tx.commit(&mut model).expect("commit");
        let staged = model.get(id).expect("staged");
        assert_eq!(staged.type_name.as_ref(), expected);
        assert_eq!(staged.attributes.len(), arity, "{expected} arity");
        tx = Transaction::new(&model);
    }
}

/// The polynomial spirals keep their terms in descending degree.
#[test]
fn the_polynomial_spirals_order_their_terms() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let position = placement(&mut tx);

    let second = spiral(
        &mut tx,
        position,
        SpiralKind::SecondOrder {
            quadratic: 4.0,
            linear: None,
            constant: Some(1.0),
        },
    )
    .expect("second order");
    let third = spiral(
        &mut tx,
        position,
        SpiralKind::ThirdOrder {
            cubic: 8.0,
            quadratic: None,
            linear: None,
            constant: None,
        },
    )
    .expect("third order");
    let seventh = spiral(
        &mut tx,
        position,
        SpiralKind::SeventhOrder {
            septic: 16.0,
            lower: [None, None, None, None, None, None, Some(2.0)],
        },
    )
    .expect("seventh order");
    tx.commit(&mut model).expect("commit");

    let staged = model.get(second).expect("staged");
    assert_eq!(staged.attributes.len(), 4);
    assert_eq!(staged.attributes[1], Value::Real(4.0), "QuadraticTerm");
    // An absent LinearTerm holds its slot, so ConstantTerm stays at 3.
    assert_eq!(staged.attributes[2], Value::Null, "LinearTerm absent");
    assert_eq!(staged.attributes[3], Value::Real(1.0), "ConstantTerm");

    assert_eq!(model.get(third).expect("staged").attributes.len(), 5);
    let staged = model.get(seventh).expect("staged");
    assert_eq!(staged.attributes.len(), 9);
    assert_eq!(staged.attributes[1], Value::Real(16.0), "SepticTerm");
    assert_eq!(staged.attributes[8], Value::Real(2.0), "ConstantTerm last");
}

/// A zero leading term describes a lower-order curve.
#[test]
fn a_zero_leading_term_is_refused() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let position = placement(&mut tx);

    assert!(
        spiral(&mut tx, position, SpiralKind::Clothoid { constant: 0.0 }).is_err(),
        "a zero clothoid constant was accepted",
    );
    assert!(
        spiral(
            &mut tx,
            position,
            SpiralKind::Clothoid { constant: f64::NAN }
        )
        .is_err(),
        "a NaN clothoid constant was accepted",
    );
}

/// ValidCoefficients: at least two axes must be given.
#[test]
fn a_polynomial_curve_needs_two_axes() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let position = placement(&mut tx);

    assert!(
        polynomial_curve(
            &mut tx,
            position,
            PolynomialCoefficients {
                x: Some(&[0.0, 1.0]),
                ..PolynomialCoefficients::default()
            },
            false,
        )
        .is_err(),
        "one axis alone was accepted",
    );
    // LIST [2:?]: a single coefficient is a constant, not a curve.
    assert!(
        polynomial_curve(
            &mut tx,
            position,
            PolynomialCoefficients {
                x: Some(&[1.0]),
                y: Some(&[0.0, 1.0]),
                z: None,
            },
            false,
        )
        .is_err(),
        "a one-entry coefficient list was accepted",
    );
    // CorrectPositionDim: Z coefficients need a 3D placement.
    assert!(
        polynomial_curve(
            &mut tx,
            position,
            PolynomialCoefficients {
                x: Some(&[0.0, 1.0]),
                y: Some(&[0.0, 1.0]),
                z: Some(&[0.0, 1.0]),
            },
            false,
        )
        .is_err(),
        "Z coefficients were accepted against a 2D position",
    );

    let id = polynomial_curve(
        &mut tx,
        position,
        PolynomialCoefficients {
            x: Some(&[0.0, 1.0]),
            y: Some(&[0.0, 0.5]),
            z: None,
        },
        false,
    )
    .expect("polynomial curve");
    tx.commit(&mut model).expect("commit");

    let staged = model.get(id).expect("staged");
    assert_eq!(staged.type_name.as_ref(), "IFCPOLYNOMIALCURVE");
    assert_eq!(staged.attributes.len(), 4);
    assert_eq!(staged.attributes[3], Value::Null, "CoefficientsZ absent");
}

/// The varying-offset and cant curves stage, and refuse empty lists.
#[test]
fn the_derived_curves_stage() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let origin = cartesian_point(&mut tx, &[0.0, 0.0]).expect("origin");
    let dir = direction(&mut tx, &[1.0, 0.0]).expect("direction");
    let basis = line(&mut tx, origin, dir);
    let offset = tx.create(Entity::new(
        "IFCPOINTBYDISTANCEEXPRESSION",
        vec![Value::Null; 5],
    ));
    let segment = tx.create(Entity::new("IFCCURVESEGMENT", vec![Value::Null; 5]));

    assert!(
        offset_curve_by_distances(&mut tx, basis, &[], None).is_err(),
        "an empty LIST [1:?] of offsets was accepted",
    );
    assert!(
        segmented_reference_curve(&mut tx, &[], None, basis, None).is_err(),
        "an empty LIST [1:?] of segments was accepted",
    );

    let offset_curve = offset_curve_by_distances(&mut tx, basis, &[offset], Some("widening"))
        .expect("offset curve by distances");
    let cant = segmented_reference_curve(&mut tx, &[segment], None, basis, None)
        .expect("segmented reference curve");
    tx.commit(&mut model).expect("commit");

    let staged = model.get(offset_curve).expect("staged");
    assert_eq!(staged.type_name.as_ref(), "IFCOFFSETCURVEBYDISTANCES");
    assert_eq!(staged.attributes.len(), 3);
    assert_eq!(staged.attributes[1], Value::List(vec![Value::Ref(offset)]));

    let staged = model.get(cant).expect("staged");
    assert_eq!(staged.type_name.as_ref(), "IFCSEGMENTEDREFERENCECURVE");
    assert_eq!(staged.attributes.len(), 4);
    // SelfIntersect is a logical: unstated is UNKNOWN, not false.
    assert_eq!(staged.attributes[1], Value::LogicalUnknown);
}
