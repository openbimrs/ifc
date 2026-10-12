//! Derived linear placements on offsets whose length is a quadrature
//! (#427).
//!
//! Beside a gradient curve or a clothoid, an offset's length has no closed
//! form, so this crate cannot state it (ADR 0004). Its pieces reach the
//! evaluator open-ended (`PathPiece::whole`, axiolid/kernel#298), each end
//! resolved to the evaluator's own length, and the joints after them are
//! located by `CurveEvaluator::path_joint_distances`, certified to a
//! thousandth of the model's precision, the window the station lowering
//! hands the kernel for the same basis (#423). A placement within that
//! window of one joint reads the incoming piece (IFC4.3 ADD2 8.9.3.48.3).
//!
//! Each case is checked against the lowered station of the same
//! `IfcPointByDistanceExpression`, resolved through the reference kernel
//! with its seam-snapping window: the derived frame equals it on the
//! joints, 4 um from them and between them.
//!
//! Fixture: `synthetic-lowering/station_offset_bases_ifc4x3.ifc`. `#90` is
//! the kerb, an `IfcOffsetCurveByDistances` beside the gradient curve `#47`
//! (a grade break at 20 m, a horizontal joint at 30 m, its own lengths at
//! those joints `grade_joint()` and `bend_joint()`); `#95` the twin, whose
//! two stations 15 um apart put two joints in one window; `#97` the tilted
//! `IfcOffsetCurve3D`, whose length the kernel does not certify; `#99` the
//! lane, 1 m left of the gradient curve `#86` over a clothoid (its joint
//! 19 m along); the model precision is 1e-5 m. Records a case needs beyond
//! the fixture are appended inline.

#![cfg(feature = "compile-reference-backend")]

use axiolid_contracts::{Backend, BackendDescriptor, GeomError, GeomResult, Operation};
use axiolid_core::{Frame3, Point3, Vec3};
use axiolid_curve::{Curve3, CurvePath, PathCurve};
use axiolid_curve_evaluate_contract::{
    CurveEvaluator, CurveMeasure, DistanceConvention, DistanceInterval, SeamSide,
    OPEN_PATH_END_UNSUPPORTED,
};
use axiolid_evaluate::ReferenceCurveEvaluator;
use axiolid_mesh_compile::station::{curve_path, resolve};
use axiolid_model::{GeometryGraph, GeometryNode, NodeId};
use ifc_alignment::AlignmentUnits;
use ifc_geometry::compile::SeamWindowRefusal;
use ifc_geometry::constraint::placement::derive::{basis_curve_path, derive_placement_transform};
use ifc_geometry::lower::{lower_point_by_distance_node, LoweringSession};
use ifc_geometry::{
    product_world_transform_with_evaluator, units, CachedPositionPolicy, GeometryError, Transform,
};
use ifc_model::{Codec, EntityId, Model, Value};
use std::path::PathBuf;

const EPS: f64 = 1e-9;

/// Beyond the fixture: points 0.5 m left of and 1 m up from the kerb on
/// its grade-break joint, 4 um past it, on its horizontal joint, 4 um
/// before it and between joints; 0.5 m left of the lane on its joint, 4 um
/// past it and between joints; and products placed on the kerb 4 um past
/// the grade break, cached with the incoming piece's frame (the closed
/// form: the joint (20, 2.5, 10.4), rising along (1, 0.025, 0.02)) and
/// with the outgoing piece's (falling along (1, 0.025, -0.01)), and
/// uncached.
const EXTRA: &str =
    "#1000=IFCPOINTBYDISTANCEEXPRESSION(IFCLENGTHMEASURE(20.006249023742555),0.5,1.,$,#90);
