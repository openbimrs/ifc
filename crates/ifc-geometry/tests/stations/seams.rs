//! Stations on tangent discontinuities and runs across them (#346).
//!
//! IFC4.3 ADD2 8.9.3.48.3: "If DistanceAlong coincides with a point of
//! tangential discontinuity (within precision limits), then the tangent of
//! the previous segment governs." 8.8.3.35.1: "If the directrix is not
//! tangent continuous, the resulting solid is created by a miter at half
//! angle between the two segments"; 8.8.3.37.1 says the same of the
//! surface. The fixture
//! is `synthetic-lowering/station_seams_ifc4x3.ifc` (its generator's
//! docstring states every case); the refusals are inline.

use axiolid_curve::Curve3;
use axiolid_model::{
    CurveRelation, GeometryNode, OrientedCurveStation, SeamSide, SolidOperation, SurfaceRelation,
};
use axiolid_reference::station::station_section3_on;

use super::common::{assert_windowed, close_point, close_vec, lower, refused, root, step, EPS};

/// The committed station-seam fixture (#346).
pub fn seams() -> ifc_model::Model {
    use ifc_model::Codec;
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../test/fixtures/synthetic-lowering/station_seams_ifc4x3.ifc");
    ifc_step::StepCodec
        .read_path(&path)
        .unwrap_or_else(|e| panic!("{}: {e:?}", path.display()))
}

/// The items of the representation `identifier` of the proxy `name`.
pub fn items(model: &ifc_model::Model, name: &str, identifier: &str) -> Vec<u64> {
    let proxy = model
        .ids_of_type("IFCBUILDINGELEMENTPROXY")
        .iter()
        .copied()
        .find(|id| model.get(*id).and_then(|e| e.text(2)) == Some(name))
        .unwrap_or_else(|| panic!("no proxy {name}"));
    let shape = model
        .get(proxy)
        .and_then(|e| e.reference(6))
        .expect("shape");
    let representations = model
        .get(shape)
        .and_then(|e| e.attribute(2))
        .and_then(|v| v.as_list())
        .expect("representations");
    representations
        .iter()
        .filter_map(|r| r.as_ref_id())
        .filter_map(|r| model.get(r))
        .find(|r| r.text(1) == Some(identifier))
        .and_then(|r| r.attribute(3))
        .and_then(|v| v.as_list())
        .expect("items")
        .iter()
        .filter_map(|v| v.as_ref_id())
        .map(|id| id.0)
        .collect()
}

/// The only item of `identifier` on `name`.
pub fn item(model: &ifc_model::Model, name: &str, identifier: &str) -> u64 {
    let items = items(model, name, identifier);
    assert_eq!(items.len(), 1, "{name}/{identifier}: {items:?}");
    items[0]
}

/// The lowered point of proxy `name`: an oriented station reading the
/// incoming side, stored at the seam's own distance.
fn on_seam(model: &ifc_model::Model, name: &str, seam: f64) -> (Curve3, OrientedCurveStation) {
    let lowered = lower(model, item(model, name, "Reference")).expect(name);
    let GeometryNode::OrientedCurveStation(station) = root(&lowered) else {
        panic!("{name}: an OrientedCurveStation: {:?}", root(&lowered));
    };
    assert_eq!(station.seam, SeamSide::Incoming, "{name}");
    assert!(station.orientation.is_base(), "{name}: unturned");
    assert_eq!(
        station.station.station.distance, seam,
        "{name}: at the seam"
    );
    let Some(GeometryNode::Curve3(curve)) = lowered.graph.get(station.station.basis) else {
        panic!("{name}: an atomic basis");
    };
    (curve.clone(), *station)
}

/// The grade breaks from 0.02 to -0.01 at 40 m (height 10.8): the point 2 m
/// up stands perpendicular to the INCOMING grade, at the break and 4 um
/// before it (inside the 1e-5 m precision).
#[test]
fn a_station_on_a_grade_break_takes_the_previous_grade() {
    let model = seams();
    let norm = 1.0004_f64.sqrt();
    let expected = [40.0 - 2.0 * 0.02 / norm, 0.0, 10.8 + 2.0 / norm];
    for name in ["GRADE_BREAK_AT", "GRADE_BREAK_NEAR"] {
        let (curve, station) = on_seam(&model, name, 40.0);
        let offsets = station.station.station.offsets;
        assert_eq!((offsets.lateral, offsets.vertical), (0.0, 2.0));
        let frame = station_section3_on(&curve, 40.0, station.seam).expect("resolves");
        close_vec(
            frame.tangent,
            [1.0 / norm, 0.0, 0.02 / norm],
            "the incoming tangent",
        );
        close_point(frame.place(0.0, 2.0, 0.0), expected, name);
        // The outgoing grade would lean the offset the other way.
        let outgoing = station_section3_on(&curve, 40.0, SeamSide::Outgoing).expect("resolves");
        assert!(outgoing.place(0.0, 2.0, 0.0).x > 40.0, "{name}");
    }
}

