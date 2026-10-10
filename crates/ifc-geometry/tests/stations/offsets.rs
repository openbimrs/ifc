//! Stations along offset curves (#414).
//!
//! Axiolid measures a station along an `IfcOffsetCurve2D`,
//! `IfcOffsetCurve3D` or `IfcOffsetCurveByDistances` since
//! `axiolid-mesh-compile` 0.3.19 (axiolid/kernel#289): in the offset's own
//! length, one offset piece per span of its basis between seams (and, by
//! distances, between stations), every joint a seam. The lowering reads the
//! joints' distances from the stored relation and snaps a station within
//! precision of one onto it, reading `SeamSide::Incoming`, IFC's previous
//! segment (8.9.3.48.3). The fixture is
//! `synthetic-lowering/station_offsets_ifc4x3.ifc` (its generator's
//! docstring states every case). With the `compile-reference-backend`
//! feature the lowered stations are resolved through the reference kernel.

use std::f64::consts::PI;

use axiolid_model::{CurveRelation, GeometryNode, OrientedCurveStation, SeamSide};
use ifc_model::Model;

use super::common::{assert_windowed, lower, refused, root, step};
use super::seams::item;

/// The offset lane's length: 10 m beside the line, a quarter turn of
/// radius 9 beside the arc.
const LANE: f64 = 10.0 + 4.5 * PI;

/// The kerb's pieces: (0,2) -> (10,0) and (10,0) -> (8,10).
fn side() -> f64 {
    104.0f64.sqrt()
}

/// The committed station-offset fixture (#414).
pub fn offsets() -> Model {
    use ifc_model::Codec;
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../test/fixtures/synthetic-lowering/station_offsets_ifc4x3.ifc");
    ifc_step::StepCodec
        .read_path(&path)
        .unwrap_or_else(|e| panic!("{}: {e:?}", path.display()))
}

/// The lowered point of proxy `name`, which must stand on a joint of an
/// offset basis.
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
            Some(GeometryNode::CurveRelation(
                CurveRelation::Offset { .. } | CurveRelation::OffsetByStations { .. }
            ))
        ),
        "{name}: an offset basis: {basis:?}"
    );
    *station
}

/// On the lane's joint (10 m: the offset of the line ends there) and 4 um
/// past it, inside the 1e-5 m precision, the station stands on the joint
/// reading the previous piece. So does the kerb's corner, the offsets'
/// own sqrt(104) m along (not the basis's 10 m).
#[test]
fn a_station_on_an_offset_joint_reads_the_previous_piece() {
    let model = offsets();
    for (name, joint) in [
        ("LANE_JOINT_AT", 10.0),
        ("LANE_JOINT_NEAR", 10.0),
        ("KERB_CORNER_AT", side()),
        ("KERB_CORNER_NEAR", side()),
    ] {
        on_joint(&model, name, joint);
    }
    // Off a joint the station is plain, at its own distance.
    for (name, distance) in [
        ("LANE_ARC", 10.0 + 2.25 * PI),
        ("LANE_END", LANE),
        ("ARC_LANE_MID", 2.75 * PI),
        ("ARC_LANE_END", 5.5 * PI),
        ("KERB_MID", side() / 2.0),
        ("CORNER_LANE_POINT", 5.0),
    ] {
        let lowered = lower(&model, item(&model, name, "Reference")).expect(name);
        let GeometryNode::CurveStation(station) = root(&lowered) else {
            panic!("{name}: a CurveStation: {:?}", root(&lowered));
        };
        assert_eq!(station.station.distance, distance, "{name}");
    }
}

/// The offset of the quarter circle is a circle of radius 11, whose
/// length 5.5 pi m the lowering states exactly: a station at its end
/// lowers, one 0.1 mm past it (beyond the 1e-5 m precision) is refused.
#[test]
fn an_offset_arc_has_its_exact_length() {
    let records = |distance: f64| {
        format!(
            "#10=IFCCARTESIANPOINT((0.,0.));
#11=IFCAXIS2PLACEMENT2D(#10,$);
#12=IFCCIRCLE(#11,10.);
#13=IFCTRIMMEDCURVE(#12,(IFCPARAMETERVALUE(0.)),(IFCPARAMETERVALUE(1.5707963267948966)),.T.,.PARAMETER.);
#14=IFCOFFSETCURVE2D(#13,-1.,.F.);
#15=IFCPOINTBYDISTANCEEXPRESSION(IFCLENGTHMEASURE({distance:?}),$,$,$,#14);"
        )
    };
    let end = 5.5 * PI;
    lower(&step(&records(end), false), 15).expect("the end lowers");
    refused(
        lower(&step(&records(end + 1e-4), false), 15),
        false,
        "beyond the basis curve's length",
    );
}

