//! `IfcPointByDistanceExpression` and `IfcAxis2PlacementLinear` (IFC4.3
//! ADD2 8.9.3.48, 8.9.3.4) as `CurveStation` and `OrientedCurveStation`.

use axiolid_model::{GeometryNode, StationFrame};
use ifc_geometry::lower::{lower_point_by_distance_node, LoweringSession};
use ifc_geometry::transform::Transform;
use ifc_geometry::units;
use ifc_model::EntityId;

use super::common::{
    close_point, close_vec, families, layout, lower, only, refused, root, section, step, EPS,
};

/// A 2D polyline turning left at (10, 0): 10 m east, then 10 m north.
const L_SHAPE: &str = "#10=IFCPOLYLINE((#11,#12,#13));
#11=IFCCARTESIANPOINT((0.,0.));
#12=IFCCARTESIANPOINT((10.,0.));
#13=IFCCARTESIANPOINT((10.,10.));";

/// The same polyline in millimetres.
const L_SHAPE_MM: &str = "#10=IFCPOLYLINE((#11,#12,#13));
#11=IFCCARTESIANPOINT((0.,0.));
#12=IFCCARTESIANPOINT((10000.,0.));
#13=IFCCARTESIANPOINT((10000.,10000.));";

/// An `IfcPointByDistanceExpression` record; `$` for an absent offset.
fn point(id: u64, along: &str, lateral: &str, vertical: &str, longitudinal: &str) -> String {
    format!("#{id}=IFCPOINTBYDISTANCEEXPRESSION({along},{lateral},{vertical},{longitudinal},#10);")
}

fn length(value: f64) -> String {
    format!("IFCLENGTHMEASURE({value:?})")
}

/// IFC: lateral positive to the LEFT facing along the curve, vertical
/// perpendicular to the tangent in the vertical plane, longitudinal along
/// the tangent. The stored station is the authored data in metres; the
/// evaluator places it where IFC says.
#[test]
fn a_point_by_distance_is_its_station_and_resolves_where_ifc_places_it() {
    for (millimetres, scale, basis) in [(false, 1.0, L_SHAPE), (true, 1000.0, L_SHAPE_MM)] {
        let records = format!(
            "{basis}\n{}\n{}",
            point(
                20,
                &length(4.0 * scale),
                &format!("{:?}", 1.5 * scale),
                &format!("{:?}", 2.0 * scale),
                &format!("{:?}", 0.5 * scale)
            ),
            point(
                21,
                &length(14.0 * scale),
                &format!("{:?}", 1.0 * scale),
                "$",
                "$"
            ),
        );
        let model = step(&records, millimetres);

        let first = lower(&model, 20).expect("a station on the first leg");
        let GeometryNode::CurveStation(station) = root(&first) else {
            panic!("a CurveStation: {:?}", root(&first));
        };
        assert_eq!(station.frame, StationFrame::Section);
        assert!((station.station.distance - 4.0).abs() < EPS, "{station:?}");
        assert!((station.station.offsets.lateral - 1.5).abs() < EPS);
        assert!((station.station.offsets.vertical - 2.0).abs() < EPS);
        assert!((station.station.offsets.longitudinal - 0.5).abs() < EPS);
        // Heading east: left is +Y, up is +Z, along is +X.
        let frame = section(&first.graph, station.basis, 4.0);
        close_point(frame.place(1.5, 2.0, 0.5), [4.5, 1.5, 2.0], "first leg");

        // Heading north after the corner: left is -X.
        let second = lower(&model, 21).expect("a station on the second leg");
        let GeometryNode::CurveStation(station) = root(&second) else {
            panic!("a CurveStation");
        };
        assert_eq!(
            station.station.offsets.vertical, 0.0,
            "an absent offset is zero"
        );
        let frame = section(&second.graph, station.basis, station.station.distance);
        close_point(frame.place(1.0, 0.0, 0.0), [9.0, 4.0, 0.0], "second leg");
    }
}

