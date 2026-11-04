//! An arbitrary profile's outline with its arcs sampled by the backend (#660).
//!
//! The slab-like outline is `(0,0) (4,0) (5,1) (4,2) (0,2) (-1,1)` in
//! metres: two lines and two arcs of radius 1 about `(4,1)` and `(0,1)`.
//! `IfcArcIndex` segments are the file's own circles, so every sample must
//! lie on them, while each authored point stays exactly the file's value.

#![cfg(feature = "compile-reference-backend")]

use std::f64::consts::TAU;

use axiolid_contracts::{
    Backend, BackendDescriptor, BackendId, ExecutionOptions, ExecutionTarget, GeomError, GeomResult,
};
use axiolid_mesh_compile_contract::{PolylineCompiler, PolylineOutcome};
use axiolid_model::{GeometryGraph, NodeId};
use ifc_geometry::compile::{
    compile_profile_outline, compile_profile_outline_with, default_backend, SampledProfileOutline,
    SampledRing, Tolerance,
};
use ifc_geometry::{profile_outline, units, GeometryError};
use ifc_model::{EntityId, Model};

// Shared with the body suites; this one uses part of it.
#[allow(dead_code)]
#[path = "body_description/step.rs"]
mod step;

const SLAB_POINTS: &str = "((0.,0.),(4.,0.),(5.,1.),(4.,2.),(0.,2.),(-1.,1.))";
const SLAB_SEGMENTS: &str = "IFCLINEINDEX((1,2)),IFCARCINDEX((2,3,4)),IFCLINEINDEX((4,5)),\
                             IFCARCINDEX((5,6,1))";

/// `#100`: the slab boundary, `#101`: its profile; `extra` adds entities.
fn slab(prefix: &str, points: &str, segments: &str, extra: &str) -> Model {
    let entities = format!(
        "#100=IFCCARTESIANPOINTLIST2D({points});
#101=IFCINDEXEDPOLYCURVE(#100,({segments}),$);
#102=IFCARBITRARYCLOSEDPROFILEDEF(.AREA.,$,#101);
{extra}"
    );
    step::model("IFC4", prefix, "1.,0.,0.", &entities, "")
}

fn sample(model: &Model, tolerance: Tolerance) -> SampledProfileOutline {
    compile_profile_outline(model, EntityId(102), tolerance).expect("the outline samples")
}

fn angle_about(centre: [f64; 2], p: [f64; 2]) -> f64 {
    (p[1] - centre[1]).atan2(p[0] - centre[0])
}

/// The arc's vertices from its authored start to its authored end.
fn arc_vertices(ring: &SampledRing, arc: usize) -> Vec<[f64; 2]> {
    let arc = &ring.arcs[arc];
    (0..arc.samples + 2)
        .map(|i| ring.vertices[(arc.start + i) % ring.vertices.len()])
        .collect()
}

#[test]
fn closing_arc_samples_lie_on_the_circle_and_authored_points_are_exact() {
    let model = slab("$", SLAB_POINTS, SLAB_SEGMENTS, "");
    let tolerance = Tolerance::MILLIMETRE;
    let outline = sample(&model, tolerance);
    let ring = &outline.outer;
    assert_eq!(outline.profile, EntityId(102));
    assert_eq!(ring.curve, EntityId(101));
    assert!(outline.inner.is_empty());
    assert_eq!(ring.arcs.len(), 2);
    assert_eq!(ring.arcs[0].segment, 1);
    assert_eq!(ring.arcs[1].segment, 3);
    assert!(ring.arcs.iter().all(|arc| arc.samples >= 1));
    assert!(ring.deviation.meets_requested());
    assert_eq!(ring.deviation.requested, tolerance.linear());
    assert!(ring.arcs.iter().all(|arc| arc.bound <= 1e-3));
    assert_eq!(
        ring.deviation.bound,
        ring.arcs.iter().map(|a| a.bound).fold(0.0, f64::max)
    );

    // The second arc ends the ring: (0,0) is vertex 0 and is not repeated.
    let last = &ring.arcs[1];
    assert_eq!(last.start + last.samples + 1, ring.vertices.len());
    assert_eq!(ring.vertices[0], [0., 0.]);
    assert_eq!(ring.vertices.iter().filter(|v| **v == [0., 0.]).count(), 1);

    // Every authored segment end point once, bitwise.
    let first = &ring.arcs[0];
    assert_eq!(ring.vertices[1], [4., 0.]);
    assert_eq!(first.start, 1);
    assert_eq!(ring.vertices[first.start + first.samples + 1], [4., 2.]);
    assert_eq!(ring.vertices[last.start], [0., 2.]);

    for (arc, centre) in [(0, [4., 1.]), (1, [0., 1.])] {
        let vertices = arc_vertices(ring, arc);
        let start = angle_about(centre, vertices[0]);
        let mut previous = 0.0;
        for p in &vertices {
            let r = (p[0] - centre[0]).hypot(p[1] - centre[1]);
            assert!((r - 1.0).abs() < 1e-9, "radius {r} at {p:?}");
            let swept = (angle_about(centre, *p) - start).rem_euclid(TAU);
            assert!(swept >= previous, "angles must be monotone");
            previous = swept;
        }
        assert!((previous - std::f64::consts::PI).abs() < 1e-9);
    }
}

