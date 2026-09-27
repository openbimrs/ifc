//! Unit tests for the B-spline surface view: grid shape, u/v convention,
//! knots and weights.

use super::*;

/// A `u_count` by `v_count` grid whose ids encode their position as
/// `u * 10 + v`, so a transposition is visible in the assertion.
fn grid(u_count: usize, v_count: usize) -> Value {
    Value::List(
        (0..u_count)
            .map(|u| {
                Value::List(
                    (0..v_count)
                        .map(|v| Value::Ref(EntityId((u * 10 + v) as u64)))
                        .collect(),
                )
            })
            .collect(),
    )
}

fn integers(values: &[i64]) -> Value {
    Value::List(values.iter().map(|i| Value::Integer(*i)).collect())
}

fn reals(values: &[f64]) -> Value {
    Value::List(values.iter().map(|r| Value::Real(*r)).collect())
}

/// Degree 1 in both directions, 2x2 control points, clamped knots.
fn bilinear() -> Entity {
    Entity::new(
        "IFCBSPLINESURFACEWITHKNOTS",
        vec![
            Value::Integer(1),
            Value::Integer(1),
            grid(2, 2),
            Value::Enum("UNSPECIFIED".into()),
            Value::Bool(false),
            Value::Bool(false),
            Value::Bool(false),
            integers(&[2, 2]),
            integers(&[2, 2]),
            reals(&[0.0, 1.0]),
            reals(&[0.0, 1.0]),
            Value::Enum("UNSPECIFIED".into()),
        ],
    )
}

#[test]
fn inherited_surface_slots_precede_the_knot_and_weight_slots() {
    let e = bilinear();
    let view = BSplineSurface::new(EntityId(1), &e);
    assert_eq!(view.u_degree().unwrap(), 1);
    assert_eq!(view.v_degree().unwrap(), 1);
    assert!(view.has_knots());
    assert!(!view.is_rational());
    assert_eq!(view.knot_spec(), KnotType::Unspecified);
}

/// Grid ids `u * 10 + v` resolve to points whose x/y record `(u, v)`.
fn grid_points(ids: &[u64]) -> Model {
    let mut model = Model::new();
    for &id in ids {
        let coords = reals(&[(id / 10) as f64, (id % 10) as f64, 0.0]);
        model.insert(EntityId(id), Entity::new("IFCCARTESIANPOINT", vec![coords]));
    }
    model
}

#[test]
fn control_point_views_resolve_the_grid_without_transposing_it() {
    let e = bilinear();
    let view = BSplineSurface::new(EntityId(1), &e);
    let model = grid_points(&[0, 1, 10, 11]);
    let points = view.control_point_views(&model).unwrap();
    assert_eq!(points.len(), 2);
    assert_eq!(points[1][0].coordinates_3d().unwrap(), [1.0, 0.0, 0.0]);
    assert_eq!(points[0][1].coordinates_3d().unwrap(), [0.0, 1.0, 0.0]);
}

#[test]
fn one_missing_control_point_fails_the_whole_grid() {
    let e = bilinear();
    let view = BSplineSurface::new(EntityId(1), &e);
    let err = view
        .control_point_views(&grid_points(&[0, 1, 10]))
        .unwrap_err();
    assert!(matches!(
        err,
        crate::GeometryError::MissingEntity {
            referrer: EntityId(1),
            missing: EntityId(11)
        }
    ));
}

/// The outer list runs along u and the inner along v. A transposed read
/// still typechecks and still passes count checks on a square grid, so it
/// is pinned by position-encoded ids on a non-square grid.
#[test]
fn the_outer_control_point_list_runs_along_u_and_the_inner_along_v() {
    let e = Entity::new(
        "IFCBSPLINESURFACE",
        vec![
            Value::Integer(1),
            Value::Integer(1),
            grid(3, 5),
            Value::Enum("UNSPECIFIED".into()),
            Value::Bool(false),
            Value::Bool(false),
            Value::Bool(false),
        ],
    );
    let points = BSplineSurface::new(EntityId(1), &e)
        .control_points()
        .unwrap();
    assert_eq!(points.u_count(), 3, "outer list length is the u count");
    assert_eq!(points.v_count(), 5, "inner list length is the v count");
    // Id u*10 + v: (2, 4) must be 24, not 42.
    assert_eq!(points.get(2, 4), Some(EntityId(24)));
    assert_eq!(points.rows().len(), 3);
}