/// What an offset basis cannot carry is refused by name, before any
/// station is resolved: an offset of an offset, a trim of an offset and an
/// offset through a circle's centre. A station on an offset whose length is
/// a quadrature (beside an ellipse), refused before #423, carries the
/// kernel's window; a run along it stays refused.
#[test]
fn offset_bases_the_kernel_refuses_are_refused_by_name() {
    let model = step(
        "#10=IFCCARTESIANPOINT((0.,0.));
#11=IFCCARTESIANPOINT((10.,0.));
#12=IFCPOLYLINE((#10,#11));
#13=IFCOFFSETCURVE2D(#12,1.,.F.);
#14=IFCOFFSETCURVE2D(#13,1.,.F.);
#15=IFCPOINTBYDISTANCEEXPRESSION(IFCLENGTHMEASURE(1.),$,$,$,#14);
#16=IFCTRIMMEDCURVE(#13,(IFCPARAMETERVALUE(0.)),(IFCPARAMETERVALUE(0.5)),.T.,.PARAMETER.);
#17=IFCPOINTBYDISTANCEEXPRESSION(IFCLENGTHMEASURE(1.),$,$,$,#16);
#30=IFCAXIS2PLACEMENT2D(#10,$);
#31=IFCCIRCLE(#30,10.);
#32=IFCTRIMMEDCURVE(#31,(IFCPARAMETERVALUE(0.)),(IFCPARAMETERVALUE(1.)),.T.,.PARAMETER.);
#33=IFCOFFSETCURVE2D(#32,10.,.F.);
#34=IFCPOINTBYDISTANCEEXPRESSION(IFCLENGTHMEASURE(1.),$,$,$,#33);
#40=IFCELLIPSE(#30,4.,2.);
#41=IFCTRIMMEDCURVE(#40,(IFCPARAMETERVALUE(0.)),(IFCPARAMETERVALUE(1.)),.T.,.PARAMETER.);
#42=IFCOFFSETCURVE2D(#41,0.5,.F.);
#43=IFCPOINTBYDISTANCEEXPRESSION(IFCLENGTHMEASURE(1.),$,$,$,#42);
#44=IFCPOINTBYDISTANCEEXPRESSION(IFCLENGTHMEASURE(1.),0.5,$,$,#42);
#45=IFCOFFSETCURVEBYDISTANCES(#42,(#44),$);",
        false,
    );
    refused(lower(&model, 15), true, "offset of an offset curve");
    refused(lower(&model, 17), true, "trim of an offset curve");
    refused(lower(&model, 34), true, "collapses");
    assert_windowed(&lower(&model, 43).expect("lowers"), 1.0, 1e-5);
    // A run's stations carry no window: a run along it is refused.
    refused(lower(&model, 45), true, "carry no seam-snapping window");
}

#[cfg(feature = "compile-reference-backend")]
mod kernel {
    use std::f64::consts::{FRAC_1_SQRT_2, PI};

    use axiolid_contracts::ExecutionOptions;
    use axiolid_core::Tolerance;
    use axiolid_mesh::TriMesh;
    use axiolid_mesh_compile::station::{curve_path, resolve, seams, ResolvedStation};
    use axiolid_mesh_compile_contract::{MeshClosure, MeshCompiler};
    use axiolid_model::GeometryNode;
    use axiolid_reference::station::CompositeBasis;
    use ifc_geometry::compile::default_backend;
    use ifc_geometry::lower::{lower_axis2_placement_linear_node, LoweringSession};
    use ifc_geometry::transform::Transform;
    use ifc_geometry::units;

    use super::super::common::{close_point, close_vec, lower, root, step};
    use super::super::seams::item;
    use super::{offsets, side, LANE};

    fn resolved(name: &str) -> ResolvedStation {
        let model = offsets();
        let lowered = lower(&model, item(&model, name, "Reference")).expect(name);
        resolve(&lowered.graph, lowered.root).unwrap_or_else(|e| panic!("{name} resolves: {e:?}"))
    }