/// On an `IfcGradientCurve` DistanceAlong is the BaseCurve's parameter
/// (8.9.3.34.1), the plan arc length: Axiolid's plan distance on
/// `Curve3::Elevated`. The fixture's plan runs along +X, so the station at
/// 20 stands at x = 20 although the curve climbs; OffsetVertical is
/// perpendicular to the climbing tangent in its vertical plane.
#[test]
fn on_a_gradient_curve_the_distance_is_plan_distance() {
    let mut model = families();
    let gradient = only(&model, "IFCGRADIENTCURVE");
    let records =
        format!("#900000=IFCPOINTBYDISTANCEEXPRESSION(IFCLENGTHMEASURE(20.),$,1.,$,#{gradient});");
    for (id, entity) in step(&records, false).iter() {
        if id.0 >= 900_000 {
            model.insert(id, entity.clone());
        }
    }
    let lowered = lower(&model, 900_000).expect("a station on the gradient curve");
    let GeometryNode::CurveStation(station) = root(&lowered) else {
        panic!("a CurveStation");
    };
    assert!(matches!(
        lowered.graph.get(station.basis),
        Some(GeometryNode::Curve3(axiolid_curve::Curve3::Elevated(_)))
    ));
    let frame = section(&lowered.graph, station.basis, 20.0);
    assert!(
        (frame.point.x - 20.0).abs() < EPS,
        "plan distance: {:?}",
        frame.point
    );
    assert!(frame.point.y.abs() < EPS);
    let lifted = frame.place(0.0, 1.0, 0.0) - frame.point;
    assert!(
        lifted.dot(frame.tangent).abs() < EPS,
        "perpendicular to the tangent"
    );
    assert!(
        lifted.y.abs() < EPS && lifted.z > 0.0,
        "in the vertical plane, upwards"
    );
    assert!(frame.tangent.z > 0.01, "the curve climbs, so up leans back");
    assert!(lifted.x < -0.01, "{lifted:?}");
}

/// `Axis` and `RefDirection` are components in the curve's (tangent, left,
/// up) frame, Axis exact (8.9.3.4). A roll of the axis by phi about the
/// tangent turns profile X (local Y) up by phi; a RefDirection turned by a
/// in plan turns the frame's X with it.
#[test]
fn a_linear_placement_reads_its_axes_in_the_curve_frame() {
    let (phi, a) = (0.1_f64, 0.3_f64);
    let records = format!(
        "{L_SHAPE}
{}
#30=IFCAXIS2PLACEMENTLINEAR(#20,#31,$);
#31=IFCDIRECTION((0.,{:?},{:?}));
#32=IFCAXIS2PLACEMENTLINEAR(#20,#33,#34);
#33=IFCDIRECTION((0.,0.,1.));
#34=IFCDIRECTION(({:?},{:?},0.));
#35=IFCAXIS2PLACEMENTLINEAR(#20,$,$);",
        point(20, &length(4.0), "$", "$", "$"),
        -phi.sin(),
        phi.cos(),
        a.cos(),
        a.sin(),
    );
    let model = step(&records, false);

    let rolled = lower(&model, 30).expect("an axis alone");
    let GeometryNode::OrientedCurveStation(oriented) = root(&rolled) else {
        panic!("an OrientedCurveStation: {:?}", root(&rolled));
    };
    let axis = oriented.orientation.axis.expect("the axis passes through");
    close_vec(axis, [0.0, -phi.sin(), phi.cos()], "axis ratios unchanged");
    assert_eq!(oriented.orientation.ref_direction, None);
    let base = section(&rolled.graph, oriented.station.basis, 4.0);
    let turned = base
        .oriented(
            oriented.orientation.axis,
            oriented.orientation.ref_direction,
        )
        .expect("orients");
    close_vec(turned.up, [0.0, -phi.sin(), phi.cos()], "local Z is Axis");
    close_vec(turned.tangent, [1.0, 0.0, 0.0], "local X stays the tangent");
    close_vec(
        turned.lateral,
        [0.0, phi.cos(), phi.sin()],
        "local Y = Z x X",
    );

    let turned_in_plan = lower(&model, 32).expect("both axes");
    let GeometryNode::OrientedCurveStation(oriented) = root(&turned_in_plan) else {
        panic!("an OrientedCurveStation");
    };
    let base = section(&turned_in_plan.graph, oriented.station.basis, 4.0);
    let turned = base
        .oriented(
            oriented.orientation.axis,
            oriented.orientation.ref_direction,
        )
        .expect("orients");
    close_vec(
        turned.tangent,
        [a.cos(), a.sin(), 0.0],
        "local X is RefDirection",
    );
    // Section profile X = Axis x RefDirection (#344).
    close_vec(
        turned.lateral,
        [-a.sin(), a.cos(), 0.0],
        "profile X = Axis x RefDirection",
    );

    let plain = lower(&model, 35).expect("no axes");
    let GeometryNode::OrientedCurveStation(oriented) = root(&plain) else {
        panic!("an OrientedCurveStation");
    };
    assert!(
        oriented.orientation.is_base(),
        "absent axes are the curve frame"
    );
}

