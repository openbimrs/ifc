//! Stations along curve relations (#346) and segments placed at stations
//! (#311).
//!
//! Axiolid measures a station along a composite, a trim and curves placed
//! at stations since `axiolid-model` 0.3.7 (axiolid/kernel#285): end to end
//! through the pieces, every joint a seam. IFC4.3 ADD2 8.9.3.48.3 reads the
//! previous segment there, so a station within precision of a joint is
//! stored at the joint reading `SeamSide::Incoming`. An `IfcCurveSegment`
//! placed by an `IfcAxis2PlacementLinear` lowers to an `InstanceAtStation`.
//! The fixture is `synthetic-lowering/station_relations_ifc4x3.ifc` (its
//! generator's docstring states every case). With the
//! `compile-reference-backend` feature the lowered stations are resolved
//! through the reference kernel, which evaluates the composite basis
//! (`axiolid_reference::station::CompositeBasis`).

use axiolid_model::{CurveRelation, GeometryNode, OrientedCurveStation, SeamSide};
use ifc_model::Model;

use super::common::{lower, refused, root, step};
use super::seams::item;

/// `sqrt(1 + 0.02^2)`: the kerb's 3D length per metre of plan.
fn norm() -> f64 {
    1.0f64.hypot(0.02)
}

/// The committed station-relation fixture (#346, #311).
pub fn relations() -> Model {
    use ifc_model::Codec;
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../test/fixtures/synthetic-lowering/station_relations_ifc4x3.ifc");
    ifc_step::StepCodec
        .read_path(&path)
        .unwrap_or_else(|e| panic!("{}: {e:?}", path.display()))
}

/// The lowered point of proxy `name`, which must stand on a joint.
fn on_joint(model: &Model, name: &str, joint: f64) -> OrientedCurveStation {
    let lowered = lower(model, item(model, name, "Reference")).expect(name);
    let GeometryNode::OrientedCurveStation(station) = root(&lowered) else {
        panic!("{name}: an OrientedCurveStation: {:?}", root(&lowered));
    };
    assert_eq!(station.seam, SeamSide::Incoming, "{name}");
    assert!(station.orientation.is_base(), "{name}: unturned");
    assert_eq!(
        station.station.station.distance, joint,
        "{name}: on the joint"
    );
    let basis = lowered.graph.get(station.station.basis);
    assert!(
        matches!(
            basis,
            Some(GeometryNode::CurveRelation(CurveRelation::Composite { .. }))
        ),
        "{name}: a composite basis: {basis:?}"
    );
    *station
}

/// On the corner joint of the plain composite (10 m), and 4 um past it
/// inside the 1e-5 m precision, the station stands on the joint reading
/// the previous segment; so does the tangent-continuous joint at 20 m.
#[test]
fn a_station_on_a_composite_joint_reads_the_previous_segment() {
    let model = relations();
    for (name, joint) in [
        ("PATH_CORNER_AT", 10.0),
        ("PATH_CORNER_NEAR", 10.0),
        ("PATH_TANGENT_JOINT", 20.0),
    ] {
        let station = on_joint(&model, name, joint);
        assert_eq!(station.station.station.offsets.lateral, 1.0, "{name}");
    }
    // Off a joint the station is plain, at its own distance.
    let arc = lower(&model, item(&model, "PATH_ARC", "Reference")).expect("on the arc");
    let GeometryNode::CurveStation(station) = root(&arc) else {
        panic!("a CurveStation: {:?}", root(&arc));
    };
    let quarter = 10.0 * std::f64::consts::FRAC_PI_2;
    assert_eq!(station.station.distance, 20.0 + quarter / 2.0);
}

