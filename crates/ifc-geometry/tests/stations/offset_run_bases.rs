//! Stations along an offset run continued to its basis's ends (#443).
//!
//! An `IfcOffsetCurveByDistances` whose offsets stop short of the ends of
//! a basis with a quadrature length lowers to
//! `CurveRelation::OffsetsAtStations`, continued to those ends by
//! `RunStation::at_basis_start` and `at_basis_end` (#433). Axiolid measures
//! a station along it as along any offset by stations
//! (`axiolid-mesh-compile` 0.3.23, `station::basis`): it resolves the
//! basis's ends and snaps the windowed stations first, then offsets each
//! span of the basis between them, in the offset's own length. The
//! lowering flattens it the same way (`relation::offset::by_stations`):
//! its pieces' lengths are quadratures, so a section or a point along it
//! carries the kernel's seam-snapping window (#423, #429), and a derived
//! linear placement reads its last span open to the basis's end (#427).
//!
//! The fixture is `synthetic-lowering/station_offset_bases_ifc4x3.ifc`,
//! axioval/engine's deck: `#366` an `IfcEllipse` centred at (30, 0) with
//! semi-axes 10 and 6, `#369` (`RING`) the run beside it, lateral 0.5 m at
//! 2 m and 0.75 m at 20 m along it; `RING_DECK` (`#380`) the 2 x 1 m deck
//! rectangle at 0 and 5 m along it; `#390`, `#393` and `#396` linear
//! placements 1 m, 10 m and 30 m along it (their
//! `IfcAxis2PlacementLinear`s `#389`, `#392` and `#395`). The engine's
//! variant beside an `IfcOffsetCurve2D` of the ellipse, an offset of an
//! offset refused while lowering, is inline.
//!
//! Through the reference kernel (`axiolid-evaluate` 0.3.13) a windowed
//! station along `RING` resolves only before its first stated offset. Its
//! joints are certified to a thousandth of the window, and the kernel
//! certifies no varying offset law along an ellipse (`UNCERTIFIED_LENGTH`)
//! and the constant piece after it not within its panel budget: the deck
//! and the placements past 2 m are refused by those names
//! (`SeamWindowRefusal::UncertifiedLength`, `CertificationBudgetExceeded`;
//! the upstream gap is #443). The deck's own geometry is checked with its
//! windows set to zero, where the kernel reads its sections as they stand.

use axiolid_model::{CurveRelation, GeometryNode, RunStation, SeamSide, SolidOperation};
use ifc_model::EntityId;

use super::common::{lower, refused, root};
use super::offset_bases::{bases, PRECISION};
use super::seams::item;

/// The deck along `RING` lowers: two sections at 0 and 5 m, each at its own
/// distance with the precision as its window, reading the incoming side,
/// along the run, which is continued to the ellipse's start and end.
#[test]
fn a_deck_along_an_offset_run_to_its_basis_ends_lowers_with_windows() {
    let model = bases();
    let body = item(&model, "RING_DECK", "Body");
    let lowered = lower(&model, body).expect("RING_DECK lowers");
    let GeometryNode::SolidOperation(SolidOperation::SectionsAtStations {
        directrix,
        sections,
        ..
    }) = root(&lowered)
    else {
        panic!("a run of sections: {:?}", root(&lowered));
    };
    let Some(GeometryNode::CurveRelation(CurveRelation::OffsetsAtStations { stations, .. })) =
        lowered.graph.get(*directrix)
    else {
        panic!("along the offset run: {:?}", lowered.graph.get(*directrix));
    };
    let ends: Vec<_> = stations
        .iter()
        .map(|station| (station.basis_end, station.station.distance))
        .collect();
    assert_eq!(stations.len(), 4, "{ends:?}");
    assert_eq!(
        stations[0],
        RunStation::at_basis_start(stations[1].station.offsets)
    );
    assert_eq!(
        stations[3],
        RunStation::at_basis_end(stations[2].station.offsets)
    );
    assert_eq!(
        (stations[1].station.distance, stations[2].station.distance),
        (2.0, 20.0)
    );
    let at: Vec<f64> = sections.iter().map(|s| s.station.distance).collect();
    assert_eq!(at, [0.0, 5.0]);
    for section in sections {
        assert_eq!(section.seam, SeamSide::Incoming);
        assert_eq!(section.seam_window, PRECISION, "the precision");
    }
    // The run itself, continued to the ellipse's ends, and the deck along
    // it.
    assert_eq!(
        lowered.provenance.seam_window_runs().collect::<Vec<_>>(),
        [EntityId(369), EntityId(body)],
        "the runs recorded"
    );
}

