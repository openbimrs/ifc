//! Authored referents, resolved by the stationing reader.
//!
//! A referent is positioned through three entities: a point stated as
//! a distance along the basis curve, a linear axis placement at that
//! point, and the placement. The stationing reader walks that whole
//! chain, so it is what proves the authoring is wired correctly.

use ifc_alignment::{
    axis2_placement_linear, cartesian_point, linear_placement, point_by_distance, referent,
    station_equations, stationing, AlignmentUnits,
};
use ifc_model::{Entity, Model, Transaction, Value};

/// An authored referent resolves to its station.
#[test]
fn an_authored_referent_resolves_to_its_station() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let curve = tx.create(Entity::new("IFCPOLYLINE", vec![Value::List(vec![])]));
    let point = point_by_distance(&mut tx, 125.0, (None, None, None), curve).expect("point");
    let axis = axis2_placement_linear(&mut tx, point, None, None).expect("axis");
    let placement = linear_placement(&mut tx, axis, None, None).expect("placement");
    let marker = referent(
        &mut tx,
        "0aBcDeFgHiJkLmNoPqRsTu",
        Some("KM 0+125"),
        Some("STATION"),
        Some(placement),
    )
    .expect("referent");
    stationing(
        &mut tx,
        "1aBcDeFgHiJkLmNoPqRsTu",
        "2aBcDeFgHiJkLmNoPqRsTu",
        marker,
        125.0,
        None,
        Some(true),
    )
    .expect("stationing");
    tx.commit(&mut model).expect("commit");

    let units = AlignmentUnits {
        length_to_metres: 1.0,
        angle_to_radians: 1.0,
    };
    let stations = station_equations(&model, units).expect("resolves");
    assert_eq!(stations.len(), 1, "the authored referent is found");
    assert!(
        (stations[0].distance_along - 125.0).abs() < 1e-9,
        "distance along survives: {}",
        stations[0].distance_along,
    );
}

/// Positions and stations that parse but locate nothing are refused.
#[test]
fn meaningless_referent_positions_are_refused() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let curve = tx.create(Entity::new("IFCPOLYLINE", vec![Value::List(vec![])]));
    let g = "0aBcDeFgHiJkLmNoPqRsTu";
    let h = "1aBcDeFgHiJkLmNoPqRsTu";
    let marker = referent(&mut tx, g, None, None, None).expect("referent");

    // LIST [1:3]: no coordinates is not a point, four is not either.
    assert!(cartesian_point(&mut tx, &[]).is_err());
    assert!(cartesian_point(&mut tx, &[0.0, 0.0, 0.0, 0.0]).is_err());
    assert!(cartesian_point(&mut tx, &[f64::NAN]).is_err());

    // A distance that is not a number places the referent nowhere.
    assert!(point_by_distance(&mut tx, f64::NAN, (None, None, None), curve).is_err());
    assert!(point_by_distance(&mut tx, 1.0, (Some(f64::INFINITY), None, None), curve).is_err(),);

    // A malformed GlobalId cannot be referenced.
    assert!(referent(&mut tx, "short", None, None, None).is_err());
    assert!(stationing(&mut tx, "short", h, marker, 1.0, None, None).is_err());
    assert!(stationing(&mut tx, g, "short", marker, 1.0, None, None).is_err());

    // A station that is not a number is not a station.
    assert!(stationing(&mut tx, g, h, marker, f64::NAN, None, None).is_err());
    assert!(stationing(&mut tx, g, h, marker, 1.0, Some(f64::NAN), None).is_err());
}