/// The L turns left at 10 m. 1 m left and 0.5 m along the incoming leg is
/// (10.5, 1, 0); the outgoing leg would put it at (9, 0.5, 0).
#[test]
fn a_station_on_a_corner_takes_the_incoming_leg() {
    let model = seams();
    for name in ["CORNER_AT", "CORNER_NEAR"] {
        let (curve, station) = on_seam(&model, name, 10.0);
        let frame = station_section3_on(&curve, 10.0, station.seam).expect("resolves");
        close_vec(frame.tangent, [1.0, 0.0, 0.0], "east");
        close_point(frame.place(1.0, 0.0, 0.5), [10.5, 1.0, 0.0], name);
    }
}

/// A run's stations keep their distances; the kernel mitres between them.
/// The deck spans the corner (5 m to 15 m); the carriageway the grade
/// break (30 m to 50 m); the kerb is carried to both ends of the L.
#[test]
fn runs_across_a_seam_lower_for_the_kernel_to_mitre() {
    let model = seams();
    let deck = lower(&model, item(&model, "DECK", "Body")).expect("the deck lowers");
    let GeometryNode::SolidOperation(SolidOperation::SectionsAtStations { sections, .. }) =
        root(&deck)
    else {
        panic!("SectionsAtStations: {:?}", root(&deck));
    };
    let at: Vec<f64> = sections.iter().map(|s| s.station.distance).collect();
    assert_eq!(at, [5.0, 15.0]);

    let surface = lower(&model, item(&model, "CARRIAGEWAY", "Body")).expect("lowers");
    let GeometryNode::SurfaceRelation(SurfaceRelation::OpenSectionsAtStations { sections, .. }) =
        root(&surface)
    else {
        panic!("OpenSectionsAtStations: {:?}", root(&surface));
    };
    let at: Vec<f64> = sections.iter().map(|s| s.station.distance).collect();
    assert_eq!(at, [30.0, 50.0]);

    let kerb = lower(&model, item(&model, "KERB", "Axis")).expect("the kerb lowers");
    let GeometryNode::CurveRelation(CurveRelation::OffsetByStations { stations, .. }) = root(&kerb)
    else {
        panic!("OffsetByStations: {:?}", root(&kerb));
    };
    let at: Vec<(f64, f64)> = stations
        .iter()
        .map(|s| (s.distance, s.offsets.lateral))
        .collect();
    assert_eq!(at, [(0.0, 1.0), (2.0, 1.0), (18.0, 1.0), (20.0, 1.0)]);
}

/// A 3D polyline with a left corner at 10 m and one that turns back on
/// itself at 10 m, a straight one, two rectangles.
const BASES: &str = "#10=IFCPOLYLINE((#11,#12,#13));
#11=IFCCARTESIANPOINT((0.,0.,0.));
#12=IFCCARTESIANPOINT((10.,0.,0.));
#13=IFCCARTESIANPOINT((10.,10.,0.));
#14=IFCPOLYLINE((#11,#12,#15));
#15=IFCCARTESIANPOINT((5.,0.,0.));
#40=IFCRECTANGLEPROFILEDEF(.AREA.,'a',#43,2.,1.);
#43=IFCAXIS2PLACEMENT2D(#44,$);
#44=IFCCARTESIANPOINT((0.,0.));";

fn position(id: u64, basis: u64, along: f64) -> String {
    format!(
        "#{id}=IFCAXIS2PLACEMENTLINEAR(#{},$,$);\n\
         #{}=IFCPOINTBYDISTANCEEXPRESSION(IFCLENGTHMEASURE({along:?}),$,$,$,#{basis});",
        id + 1,
        id + 1
    )
}

/// A linear placement on a seam reads the incoming side too, with its
/// orientation kept; within the precision (1e-5) it is snapped there.
#[test]
fn a_linear_placement_on_a_corner_reads_the_incoming_leg() {
    let records = format!(
        "{BASES}\n{}\n{}\n#24=IFCAXIS2PLACEMENTLINEAR(#21,#25,$);\n#25=IFCDIRECTION((0.,0.,1.));",
        position(20, 10, 10.0),
        position(22, 10, 10.000004),
    );
    let model = step(&records, false);
    for id in [20, 22, 24] {
        let lowered = lower(&model, id).expect("on the corner");
        let GeometryNode::OrientedCurveStation(station) = root(&lowered) else {
            panic!("an OrientedCurveStation");
        };
        assert_eq!(station.seam, SeamSide::Incoming, "#{id}");
        assert_eq!(station.station.station.distance, 10.0, "#{id}");
    }
    let lowered = lower(&model, 24).expect("oriented");
    let GeometryNode::OrientedCurveStation(station) = root(&lowered) else {
        panic!("an OrientedCurveStation");
    };
    assert!(station.orientation.axis.is_some(), "the axis is kept");
    // Clear of the corner by more than the precision: the plain station.
    let clear = step(&format!("{BASES}\n{}", position(20, 10, 10.001)), false);
    let lowered = lower(&clear, 20).expect("clear");
    let GeometryNode::OrientedCurveStation(station) = root(&lowered) else {
        panic!("an OrientedCurveStation");
    };
    assert_eq!(station.seam, SeamSide::Outgoing);
    assert!((station.station.station.distance - 10.001).abs() < EPS);
}