#[test]
fn a_finer_tolerance_samples_more_and_is_deterministic() {
    let model = slab("$", SLAB_POINTS, SLAB_SEGMENTS, "");
    let coarse = sample(&model, Tolerance::MILLIMETRE);
    let fine = sample(&model, Tolerance::new(1e-4, 1e-9).expect("valid tolerance"));
    for arc in 0..2 {
        assert!(fine.outer.arcs[arc].samples > coarse.outer.arcs[arc].samples);
    }
    assert!(fine.outer.deviation.meets_requested());
    assert_eq!(fine, sample(&model, Tolerance::new(1e-4, 1e-9).unwrap()));
}

#[test]
fn a_boundary_without_arcs_is_the_exact_outline() {
    let model = slab(
        "$",
        "((0.,0.),(4.,0.),(4.,2.),(0.,2.))",
        "IFCLINEINDEX((1,2,3)),IFCLINEINDEX((3,4,1))",
        "#110=IFCCARTESIANPOINT((0.,0.));
#111=IFCCARTESIANPOINT((4.,0.));
#112=IFCCARTESIANPOINT((4.,2.));
#113=IFCPOLYLINE((#110,#111,#112,#110));
#114=IFCARBITRARYCLOSEDPROFILEDEF(.AREA.,$,#113);",
    );
    let scale = units::resolve(&model).unwrap();
    for profile in [EntityId(102), EntityId(114)] {
        let exact = profile_outline(&model, &scale, profile).expect("exact outline");
        let sampled = compile_profile_outline(&model, profile, Tolerance::MILLIMETRE).unwrap();
        assert_eq!(sampled.outer.vertices, exact.outer);
        assert!(sampled.outer.arcs.is_empty());
        assert_eq!(sampled.outer.deviation.bound, 0.0);
        assert!(sampled.outer.deviation.meets_requested());
    }
}

#[test]
fn a_circular_hole_of_two_arcs_is_sampled() {
    let model = step::metres(
        "#100=IFCCARTESIANPOINTLIST2D(((-2.,-2.),(2.,-2.),(2.,2.),(-2.,2.)));
#101=IFCINDEXEDPOLYCURVE(#100,(IFCLINEINDEX((1,2,3,4,1))),$);
#102=IFCCARTESIANPOINTLIST2D(((1.,0.),(0.,1.),(-1.,0.),(0.,-1.)));
#103=IFCINDEXEDPOLYCURVE(#102,(IFCARCINDEX((1,2,3)),IFCARCINDEX((3,4,1))),$);
#104=IFCARBITRARYPROFILEDEFWITHVOIDS(.AREA.,$,#101,(#103));",
        "",
    );
    let outline = compile_profile_outline(&model, EntityId(104), Tolerance::MILLIMETRE).unwrap();
    assert_eq!(outline.outer.vertices.len(), 4);
    assert!(outline.outer.arcs.is_empty());
    assert_eq!(outline.inner.len(), 1);
    let hole = &outline.inner[0];
    assert_eq!(hole.curve, EntityId(103));
    assert_eq!(hole.arcs.len(), 2);
    assert_eq!(hole.vertices[0], [1., 0.]);
    for p in &hole.vertices {
        assert!((p[0].hypot(p[1]) - 1.0).abs() < 1e-9);
    }
    assert!(hole.deviation.meets_requested());
}

