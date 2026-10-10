//! Stations along offsets whose length is a quadrature (#423).
//!
//! Beside a gradient curve, a spiral or a B-spline, an offset's joints lie
//! at arc-length integrals, which this crate does not compute (ADR 0004).
//! A station along such a basis is lowered at its own distance as an
//! `OrientedCurveStation` reading `SeamSide::Incoming`, with the model's
//! precision as its seam-snapping window (axiolid/kernel#294): the kernel
//! reads it on a joint within that window, from the previous piece (IFC4.3
//! ADD2 8.9.3.48.3), at the joint's own certified distance. The fixture is
//! `synthetic-lowering/station_offset_bases_ifc4x3.ifc` (its generator's
//! docstring states every case). With the `compile-reference-backend`
//! feature the stations are resolved through the reference kernel, and its
//! refusals of a window surface as `GeometryError::StationSeamWindowRefused`
//! naming the station.
//!
//! An offset of a banked curve, the first case whose length the kernel
//! refuses to certify, cannot be lowered from IFC: `IfcSegmentedReferenceCurve`
//! is refused while lowering (#311), and a banked basis by name. The
//! `TILTED` offset is the uncertified case IFC reaches.

use axiolid_model::{CurveRelation, GeometryNode, OrientedCurveStation, SeamSide};
use ifc_geometry::lower::LoweredGeometry;
use ifc_model::{EntityId, Model};

use super::common::{lower, root};
use super::seams::item;

/// The committed station fixture for offsets whose length is a quadrature
/// (#423).
pub fn bases() -> Model {
    use ifc_model::Codec;
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../test/fixtures/synthetic-lowering/station_offset_bases_ifc4x3.ifc");
    ifc_step::StepCodec
        .read_path(&path)
        .unwrap_or_else(|e| panic!("{}: {e:?}", path.display()))
}

/// The declared precision, the window every station carries.
const PRECISION: f64 = 1e-5;

/// The kerb's own plan length at its joint at the grade break (20 m along
/// the alignment): beside the line it runs from (0, 2) to (20, 2.5).
fn grade_joint() -> f64 {
    20.0f64.hypot(0.5)
}

/// At the horizontal joint (30 m): on to (30, 2.75).
fn bend_joint() -> f64 {
    grade_joint() + 10.0f64.hypot(0.25)
}

/// The proxy named `name`.
#[cfg(feature = "compile-reference-backend")]
fn product(model: &Model, name: &str) -> EntityId {
    model
        .ids_of_type("IFCBUILDINGELEMENTPROXY")
        .iter()
        .copied()
        .find(|id| model.get(*id).and_then(|e| e.text(2)) == Some(name))
        .unwrap_or_else(|| panic!("no proxy {name}"))
}

/// The lowered point of proxy `name`: an unturned oriented station at its
/// own distance, reading the incoming side within the precision's window.
fn windowed(model: &Model, name: &str) -> (LoweredGeometry, OrientedCurveStation) {
    let point = item(model, name, "Reference");
    let lowered = lower(model, point).expect(name);
    let GeometryNode::OrientedCurveStation(station) = root(&lowered) else {
        panic!("{name}: an OrientedCurveStation: {:?}", root(&lowered));
    };
    let station = *station;
    assert_eq!(station.seam, SeamSide::Incoming, "{name}");
    assert_eq!(station.seam_window, PRECISION, "{name}: the precision");
    assert!(station.orientation.is_base(), "{name}: unturned");
    assert!(
        matches!(
            lowered.graph.get(station.station.basis),
            Some(GeometryNode::CurveRelation(
                CurveRelation::OffsetByStations { .. } | CurveRelation::Offset { .. }
            ))
        ),
        "{name}: an offset basis"
    );
    assert_eq!(
        lowered.provenance.seam_windows().collect::<Vec<_>>(),
        [EntityId(point)],
        "{name}: recorded"
    );
    (lowered, station)
}

/// Every point is stored at its own distance, which the lowering cannot
/// compare with joints it cannot locate: the kernel snaps it.
#[test]
fn stations_on_quadrature_offsets_carry_the_precision_as_window() {
    let model = bases();
    for (name, distance) in [
        ("KERB_GRADE_AT", grade_joint()),
        ("KERB_GRADE_NEAR", grade_joint() + 4e-6),
        ("KERB_BEND_AT", bend_joint()),
        ("KERB_BEND_NEAR", bend_joint() - 4e-6),
        ("KERB_MID", grade_joint() / 2.0),
        ("LANE_JOINT_AT", 19.0),
        ("LANE_JOINT_NEAR", 19.000004),
        ("LANE_MID", 9.5),
    ] {
        let (_, station) = windowed(&model, name);
        let stored = station.station.station.distance;
        assert!(
            (stored - distance).abs() <= 1e-12 * distance,
            "{name}: {stored} != {distance}"
        );
    }
    for name in ["KERB_EDGE", "TWIN_POINT"] {
        windowed(&model, name);
    }
}