#1001=IFCPOINTBYDISTANCEEXPRESSION(IFCLENGTHMEASURE(20.006253023742556),0.5,1.,$,#90);
#1002=IFCPOINTBYDISTANCEEXPRESSION(IFCLENGTHMEASURE(30.009373535613832),0.5,1.,$,#90);
#1003=IFCPOINTBYDISTANCEEXPRESSION(IFCLENGTHMEASURE(30.009369535613832),0.5,1.,$,#90);
#1004=IFCPOINTBYDISTANCEEXPRESSION(IFCLENGTHMEASURE(10.003124511871277),0.5,1.,$,#90);
#1005=IFCPOINTBYDISTANCEEXPRESSION(IFCLENGTHMEASURE(19.),0.5,$,$,#99);
#1006=IFCPOINTBYDISTANCEEXPRESSION(IFCLENGTHMEASURE(19.000004),0.5,$,$,#99);
#1007=IFCPOINTBYDISTANCEEXPRESSION(IFCLENGTHMEASURE(9.5),0.5,$,$,#99);
#1008=IFCAXIS2PLACEMENTLINEAR(#1001,$,$);
#1009=IFCCARTESIANPOINT((19.967520390419608,2.999344235354054,11.399800184827038));
#1010=IFCAXIS2PLACEMENT3D(#1009,$,$);
#1011=IFCLINEARPLACEMENT($,#1008,#1010);
#1012=IFCBUILDINGELEMENTPROXY('1Y0f2uCbr0gfm5cmtZmo01',$,'CACHE_KERB_INCOMING',$,$,#1011,$,$,$);
#1013=IFCCARTESIANPOINT((19.99749715898555,3.0000936545682024,11.399950034975486));
#1014=IFCAXIS2PLACEMENT3D(#1013,$,$);
#1015=IFCLINEARPLACEMENT($,#1008,#1014);
#1016=IFCBUILDINGELEMENTPROXY('1Y0f2uCbr0gfm5cmtZmo02',$,'CACHE_KERB_OUTGOING',$,$,#1015,$,$,$);
#1017=IFCLINEARPLACEMENT($,#1008,$);
#1018=IFCBUILDINGELEMENTPROXY('1Y0f2uCbr0gfm5cmtZmo03',$,'UNCACHED_KERB',$,$,#1017,$,$,$);";

fn bases() -> Model {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../test/fixtures/synthetic-lowering/station_offset_bases_ifc4x3.ifc");
    let text = std::fs::read_to_string(&path).expect("fixture reads");
    let text = text.replacen(
        "ENDSEC;\nEND-ISO-10303-21;",
        &format!("{EXTRA}\nENDSEC;\nEND-ISO-10303-21;"),
        1,
    );
    assert!(text.contains(EXTRA), "records appended");
    ifc_step::StepCodec
        .read_bytes(text.as_bytes())
        .unwrap_or_else(|e| panic!("{e:?}"))
}

/// The placement `expression` derives through `evaluator`, in metres.
fn derived(
    model: &Model,
    expression: u64,
    evaluator: &dyn CurveEvaluator,
) -> Result<Transform, GeometryError> {
    let scale = units::resolve(model);
    let file_units = AlignmentUnits {
        length_to_metres: 1.0,
        angle_to_radians: scale.angle_to_radians,
    };
    let point = ifc_alignment::resolve_point_by_distance(model, EntityId(expression), file_units)
        .expect("the expression reads");
    derive_placement_transform(model, &scale, EntityId(expression), &point, evaluator)
}

/// The lowered station of `expression`: the graph, its root and its basis.
/// Every one carries the precision as its window and reads the incoming
/// side (#423).
fn lowered(model: &Model, expression: u64) -> (GeometryGraph, NodeId, NodeId) {
    let scale = units::resolve(model);
    let mut session = LoweringSession::new(model, &scale);
    let root =
        lower_point_by_distance_node(&mut session, EntityId(expression), Transform::identity())
            .expect("the station lowers");
    let lowered = session.finish(root).expect("a valid graph");
    let Some(GeometryNode::OrientedCurveStation(station)) = lowered.graph.get(lowered.root) else {
        panic!("#{expression}: a windowed station");
    };
    assert_eq!(station.seam, SeamSide::Incoming, "#{expression}");
    assert_eq!(station.seam_window, 1e-5, "#{expression}: the precision");
    let basis = station.station.basis;
    (lowered.graph, lowered.root, basis)
}

fn close(actual: [f64; 3], expected: [f64; 3], what: &str) {
    let off = (0..3)
        .map(|i| (actual[i] - expected[i]).abs())
        .fold(0.0, f64::max);
    assert!(off < EPS, "{what}: got {actual:?}, expected {expected:?}");
}

fn array(v: Vec3) -> [f64; 3] {
    [v.x, v.y, v.z]
}

fn unit(v: [f64; 3]) -> [f64; 3] {
    let n = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    [v[0] / n, v[1] / n, v[2] / n]
}

