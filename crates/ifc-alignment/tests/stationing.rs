//! Stationing per alignment, and station to distance-along lookup (#240).
//!
//! Each alignment's stationing comes from the referents it nests or
//! positions, never from every referent in the file. Lookups round-trip
//! across station equations, refuse a station that steps back onto an
//! earlier one as ambiguous, and follow decreasing stationing.

mod support;

use ifc_alignment::{AlignmentError, Stationing};
use support::{metres, Builder};

fn close(left: f64, right: f64) -> bool {
    (left - right).abs() <= 1e-9 * left.abs().max(right.abs()).max(1.0)
}

#[test]
fn two_alignments_yield_two_tables() {
    let mut b = Builder::new();
    let (main, spur) = (b.alignment("main"), b.alignment("spur"));
    let (main_curve, spur_curve) = (b.curve(), b.curve());
    let m0 = b.station(main_curve, 0.0, 1_000.0, None, None);
    let m1 = b.station(main_curve, 100.0, 1_200.0, Some(1_100.0), None);
    let s0 = b.station(spur_curve, 0.0, 0.0, None, None);
    let s1 = b.station(spur_curve, 40.0, 40.0, None, None);
    b.nest(main, &[m0, m1]);
    b.nest(spur, &[s0]);
    // A referent may also be related through IfcRelPositions.
    b.positions(spur, &[s1]);
    let model = b.finish();

    let main_table = Stationing::resolve(&model, main, metres()).expect("main");
    let spur_table = Stationing::resolve(&model, spur, metres()).expect("spur");
    let referents =
        |table: &Stationing| -> Vec<_> { table.equations().iter().map(|e| e.referent).collect() };
    assert_eq!(main_table.alignment, main);
    assert_eq!(referents(&main_table), vec![m0, m1]);
    assert_eq!(referents(&spur_table), vec![s0, s1]);
    assert_eq!(main_table.station_at(50.0), Ok(1_050.0));
    assert_eq!(spur_table.station_at(50.0), Ok(50.0));

    // The model-wide reader still works, and still mixes the two.
    #[allow(deprecated)]
    let mixed = ifc_alignment::station_equations(&model, metres()).expect("model-wide");
    assert_eq!(mixed.len(), 4);
}

/// A gap equation at 100 m: the back station 1100 is followed by 1200.
#[test]
fn station_and_distance_round_trip_across_an_equation() {
    let mut b = Builder::new();
    let road = b.alignment("road");
    let curve = b.curve();
    let r0 = b.station(curve, 0.0, 1_000.0, None, Some(true));
    let r1 = b.station(curve, 100.0, 1_200.0, Some(1_100.0), Some(true));
    b.nest(road, &[r0, r1]);
    let model = b.finish();
    let table = Stationing::resolve(&model, road, metres()).expect("resolves");

    for distance in [0.0, 12.5, 99.0, 100.5, 150.0, 1_000.0] {
        let station = table.station_at(distance).expect("station");
        let back = table.distance_at(station).expect("distance");
        assert!(close(back, distance), "{distance} -> {station} -> {back}");
    }
    assert_eq!(table.station_at(150.0), Ok(1_250.0));
    // At the equation the station ahead applies; the back station maps to
    // the same point.
    assert_eq!(table.station_at(100.0), Ok(1_200.0));
    assert_eq!(table.distance_at(1_100.0), Ok(100.0));
    assert_eq!(table.distance_at(1_200.0), Ok(100.0));
    // Stations inside the gap label no point.
    assert!(matches!(
        table.distance_at(1_150.0),
        Err(AlignmentError::OutOfRange {
            quantity: "station",
            ..
        })
    ));
}

/// An overlap equation at 100 m: 100 is followed by 80, so 80..100 occur
/// twice along the alignment.
#[test]
fn a_station_that_occurs_twice_is_ambiguous_not_guessed() {
    let mut b = Builder::new();
    let road = b.alignment("road");
    let curve = b.curve();
    let r0 = b.station(curve, 0.0, 0.0, None, None);
    let r1 = b.station(curve, 100.0, 80.0, Some(100.0), None);
    b.nest(road, &[r0, r1]);
    let model = b.finish();
    let table = Stationing::resolve(&model, road, metres()).expect("resolves");

    assert_eq!(table.distances_at(90.0), Ok(vec![90.0, 110.0]));
    assert!(matches!(
        table.distance_at(90.0),
        Err(AlignmentError::AmbiguousStation { station, ref distances, .. })
            if station == 90.0 && distances == &vec![90.0, 110.0]
    ));
    assert_eq!(table.distance_at(50.0), Ok(50.0));
    assert_eq!(table.distance_at(150.0), Ok(170.0));
}