/// A graph holding a window is wire format 1.1 (Axiolid ADR 0085), and
/// reads back to itself.
#[cfg(feature = "wire")]
#[test]
fn a_windowed_station_writes_wire_format_1_1() {
    use axiolid_model::GeometryGraph;

    let model = bases();
    let (lowered, _) = windowed(&model, "KERB_GRADE_NEAR");
    let json = lowered.graph.to_json().expect("writes");
    assert!(json.contains("\"version\":\"1.1\""), "{json}");
    assert!(json.contains("\"seam_window\":0.00001"), "{json}");
    let read = GeometryGraph::from_json(&json).expect("reads back");
    assert_eq!(read.to_json().expect("writes"), json);
}

#[cfg(feature = "compile-reference-backend")]
mod kernel {
    use axiolid_curve_evaluate_contract::CurveEvaluator;
    use axiolid_evaluate::ReferenceCurveEvaluator;
    use axiolid_mesh_compile::station::{curve_path, resolve, ResolvedStation};
    use axiolid_model::{CurveRelation, GeometryNode};
    use axiolid_reference::station::station_section3;
    use ifc_geometry::compile::{compile_product_mesh, SeamWindowRefusal, Tolerance};
    use ifc_geometry::constraint::placement::derive::derive_linear_placement_transform;
    use ifc_geometry::{units, GeometryError};

    use super::super::common::{close_point, close_vec};
    use super::{bases, bend_joint, grade_joint, product, windowed, PRECISION};

    fn resolved(name: &str) -> ResolvedStation {
        let model = bases();
        let (lowered, _) = windowed(&model, name);
        resolve(&lowered.graph, lowered.root).unwrap_or_else(|e| panic!("{name} resolves: {e:?}"))
    }

    fn unit(v: [f64; 3]) -> [f64; 3] {
        let n = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
        [v[0] / n, v[1] / n, v[2] / n]
    }

    /// On the kerb's joint at the grade break and 4 um past it, the kernel
    /// reads the joint's own point (20, 2.5, 10.4) from the INCOMING piece,
    /// rising at 0.02: the point 1 m up stands perpendicular to it. On the
    /// horizontal joint and 4 um before it, (30, 2.75, 10.3) on the piece
    /// beside the line, where beside the arc the kerb runs along
    /// (0.945, 0.025, -0.01). Near no joint, the point is the station's own.
    #[test]
    fn the_kernel_snaps_onto_the_kerbs_joints_and_reads_the_previous_piece() {
        let rising = unit([1.0, 0.025, 0.02]);
        // Up: perpendicular to the tangent in its vertical plane.
        let horizontal = (rising[0] * rising[0] + rising[1] * rising[1]).sqrt();
        let up = [
            -rising[2] * rising[0] / horizontal,
            -rising[2] * rising[1] / horizontal,
            horizontal,
        ];
        for name in ["KERB_GRADE_AT", "KERB_GRADE_NEAR"] {
            let station = resolved(name);
            close_vec(station.section.tangent, rising, "the incoming grade");
            close_point(
                station.point,
                [20.0 + up[0], 2.5 + up[1], 10.4 + up[2]],
                name,
            );
        }
        for name in ["KERB_BEND_AT", "KERB_BEND_NEAR"] {
            let station = resolved(name);
            close_vec(
                station.section.tangent,
                unit([1.0, 0.025, -0.01]),
                "beside the line",
            );
            close_point(station.point, [30.0, 2.75, 10.3], name);
        }
        close_point(resolved("KERB_MID").point, [10.0, 2.25, 10.2], "KERB_MID");
    }

    /// The lane beside the clothoid: its joint 19 m along (the offset of a
    /// 1 rad turn, 1 m to its left, is 1 m shorter than the clothoid), on
    /// it and 4 um past it, is 1 m left of the clothoid's end, heading
    /// 1 rad. Without the window the point 4 um past would be 4 um on.
    #[test]
    fn the_kernel_snaps_onto_a_joint_beside_a_clothoid() {
        let model = bases();
        let (lowered, station) = windowed(&model, "LANE_JOINT_AT");
        let Some(GeometryNode::CurveRelation(CurveRelation::OffsetByStations { basis, .. })) =
            lowered.graph.get(station.station.basis)
        else {
            panic!("an offset by distances");
        };
        let Some(GeometryNode::Curve3(alignment)) = lowered.graph.get(*basis) else {
            panic!("an elevated alignment");
        };
        let end = station_section3(alignment, 20.0).expect("the clothoid's end");
        let expected = end.place(1.0, 0.0, 0.0);
        for name in ["LANE_JOINT_AT", "LANE_JOINT_NEAR"] {
            let station = resolved(name);
            close_point(station.point, [expected.x, expected.y, expected.z], name);
            close_vec(
                station.section.tangent,
                [1.0f64.cos(), 1.0f64.sin(), 0.0],
                "the joint's heading",
            );
        }
        let mid = resolved("LANE_MID").point;
        assert!((mid - expected).length() > 8.0, "LANE_MID is no joint");
    }