    /// Through the reference kernel, each station at its own distance
    /// along the offset: on the lane's joint 0.5 m left of (10, 1); half
    /// way round its arc, 9 m from (10, 10) at 45 degrees; at its end
    /// (19, 10). Half way round the radius-11 arc, and at its end. On the
    /// kerb's corner, 1 m left of the INCOMING piece (10, -2) / sqrt(104):
    /// (10, 0) + (2, 10) / sqrt(104), where the outgoing piece would put
    /// it at (10, 0) + (-10, -2) / sqrt(104).
    #[test]
    fn the_kernel_resolves_offset_stations_in_their_own_length() {
        let s = side();
        let arc = 9.0 * FRAC_1_SQRT_2;
        for (name, expected) in [
            ("LANE_JOINT_AT", [10.0, 1.5, 0.0]),
            ("LANE_JOINT_NEAR", [10.0, 1.5, 0.0]),
            ("LANE_ARC", [10.0 + arc, 10.0 - arc, 0.0]),
            ("LANE_END", [19.0, 10.0, 0.0]),
            (
                "ARC_LANE_MID",
                [11.0 * FRAC_1_SQRT_2, 11.0 * FRAC_1_SQRT_2, 0.0],
            ),
            ("ARC_LANE_END", [0.0, 11.0, 0.0]),
            ("KERB_CORNER_AT", [10.0 + 2.0 / s, 10.0 / s, 0.0]),
            ("KERB_CORNER_NEAR", [10.0 + 2.0 / s, 10.0 / s, 0.0]),
            ("KERB_MID", [5.0, 1.0, 0.0]),
        ] {
            close_point(resolved(name).point, expected, name);
        }
        close_vec(
            resolved("KERB_CORNER_AT").section.tangent,
            [10.0 / s, -2.0 / s, 0.0],
            "the incoming piece",
        );
    }

    /// The linear placement on the kerb's corner frames the incoming
    /// piece: its local x runs (10, -2) / sqrt(104).
    #[test]
    fn a_linear_placement_on_an_offset_corner_frames_the_previous_piece() {
        let model = offsets();
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
        let s = side();
        close_point(station.point, [10.0, 0.0, 0.0], "on the corner");
        close_vec(station.frame.x, [10.0 / s, -2.0 / s, 0.0], "local x");
    }

    /// The joints the lowering snaps to are the kernel's own: on the lane
    /// (the offset of a line, then of a circle, both closed forms) the
    /// compiler's exact `station::seams`; on the kerb, whose polyline
    /// offsets the kernel measures by quadrature (and so reports
    /// inexact), the composite's own seams read along `curve_path`. Each
    /// within Axiolid's `ARC_LENGTH_TOLERANCE * max(1, s)`, the window in
    /// which the kernel finds a station on a joint; so are the lengths the
    /// lowering states.
    #[test]
    fn snapped_joints_and_lengths_are_the_kernels() {
        let model = offsets();
        let options = ExecutionOptions::new(Tolerance::MILLIMETRE);
        let within = |a: f64, b: f64| (a - b).abs() <= 1e-12 * a.abs().max(1.0);
        for (name, exact) in [("LANE_JOINT_NEAR", true), ("KERB_CORNER_NEAR", false)] {
            let lowered = lower(&model, item(&model, name, "Reference")).expect(name);
            let GeometryNode::OrientedCurveStation(station) = root(&lowered) else {
                panic!("{name}: on a joint");
            };
            let basis = station.station.basis;
            let distance = station.station.station.distance;
            let kernel: Vec<f64> = if exact {
                seams(&lowered.graph, basis, &options)
                    .unwrap_or_else(|e| panic!("{name}: the kernel's seams: {e:?}"))
                    .iter()
                    .map(|seam| seam.distance)
                    .collect()
            } else {
                let path = curve_path(&lowered.graph, basis).expect("a path");
                CompositeBasis::from_path(&path)
                    .expect("a composite")
                    .seams()
                    .expect("its seams")
                    .iter()
                    .map(|seam| seam.distance)
                    .collect()
            };
            assert!(
                kernel.iter().any(|seam| within(*seam, distance)),
                "{name}: {distance} not among {kernel:?}"
            );
        }
        for (name, length) in [("LANE_END", LANE), ("ARC_LANE_END", 5.5 * PI)] {
            let lowered = lower(&model, item(&model, name, "Reference")).expect(name);
            let GeometryNode::CurveStation(station) = root(&lowered) else {
                panic!("{name}: a station");
            };
            let path = curve_path(&lowered.graph, station.basis).expect("a path");
            let kernel = CompositeBasis::from_path(&path)
                .expect("a composite")
                .length();
            assert!(within(kernel, length), "{name}: {kernel} != {length}");
        }
    }

    /// The offset of the L polyline 1 m to its left has no joint at the
    /// corner: its sides end at (10, 1) and start at (9, 0). The kernel
    /// refuses a station on it by name, as a typed error.
    #[test]
    fn an_offset_across_a_corner_is_refused_by_the_kernel() {
        let model = offsets();
        let lowered =
            lower(&model, item(&model, "CORNER_LANE_POINT", "Reference")).expect("lowers");
        let error = resolve(&lowered.graph, lowered.root).expect_err("a corner");
        assert!(
            error.to_string().contains("offset across a corner"),
            "{error}"
        );
    }