/// The kerb's segments are placed at stations 10 and 30 of the gradient
/// curve, and a station along the kerb is measured in their own arc
/// length: its joint lies at `20 sqrt(1.0004)`.
#[test]
fn segments_placed_at_stations_lower_to_instances_at_stations() {
    let model = relations();
    let kerb = lower(&model, item(&model, "KERB", "Axis")).expect("the kerb lowers");
    let GeometryNode::CurveRelation(CurveRelation::Composite { segments }) = root(&kerb) else {
        panic!("a composite: {:?}", root(&kerb));
    };
    let stations: Vec<f64> = segments
        .iter()
        .map(|segment| match kerb.graph.get(segment.curve) {
            Some(GeometryNode::InstanceAtStation(placed)) => {
                let offsets = placed.station.station.station.offsets;
                assert_eq!((offsets.lateral, offsets.vertical), (3.0, 0.0));
                placed.station.station.station.distance
            }
            other => panic!("an InstanceAtStation: {other:?}"),
        })
        .collect();
    assert_eq!(stations, [10.0, 30.0]);
    on_joint(&model, "KERB_JOINT", 20.0 * norm());
}

/// What a relation basis cannot carry is refused by name: an ellipse
/// piece, whose length is a quadrature, puts the joints where no
/// precision can find them; Axiolid measures no station along an offset
/// curve.
#[test]
fn relation_bases_without_stated_joints_are_refused_by_name() {
    let model = step(
        "#10=IFCCARTESIANPOINT((0.,0.));
#11=IFCAXIS2PLACEMENT2D(#10,$);
#12=IFCELLIPSE(#11,4.,2.);
#13=IFCTRIMMEDCURVE(#12,(IFCPARAMETERVALUE(0.)),(IFCPARAMETERVALUE(1.5707963267948966)),.T.,.PARAMETER.);
#14=IFCCARTESIANPOINT((0.,2.));
#15=IFCCARTESIANPOINT((-5.,2.));
#16=IFCPOLYLINE((#14,#15));
#17=IFCCOMPOSITECURVESEGMENT(.CONTINUOUS.,.T.,#13);
#18=IFCCOMPOSITECURVESEGMENT(.DISCONTINUOUS.,.T.,#16);
#19=IFCCOMPOSITECURVE((#17,#18),.F.);
#20=IFCPOINTBYDISTANCEEXPRESSION(IFCLENGTHMEASURE(1.),$,$,$,#19);
#21=IFCCARTESIANPOINT((10.,0.));
#22=IFCPOLYLINE((#10,#21));
#23=IFCOFFSETCURVE2D(#22,1.,.F.);
#24=IFCPOINTBYDISTANCEEXPRESSION(IFCLENGTHMEASURE(1.),$,$,$,#23);",
        false,
    );
    refused(lower(&model, 20), true, "not stated by its data");
    refused(lower(&model, 24), true, "offset curve");
}

#[cfg(feature = "compile-reference-backend")]
mod kernel {
    use axiolid_contracts::ExecutionOptions;
    use axiolid_core::Tolerance;
    use axiolid_mesh_compile::station::{placement, resolve, seams, ResolvedStation};
    use ifc_geometry::lower::{lower_axis2_placement_linear_node, LoweringSession};
    use ifc_geometry::transform::Transform;
    use ifc_geometry::units;

    use super::super::common::{close_point, close_vec, lower};
    use super::super::seams::item;
    use super::{norm, relations, root};
    use axiolid_model::{CurveRelation, GeometryNode};

    fn resolved(name: &str) -> ResolvedStation {
        let model = relations();
        let lowered = lower(&model, item(&model, name, "Reference")).expect(name);
        resolve(&lowered.graph, lowered.root).unwrap_or_else(|e| panic!("{name} resolves: {e:?}"))
    }

    /// Through the reference kernel: 1 m left on the corner is on the
    /// incoming leg, (10, 1); the outgoing leg would put it at (9, 0). On
    /// the tangent-continuous joint it is (9, 10); a quarter of the way
    /// round the arc about (0, 10), 1 m inside it.
    #[test]
    fn the_kernel_resolves_composite_stations_on_the_previous_segment() {
        let r = 9.0 * std::f64::consts::FRAC_1_SQRT_2;
        for (name, expected) in [
            ("PATH_CORNER_AT", [10.0, 1.0, 0.0]),
            ("PATH_CORNER_NEAR", [10.0, 1.0, 0.0]),
            ("PATH_TANGENT_JOINT", [9.0, 10.0, 0.0]),
            ("PATH_ARC", [r, 10.0 + r, 0.0]),
        ] {
            let station = resolved(name);
            close_point(station.point, expected, name);
        }
        close_vec(
            resolved("PATH_CORNER_AT").section.tangent,
            [1.0, 0.0, 0.0],
            "east",
        );
    }