#[test]
fn decreasing_stations_map_both_ways_across_an_equation() {
    let mut b = Builder::new();
    let road = b.alignment("road");
    let curve = b.curve();
    let r0 = b.station(curve, 0.0, 5_000.0, None, Some(false));
    let r1 = b.station(curve, 300.0, 4_600.0, Some(4_700.0), Some(false));
    b.nest(road, &[r0, r1]);
    let model = b.finish();
    let table = Stationing::resolve(&model, road, metres()).expect("resolves");

    assert_eq!(table.station_at(250.0), Ok(4_750.0));
    assert_eq!(table.station_at(350.0), Ok(4_550.0));
    assert_eq!(table.distance_at(4_750.0), Ok(250.0));
    assert_eq!(table.distance_at(4_550.0), Ok(350.0));
    for distance in [0.0, 10.0, 299.0, 301.0, 2_000.0] {
        let station = table.station_at(distance).expect("station");
        assert!(close(table.distance_at(station).expect("back"), distance));
    }
    // Above the start station there is nothing.
    assert!(table.distance_at(5_001.0).is_err());
}

#[test]
fn a_distance_before_the_first_referent_is_out_of_range() {
    let mut b = Builder::new();
    let road = b.alignment("road");
    let curve = b.curve();
    let r0 = b.station(curve, 10.0, 0.0, None, None);
    b.nest(road, &[r0]);
    let model = b.finish();
    let table = Stationing::resolve(&model, road, metres()).expect("resolves");
    assert!(matches!(
        table.station_at(9.0),
        Err(AlignmentError::OutOfRange { quantity: "distance along", entity, .. }) if entity == road
    ));
    assert!(table.station_at(f64::NAN).is_err());
    assert!(table.distance_at(-1.0).is_err());
}

#[test]
fn an_alignment_without_stationing_has_an_empty_table() {
    let mut b = Builder::new();
    let road = b.alignment("road");
    let curve = b.curve();
    // A referent without Pset_Stationing (a pier marker) is not stationing.
    let pier = b.marker(curve, 20.0);
    b.nest(road, &[pier]);
    let model = b.finish();
    let table = Stationing::resolve(&model, road, metres()).expect("resolves");
    assert!(table.equations().is_empty());
    assert!(table.station_at(20.0).is_err());
}

#[test]
fn contradictory_stations_are_refused() {
    let mut b = Builder::new();
    let (wrong_incoming, silent_jump, same_place) = (
        b.alignment("wrong incoming"),
        b.alignment("silent jump"),
        b.alignment("same place"),
    );
    let curve = b.curve();
    // 0 + 100 is 100, not the stated 90.
    let a0 = b.station(curve, 0.0, 0.0, None, None);
    let a1 = b.station(curve, 100.0, 200.0, Some(90.0), None);
    b.nest(wrong_incoming, &[a0, a1]);
    // A jump without IncomingStation is not an equation.
    let s0 = b.station(curve, 0.0, 0.0, None, None);
    let s1 = b.station(curve, 100.0, 200.0, None, None);
    b.nest(silent_jump, &[s0, s1]);
    let p0 = b.station(curve, 50.0, 0.0, None, None);
    let p1 = b.station(curve, 50.0, 0.0, None, None);
    b.nest(same_place, &[p0, p1]);
    let model = b.finish();

    for (alignment, referent) in [(wrong_incoming, a1), (silent_jump, s1), (same_place, p1)] {
        assert!(
            matches!(
                Stationing::resolve(&model, alignment, metres()),
                Err(AlignmentError::SemanticViolation { entity: Some(e), .. }) if e == referent
            ),
            "{alignment}"
        );
    }
}

#[test]
fn stationing_of_something_other_than_an_alignment_is_refused() {
    let mut b = Builder::new();
    let curve = b.curve();
    let marker = b.station(curve, 0.0, 0.0, None, None);
    let model = b.finish();
    assert!(matches!(
        Stationing::resolve(&model, marker, metres()),
        Err(AlignmentError::WrongType { entity, .. }) if entity == marker
    ));
}