/// The committed layout's referent placement: Axis (1, 0, 0), RefDirection
/// (0, 1, 0), 5 m along an `IfcLine`, 0.5 m left.
#[test]
fn the_layout_fixture_placement_lowers_with_its_axes() {
    let model = layout();
    let lowered = lower(&model, only(&model, "IFCAXIS2PLACEMENTLINEAR")).expect("lowers");
    let GeometryNode::OrientedCurveStation(oriented) = root(&lowered) else {
        panic!("an OrientedCurveStation");
    };
    close_vec(
        oriented.orientation.axis.expect("axis"),
        [1.0, 0.0, 0.0],
        "axis",
    );
    close_vec(
        oriented.orientation.ref_direction.expect("ref"),
        [0.0, 1.0, 0.0],
        "ref",
    );
    assert_eq!(oriented.station.station.distance, 5.0);
    assert_eq!(oriented.station.station.offsets.lateral, 0.5);
}

/// Every refusal names its reason.
#[test]
fn stations_ifc_and_axiolid_cannot_share_are_refused_by_name() {
    let records = format!(
        "{L_SHAPE}
{}
{}
{}
{}
{}
{}
{}
#30=IFCAXIS2PLACEMENTLINEAR(#27,#31,#32);
#31=IFCDIRECTION((0.,0.,1.));
#32=IFCDIRECTION((0.,0.,-2.));
#33=IFCAXIS2PLACEMENTLINEAR(#27,#34,$);
#34=IFCDIRECTION((0.,1.));
#40=IFCLINE(#41,#42);
#41=IFCCARTESIANPOINT((0.,0.,0.));
#42=IFCVECTOR(#43,1.);
#43=IFCDIRECTION((1.,0.,0.));
#44=IFCTRIMMEDCURVE(#40,(IFCPARAMETERVALUE(0.)),(IFCPARAMETERVALUE(5.)),.T.,.PARAMETER.);
#45=IFCPOINTBYDISTANCEEXPRESSION(IFCLENGTHMEASURE(1.),$,$,$,#44);
#46=IFCPOINTBYDISTANCEEXPRESSION(IFCLENGTHMEASURE(1000.),$,$,$,#40);",
        point(20, "IFCPARAMETERVALUE(0.5)", "$", "$", "$"),
        point(21, &length(10.0), "$", "$", "$"),
        point(22, &length(20.5), "$", "$", "$"),
        point(23, &length(-1.0), "$", "$", "$"),
        point(24, "4.", "$", "$", "$"),
        point(26, &length(10.000001), "$", "$", "$"),
        point(27, &length(4.0), "$", "$", "$"),
    );
    let model = step(&records, false);
    refused(lower(&model, 20), true, "IfcParameterValue");
    // At the corner, and within the declared precision (1E-5) of it, the
    // incoming tangent governs (8.9.3.48.3; #346, `seams.rs`).
    for id in [21, 26] {
        let lowered = lower(&model, id).expect("on the corner");
        let GeometryNode::OrientedCurveStation(station) = root(&lowered) else {
            panic!("an OrientedCurveStation reading the incoming side");
        };
        assert_eq!(station.seam, axiolid_model::SeamSide::Incoming);
        assert_eq!(station.station.station.distance, 10.0);
    }
    refused(lower(&model, 22), false, "beyond the basis curve's length");
    refused(lower(&model, 23), false, "before the basis curve's start");
    refused(lower(&model, 24), false, "typed IfcLengthMeasure");
    refused(lower(&model, 30), false, "WR2");
    refused(lower(&model, 33), false, "direction ratios");
    // A trimmed line is a curve relation, measured along its piece (#346).
    let trimmed = lower(&model, 45).expect("a station along a trimmed curve");
    let GeometryNode::CurveStation(station) = root(&trimmed) else {
        panic!("a CurveStation: {:?}", root(&trimmed));
    };
    assert_eq!(station.station.distance, 1.0);
    // An unbounded line has no end to be beyond.
    lower(&model, 46).expect("a line is unbounded");

    // A scaled frame changes the measure along the curve.
    let scale = units::resolve(&model);
    let mut session = LoweringSession::new(&model, &scale);
    let mut frame = Transform::identity();
    frame.basis = [[2.0, 0.0, 0.0], [0.0, 2.0, 0.0], [0.0, 0.0, 2.0]];
    let error = lower_point_by_distance_node(&mut session, EntityId(46), frame)
        .expect_err("a scaled frame");
    assert!(
        error.is_unsupported() && error.to_string().contains("rigid"),
        "{error}"
    );
}

/// A station clear of the corner by more than the precision is a plain
/// station at its own distance.
#[test]
fn a_station_just_clear_of_a_corner_lowers() {
    let records = format!("{L_SHAPE}\n{}", point(20, &length(10.001), "$", "$", "$"));
    let model = step(&records, false);
    let lowered = lower(&model, 20).expect("clear of the corner");
    let GeometryNode::CurveStation(station) = root(&lowered) else {
        panic!("a CurveStation");
    };
    assert!((station.station.distance - 10.001).abs() < EPS);
}
