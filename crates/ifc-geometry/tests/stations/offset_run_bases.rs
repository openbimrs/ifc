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
//! Through the reference kernel (`axiolid-evaluate` 0.3.15,
//! `axiolid-mesh-compile` 0.3.25) the deck compiles and every placement
//! derives: the kernel certifies the run's varying piece beside the
//! ellipse, its law read in the ellipse's arc length, and the constant
//! piece after it to a thousandth of the window, within its panel budget
//! (axiolid/kernel#307). With 0.3.13 they were refused as
//! `SeamWindowRefusal::UncertifiedLength` and `CertificationBudgetExceeded`
//! (#444). The windows change nothing here: the deck compiles to the same
//! mesh, bit for bit, as with its windows set to zero, where the kernel
//! reads its sections as they stand.

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

/// A curve, a station on it, and the name it is refused by, if any.
type OffsetCase = (u64, u64, Option<&'static str>);

/// Every offset relation a station basis can lower to, alone and nested,
/// as `(curve, station, expected)`: the station is a point 1 m along the
/// curve, and `expected` is `None` where it lowers, else the name it is
/// refused by. The single offsets lower to `CurveRelation::Offset` (2D and
/// 3D), `OffsetByStations` (offsets stated to a polyline's ends) and
/// `OffsetsAtStations` (offsets stopping short of an ellipse's ends, #433);
/// every offset of one of them is an offset of an offset
/// (axiolid/kernel#289), every trim of one a trim of an offset.
fn every_offset_relation() -> (ifc_model::Model, Vec<OffsetCase>) {
    let mut records = String::from(
        "#10=IFCCARTESIANPOINT((30.,0.));
#11=IFCDIRECTION((1.,0.));
#12=IFCAXIS2PLACEMENT2D(#10,#11);
#13=IFCELLIPSE(#12,10.,6.);
#14=IFCCARTESIANPOINT((0.,0.));
#15=IFCCARTESIANPOINT((100.,0.));
#16=IFCPOLYLINE((#14,#15));
#17=IFCCARTESIANPOINT((0.,0.,0.));
#18=IFCCARTESIANPOINT((100.,0.,0.));
#19=IFCPOLYLINE((#17,#18));
#20=IFCDIRECTION((0.,0.,1.));
",
    );
    let next = std::cell::Cell::new(100_u64);
    let id = || {
        next.set(next.get() + 1);
        next.get()
    };
    // An offset of `basis` of each kind its dimension admits.
    let offsets_of = |records: &mut String, basis: u64, three_d: bool| {
        let mut out = Vec::new();
        if three_d {
            let o = id();
            records.push_str(&format!("#{o}=IFCOFFSETCURVE3D(#{basis},0.25,.F.,#20);\n"));
            out.push(o);
        } else {
            let o = id();
            records.push_str(&format!("#{o}=IFCOFFSETCURVE2D(#{basis},0.25,.F.);\n"));
            out.push(o);
        }
        // Stated short of the ends: `OffsetsAtStations` along a quadrature
        // basis, a stated continuation along a stated one.
        let (a, b, o) = (id(), id(), id());
        records.push_str(&format!(
            "#{a}=IFCPOINTBYDISTANCEEXPRESSION(IFCLENGTHMEASURE(2.),0.5,$,$,#{basis});
#{b}=IFCPOINTBYDISTANCEEXPRESSION(IFCLENGTHMEASURE(20.),0.75,$,$,#{basis});
#{o}=IFCOFFSETCURVEBYDISTANCES(#{basis},(#{a},#{b}),$);\n"
        ));
        out.push(o);
        out
    };
    let mut cases = Vec::new();
    let mut station = |records: &mut String, curve: u64, expected| {
        let s = id();
        records.push_str(&format!(
            "#{s}=IFCPOINTBYDISTANCEEXPRESSION(IFCLENGTHMEASURE(1.),$,$,$,#{curve});\n"
        ));
        cases.push((curve, s, expected));
    };
    let mut singles = Vec::new();
    for (basis, three_d) in [(13, false), (16, false), (19, true)] {
        singles.extend(
            offsets_of(&mut records, basis, three_d)
                .into_iter()
                .map(|o| (o, three_d)),
        );
    }
    // Offsets stated to the polyline's ends: `OffsetByStations`.
    records.push_str(
        "#90=IFCPOINTBYDISTANCEEXPRESSION(IFCLENGTHMEASURE(0.),0.5,$,$,#16);
#91=IFCPOINTBYDISTANCEEXPRESSION(IFCLENGTHMEASURE(100.),0.75,$,$,#16);
#92=IFCOFFSETCURVEBYDISTANCES(#16,(#90,#91),$);\n",
    );
    singles.push((92, false));
    for &(single, three_d) in &singles {
        station(&mut records, single, None);
        for outer in offsets_of(&mut records, single, three_d) {
            station(&mut records, outer, Some("offset of an offset curve"));
        }
        let trim = id();
        records.push_str(&format!(
            "#{trim}=IFCTRIMMEDCURVE(#{single},(IFCPARAMETERVALUE(0.)),(IFCPARAMETERVALUE(1.)),.T.,.PARAMETER.);\n"
        ));
        station(&mut records, trim, Some("trim of an offset curve"));
    }
    (super::common::step(&records, false), cases)
}

/// No offset relation, alone, nested or trimmed, reaches the catch-all
/// reason for relations the kernel does not read (#443): a station along
/// each single offset lowers, and one along an offset of an offset or a
/// trim of an offset is refused by its own name. Every offset relation a
/// station basis lowers to is among them.
#[test]
fn the_catch_all_fires_for_no_offset_relation() {
    let (model, cases) = every_offset_relation();
    let mut kinds = std::collections::BTreeSet::new();
    for (curve, station, expected) in cases {
        let result = lower(&model, station);
        match expected {
            None => {
                let lowered =
                    result.unwrap_or_else(|e| panic!("a station on #{curve} lowers: {e}"));
                for (_, node) in lowered.graph.iter() {
                    if let GeometryNode::CurveRelation(relation) = node {
                        kinds.insert(match relation {
                            CurveRelation::Offset { .. } => "Offset",
                            CurveRelation::OffsetByStations { .. } => "OffsetByStations",
                            CurveRelation::OffsetsAtStations { .. } => "OffsetsAtStations",
                            _ => "other",
                        });
                    }
                }
            }
            Some(needle) => {
                let error = result.map(|_| ()).expect_err(needle);
                let text = error.to_string();
                assert!(error.is_unsupported(), "#{curve}: {text}");
                assert!(text.contains(needle), "#{curve}: {text}");
                assert!(!text.contains("parameter-space"), "#{curve}: {text}");
            }
        }
    }
    for kind in ["Offset", "OffsetByStations", "OffsetsAtStations"] {
        assert!(kinds.contains(kind), "{kind} among {kinds:?}");
    }
}

#[cfg(feature = "compile-reference-backend")]
mod kernel {
    use axiolid_contracts::ExecutionOptions;
    use axiolid_core::{Point3, Vec3};
    use axiolid_curve_evaluate_contract::{CurveEvaluator, CurveMeasure};
    use axiolid_evaluate::ReferenceCurveEvaluator;
    use axiolid_mesh_compile::station::{curve_path, resolve};
    use axiolid_mesh_compile::DeviationBound;
    use ifc_geometry::compile::{self, compile_product_mesh, SeamWindowRefusal, Tolerance};
    use ifc_geometry::constraint::placement::derive::{
        basis_curve_path, derive_linear_placement_transform,
    };
    use ifc_geometry::lower::{lower_axis2_placement_linear_node, LoweringSession};
    use ifc_geometry::transform::Transform;
    use ifc_geometry::{units, GeometryError};
    use ifc_model::{EntityId, Model};

    use super::super::common::lower;
    use super::super::offset_bases::{bases, product};

    /// The deck along `RING` compiles through the reference kernel, its
    /// sections' windows resolved: the joints the section at 5 m must be
    /// shown clear of follow the run's varying piece beside the ellipse,
    /// which `axiolid-evaluate` 0.3.15 certifies (axiolid/kernel#307). It
    /// is a closed solid of 2 m2 x 5 m by Pappus, within the deck's surface
    /// area times the 1 mm tolerance. Its deviation from the exact solid is
    /// not certified. Since `axiolid-mesh-compile` 0.3.28 the frame law
    /// along the ellipse's offset, read by quadrature, is bounded
    /// (axiolid/kernel#316); what stays unbounded is the cell beside the
    /// mitre the deck's widening law makes at station 2, a 0.8 degree
    /// corner on a curved piece (`MITRED_SECTIONS`, axiolid/kernel#335).
    /// The mesh and its volume are unchanged.
    #[test]
    fn the_ring_deck_compiles_through_the_kernel() {
        let model = bases();
        let id = product(&model, "RING_DECK");
        let mesh = compile_product_mesh(&model, id, Tolerance::MILLIMETRE)
            .unwrap_or_else(|e| panic!("the deck compiles: {e}"))
            .expect("a body");
        let (area, length) = (2.0, 5.0);
        let bound = (2.0 * (2.0 + 1.0) * length + 2.0 * area) * 1e-3;
        let volume = signed_volume(&mesh);
        assert!(
            (volume - area * length).abs() <= bound,
            "volume {volume} != {} within {bound}",
            area * length
        );
        let lowered = lower(
            &model,
            super::super::seams::item(&model, "RING_DECK", "Body"),
        )
        .expect("lowers");
        let (outcome, report) = compile::default_backend()
            .compile_mesh_with_deviation(
                &lowered.graph,
                lowered.root,
                &ExecutionOptions::new(Tolerance::MILLIMETRE),
            )
            .unwrap_or_else(|e| panic!("compiles with a report: {e:?}"));
        assert_eq!(outcome.mesh.positions, mesh.positions, "the same mesh");
        assert_eq!(report.bound, None, "{report:?}");
        assert!(
            report.unbounded().any(|c| matches!(
                c.bound,
                DeviationBound::Unbounded(reason)
                    if reason.contains("a cell beside a mitred seam on a curved piece")
            )),
            "{report:?}"
        );
        assert!(
            !report.unbounded().any(|c| matches!(
                c.bound,
                DeviationBound::Unbounded(reason)
                    if reason.contains("frame law along this basis has no certified derivative bounds")
            )),
            "the frame law is certified (axiolid/kernel#316): {report:?}"
        );
    }

    fn signed_volume(mesh: &axiolid_mesh::TriMesh) -> f64 {
        let base = mesh.positions[0];
        mesh.indices
            .as_chunks::<3>()
            .0
            .iter()
            .map(|t| {
                let [a, b, c] = [t[0], t[1], t[2]].map(|i| mesh.positions[i as usize] - base);
                a.dot(b.cross(c)) / 6.0
            })
            .sum()
    }

    /// axioval/engine's deck compiles to the prism the lowering states,
    /// and to the same mesh, bit for bit, as with its sections' windows set
    /// to zero, where the kernel reads them as they stand: no seam lies
    /// within 1e-5 m of 0 or 5 m along the run. The centred 2 x 1 m
    /// rectangle swept 5 m along the run in its own length holds 2 x 5 =
    /// 10 m3 by Pappus (the half-angle mitre at the run's joint, where its
    /// lateral starts to change from 0.5 m to 0.75 m, neither gains nor
    /// loses for a centred section). The mesh follows the curved run by
    /// chords within the 1 mm tolerance, so its volume lies within the
    /// deck's surface area times 1 mm of that. Its first section stands
    /// 0.5 m inside the ellipse's start (40, 0), across the run's tangent
    /// +Y: its corners are (39.5 -+ 1, 0, -+0.5). Every vertex lies in the
    /// ellipse's quarter beside its start, at most 0.5 m off its plane.
    #[cfg(feature = "wire")]
    #[test]
    fn the_ring_deck_is_the_prism_it_states_with_or_without_windows() {
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
        let backend = default_backend();
        let outcome = backend
            .compile_mesh_reported(&graph, graph.roots()[0], &options)
            .unwrap_or_else(|e| panic!("compiles without windows: {e:?}"));
        assert_eq!(outcome.closure, MeshClosure::Solid);
        let mesh = outcome.mesh;
        let windowed = backend
            .compile_mesh_reported(&lowered.graph, lowered.root, &options)
            .unwrap_or_else(|e| panic!("compiles with windows: {e:?}"));
        assert_eq!(windowed.closure, MeshClosure::Solid);
        assert_eq!(windowed.mesh.indices, mesh.indices, "the same triangles");
        assert_eq!(
            windowed.mesh.positions, mesh.positions,
            "the same vertices, bit for bit"
        );
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

    /// The placements 1 m, 10 m and 30 m along `RING`, before, inside and
    /// past its varying piece, derive (#427, #436) and equal the frames the
    /// reference kernel resolves their lowered stations to: the joints
    /// their windows must be shown clear of follow a constant offset and
    /// the varying one beside the ellipse, which `axiolid-evaluate` 0.3.15
    /// certifies to a thousandth of the window (axiolid/kernel#307; 0.3.13
    /// refused the last two, #444).
    #[test]
    fn linear_placements_on_the_ring_derive_as_the_kernel_resolves_them() {
        let model = bases();
        let scale = units::resolve(&model);
        let evaluator = ReferenceCurveEvaluator::default();
        let close = |a: [f64; 3], b: Vec3, what: &str| {
            let off = (a[0] - b.x)
                .abs()
                .max((a[1] - b.y).abs())
                .max((a[2] - b.z).abs());
            assert!(off < 1e-9, "{what}: {a:?} != {b:?}");
        };
        for (placement, axes) in [(390, 389), (393, 392), (396, 395)] {
            let derived =
                derive_linear_placement_transform(&model, &scale, EntityId(placement), &evaluator)
                    .unwrap_or_else(|e| panic!("#{placement} derives: {e}"));
            let station =
                kernel_station(&model, axes).unwrap_or_else(|e| panic!("#{axes} resolves: {e:?}"));
            let at = format!("#{placement}");
            close(derived.origin, station.point - Point3::ZERO, &at);
            close(derived.basis[0], station.section.tangent, &at);
            close(derived.basis[1], station.section.lateral, &at);
            close(derived.basis[2], station.section.up, &at);
        }
    }

    /// A placement just past the joint of a composite whose first piece is
    /// a cubic B-spline zig-zagging 0.5 m either side across 40 control
    /// points 0.1 m apart: the evaluator bounds that piece's derivatives
    /// only coarsely, so certifying its length to a thousandth of the
    /// 1e-5 m window needs more panels than its budget. The derivation
    /// refuses it as `CertificationBudgetExceeded`, and the kernel refuses
    /// the lowered station with the `BudgetExceeded` resource
    /// `compile::window` repeats, which pins that private name (#443).
    #[test]
    fn a_certification_past_the_panel_budget_is_refused_by_name() {
        let (n, dx, amplitude) = (40_usize, 0.1_f64, 0.5_f64);
        let side = |i: usize| {
            if i.is_multiple_of(2) {
                amplitude
            } else {
                -amplitude
            }
        };
        let mut records = String::new();
        let mut points = Vec::new();
        let mut polygon = 0.0;
        for i in 0..n {
            records += &format!(
                "#{}=IFCCARTESIANPOINT(({:?},{:?}));\n",
                1000 + i,
                i as f64 * dx,
                side(i)
            );
            points.push(format!("#{}", 1000 + i));
            if i > 0 {
                polygon += dx.hypot(2.0 * amplitude);
            }
        }
        let mut multiplicities = vec!["4".to_string()];
        let mut knots = vec!["0.".to_string()];
        for k in 1..n - 3 {
            multiplicities.push("1".into());
            knots.push(format!("{k}."));
        }
        multiplicities.push("4".into());
        knots.push(format!("{}.", n - 3));
        let (end_x, end_y) = ((n - 1) as f64 * dx, side(n - 1));
        // The B-spline is no longer than its control polygon, so 1 m past
        // the polygon's length lies on the line after the joint.
        records += &format!(
            "#60=IFCBSPLINECURVEWITHKNOTS(3,({}),.UNSPECIFIED.,.F.,.F.,({}),({}),.UNSPECIFIED.);
#61=IFCCARTESIANPOINT(({end_x:?},{end_y:?}));
#62=IFCCARTESIANPOINT(({:?},{end_y:?}));
#63=IFCPOLYLINE((#61,#62));
#64=IFCCOMPOSITECURVESEGMENT(.CONTINUOUS.,.T.,#60);
#65=IFCCOMPOSITECURVESEGMENT(.CONTINUOUS.,.T.,#63);
#66=IFCCOMPOSITECURVE((#64,#65),.F.);
#41=IFCPOINTBYDISTANCEEXPRESSION(IFCLENGTHMEASURE({:?}),$,$,$,#66);
#42=IFCAXIS2PLACEMENTLINEAR(#41,$,$);
#43=IFCLINEARPLACEMENT($,#42,$);",
            points.join(","),
            multiplicities.join(","),
            knots.join(","),
            end_x + 100.0,
            polygon + 1.0
        );
        let model = super::super::common::step(&records, false);
        let scale = units::resolve(&model);
        let evaluator = ReferenceCurveEvaluator::default();
        let error = derive_linear_placement_transform(&model, &scale, EntityId(43), &evaluator)
            .expect_err("refused");
        let GeometryError::PlacementSeamWindowRefused {
            placement,
            basis,
            refusal,
            ..
        } = error
        else {
            panic!("{error:?}");
        };
        assert_eq!(
            (placement, basis, refusal),
            (
                EntityId(43),
                EntityId(66),
                SeamWindowRefusal::CertificationBudgetExceeded
            )
        );
        assert!(error.is_unsupported(), "{error}");
        let station = kernel_station(&model, 42).expect_err("the kernel refuses it");
        assert!(
            matches!(
                station,
                axiolid_contracts::GeomError::BudgetExceeded {
                    resource: "certified length panels"
                }
            ),
            "{station:?}"
        );
    }
}
