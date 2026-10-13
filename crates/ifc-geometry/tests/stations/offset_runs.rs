//! Offset runs along offsets whose length is a quadrature, continued to
//! their basis's ends (#433).
//!
//! The gutters of `synthetic-lowering/station_offset_bases_ifc4x3.ifc` are
//! `IfcOffsetCurveByDistances` beside the kerb (itself an offset beside an
//! `IfcGradientCurve`) whose offsets stop short of both of the kerb's
//! ends. IFC4.3 ADD2 8.9.3.42.3 continues them unchanged to those ends,
//! whose distances are quadratures: the run starts with
//! `RunStation::at_basis_start` and ends in `RunStation::at_basis_end`
//! (axiolid/kernel#303), whose distances the kernel measures, and its
//! stated stations carry the kernel's seam-snapping window (#429). A
//! stated station past the kerb's end surfaces as
//! `SeamWindowRefusal::PastBasisEnd` naming the run.

use axiolid_model::{CurveRelation, GeometryNode, RunStation, SeamSide, StationOffsets};
use ifc_geometry::lower::LoweredGeometry;
use ifc_model::{EntityId, Model};

use super::common::{lower, root};
use super::offset_bases::{bases, bend_joint, grade_joint, PRECISION};
use super::seams::item;

/// The offset run of proxy `name` (#433), its `Axis` item lowered: an
/// `OffsetsAtStations` beside the kerb whose stated stations carry the
/// precision as window, reading the incoming side, continued to the
/// kerb's start and end, and the run recorded.
fn gutter(model: &Model, name: &str) -> (LoweredGeometry, Vec<RunStation>) {
    let axis = item(model, name, "Axis");
    let lowered = lower(model, axis).expect(name);
    let GeometryNode::CurveRelation(CurveRelation::OffsetsAtStations {
        basis, stations, ..
    }) = root(&lowered)
    else {
        panic!("{name}: an offset run: {:?}", root(&lowered));
    };
    assert!(
        matches!(
            lowered.graph.get(*basis),
            Some(GeometryNode::CurveRelation(
                CurveRelation::OffsetByStations { .. }
            ))
        ),
        "{name}: beside the kerb"
    );
    let stations = stations.clone();
    let (start, rest) = stations.split_first().expect("stations");
    let (end, stated) = rest.split_last().expect("stations");
    assert_eq!(
        *start,
        RunStation::at_basis_start(StationOffsets::new(0.5, 0.0, 0.0)),
        "{name}: the head"
    );
    assert_eq!(
        *end,
        RunStation::at_basis_end(StationOffsets::new(0.75, 0.0, 0.0)),
        "{name}: the tail"
    );
    for station in stated {
        assert_eq!(station.basis_end, None, "{name}");
        assert_eq!(station.seam, SeamSide::Incoming, "{name}");
        assert_eq!(station.seam_window, PRECISION, "{name}: the precision");
    }
    assert_eq!(
        lowered.provenance.seam_window_runs().collect::<Vec<_>>(),
        [EntityId(axis)],
        "{name}: the run recorded"
    );
    (lowered, stations)
}

/// The gutters' offsets stop short of both of the kerb's ends, whose
/// distances are quadratures: they are continued by stations at the
/// kerb's start and end, which the kernel measures (#433), and stand at
/// their own distances in between, which the kernel snaps.
#[test]
fn offset_runs_along_the_kerb_continue_to_its_ends() {
    let model = bases();
    let distances = |stations: &[RunStation]| {
        stations[1..stations.len() - 1]
            .iter()
            .map(|station| station.station.distance)
            .collect::<Vec<_>>()
    };
    let (_, stations) = gutter(&model, "GUTTER");
    let arc_joint = 39.437_688_610_242_96;
    let expected = [
        grade_joint() / 2.0,
        grade_joint() + 4e-6,
        bend_joint() - 4e-6,
        arc_joint - 4e-6,
    ];
    let got = distances(&stations);
    assert_eq!(got.len(), expected.len(), "{got:?}");
    for (got, want) in got.iter().zip(expected) {
        assert!((got - want).abs() <= 1e-12 * want, "{got} != {want}");
    }
    let (_, stations) = gutter(&model, "GUTTER_AT");
    assert_eq!(stations.len(), 6);
    // Stated past the kerb's end, which only the kernel can tell.
    let (_, stations) = gutter(&model, "GUTTER_PAST");
    assert_eq!(distances(&stations), [grade_joint() / 2.0, 60.0]);
}

/// A graph holding a station at its basis's end is wire format 1.3
/// (Axiolid ADR 0085, axiolid/kernel#303), the gutter's curve and its
/// tube alike, and reads back byte for byte: `axiolid-model` 0.3.13 reads
/// every JSON real back exactly (`serde_json` `float_roundtrip`).
#[cfg(feature = "wire")]
#[test]
fn an_offset_run_to_the_basis_end_writes_wire_format_1_3() {
    use axiolid_model::GeometryGraph;

    let model = bases();
    let (curve, _) = gutter(&model, "GUTTER");
    let tube = lower(&model, item(&model, "GUTTER", "Body")).expect("the tube");
    for (what, lowered) in [("curve", curve), ("tube", tube)] {
        let json = lowered.graph.to_json().expect("writes");
        assert!(json.contains("\"version\":\"1.3\""), "{what}: {json}");
        assert!(json.contains("\"basis_end\":\"End\""), "{what}: {json}");
        assert!(json.contains("\"basis_end\":\"Start\""), "{what}: {json}");
        let read = GeometryGraph::from_json(&json).expect("reads back");
        assert_eq!(read.len(), lowered.graph.len(), "{what}");
        assert_eq!(read.to_json().expect("writes"), json, "{what}");
    }
}