/// The derived placement of `expression` equals its lowered, windowed
/// station's frame, resolved by the reference compiler.
fn assert_matches_station(model: &Model, expression: u64) -> Transform {
    let what = format!("#{expression}");
    let transform = derived(model, expression, &ReferenceCurveEvaluator::default())
        .unwrap_or_else(|e| panic!("{what}: {e}"));
    let (graph, root, _) = lowered(model, expression);
    let station = resolve(&graph, root).expect("the station resolves");
    close(
        transform.origin,
        array(station.point),
        &format!("{what}: origin"),
    );
    close(
        transform.basis[0],
        array(station.section.tangent),
        &format!("{what}: x, the tangent"),
    );
    close(
        transform.basis[1],
        array(station.section.lateral),
        &format!("{what}: y, the left"),
    );
    close(
        transform.basis[2],
        array(station.section.up),
        &format!("{what}: z, up"),
    );
    transform
}

// ---- the path ---------------------------------------------------------------

/// The kerb reads as open offset pieces, one per span of the gradient
/// curve between its seams and stations, the same pieces the reference
/// compiler flattens it into with their lengths stated; the evaluator reads
/// both alike, bit for bit, since it resolves each open end to the length
/// it states there.
#[test]
fn the_kerb_reads_as_the_compilers_path_with_open_ends() {
    let model = bases();
    let scale = units::resolve(&model);
    let evaluator = ReferenceCurveEvaluator::default();
    for (expression, basis) in [(1000, 90), (1005, 99)] {
        let ours = basis_curve_path(
            &model,
            &scale,
            EntityId(expression),
            EntityId(basis),
            &evaluator,
        )
        .unwrap_or_else(|e| panic!("#{basis}: {e}"));
        let (graph, _, node) = lowered(&model, expression);
        let theirs = curve_path(&graph, node).expect("the compiler flattens it");
        assert_eq!(
            ours.pieces().len(),
            theirs.pieces().len(),
            "#{basis}: {ours:?}\nvs\n{theirs:?}"
        );
        for (a, b) in ours.pieces().iter().zip(theirs.pieces()) {
            assert!(a.open_end && !a.is_stated(), "#{basis}: open");
            assert!(b.is_stated(), "#{basis}: theirs stated");
            assert_eq!(a.curve, b.curve, "#{basis}: the same offset");
            assert!(matches!(a.curve, PathCurve::Offset(_)), "#{basis}");
            assert_eq!((a.start, a.reversed), (0.0, b.reversed), "#{basis}");
        }
        assert_eq!(
            evaluator.path_distance_convention(&ours),
            DistanceConvention::PlanDistance,
            "#{basis}: the offset's own plan length"
        );
        let length = theirs.length();
        for step in 0..=40 {
            let at = CurveMeasure::Distance(length * f64::from(step) / 40.0);
            for side in [SeamSide::Incoming, SeamSide::Outgoing] {
                let a = evaluator.path_frame_at_on(&ours, at, side).expect("ours");
                let b = evaluator
                    .path_frame_at_on(&theirs, at, side)
                    .expect("theirs");
                assert_eq!(a, b, "#{basis} at {at:?} {side:?}");
            }
        }
        assert_eq!(
            evaluator.path_joint_distances(&ours, 1e-8),
            evaluator.path_joint_distances(&theirs, 1e-8),
            "#{basis}: the same certified joints"
        );
    }
}

// ---- the derived frame equals the lowered station's ---------------------

/// On the kerb's grade-break joint and 4 um past it the incoming piece
/// governs, rising at 0.02; on the horizontal joint and 4 um before it the
/// piece beside the line. Between joints the point is its own.
#[test]
fn a_placement_on_the_kerb_reads_the_incoming_piece_on_and_near_joints() {
    let model = bases();
    let rising = unit([1.0, 0.025, 0.02]);
    for expression in [1000, 1001] {
        let transform = assert_matches_station(&model, expression);
        close(transform.basis[0], rising, "the incoming grade");
        close(
            transform.origin,
            [19.967520390419608, 2.999344235354054, 11.399800184827038],
            "the joint, 0.5 m left and 1 m up",
        );
    }
    for expression in [1002, 1003] {
        let transform = assert_matches_station(&model, expression);
        close(
            transform.basis[0],
            unit([1.0, 0.025, -0.01]),
            "beside the line",
        );
    }
    assert_matches_station(&model, 1004);
}

/// The lane beside the clothoid: on its joint 19 m along and 4 um past it,
/// the joint's heading of 1 rad; half way along, its own point.
#[test]
fn a_placement_beside_a_clothoid_reads_the_incoming_piece_on_its_joint() {
    let model = bases();
    for expression in [1005, 1006] {
        let transform = assert_matches_station(&model, expression);
        close(
            transform.basis[0],
            [1.0f64.cos(), 1.0f64.sin(), 0.0],
            "the joint's heading",
        );
    }
    let joint = derived(&model, 1005, &ReferenceCurveEvaluator::default()).expect("derived");
    let near = derived(&model, 1006, &ReferenceCurveEvaluator::default()).expect("derived");
    close(near.origin, joint.origin, "snapped onto the joint");
    assert_matches_station(&model, 1007);
}

