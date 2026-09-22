//! Sweep the geometry writers and enum arms no test had reached.
//!
//! These writers all existed; nothing had ever called them, so a wrong
//! type name or slot layout in any of them compiled and passed the
//! suite. Each one is exercised against a real operand here.

use ifc_geometry::authoring::{
    arbitrary_closed_profile, boolean_result, cartesian_point_list_2d, cartesian_point_list_3d,
    circle_profile, connection_geometry, extruded_area_solid, pcurve, polyline,
    revolved_area_solid, surface_curve, ConnectionKind, SurfaceCurveKind,
    SurfaceCurveRepresentation,
};
use ifc_geometry::solid::boolean::IfcBooleanOperator;
use ifc_model::{Entity, EntityId, Model, Transaction, Value};

fn point(tx: &mut Transaction) -> EntityId {
    tx.create(Entity::new(
        "IFCCARTESIANPOINT",
        vec![Value::List(vec![
            Value::Real(0.0),
            Value::Real(0.0),
            Value::Real(0.0),
        ])],
    ))
}

fn direction(tx: &mut Transaction) -> EntityId {
    tx.create(Entity::new(
        "IFCDIRECTION",
        vec![Value::List(vec![
            Value::Real(0.0),
            Value::Real(0.0),
            Value::Real(1.0),
        ])],
    ))
}

fn surface(tx: &mut Transaction) -> EntityId {
    tx.create(Entity::new("IFCPLANE", vec![Value::Null; 1]))
}

/// Both profile writers stage their own type.
#[test]
fn the_profile_writers_stage() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);

    let corners: Vec<_> = (0..3).map(|_| point(&mut tx)).collect();
    let outline = polyline(&mut tx, &corners).expect("polyline");
    let arbitrary = arbitrary_closed_profile(&mut tx, Some("Slab outline"), outline);
    let circle = circle_profile(&mut tx, Some("Bar"), None, 0.012).expect("circle profile");
    tx.commit(&mut model).expect("commit");

    assert_eq!(
        model.get(arbitrary).expect("staged").type_name.as_ref(),
        "IFCARBITRARYCLOSEDPROFILEDEF",
    );
    assert_eq!(
        model.get(circle).expect("staged").type_name.as_ref(),
        "IFCCIRCLEPROFILEDEF",
    );
}

/// A circle profile's radius must be positive and finite.
#[test]
fn a_circle_profile_refuses_a_non_positive_radius() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    for radius in [0.0, -1.0, f64::NAN] {
        assert!(
            circle_profile(&mut tx, None, None, radius).is_err(),
            "accepted radius {radius}",
        );
    }
}

/// The revolved solid and the boolean result stage.
///
/// `IfcBooleanResult` is the operator-carrying supertype of
/// `IfcBooleanClippingResult`, and only the clipping form had been run.
#[test]
fn the_solid_writers_stage() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);

    let profile = circle_profile(&mut tx, None, None, 0.5).expect("profile");
    let axis = tx.create(Entity::new("IFCAXIS1PLACEMENT", vec![Value::Null; 2]));
    // Angle is radians in (0, 2pi], not degrees.
    let revolved = revolved_area_solid(&mut tx, profile, None, axis, std::f64::consts::FRAC_PI_2)
        .expect("revolved area solid");

    let up = direction(&mut tx);
    let extruded = extruded_area_solid(&mut tx, profile, None, up, 3.0).expect("extruded");
    let combined = boolean_result(&mut tx, IfcBooleanOperator::Difference, revolved, extruded)
        .expect("boolean result");
    tx.commit(&mut model).expect("commit");

    assert_eq!(
        model.get(revolved).expect("staged").type_name.as_ref(),
        "IFCREVOLVEDAREASOLID",
    );
    assert_eq!(
        model.get(combined).expect("staged").type_name.as_ref(),
        "IFCBOOLEANRESULT",
    );
}

/// The 2D point list stages, and its coordinates keep their arity.
///
/// The 3D sibling was covered; the 2D one writes a different type name
/// into the same shape, so nothing but the name distinguishes them.
#[test]
fn the_2d_point_list_stages() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let flat = cartesian_point_list_2d(&mut tx, &[[0.0, 0.0], [1.0, 0.0]], None).expect("2d list");
    let spatial =
        cartesian_point_list_3d(&mut tx, &[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]], None).expect("3d");
    tx.commit(&mut model).expect("commit");

    assert_eq!(
        model.get(flat).expect("staged").type_name.as_ref(),
        "IFCCARTESIANPOINTLIST2D",
    );
    assert_eq!(
        model.get(spatial).expect("staged").type_name.as_ref(),
        "IFCCARTESIANPOINTLIST3D",
    );

    let Value::List(rows) = &model.get(flat).expect("staged").attributes[0] else {
        panic!("CoordList is a list");
    };
    let Value::List(first) = &rows[0] else {
        panic!("each row is a list");
    };
    assert_eq!(first.len(), 2, "a 2D list holds pairs, not triples");
}

/// Every `ConnectionKind` arm stages its own type.
#[test]
fn every_connection_kind_stages() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let geometry = point(&mut tx);

    let cases = [
        (ConnectionKind::Point, "IFCCONNECTIONPOINTGEOMETRY"),
        (ConnectionKind::Curve, "IFCCONNECTIONCURVEGEOMETRY"),
        (ConnectionKind::Surface, "IFCCONNECTIONSURFACEGEOMETRY"),
        (ConnectionKind::Volume, "IFCCONNECTIONVOLUMEGEOMETRY"),
    ];
    let staged: Vec<_> = cases
        .iter()
        .map(|(kind, _)| connection_geometry(&mut tx, *kind, geometry, None))
        .collect();
    tx.commit(&mut model).expect("commit");

    for (id, (_, expected)) in staged.iter().zip(cases) {
        assert_eq!(model.get(*id).expect("staged").type_name.as_ref(), expected);
    }
}

/// Every `SurfaceCurveKind` arm stages, including the seam form.
///
/// The intersection and seam forms require exactly two pcurves; the
/// plain form accepts one.
#[test]
fn every_surface_curve_kind_stages() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let corners: Vec<_> = (0..2).map(|_| point(&mut tx)).collect();
    let curve_3d = polyline(&mut tx, &corners).expect("polyline");
    let basis = surface(&mut tx);
    let first = pcurve(&mut tx, basis, curve_3d);
    let second = pcurve(&mut tx, basis, curve_3d);

    let cases = [
        (SurfaceCurveKind::Plain, &[first][..], "IFCSURFACECURVE"),
        (
            SurfaceCurveKind::Intersection,
            &[first, second][..],
            "IFCINTERSECTIONCURVE",
        ),
        (SurfaceCurveKind::Seam, &[first, second][..], "IFCSEAMCURVE"),
    ];
    let staged: Vec<_> = cases
        .iter()
        .map(|(kind, geometry, _)| {
            surface_curve(
                &mut tx,
                *kind,
                curve_3d,
                geometry,
                SurfaceCurveRepresentation::Curve3D,
            )
            .expect("surface curve")
        })
        .collect();
    // The seam form needs two: one is refused. Checked before the
    // commit consumes the transaction.
    assert!(
        surface_curve(
            &mut tx,
            SurfaceCurveKind::Seam,
            curve_3d,
            &[first],
            SurfaceCurveRepresentation::Curve3D,
        )
        .is_err(),
        "a seam curve accepted a single pcurve",
    );

    tx.commit(&mut model).expect("commit");

    for (id, (_, _, expected)) in staged.iter().zip(cases) {
        assert_eq!(model.get(*id).expect("staged").type_name.as_ref(), expected);
    }
}