/// The engine's deck beside an `IfcOffsetCurve2D` 1.5 m inside the
/// ellipse is along an offset of an offset, which Axiolid measures no
/// station along (axiolid/kernel#289): refused while lowering by that
/// name, never by the catch-all for relations the kernel does not read.
/// The run itself lowers, as a curve. So are an offset run of an offset
/// run and a trim of a run refused by their own names, while a point along
/// the run lowers with the kernel's window.
#[test]
fn no_offset_relation_falls_to_the_catch_all() {
    let model = super::common::step(
        "#30=IFCCARTESIANPOINT((30.,0.));
#31=IFCDIRECTION((1.,0.));
#32=IFCAXIS2PLACEMENT2D(#30,#31);
#33=IFCELLIPSE(#32,10.,6.);
#34=IFCOFFSETCURVE2D(#33,1.5,.F.);
#35=IFCPOINTBYDISTANCEEXPRESSION(IFCLENGTHMEASURE(2.),0.5,$,$,#34);
#36=IFCPOINTBYDISTANCEEXPRESSION(IFCLENGTHMEASURE(20.),0.75,$,$,#34);
#37=IFCOFFSETCURVEBYDISTANCES(#34,(#35,#36),$);
#38=IFCCARTESIANPOINT((0.,0.));
#39=IFCAXIS2PLACEMENT2D(#38,$);
#40=IFCRECTANGLEPROFILEDEF(.AREA.,'deck',#39,2.,1.);
#41=IFCPOINTBYDISTANCEEXPRESSION(IFCLENGTHMEASURE(0.),$,$,$,#37);
#42=IFCAXIS2PLACEMENTLINEAR(#41,$,$);
#43=IFCPOINTBYDISTANCEEXPRESSION(IFCLENGTHMEASURE(5.),$,$,$,#37);
#44=IFCAXIS2PLACEMENTLINEAR(#43,$,$);
#45=IFCSECTIONEDSOLIDHORIZONTAL(#37,(#40,#40),(#42,#44));
#10=IFCCARTESIANPOINT((30.,0.));
#11=IFCDIRECTION((1.,0.));
#12=IFCAXIS2PLACEMENT2D(#10,#11);
#13=IFCELLIPSE(#12,10.,6.);
#14=IFCPOINTBYDISTANCEEXPRESSION(IFCLENGTHMEASURE(2.),0.5,$,$,#13);
#15=IFCPOINTBYDISTANCEEXPRESSION(IFCLENGTHMEASURE(20.),0.75,$,$,#13);
#16=IFCOFFSETCURVEBYDISTANCES(#13,(#14,#15),$);
#17=IFCPOINTBYDISTANCEEXPRESSION(IFCLENGTHMEASURE(5.),0.25,$,$,#16);
#18=IFCOFFSETCURVEBYDISTANCES(#16,(#17),$);
#19=IFCPOINTBYDISTANCEEXPRESSION(IFCLENGTHMEASURE(1.),$,$,$,#18);
#20=IFCPOINTBYDISTANCEEXPRESSION(IFCLENGTHMEASURE(1.),$,$,$,#16);
#21=IFCTRIMMEDCURVE(#16,(IFCPARAMETERVALUE(0.)),(IFCPARAMETERVALUE(1.)),.T.,.PARAMETER.);
#22=IFCPOINTBYDISTANCEEXPRESSION(IFCLENGTHMEASURE(0.5),$,$,$,#21);",
        false,
    );
    let error = lower(&model, 45).map(|_| ()).expect_err("the nested deck");
    let text = error.to_string();
    assert!(error.is_unsupported(), "{text}");
    assert!(text.contains("offset of an offset curve"), "{text}");
    assert!(text.contains("axiolid/kernel#289"), "{text}");
    assert!(!text.contains("parameter-space"), "{text}");
    lower(&model, 37).expect("the nested run lowers");
    // An offset run of an offset run.
    refused(lower(&model, 19), true, "offset of an offset curve");
    // A point along the run itself lowers with the kernel's window.
    let lowered = lower(&model, 20).expect("a point on the run lowers");
    super::common::assert_windowed(&lowered, 1.0, 1e-5);
    // A trim of it.
    refused(lower(&model, 22), true, "trim of an offset curve");
}