#[test]
fn a_millimetre_model_gives_metres() {
    let model = slab(
        ".MILLI.",
        "((0.,0.),(4000.,0.),(5000.,1000.),(4000.,2000.),(0.,2000.),(-1000.,1000.))",
        SLAB_SEGMENTS,
        "",
    );
    let metres = sample(
        &slab("$", SLAB_POINTS, SLAB_SEGMENTS, ""),
        Tolerance::MILLIMETRE,
    );
    let outline = sample(&model, Tolerance::MILLIMETRE);
    let scale = units::resolve(&model).unwrap();
    assert_eq!(outline.outer.vertices[1], [scale.length(4000.), 0.]);
    assert_eq!(outline.outer.vertices.len(), metres.outer.vertices.len());
    for (a, e) in outline.outer.vertices.iter().zip(&metres.outer.vertices) {
        assert!((a[0] - e[0]).abs() < 1e-9 && (a[1] - e[1]).abs() < 1e-9);
    }
}

fn degenerate(error: &GeometryError, entity: u64) {
    match error {
        GeometryError::Degenerate { entity: e, .. } => assert_eq!(*e, EntityId(entity), "{error}"),
        other => panic!("expected Degenerate, got {other:?}"),
    }
}

fn refusal(model: &Model, profile: u64) -> GeometryError {
    compile_profile_outline(model, EntityId(profile), Tolerance::MILLIMETRE)
        .expect_err("the outline is refused")
}

#[test]
fn malformed_boundaries_are_degenerate() {
    // Open: the last segment does not end on the first index.
    let open = slab(
        "$",
        SLAB_POINTS,
        "IFCLINEINDEX((1,2)),IFCARCINDEX((2,3,4))",
        "",
    );
    degenerate(&refusal(&open, 102), 101);
    // Not consecutive.
    let gap = slab(
        "$",
        SLAB_POINTS,
        "IFCLINEINDEX((1,2)),IFCARCINDEX((3,4,5)),IFCLINEINDEX((5,1))",
        "",
    );
    degenerate(&refusal(&gap, 102), 101);
    // A 3D point list.
    let three_d = step::metres(
        "#100=IFCCARTESIANPOINTLIST3D(((0.,0.,0.),(4.,0.,0.),(5.,1.,0.),(4.,2.,0.)));
#101=IFCINDEXEDPOLYCURVE(#100,(IFCLINEINDEX((1,2)),IFCARCINDEX((2,3,4)),IFCLINEINDEX((4,1))),$);
#102=IFCARBITRARYCLOSEDPROFILEDEF(.AREA.,$,#101);",
        "",
    );
    match refusal(&three_d, 102) {
        GeometryError::Degenerate { entity, .. } => assert_eq!(entity, EntityId(100)),
        other => panic!("expected Degenerate, got {other:?}"),
    }
    // Coincident arc points define no circle: refused by the lowering.
    let coincident = slab(
        "$",
        "((0.,0.),(4.,0.),(5.,1.),(5.,1.))",
        "IFCLINEINDEX((1,2)),IFCARCINDEX((2,3,4)),IFCLINEINDEX((4,1))",
        "",
    );
    degenerate(&refusal(&coincident, 102), 101);
}

#[test]
fn other_curve_and_profile_families_are_unsupported() {
    let model = step::metres(
        "#100=IFCCOMPOSITECURVE((),.F.);
#101=IFCARBITRARYCLOSEDPROFILEDEF(.AREA.,$,#100);
#102=IFCRECTANGLEPROFILEDEF(.AREA.,$,$,1.,2.);",
        "",
    );
    for (profile, named) in [(101, 100), (102, 102)] {
        match refusal(&model, profile) {
            GeometryError::Unsupported { entity, .. } => assert_eq!(entity, EntityId(named)),
            other => panic!("expected Unsupported, got {other:?}"),
        }
    }
}

/// A backend that refuses every curve, as the contract allows.
#[derive(Debug, Default)]
struct Refusing;