    /// An `IfcOffsetCurve3D` of a sloped line, 2 m along `V x T` with `V`
    /// up: to the line's left, level, so 5 m along it is
    /// (10, 0, 5) / sqrt(125) * 5 + (0, 2, 0). Of a line whose tangent
    /// runs along `V`, the offset direction is undefined (IFC: "T shall not
    /// at any point of the curve be in the same, or opposite, direction as
    /// V") and the kernel refuses it by name.
    #[test]
    fn a_3d_offset_runs_along_v_cross_t() {
        let model = step(
            "#10=IFCCARTESIANPOINT((0.,0.,0.));
#11=IFCCARTESIANPOINT((10.,0.,5.));
#12=IFCPOLYLINE((#10,#11));
#13=IFCDIRECTION((0.,0.,1.));
#14=IFCOFFSETCURVE3D(#12,2.,.F.,#13);
#15=IFCPOINTBYDISTANCEEXPRESSION(IFCLENGTHMEASURE(5.),$,$,$,#14);
#16=IFCCARTESIANPOINT((10.,0.,10.));
#17=IFCPOLYLINE((#10,#16));
#20=IFCDIRECTION((1.,0.,1.));
#18=IFCOFFSETCURVE3D(#17,2.,.F.,#20);
#19=IFCPOINTBYDISTANCEEXPRESSION(IFCLENGTHMEASURE(5.),$,$,$,#18);",
            false,
        );
        let lowered = lower(&model, 15).expect("lowers");
        let station = resolve(&lowered.graph, lowered.root).expect("resolves");
        let n = 125.0f64.sqrt();
        close_point(
            station.point,
            [50.0 / n, 2.0, 25.0 / n],
            "5 m along, 2 m left",
        );
        let lowered = lower(&model, 19).expect("lowers");
        let error = resolve(&lowered.graph, lowered.root).expect_err("parallel");
        assert!(
            error
                .to_string()
                .contains("parallel to its reference direction"),
            "{error}"
        );
    }

    fn signed_volume(mesh: &TriMesh) -> f64 {
        let base = mesh.positions[0];
        mesh.indices
            .chunks_exact(3)
            .map(|t| {
                let [a, b, c] = [t[0], t[1], t[2]].map(|i| mesh.positions[i as usize] - base);
                a.dot(b.cross(c)) / 6.0
            })
            .sum()
    }

    fn bounds(mesh: &TriMesh) -> ([f64; 3], [f64; 3]) {
        let mut lo = [f64::INFINITY; 3];
        let mut hi = [f64::NEG_INFINITY; 3];
        for p in &mesh.positions {
            for (axis, value) in [p.x, p.y, p.z].into_iter().enumerate() {
                lo[axis] = lo[axis].min(value);
                hi[axis] = hi[axis].max(value);
            }
        }
        (lo, hi)
    }

    /// A run of sections along the kerb, 5 m to 15 m of its own length,
    /// across its corner: mitred at half angle (8.8.3.35.1), the centred
    /// 2 x 1 m rectangle over 10 m of centreline holds 20 m3.
    #[test]
    fn a_sectioned_solid_runs_along_an_offset() {
        let model = offsets();
        let lowered = lower(&model, item(&model, "DECK", "Body")).expect("lowers");
        let options = ExecutionOptions::new(Tolerance::MILLIMETRE);
        let outcome = default_backend()
            .compile_mesh_reported(&lowered.graph, lowered.root, &options)
            .unwrap_or_else(|e| panic!("the deck compiles: {e:?}"));
        assert_eq!(outcome.closure, MeshClosure::Solid);
        let volume = signed_volume(&outcome.mesh);
        assert!((volume - 20.0).abs() < 1e-9, "volume {volume}");
    }

    /// An offset run along the lane, carried on to its end at the length
    /// the lowering states: a 0.1 m disk swept 0.5 m right of the lane runs
    /// from (0, 0.5) along y = 0.5 and round 9.5 m from (10, 10) to
    /// (19.5, 10).
    #[test]
    fn an_offset_run_follows_an_offset() {
        let model = offsets();
        let lowered = lower(&model, item(&model, "LANE_EDGE", "Body")).expect("lowers");
        let options = ExecutionOptions::new(Tolerance::MILLIMETRE);
        let outcome = default_backend()
            .compile_mesh_reported(&lowered.graph, lowered.root, &options)
            .unwrap_or_else(|e| panic!("the edge compiles: {e:?}"));
        let (lo, hi) = bounds(&outcome.mesh);
        for (got, want, what) in [
            (lo[0], 0.0, "x min"),
            (lo[1], 0.4, "y min"),
            (hi[0], 19.6, "x max"),
            (hi[1], 10.0, "y max"),
        ] {
            assert!((got - want).abs() < 2e-3, "{what}: {got} != {want}");
        }
    }
}