/// A ragged grid is not a tensor-product surface and cannot be evaluated.
#[test]
fn a_ragged_control_point_grid_is_rejected() {
    let mut e = bilinear();
    e.attributes[slot::CONTROL_POINTS] = Value::List(vec![
        Value::List(vec![Value::Ref(EntityId(1)), Value::Ref(EntityId(2))]),
        Value::List(vec![Value::Ref(EntityId(3))]),
    ]);
    let err = BSplineSurface::new(EntityId(7), &e)
        .control_points()
        .unwrap_err();
    assert!(err.to_string().contains("rectangular"), "got: {err}");
    assert!(err.to_string().contains("#7"), "got: {err}");
}

#[test]
fn both_knot_vectors_are_checked_against_their_own_control_point_count() {
    let e = bilinear();
    let view = BSplineSurface::new(EntityId(1), &e);
    let u = view.u_knots().unwrap().unwrap();
    let v = view.v_knots().unwrap().unwrap();
    assert_eq!(u.expanded(), Some(vec![0.0, 0.0, 1.0, 1.0]));
    assert_eq!(v.expanded(), Some(vec![0.0, 0.0, 1.0, 1.0]));
    assert!(u.is_clamped(1));
}

/// The u and v checks must be independent: a wrong v multiplicity must not
/// be masked by a correct u one.
#[test]
fn a_wrong_v_multiplicity_sum_is_caught_even_when_u_is_right() {
    let mut e = bilinear();
    e.attributes[slot::V_MULTIPLICITIES] = integers(&[2, 3]);
    let view = BSplineSurface::new(EntityId(1), &e);
    assert!(view.u_knots().is_ok(), "u is untouched and must still pass");
    let err = view.v_knots().unwrap_err();
    assert!(err.to_string().contains("VMultiplicities"), "got: {err}");
}

#[test]
fn parallel_knot_lists_of_different_lengths_are_rejected() {
    let mut e = bilinear();
    e.attributes[slot::U_KNOTS] = reals(&[0.0, 0.5, 1.0]);
    let err = BSplineSurface::new(EntityId(1), &e).u_knots().unwrap_err();
    assert!(err.to_string().contains("parallel"), "got: {err}");
}

#[test]
fn non_increasing_knot_values_are_rejected() {
    let mut e = bilinear();
    e.attributes[slot::U_KNOTS] = reals(&[1.0, 0.0]);
    let err = BSplineSurface::new(EntityId(1), &e).u_knots().unwrap_err();
    assert!(err.to_string().contains("increasing"), "got: {err}");
}

#[test]
fn a_surface_without_knots_reports_none_rather_than_failing() {
    let e = Entity::new(
        "IFCBSPLINESURFACE",
        vec![
            Value::Integer(1),
            Value::Integer(1),
            grid(2, 2),
            Value::Enum("UNSPECIFIED".into()),
            Value::Bool(false),
            Value::Bool(false),
            Value::Bool(false),
        ],
    );
    let view = BSplineSurface::new(EntityId(1), &e);
    assert_eq!(view.u_knots().unwrap(), None);
    assert_eq!(view.v_knots().unwrap(), None);
    assert_eq!(view.weights().unwrap(), None);
}

#[test]
fn rational_weights_form_a_grid_of_the_same_shape_as_the_control_points() {
    let mut attributes = bilinear().attributes;
    attributes.push(Value::List(vec![reals(&[1.0, 0.5]), reals(&[0.5, 1.0])]));
    let e = Entity::new("IFCRATIONALBSPLINESURFACEWITHKNOTS", attributes);
    let view = BSplineSurface::new(EntityId(1), &e);
    assert!(view.is_rational());
    assert_eq!(
        view.weights().unwrap().unwrap(),
        vec![vec![1.0, 0.5], vec![0.5, 1.0]]
    );
}

#[test]
fn a_weight_grid_of_the_wrong_shape_is_rejected() {
    let mut attributes = bilinear().attributes;
    attributes.push(Value::List(vec![reals(&[1.0, 1.0])]));
    let e = Entity::new("IFCRATIONALBSPLINESURFACEWITHKNOTS", attributes);
    let err = BSplineSurface::new(EntityId(1), &e).weights().unwrap_err();
    assert!(err.to_string().contains("rows"), "got: {err}");
}