impl Backend for Refusing {
    fn descriptor(&self) -> BackendDescriptor {
        BackendDescriptor::new(BackendId::new("refusing"), ExecutionTarget::PortableCpu)
    }
}

impl PolylineCompiler for Refusing {
    fn compile_polyline(
        &self,
        _graph: &GeometryGraph,
        _root: NodeId,
        _options: &ExecutionOptions,
    ) -> GeomResult<PolylineOutcome> {
        Err(GeomError::InvalidInput("no arcs here".to_owned()))
    }
}

#[test]
fn a_refusing_backend_refuses_the_outline_naming_the_curve() {
    let model = slab("$", SLAB_POINTS, SLAB_SEGMENTS, "");
    let error =
        compile_profile_outline_with(&Refusing, &model, EntityId(102), Tolerance::MILLIMETRE)
            .expect_err("the backend refuses");
    match error {
        GeometryError::CompilationRefused { entity, reason } => {
            assert_eq!(entity, EntityId(101));
            assert!(reason.contains("no arcs here"), "{reason}");
        }
        other => panic!("expected CompilationRefused, got {other:?}"),
    }
}

/// The reference backend, its answer for an arc altered the way a faulty
/// backend might.
#[derive(Debug)]
enum Fault {
    OffPlane,
    SinglePoint,
    EndsMoved,
    Reversed,
}

#[derive(Debug)]
struct Faulty(Fault);

impl Backend for Faulty {
    fn descriptor(&self) -> BackendDescriptor {
        BackendDescriptor::new(BackendId::new("faulty"), ExecutionTarget::PortableCpu)
    }
}

impl PolylineCompiler for Faulty {
    fn compile_polyline(
        &self,
        graph: &GeometryGraph,
        root: NodeId,
        options: &ExecutionOptions,
    ) -> GeomResult<PolylineOutcome> {
        let mut outcome = default_backend().compile_polyline(graph, root, options)?;
        let n = outcome.points.len();
        match self.0 {
            Fault::OffPlane => outcome.points[n / 2].z = 1.0,
            Fault::SinglePoint => outcome.points.truncate(1),
            Fault::EndsMoved => {
                outcome.points[0].x += 0.01;
                outcome.points[n - 1].x += 0.01;
            }
            Fault::Reversed => outcome.points.reverse(),
        }
        Ok(outcome)
    }
}

fn faulty(fault: Fault) -> GeometryError {
    let model = slab("$", SLAB_POINTS, SLAB_SEGMENTS, "");
    compile_profile_outline_with(&Faulty(fault), &model, EntityId(102), Tolerance::MILLIMETRE)
        .expect_err("the faulty backend is refused")
}

fn refused_naming_curve(error: GeometryError, needle: &str) {
    match error {
        GeometryError::CompilationRefused { entity, reason } => {
            assert_eq!(entity, EntityId(101));
            assert!(reason.contains(needle), "{reason}");
        }
        other => panic!("expected CompilationRefused, got {other:?}"),
    }
}

#[test]
fn a_faulty_backend_is_refused() {
    degenerate(&faulty(Fault::OffPlane), 101);
    refused_naming_curve(faulty(Fault::SinglePoint), "fewer than two");
    refused_naming_curve(faulty(Fault::EndsMoved), "authored end points");
    refused_naming_curve(faulty(Fault::Reversed), "end to its start");
}

#[test]
fn a_collinear_arc_index_is_a_zero_bound_arc() {
    let model = slab(
        "$",
        "((0.,0.),(4.,0.),(5.,0.),(6.,0.),(6.,2.),(0.,2.))",
        "IFCLINEINDEX((1,2)),IFCARCINDEX((2,3,4)),IFCLINEINDEX((4,5,6,1))",
        "",
    );
    let ring = sample(&model, Tolerance::MILLIMETRE).outer;
    assert_eq!(ring.arcs.len(), 1);
    assert_eq!(ring.arcs[0].segment, 1);
    assert_eq!(ring.arcs[0].bound, 0.0);
    assert_eq!(ring.deviation.bound, 0.0);
    // The mid point lies between the ends: the polyline is start to end.
    assert_eq!(ring.arcs[0].samples, 0);
    assert_eq!(ring.vertices.len(), 5);
}