    /// The linear placement on the corner frames the incoming leg: its
    /// local x is east.
    #[test]
    fn a_linear_placement_on_a_joint_frames_the_previous_segment() {
        let model = relations();
        let id = *model
            .ids_of_type("IFCAXIS2PLACEMENTLINEAR")
            .last()
            .expect("the corner frame");
        let scale = units::resolve(&model);
        let mut session = LoweringSession::new(&model, &scale);
        let node = lower_axis2_placement_linear_node(&mut session, id, Transform::identity())
            .expect("the frame lowers");
        let lowered = session.finish(node).expect("a valid graph");
        let station = resolve(&lowered.graph, lowered.root).expect("resolves");
        close_point(station.point, [10.0, 0.0, 0.0], "on the corner");
        close_vec(
            station.frame.x,
            [1.0, 0.0, 0.0],
            "local x along the incoming leg",
        );
    }

    /// The joints the lowering snaps to are the kernel's own seams
    /// (`axiolid_mesh_compile::station::seams`, the composite's
    /// `exact_seams`) within Axiolid's `ARC_LENGTH_TOLERANCE * max(1, s)`,
    /// the window in which the kernel finds a station on a joint. They
    /// differ by rounding only: the kernel measures a polyline piece through
    /// its arc-length reader.
    #[test]
    fn snapped_joints_are_the_kernels_seams() {
        let model = relations();
        let options = ExecutionOptions::new(Tolerance::MILLIMETRE);
        for name in ["PATH_CORNER_NEAR", "PATH_TANGENT_JOINT", "KERB_JOINT"] {
            let lowered = lower(&model, item(&model, name, "Reference")).expect(name);
            let GeometryNode::OrientedCurveStation(station) = root(&lowered) else {
                panic!("{name}: on a joint");
            };
            let kernel = seams(&lowered.graph, station.station.basis, &options)
                .unwrap_or_else(|e| panic!("{name}: the kernel's seams: {e:?}"));
            let distance = station.station.station.distance;
            assert!(
                kernel
                    .iter()
                    .any(|seam| (seam.distance - distance).abs() <= 1e-12 * distance.max(1.0)),
                "{name}: {distance} not among {kernel:?}"
            );
        }
    }

    /// A composite whose pieces do not meet is refused by the kernel,
    /// naming the gap.
    #[test]
    fn a_gap_is_refused_by_the_kernel() {
        let model = relations();
        let lowered = lower(&model, item(&model, "GAP", "Reference")).expect("lowers");
        let error = resolve(&lowered.graph, lowered.root).expect_err("a gap");
        assert!(error.to_string().contains("gap"), "{error}");
    }

    /// The kerb's first segment stands at station 10 of the gradient
    /// curve, 3 m left, laid along the 0.02 grade: (10, 3, 10.2) with local
    /// x the grade's tangent. A station 5 m along the kerb is on it; on the
    /// joint, 1 m up is perpendicular to the incoming segment (the placed
    /// curve's own reference-up frame, since its placement tilts +Z).
    #[test]
    fn the_kernel_places_the_kerb_segments_at_their_stations() {
        let model = relations();
        let n = norm();
        let kerb = lower(&model, item(&model, "KERB", "Axis")).expect("lowers");
        let GeometryNode::CurveRelation(CurveRelation::Composite { segments }) = root(&kerb) else {
            panic!("a composite");
        };
        let first = placement(&kerb.graph, segments[0].curve).expect("placed");
        close_point(
            first.transform.translation,
            [10.0, 3.0, 10.2],
            "first start",
        );
        let second = placement(&kerb.graph, segments[1].curve).expect("placed");
        close_point(
            second.transform.translation,
            [30.0, 3.0, 10.6],
            "second start",
        );

        close_point(
            resolved("KERB_START").point,
            [10.0 + 5.0 / n, 3.0, 10.2 + 0.1 / n],
            "5 m along",
        );
        close_point(
            resolved("KERB_JOINT").point,
            [30.0 - 0.02 / n, 3.0, 10.6 + 1.0 / n],
            "1 m up on the joint",
        );
    }
}