/// A section within precision of the corner is stored at it, where the
/// kernel stands it in the mitre plane; two that collapse onto it are out
/// of order.
#[test]
fn a_section_within_precision_of_a_corner_is_stored_at_it() {
    let records = format!(
        "{BASES}\n{}\n{}\n{}\n{}\n\
         #30=IFCSECTIONEDSOLIDHORIZONTAL(#10,(#40,#40,#40),(#20,#22,#24));\n\
         #31=IFCSECTIONEDSOLIDHORIZONTAL(#10,(#40,#40),(#22,#26));",
        position(20, 10, 5.0),
        position(22, 10, 9.999996),
        position(24, 10, 15.0),
        position(26, 10, 10.000004),
    );
    let model = step(&records, false);
    let lowered = lower(&model, 30).expect("lowers");
    let GeometryNode::SolidOperation(SolidOperation::SectionsAtStations { sections, .. }) =
        root(&lowered)
    else {
        panic!("SectionsAtStations");
    };
    let at: Vec<f64> = sections.iter().map(|s| s.station.distance).collect();
    assert_eq!(at, [5.0, 10.0, 15.0]);
    refused(lower(&model, 31), false, "strictly increasing");
}

/// What stays refused, by name: a run across a seam where the curve turns
/// back on itself (no mitre plane). A composite's joint, refused until
/// Axiolid measured stations along curve relations (axiolid/kernel#285),
/// is a seam like a corner. A station on a B-spline with a corner knot,
/// whose distance is a quadrature, carries the kernel's seam-snapping
/// window (#423; refused by name before).
#[test]
fn reversals_and_unreadable_seams_are_refused_by_name_and_joints_lower() {
    let records = format!(
        "{BASES}\n{}\n{}\n\
         #30=IFCSECTIONEDSOLIDHORIZONTAL(#14,(#40,#40),(#20,#22));\n\
         #31=IFCOFFSETCURVEBYDISTANCES(#14,(#33),$);\n\
         #33=IFCPOINTBYDISTANCEEXPRESSION(IFCLENGTHMEASURE(2.),1.,$,$,#14);\n\
         #50=IFCCOMPOSITECURVE((#51,#52),.F.);\n\
         #51=IFCCOMPOSITECURVESEGMENT(.CONTINUOUS.,.T.,#53);\n\
         #52=IFCCOMPOSITECURVESEGMENT(.CONTINUOUS.,.T.,#54);\n\
         #53=IFCPOLYLINE((#11,#12));\n\
         #54=IFCPOLYLINE((#12,#13));\n\
         #55=IFCPOINTBYDISTANCEEXPRESSION(IFCLENGTHMEASURE(4.),$,$,$,#50);\n\
         #56=IFCPOINTBYDISTANCEEXPRESSION(IFCLENGTHMEASURE(10.),$,$,$,#50);\n\
         #60=IFCBSPLINECURVEWITHKNOTS(1,(#11,#12,#13),.UNSPECIFIED.,.F.,.F.,(2,1,2),(0.,1.,2.),.UNSPECIFIED.);\n\
         #61=IFCPOINTBYDISTANCEEXPRESSION(IFCLENGTHMEASURE(4.),$,$,$,#60);",
        position(20, 14, 5.0),
        position(22, 14, 12.0),
    );
    let model = step(&records, false);
    refused(lower(&model, 30), true, "turns back on itself");
    refused(lower(&model, 31), true, "turns back on itself");
    // A composite's joint is a seam like a polyline corner (#346).
    let inside = lower(&model, 55).expect("a station inside a piece");
    assert!(matches!(root(&inside), GeometryNode::CurveStation(_)));
    let joint = lower(&model, 56).expect("a station on a joint");
    let GeometryNode::OrientedCurveStation(station) = root(&joint) else {
        panic!("an OrientedCurveStation: {:?}", root(&joint));
    };
    assert_eq!(station.seam, SeamSide::Incoming);
    assert_eq!(station.station.station.distance, 10.0);
    assert_windowed(&lower(&model, 61).expect("lowers"), 4.0, 1e-5);
}