    /// The joints the window snaps to are the kernel's certified ones, each
    /// no wider than the window's 1e-3, and the closed forms lie in them:
    /// the kerb's first two joints beside the alignment's line, and its
    /// third, beside the arc, by the generator's quadrature.
    #[test]
    fn the_kernels_joints_contain_the_closed_forms() {
        let model = bases();
        let (lowered, station) = windowed(&model, "KERB_EDGE");
        let path = curve_path(&lowered.graph, station.station.basis).expect("a path");
        let tolerance = PRECISION * 1e-3;
        let joints = ReferenceCurveEvaluator::default()
            .path_joint_distances(&path, tolerance)
            .expect("certified");
        assert!(joints.len() >= 3, "{joints:?}");
        for (joint, exact) in joints.iter().zip([grade_joint(), bend_joint()]) {
            assert!(
                joint.lower <= exact + 1e-12 && joint.upper >= exact - 1e-12,
                "{joint:?} holds {exact}"
            );
            assert!(joint.width() <= tolerance, "{joint:?}");
        }
        // KERB_EDGE stands one window past the third joint.
        let third = station.station.station.distance - PRECISION;
        assert!(joints[2].width() <= tolerance, "{:?}", joints[2]);
        assert!(
            joints[2].lower - 1e-9 <= third && third <= joints[2].upper + 1e-9,
            "{:?} holds {third}",
            joints[2]
        );
    }

    /// A rail placed at a station 4 um past the grade break compiles: the
    /// kernel snaps its placement onto the joint. Placed where the window
    /// holds two joints, where a joint's certified distance straddles its
    /// edge, or along an offset whose length the kernel does not certify,
    /// it is refused as a typed error naming the station.
    #[test]
    fn window_refusals_name_the_station() {
        let model = bases();
        let mesh =
            compile_product_mesh(&model, product(&model, "RAIL_NEAR"), Tolerance::MILLIMETRE)
                .expect("compiles")
                .expect("a body");
        assert!(!mesh.positions.is_empty());
        for (name, refusal, unsupported) in [
            ("RAIL_TWIN", SeamWindowRefusal::Ambiguous, false),
            ("RAIL_EDGE", SeamWindowRefusal::Straddled, false),
            ("RAIL_TILTED", SeamWindowRefusal::UncertifiedLength, true),
        ] {
            let id = product(&model, name);
            let error = compile_product_mesh(&model, id, Tolerance::MILLIMETRE).expect_err(name);
            let GeometryError::StationSeamWindowRefused {
                product: refused_product,
                ref stations,
                refusal: got,
            } = error
            else {
                panic!("{name}: {error:?}");
            };
            assert_eq!((refused_product, got), (id, refusal), "{name}");
            assert_eq!(stations.len(), 1, "{name}: {stations:?}");
            let station = model.get(stations[0]).expect("the station");
            assert_eq!(
                station.type_name.as_ref(),
                "IFCAXIS2PLACEMENTLINEAR",
                "{name}"
            );
            assert_eq!(error.entity(), Some(stations[0]), "{name}");
            assert_eq!(error.is_unsupported(), unsupported, "{name}");
            assert!(
                error.to_string().contains(&stations[0].to_string()),
                "{error}"
            );
        }
    }

    /// A linear placement on the kerb cannot be derived: the evaluator's
    /// `CurvePath` states an offset piece's span in its own length, which
    /// only a provider computes (#427). Refused by name, never derived from
    /// an estimate.
    #[test]
    fn a_linear_placement_on_a_quadrature_offset_is_refused_by_name() {
        let model = bases();
        let placement = model.ids_of_type("IFCLINEARPLACEMENT")[0];
        let error = derive_linear_placement_transform(
            &model,
            &units::resolve(&model),
            placement,
            &ReferenceCurveEvaluator::default(),
        )
        .expect_err("refused");
        assert!(
            matches!(
                error,
                GeometryError::PathPieceLengthUnstated { placement: p, .. } if p == placement
            ),
            "{error:?}"
        );
        assert!(error.is_unsupported());
        assert!(error.to_string().contains("#427"), "{error}");
    }
}