#[cfg(feature = "compile-reference-backend")]
mod kernel {
    use axiolid_contracts::ExecutionOptions;
    use axiolid_core::{Point3, Vec3};
    use axiolid_curve_evaluate_contract::{CurveEvaluator, CurveMeasure};
    use axiolid_evaluate::ReferenceCurveEvaluator;
    use axiolid_mesh_compile::station::{curve_path, resolve};
    use ifc_geometry::compile::{compile_product_mesh, SeamWindowRefusal, Tolerance};
    use ifc_geometry::constraint::placement::derive::{
        basis_curve_path, derive_linear_placement_transform,
    };
    use ifc_geometry::lower::{lower_axis2_placement_linear_node, LoweringSession};
    use ifc_geometry::transform::Transform;
    use ifc_geometry::{units, GeometryError};
    use ifc_model::{EntityId, Model};

    use super::super::common::lower;
    use super::super::offset_bases::{bases, product};

    /// The deck along `RING` lowers, and the reference kernel refuses its
    /// sections' windows as a typed error naming the runs: the joint the
    /// section at 5 m must be shown clear of follows the run's varying
    /// piece beside the ellipse, whose length `axiolid-evaluate` 0.3.13
    /// does not certify (#443).
    #[test]
    fn the_ring_deck_is_refused_by_the_kernels_certification() {
        let model = bases();
        let id = product(&model, "RING_DECK");
        let error = compile_product_mesh(&model, id, Tolerance::MILLIMETRE).expect_err("refused");
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
            (id, SeamWindowRefusal::UncertifiedLength)
        );
        assert_eq!(runs, &[EntityId(369), EntityId(380)]);
        assert!(stations.is_empty(), "{stations:?}");
        assert!(error.is_unsupported());
        assert!(!error.to_string().contains("parameter-space"), "{error}");
    }

    fn signed_volume(mesh: &axiolid_mesh::TriMesh) -> f64 {
        let base = mesh.positions[0];
        mesh.indices
            .chunks_exact(3)
            .map(|t| {
                let [a, b, c] = [t[0], t[1], t[2]].map(|i| mesh.positions[i as usize] - base);
                a.dot(b.cross(c)) / 6.0
            })
            .sum()
    }

    /// axioval/engine's deck, its sections' windows set to zero so the
    /// kernel reads them as they stand, compiles to the prism the lowering
    /// states. The centred 2 x 1 m rectangle swept 5 m along the run in its
    /// own length holds 2 x 5 = 10 m3 by Pappus (the half-angle mitre at
    /// the run's joint, where its lateral starts to change from 0.5 m to
    /// 0.75 m, neither gains nor loses for a centred section). The mesh
    /// follows the curved run by chords within the 1 mm tolerance, so its
    /// volume lies within the deck's surface area times 1 mm of that. Its
    /// first section stands 0.5 m inside the ellipse's start (40, 0),
    /// across the run's tangent +Y: its corners are (39.5 -+ 1, 0, -+0.5).
    /// Every vertex lies in the ellipse's quarter beside its start, at most
    /// 0.5 m off its plane.
    #[cfg(feature = "wire")]
    #[test]
    fn the_ring_deck_without_windows_is_the_prism_it_states() {
        use axiolid_mesh_compile_contract::{MeshClosure, MeshCompiler};
        use axiolid_model::GeometryGraph;
        use ifc_geometry::compile::default_backend;

        use super::super::seams::item;

        let model = bases();
        let lowered = lower(&model, item(&model, "RING_DECK", "Body")).expect("lowers");
        let json = lowered.graph.to_json().expect("writes");
        assert_eq!(json.matches("\"seam_window\":0.00001").count(), 2, "{json}");
        let graph = GeometryGraph::from_json(
            &json.replace("\"seam_window\":0.00001", "\"seam_window\":0.0"),
        )
        .expect("reads back");
        let options = ExecutionOptions::new(Tolerance::MILLIMETRE);
        let outcome = default_backend()
            .compile_mesh_reported(&graph, graph.roots()[0], &options)
            .unwrap_or_else(|e| panic!("compiles without windows: {e:?}"));
        assert_eq!(outcome.closure, MeshClosure::Solid);
        let mesh = outcome.mesh;
        let (area, length) = (2.0, 5.0);
        let expected = area * length;
        let bound = (2.0 * (2.0 + 1.0) * length + 2.0 * area) * 1e-3;
        let volume = signed_volume(&mesh);
        assert!(
            (volume - expected).abs() <= bound,
            "volume {volume} != {expected} within {bound}"
        );
        for corner in [
            [38.5, 0.0, -0.5],
            [38.5, 0.0, 0.5],
            [40.5, 0.0, -0.5],
            [40.5, 0.0, 0.5],
        ] {
            assert!(
                mesh.positions
                    .iter()
                    .any(|p| (*p - Point3::new(corner[0], corner[1], corner[2])).length() < 1e-9),
                "the first section's corner {corner:?}"
            );
        }
        for p in &mesh.positions {
            assert!(p.z.abs() <= 0.5 + 1e-9, "{p:?}");
            assert!((-1e-9..=6.0).contains(&p.y), "{p:?}");
            assert!((30.0..=40.5 + 1e-9).contains(&p.x), "{p:?}");
        }
    }

    /// `RING` read as a curve path for a derived placement is the path the
    /// reference compiler flattens it into, its last piece open to the
    /// ellipse's end: the evaluator reads both alike, bit for bit, before,
    /// between and past the run's stated offsets.
    #[test]
    fn the_ring_reads_as_the_compilers_path() {
        let model = bases();
        let scale = units::resolve(&model);
        let evaluator = ReferenceCurveEvaluator::default();
        let ours = basis_curve_path(&model, &scale, EntityId(390), EntityId(369), &evaluator)
            .expect("the ring's path");
        assert_eq!(ours.pieces().len(), 3);
        let lowered = lower(&model, 369).expect("the ring lowers");
        let theirs = curve_path(&lowered.graph, lowered.root).expect("the compiler's path");
        assert_eq!(theirs.pieces().len(), 3);
        for distance in [1.0, 10.0, 30.0] {
            let at = CurveMeasure::Distance(distance);
            let a = evaluator.path_frame_at(&ours, at).expect("ours");
            let b = evaluator
                .path_frame_at(&theirs, at)
                .expect("the compiler's");
            assert_eq!((a.origin, a.x, a.y), (b.origin, b.x, b.y), "at {distance}");
        }
    }

    /// The station `axes` lowered and resolved by the reference kernel.
    fn kernel_station(
        model: &Model,
        axes: u64,
    ) -> Result<axiolid_mesh_compile::station::ResolvedStation, axiolid_contracts::GeomError> {
        let scale = units::resolve(model);
        let mut session = LoweringSession::new(model, &scale);
        let root =
            lower_axis2_placement_linear_node(&mut session, EntityId(axes), Transform::identity())
                .expect("the station lowers");
        let lowered = session.finish(root).expect("a graph");
        resolve(&lowered.graph, lowered.root)
    }

    /// The placement 1 m along `RING`, before its first stated offset,
    /// derives (#427) and equals the frame the reference kernel resolves
    /// its lowered station to: the one joint its window must be shown
    /// clear of ends a constant offset beside the ellipse, which the kernel
    /// certifies. Past it, where that joint follows the varying piece, the
    /// placements 10 m and 30 m along are refused by name, as the kernel
    /// refuses their stations (#443).
    #[test]
    fn linear_placements_on_the_ring_derive_or_are_refused_by_name() {
        let model = bases();
        let scale = units::resolve(&model);
        let evaluator = ReferenceCurveEvaluator::default();
        let derive = |placement: u64| {
            derive_linear_placement_transform(&model, &scale, EntityId(placement), &evaluator)
        };
        let derived = derive(390).unwrap_or_else(|e| panic!("#390 derives: {e}"));
        let station = kernel_station(&model, 389).expect("#389 resolves");
        let close = |a: [f64; 3], b: Vec3, what: &str| {
            let off = (a[0] - b.x)
                .abs()
                .max((a[1] - b.y).abs())
                .max((a[2] - b.z).abs());
            assert!(off < 1e-9, "{what}: {a:?} != {b:?}");
        };
        close(derived.origin, station.point - Point3::ZERO, "origin");
        close(derived.basis[0], station.section.tangent, "tangent");
        close(derived.basis[1], station.section.lateral, "left");
        close(derived.basis[2], station.section.up, "up");
        for placement in [393, 396] {
            let error = derive(placement).expect_err("refused");
            let GeometryError::PlacementSeamWindowRefused {
                placement: refused,
                basis,
                refusal,
                ..
            } = error
            else {
                panic!("#{placement}: {error:?}");
            };
            assert_eq!(
                (refused, basis, refusal),
                (
                    EntityId(placement),
                    EntityId(369),
                    SeamWindowRefusal::CertificationBudgetExceeded
                ),
                "#{placement}"
            );
            assert!(error.is_unsupported(), "#{placement}");
        }
        // The kernel refuses the same stations: the one 10 m along by the
        // varying piece it does not certify, the one 30 m along by its
        // panel budget.
        let uncertified = kernel_station(&model, 392).expect_err("#392");
        assert!(
            uncertified
                .to_string()
                .contains("an offset by distances along an ellipse"),
            "{uncertified}"
        );
        let budget = kernel_station(&model, 395).expect_err("#395");
        assert!(
            matches!(
                budget,
                axiolid_contracts::GeomError::BudgetExceeded {
                    resource: "certified length panels"
                }
            ),
            "{budget:?}"
        );
    }
}