// ---- #354: Verify compares against the incoming piece's frame -----------

/// The proxy named `name`.
fn product(model: &Model, name: &str) -> EntityId {
    let found: Vec<EntityId> = model
        .of_type("IFCBUILDINGELEMENTPROXY")
        .filter(|(_, e)| matches!(e.attributes.get(2), Some(Value::Text(t)) if &**t == name))
        .map(|(id, _)| id)
        .collect();
    assert_eq!(found.len(), 1, "one proxy named {name}");
    found[0]
}

fn world(
    model: &Model,
    name: &str,
    evaluator: &dyn CurveEvaluator,
) -> Result<Transform, GeometryError> {
    product_world_transform_with_evaluator(
        model,
        &units::resolve(model),
        product(model, name),
        evaluator,
        CachedPositionPolicy::Verify,
    )
}

#[test]
fn verify_accepts_a_cache_computed_with_the_incoming_piece() {
    let model = bases();
    let evaluator = ReferenceCurveEvaluator::default();
    let cached = world(&model, "CACHE_KERB_INCOMING", &evaluator).expect("accepted");
    let uncached = world(&model, "UNCACHED_KERB", &evaluator).expect("derived");
    assert_eq!(cached, uncached);
    let expected = derived(&model, 1001, &evaluator).expect("derived");
    close(cached.origin, expected.origin, "the derived frame");
}

#[test]
fn verify_refuses_a_cache_computed_with_the_outgoing_piece() {
    let model = bases();
    match world(
        &model,
        "CACHE_KERB_OUTGOING",
        &ReferenceCurveEvaluator::default(),
    ) {
        Err(GeometryError::CachedPlacementMismatch {
            cached,
            derived,
            distance,
            tolerance,
            ..
        }) => {
            close(
                cached,
                [19.99749715898555, 3.0000936545682024, 11.399950034975486],
                "the stale cache",
            );
            close(
                derived,
                [19.967520390419608, 2.999344235354054, 11.399800184827038],
                "the incoming frame",
            );
            assert!(distance > 100.0 * tolerance, "{distance} vs {tolerance}");
        }
        other => panic!("expected CachedPlacementMismatch, got {other:?}"),
    }
}

// ---- refusals -------------------------------------------------------------

/// Where the model's precision names no one joint, or the evaluator cannot
/// certify one, the placement is refused as the kernel refuses the same
/// station (#423): two joints of the twin in one window, a joint of the
/// kerb straddling the window's edge, the tilted offset's uncertified
/// length.
#[test]
fn window_refusals_name_the_placement() {
    let model = bases();
    for (expression, basis, refusal, unsupported) in [
        (200, 95, SeamWindowRefusal::Ambiguous, false),
        (189, 90, SeamWindowRefusal::Straddled, false),
        (211, 97, SeamWindowRefusal::UncertifiedLength, true),
    ] {
        let error =
            derived(&model, expression, &ReferenceCurveEvaluator::default()).expect_err("refused");
        let GeometryError::PlacementSeamWindowRefused {
            placement,
            basis: got_basis,
            refusal: got,
            ..
        } = error
        else {
            panic!("#{expression}: {error:?}");
        };
        assert_eq!(
            (placement, got_basis, got),
            (EntityId(expression), EntityId(basis), refusal),
            "#{expression}"
        );
        assert_eq!(error.entity(), Some(EntityId(expression)));
        assert_eq!(error.is_unsupported(), unsupported, "#{expression}");
        assert!(
            error.to_string().contains(&format!("#{expression}")),
            "{error}"
        );
    }
}

/// Delegates every query to the reference evaluator, but refuses a path
/// with an open end as `OPEN_PATH_END_UNSUPPORTED`, as a provider that
/// reads stated spans only does; with `joints` false it also leaves
/// `path_joint_distances` to the contract's default refusal.
#[derive(Debug, Default)]
struct Stated {
    reference: ReferenceCurveEvaluator,
    open_ends: bool,
}

impl Stated {
    fn check(&self, path: &CurvePath) -> GeomResult<()> {
        if !self.open_ends && path.pieces().iter().any(|piece| !piece.is_stated()) {
            return Err(GeomError::UnsupportedInput {
                backend: self.descriptor().id,
                operation: Operation::CurveEvaluation,
                input: OPEN_PATH_END_UNSUPPORTED,
            });
        }
        Ok(())
    }
}