#[test]
fn a_non_positive_weight_anywhere_in_the_grid_is_degenerate() {
    for bad in [0.0, -1.0] {
        let mut attributes = bilinear().attributes;
        attributes.push(Value::List(vec![reals(&[1.0, 1.0]), reals(&[1.0, bad])]));
        let e = Entity::new("IFCRATIONALBSPLINESURFACEWITHKNOTS", attributes);
        let err = BSplineSurface::new(EntityId(1), &e).weights().unwrap_err();
        assert!(err.to_string().contains("positive"), "weight {bad}: {err}");
        assert!(err.to_string().contains("[1][1]"), "weight {bad}: {err}");
    }
}

#[test]
fn multiplicity_overflow_is_a_typed_error_not_a_panic() {
    let mut e = bilinear();
    e.attributes[slot::U_MULTIPLICITIES] = integers(&[i64::MAX, i64::MAX, i64::MAX]);
    e.attributes[slot::U_KNOTS] = reals(&[0.0, 0.5, 1.0]);
    let err = BSplineSurface::new(EntityId(8), &e).u_knots().unwrap_err();
    assert!(err.to_string().contains("overflow"), "got: {err}");
}

#[test]
fn each_degree_must_not_exceed_its_control_point_upper_index() {
    let mut e = bilinear();
    e.attributes[slot::U_DEGREE] = Value::Integer(2);
    assert!(BSplineSurface::new(EntityId(9), &e)
        .u_degree()
        .unwrap_err()
        .to_string()
        .contains("control points"));

    let mut e = bilinear();
    e.attributes[slot::V_DEGREE] = Value::Integer(2);
    assert!(BSplineSurface::new(EntityId(10), &e)
        .v_degree()
        .unwrap_err()
        .to_string()
        .contains("control points"));
}

#[test]
fn rational_subtype_requires_weights_and_polynomial_rejects_them() {
    let attributes = bilinear().attributes;
    let rational = Entity::new("IFCRATIONALBSPLINESURFACEWITHKNOTS", attributes.clone());
    assert!(BSplineSurface::new(EntityId(11), &rational)
        .weights()
        .unwrap_err()
        .to_string()
        .contains("missing WeightsData"));

    let mut polynomial_attributes = attributes;
    polynomial_attributes.push(Value::List(vec![reals(&[1.0, 1.0]), reals(&[1.0, 1.0])]));
    let polynomial = Entity::new("IFCBSPLINESURFACEWITHKNOTS", polynomial_attributes);
    assert!(BSplineSurface::new(EntityId(12), &polynomial)
        .weights()
        .unwrap_err()
        .to_string()
        .contains("must not carry WeightsData"));
}

#[test]
fn degree_zero_in_either_direction_is_rejected() {
    let mut e = bilinear();
    e.attributes[slot::U_DEGREE] = Value::Integer(0);
    assert!(BSplineSurface::new(EntityId(1), &e).u_degree().is_err());

    let mut e = bilinear();
    e.attributes[slot::V_DEGREE] = Value::Integer(0);
    assert!(BSplineSurface::new(EntityId(1), &e).v_degree().is_err());
}

/// The form is provenance, never a licence to swap in an analytic surface.
#[test]
fn surface_form_tokens_parse_without_replacing_the_control_points() {
    assert_eq!(
        BSplineSurfaceForm::from_token("SURF_OF_LINEAR_EXTRUSION"),
        Some(BSplineSurfaceForm::SurfOfLinearExtrusion)
    );
    assert_eq!(
        BSplineSurfaceForm::from_token("CYLINDRICAL_SURF"),
        Some(BSplineSurfaceForm::CylindricalSurf)
    );
    assert_eq!(BSplineSurfaceForm::from_token("BLOB"), None);
}

#[test]
fn closure_flags_are_read_independently_for_u_and_v() {
    let mut e = bilinear();
    e.attributes[slot::U_CLOSED] = Value::Bool(true);
    e.attributes[slot::V_CLOSED] = Value::LogicalUnknown;
    let view = BSplineSurface::new(EntityId(1), &e);
    assert_eq!(view.u_closed(), Some(true));
    assert_eq!(view.v_closed(), None, ".U. must not become false");
}