#[cfg(feature = "compile-reference-backend")]
mod kernel {
    use axiolid_contracts::ExecutionOptions;
    use axiolid_mesh_compile_contract::MeshCompiler;
    use ifc_geometry::compile::{
        compile_product_mesh, default_backend, SeamWindowRefusal, Tolerance,
    };
    use ifc_geometry::GeometryError;

    use super::super::common::{ring_around, swept_along};
    use super::super::offset_bases::{bases, product};
    use super::super::seams::item;

    /// The gutters' tube radius.
    const TUBE: f64 = 0.05;

    /// The gutter's start: 0.5 m left of the kerb's start (0, 2, 10),
    /// where the kerb runs along (1, 0.025) in plan.
    fn gutter_start() -> [f64; 3] {
        let norm = 1.0f64.hypot(0.025);
        [-0.5 * 0.025 / norm, 2.0 + 0.5 / norm, 10.0]
    }

    /// The gutter's end: 0.75 m left of the kerb's end, which is 3 m left
    /// of the alignment's end, its 30 m line and 30 m arc of radius 50
    /// turning 0.6 rad, at height 10.4 - 0.01 x 40. The kerb beside the arc
    /// is concentric with it, so its left is the alignment's.
    fn gutter_end() -> [f64; 3] {
        let turn = 0.6f64;
        let (x, y) = (30.0 + 50.0 * turn.sin(), 50.0 * (1.0 - turn.cos()));
        [x - 3.75 * turn.sin(), y + 3.75 * turn.cos(), 10.0]
    }

    /// The gutter resolves through the reference kernel (#433): swept
    /// along its whole extent, it starts 0.5 m beside the kerb's start and
    /// ends 0.75 m beside the kerb's end, both measured by the kernel. Its
    /// stations 4 um from the kerb's joints snap onto them: its tube is
    /// bitwise the one along `GUTTER_AT`, whose stations stand on them.
    #[test]
    fn the_gutter_runs_from_the_kerbs_start_to_its_end() {
        let model = bases();
        let options = ExecutionOptions::new(Tolerance::MILLIMETRE);
        let whole = |name: &str| {
            let lowered = swept_along(&model, item(&model, name, "Axis"), TUBE);
            default_backend()
                .compile_mesh(&lowered.graph, lowered.root, &options)
                .unwrap_or_else(|e| panic!("{name} compiles: {e:?}"))
        };
        let near = whole("GUTTER");
        ring_around(&near, gutter_start(), TUBE, "the kerb's start");
        ring_around(&near, gutter_end(), TUBE, "the kerb's end");
        let at = whole("GUTTER_AT");
        assert_eq!(near.indices, at.indices, "snapped onto the joints");
        assert_eq!(near.positions, at.positions, "snapped onto the joints");
        // The products' tubes, 0 to 50 m along the kerb, alike.
        let tube = |name: &str| {
            compile_product_mesh(&model, product(&model, name), Tolerance::MILLIMETRE)
                .unwrap_or_else(|e| panic!("{name} compiles: {e}"))
                .expect("a body")
        };
        let (near, at) = (tube("GUTTER"), tube("GUTTER_AT"));
        ring_around(&near, gutter_start(), TUBE, "the tube's start");
        assert_eq!(near.indices, at.indices, "the tubes");
        assert_eq!(near.positions, at.positions, "the tubes");
    }

    /// A gutter stated at 60 m, the alignment's length, past the kerb's own
    /// end (about 58.24 m), is refused by the kernel once it has measured
    /// that end, as a typed error naming the run (#433).
    #[test]
    fn a_gutter_past_the_kerbs_end_is_refused_naming_the_run() {
        let model = bases();
        let id = product(&model, "GUTTER_PAST");
        let error = compile_product_mesh(&model, id, Tolerance::MILLIMETRE).expect_err("past");
        let GeometryError::RunSeamWindowRefused {
            product: refused_product,
            ref runs,
            ref stations,
            refusal,
        } = error
        else {
            panic!("{error:?}");
        };
        assert_eq!(
            (refused_product, refusal),
            (id, SeamWindowRefusal::PastBasisEnd)
        );
        assert!(stations.is_empty(), "{stations:?}");
        assert_eq!(runs.len(), 1, "{runs:?}");
        let run = model.get(runs[0]).expect("the run");
        assert_eq!(run.type_name.as_ref(), "IFCOFFSETCURVEBYDISTANCES");
        assert_eq!(error.entity(), Some(runs[0]));
        assert!(!error.is_unsupported());
        assert!(error.to_string().contains(&runs[0].to_string()), "{error}");
    }
}