impl Backend for Stated {
    fn descriptor(&self) -> BackendDescriptor {
        self.reference.descriptor()
    }
}

impl CurveEvaluator for Stated {
    fn distance_convention(&self, curve: &Curve3) -> DistanceConvention {
        self.reference.distance_convention(curve)
    }
    fn point_at(&self, curve: &Curve3, at: CurveMeasure) -> GeomResult<Point3> {
        self.reference.point_at(curve, at)
    }
    fn tangent_at(&self, curve: &Curve3, at: CurveMeasure) -> GeomResult<Vec3> {
        self.reference.tangent_at(curve, at)
    }
    fn frame_at(&self, curve: &Curve3, at: CurveMeasure) -> GeomResult<Frame3> {
        self.reference.frame_at(curve, at)
    }
    fn frame_at_on(&self, curve: &Curve3, at: CurveMeasure, side: SeamSide) -> GeomResult<Frame3> {
        self.reference.frame_at_on(curve, at, side)
    }
    fn path_distance_convention(&self, path: &CurvePath) -> DistanceConvention {
        if self.check(path).is_err() {
            return DistanceConvention::Unsupported;
        }
        self.reference.path_distance_convention(path)
    }
    fn path_point_at(&self, path: &CurvePath, at: CurveMeasure) -> GeomResult<Point3> {
        self.check(path)?;
        self.reference.path_point_at(path, at)
    }
    fn path_tangent_at(&self, path: &CurvePath, at: CurveMeasure) -> GeomResult<Vec3> {
        self.check(path)?;
        self.reference.path_tangent_at(path, at)
    }
    fn path_frame_at(&self, path: &CurvePath, at: CurveMeasure) -> GeomResult<Frame3> {
        self.check(path)?;
        self.reference.path_frame_at(path, at)
    }
    fn path_frame_at_on(
        &self,
        path: &CurvePath,
        at: CurveMeasure,
        side: SeamSide,
    ) -> GeomResult<Frame3> {
        self.check(path)?;
        self.reference.path_frame_at_on(path, at, side)
    }
}

#[test]
fn an_evaluator_without_open_ends_is_refused_naming_the_basis() {
    let model = bases();
    for (expression, basis) in [(1001, 90), (1004, 90), (1005, 99)] {
        let error = derived(&model, expression, &Stated::default())
            .expect_err("never a length estimated here");
        assert_eq!(
            error,
            GeometryError::CurvePathOpenEndUnsupported {
                placement: EntityId(expression),
                basis: EntityId(basis),
            },
            "#{expression}"
        );
        assert!(error.is_unsupported());
        assert!(error.to_string().contains("stated spans"), "{error}");
    }
    // Lowering a product with a cache refuses alike.
    assert!(matches!(
        world(&model, "CACHE_KERB_INCOMING", &Stated::default()),
        Err(GeometryError::CurvePathOpenEndUnsupported { .. })
    ));
}

#[test]
fn an_evaluator_without_certified_joints_is_refused_naming_the_basis() {
    let model = bases();
    let evaluator = Stated {
        open_ends: true,
        ..Stated::default()
    };
    // It reads the open path, but cannot tell where its joints lie.
    let error = derived(&model, 1004, &evaluator).expect_err("joints uncertified");
    assert_eq!(
        error,
        GeometryError::PathJointDistancesUnsupported {
            placement: EntityId(1004),
            basis: EntityId(90),
        }
    );
    assert!(error.is_unsupported());
}

/// The certified joints this crate snaps within are the kernel's own: the
/// kerb's first two contain their closed forms, each no wider than a
/// thousandth of the window.
#[test]
fn the_joints_snapped_to_are_certified_to_a_thousandth_of_the_window() {
    let model = bases();
    let scale = units::resolve(&model);
    let evaluator = ReferenceCurveEvaluator::default();
    let path =
        basis_curve_path(&model, &scale, EntityId(1000), EntityId(90), &evaluator).expect("a path");
    let joints: Vec<DistanceInterval> = evaluator
        .path_joint_distances(&path, 1e-8)
        .expect("certified");
    let grade = 20.0f64.hypot(0.5);
    for (joint, exact) in joints.iter().zip([grade, grade + 10.0f64.hypot(0.25)]) {
        assert!(joint.contains(exact) || (joint.midpoint() - exact).abs() < 1e-12);
        assert!(joint.width() <= 1e-8, "{joint:?}");
    }
}
