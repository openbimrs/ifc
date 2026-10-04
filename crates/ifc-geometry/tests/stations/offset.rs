//! `IfcOffsetCurveByDistances` (IFC4.3 ADD2 8.9.3.42) as
//! `CurveRelation::OffsetByStations`.

use axiolid_model::{CurveRelation, GeometryNode, Station, StationFrame};

use super::common::{families, lower, only, refused, root, step};

/// A straight 20 m 2D polyline, and one with a corner at 10 m.
const STRAIGHT: &str = "#10=IFCPOLYLINE((#11,#12));
#11=IFCCARTESIANPOINT((0.,0.));
#12=IFCCARTESIANPOINT((20.,0.));
#13=IFCPOLYLINE((#11,#14,#15));
#14=IFCCARTESIANPOINT((10.,0.));
#15=IFCCARTESIANPOINT((10.,10.));
#16=IFCLINE(#17,#18);
#17=IFCCARTESIANPOINT((0.,0.));
#18=IFCVECTOR(#19,1.);
#19=IFCDIRECTION((1.,0.));";

fn offset(id: u64, basis: u64, along: f64, lateral: f64, longitudinal: &str) -> String {
    format!(
        "#{id}=IFCPOINTBYDISTANCEEXPRESSION(IFCLENGTHMEASURE({along:?}),{lateral:?},$,\
         {longitudinal},#{basis});"
    )
}

fn stations(lowered: &ifc_geometry::lower::LoweredGeometry) -> Vec<Station> {
    let GeometryNode::CurveRelation(CurveRelation::OffsetByStations {
        stations, frame, ..
    }) = root(lowered)
    else {
        panic!("OffsetByStations: {:?}", root(lowered));
    };
    assert_eq!(*frame, StationFrame::Section);
    stations.clone()
}

fn assert_runs(actual: &[Station], expected: &[(f64, f64)]) {
    let got: Vec<(f64, f64)> = actual
        .iter()
        .map(|s| (s.distance, s.offsets.lateral))
        .collect();
    assert_eq!(got, expected);
    assert!(actual
        .iter()
        .all(|s| s.offsets.vertical == 0.0 && s.offsets.longitudinal == 0.0));
}

/// The committed kerb: 2 m left at 0, 2.5 m at 40, which spans the plan.
#[test]
fn the_fixture_kerb_is_its_stations() {
    let model = families();
    let lowered = lower(&model, only(&model, "IFCOFFSETCURVEBYDISTANCES")).expect("lowers");
    assert_runs(&stations(&lowered), &[(0.0, 2.0), (40.0, 2.5)]);
}

/// "If the offsets do not span the full extent of the basis curve (e.g. if
/// the list contains only one item), then the lateral and vertical offsets
/// implicitly continue with the same value" (8.9.3.42.3): stated stations
/// carry it to both ends. A zero OffsetLongitudinal is no offset.
#[test]
fn offsets_short_of_the_ends_continue_unchanged_to_them() {
    let records = format!(
        "{STRAIGHT}
{}
#30=IFCOFFSETCURVEBYDISTANCES(#10,(#20),'one');
{}
{}
#31=IFCOFFSETCURVEBYDISTANCES(#10,(#21,#22),$);",
        offset(20, 10, 10.0, 1.0, "$"),
        offset(21, 10, 5.0, 1.0, "0."),
        offset(22, 10, 15.0, 3.0, "$"),
    );
    let model = step(&records, false);
    assert_runs(
        &stations(&lower(&model, 30).expect("one offset")),
        &[(0.0, 1.0), (10.0, 1.0), (20.0, 1.0)],
    );
    assert_runs(
        &stations(&lower(&model, 31).expect("two offsets")),
        &[(0.0, 1.0), (5.0, 1.0), (15.0, 3.0), (20.0, 3.0)],
    );
}

#[test]
fn offset_curves_ifc_and_axiolid_read_differently_are_refused_by_name() {
    let records = format!(
        "{STRAIGHT}
{}
#30=IFCOFFSETCURVEBYDISTANCES(#16,(#20),$);
{}
#31=IFCOFFSETCURVEBYDISTANCES(#10,(#21),$);
{}
#32=IFCOFFSETCURVEBYDISTANCES(#10,(#22),$);
{}
{}
#33=IFCOFFSETCURVEBYDISTANCES(#10,(#23,#24),$);
{}
{}
#34=IFCOFFSETCURVEBYDISTANCES(#13,(#25,#26),$);
{}
#35=IFCOFFSETCURVEBYDISTANCES(#10,(#27),$);",
        offset(20, 16, 3.0, 1.0, "$"),
        offset(21, 10, 3.0, 1.0, "0.2"),
        offset(22, 13, 3.0, 1.0, "$"),
        offset(23, 10, 8.0, 1.0, "$"),
        offset(24, 10, 8.0, 2.0, "$"),
        offset(25, 13, 2.0, 1.0, "$"),
        offset(26, 13, 8.0, 1.0, "$"),
        offset(27, 10, 25.0, 1.0, "$"),
    );
    let model = step(&records, false);
    // An unbounded line: nowhere to continue the offsets to.
    refused(lower(&model, 30), true, "states no length");
    refused(lower(&model, 31), true, "OffsetLongitudinal");
    refused(lower(&model, 32), true, "another BasisCurve");
    refused(lower(&model, 33), false, "increase strictly");
    // Carried to the ends, the run crosses the corner at 10 m.
    refused(lower(&model, 34), true, "tangent discontinuity");
    refused(lower(&model, 35), false, "beyond the basis curve's length");
}
