//! Derived linear placements on curve-relation bases (#418).
//!
//! A plain `IfcCompositeCurve`, an `IfcTrimmedCurve` and a composite of
//! segments placed at stations lower to a curve relation, along which the
//! station lowering measures end to end (#346). The derivation reads the
//! same basis as Axiolid's neutral `CurvePath` through the evaluator's
//! `path_*` queries (axiolid/kernel#290), reading a joint from the segment
//! that ends there (IFC4.3 ADD2 8.9.3.48.3).
//!
//! Each case is checked three ways: the path the derivation builds from the
//! stored relation equals the one `axiolid-mesh-compile`'s
//! `station::curve_path` builds from the lowered graph; the derived frame
//! equals the lowered station's, resolved by `axiolid-mesh-compile`; and
//! `CachedPositionPolicy::Verify` accepts a cache computed with the
//! incoming segment and refuses one computed with the outgoing segment.
//!
//! Fixture: `synthetic-lowering/station_relations_ifc4x3.ifc`. `#29` is a
//! plain composite of a polyline east (0 to 10 m), a polyline north
//! traversed backwards (a left corner at 10 m) and a quarter circle trimmed
//! by Cartesian points (a tangent-continuous joint at 20 m); `#19` is that
//! trimmed circle; `#38` a composite with a 1 m gap; `#75` the kerb, two
//! `IfcLine` segments placed 3 m left of the gradient curve `#64` at its
//! stations 10 and 30; the model precision is 1e-5 m. Records a case needs
//! beyond the fixture are appended inline.

#![cfg(feature = "compile-reference-backend")]

use axiolid_contracts::{Backend, BackendDescriptor, GeomResult};
use axiolid_core::{Frame3, Point3, Vec3};
use axiolid_curve::{Curve3, CurvePath};
use axiolid_curve_evaluate_contract::{CurveEvaluator, CurveMeasure, DistanceConvention, SeamSide};
use axiolid_evaluate::ReferenceCurveEvaluator;
use axiolid_mesh_compile::station::{curve_path, resolve};
use axiolid_model::{GeometryGraph, GeometryNode, NodeId};
use ifc_alignment::AlignmentUnits;
use ifc_geometry::constraint::placement::derive::{basis_curve_path, derive_placement_transform};
use ifc_geometry::lower::{lower_point_by_distance_node, LoweringSession};
use ifc_geometry::{
    product_world_transform_with_evaluator, units, CachedPositionPolicy, GeometryError, Transform,
};
use ifc_model::{Codec, EntityId, Model, Value};
use std::path::PathBuf;

const EPS: f64 = 1e-9;

/// Beyond the fixture: a point on the trimmed circle `#19`, a point on the
/// composite's first leg, a parameter on the composite, and products
/// placed 1 m left of and 0.5 m along the composite's corner, with caches
/// computed with the incoming and the outgoing segment, and uncached on the
/// corner and on the kerb's joint.
const EXTRA: &str = "#1000=IFCPOINTBYDISTANCEEXPRESSION(IFCLENGTHMEASURE(5.),1.,$,$,#19);
#1001=IFCPOINTBYDISTANCEEXPRESSION(IFCLENGTHMEASURE(5.),1.,$,0.5,#29);
#1002=IFCPOINTBYDISTANCEEXPRESSION(IFCPARAMETERVALUE(1.),1.,$,$,#29);
#1003=IFCPOINTBYDISTANCEEXPRESSION(IFCLENGTHMEASURE(10.),1.,$,0.5,#29);
#1004=IFCAXIS2PLACEMENTLINEAR(#1003,$,$);
#1005=IFCCARTESIANPOINT((10.5,1.,0.));
#1006=IFCAXIS2PLACEMENT3D(#1005,$,$);
#1007=IFCLINEARPLACEMENT($,#1004,#1006);
#1008=IFCBUILDINGELEMENTPROXY('1Y0f2uCbr0gfm5cmtZmh01',$,'CACHE_PATH_INCOMING',$,$,#1007,$,$,$);
#1009=IFCCARTESIANPOINT((9.,0.5,0.));
#1010=IFCAXIS2PLACEMENT3D(#1009,$,$);
#1011=IFCLINEARPLACEMENT($,#1004,#1010);
#1012=IFCBUILDINGELEMENTPROXY('1Y0f2uCbr0gfm5cmtZmh02',$,'CACHE_PATH_OUTGOING',$,$,#1011,$,$,$);
#1013=IFCLINEARPLACEMENT($,#1004,$);
#1014=IFCBUILDINGELEMENTPROXY('1Y0f2uCbr0gfm5cmtZmh03',$,'UNCACHED_PATH_CORNER',$,$,#1013,$,$,$);
#1015=IFCAXIS2PLACEMENTLINEAR(#137,$,$);
#1016=IFCLINEARPLACEMENT($,#1015,$);
#1017=IFCBUILDINGELEMENTPROXY('1Y0f2uCbr0gfm5cmtZmh04',$,'UNCACHED_KERB_JOINT',$,$,#1016,$,$,$);";

fn relations() -> Model {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../test/fixtures/synthetic-lowering/station_relations_ifc4x3.ifc");
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
fn lowered(model: &Model, expression: u64) -> (GeometryGraph, NodeId, NodeId, SeamSide) {
    let scale = units::resolve(model);
    let mut session = LoweringSession::new(model, &scale);
    let root =
        lower_point_by_distance_node(&mut session, EntityId(expression), Transform::identity())
            .expect("the station lowers");
    let lowered = session.finish(root).expect("a valid graph");
    let (basis, side) = match lowered.graph.get(lowered.root) {
        Some(GeometryNode::CurveStation(station)) => (station.basis, SeamSide::Outgoing),
        Some(GeometryNode::OrientedCurveStation(oriented)) => {
            assert!(oriented.orientation.is_base(), "#{expression}: unturned");
            (oriented.station.basis, oriented.seam)
        }
        other => panic!("#{expression}: a station, got {other:?}"),
    };
    (lowered.graph, lowered.root, basis, side)
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

/// The derived placement of `expression` equals its lowered station's
/// frame, resolved by the reference compiler, read from `side`.
fn assert_matches_station(model: &Model, expression: u64, side: SeamSide) -> Transform {
    let what = format!("#{expression}");
    let transform = derived(model, expression, &ReferenceCurveEvaluator::default())
        .unwrap_or_else(|e| panic!("{what}: {e}"));
    let (graph, root, _, lowered_side) = lowered(model, expression);
    assert_eq!(lowered_side, side, "{what}: the station's side");
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

// ---- the path equals the reference compiler's ---------------------------

/// The largest difference between two paths' numbers, or `None` when
/// their structure differs (piece count, curves, senses, exactness,
/// placed or not).
fn path_deviation(ours: &CurvePath, theirs: &CurvePath) -> Option<f64> {
    if ours.pieces().len() != theirs.pieces().len() {
        return None;
    }
    let mut worst = 0.0_f64;
    for (a, b) in ours.pieces().iter().zip(theirs.pieces()) {
        if a.curve != b.curve || a.reversed != b.reversed || a.placement_exact != b.placement_exact
        {
            return None;
        }
        worst = worst
            .max((a.start - b.start).abs())
            .max((a.end - b.end).abs());
        match (a.placement, b.placement) {
            (None, None) => {}
            (Some(p), Some(q)) => {
                let (p, q) = (p.to_cols_array(), q.to_cols_array());
                for (x, y) in p.iter().zip(&q) {
                    worst = worst.max((x - y).abs());
                }
            }
            _ => return None,
        }
    }
    Some(worst)
}

#[test]
fn the_derived_path_equals_the_reference_compilers() {
    let model = relations();
    let scale = units::resolve(&model);
    let evaluator = ReferenceCurveEvaluator::default();
    // A plain composite, a trimmed circle, the kerb placed at stations.
    for (expression, basis, pieces) in [(145, 29, 3), (1000, 19, 1), (130, 75, 2)] {
        let ours = basis_curve_path(
            &model,
            &scale,
            EntityId(expression),
            EntityId(basis),
            &evaluator,
        )
        .unwrap_or_else(|e| panic!("#{basis}: {e}"));
        let (graph, _, node, _) = lowered(&model, expression);
        let theirs = curve_path(&graph, node).expect("the compiler flattens it");
        assert_eq!(ours.pieces().len(), pieces, "#{basis}: pieces");
        let deviation = path_deviation(&ours, &theirs)
            .unwrap_or_else(|| panic!("#{basis}: {ours:?}\nvs\n{theirs:?}"));
        // The same pieces, curves, senses and exactness, and bit for bit
        // the same numbers, the kerb's placements framed through the
        // evaluator included; except a whole polyline's span, its edges
        // summed here and its length as Axiolid measures it there (10
        // against 10.000000000000002).
        if basis == 29 {
            assert!(deviation <= 1e-14, "#{basis}: deviation {deviation:e}");
        } else {
            assert_eq!(ours, theirs, "#{basis}: bit for bit");
        }
        assert_eq!(
            evaluator.path_distance_convention(&ours),
            DistanceConvention::ArcLength3d,
            "#{basis}"
        );
        // Read alike: every frame along both, from both sides of a joint.
        let length = ours.length();
        for step in 0..=40 {
            let at = CurveMeasure::Distance(length * f64::from(step) / 40.0);
            for side in [SeamSide::Incoming, SeamSide::Outgoing] {
                let a = evaluator.path_frame_at_on(&ours, at, side).expect("ours");
                let b = evaluator
                    .path_frame_at_on(&theirs, at, side)
                    .expect("theirs");
                let what = format!("#{basis} at {at:?} {side:?}");
                close(array(a.origin), array(b.origin), &what);
                close(array(a.x), array(b.x), &what);
                close(array(a.y), array(b.y), &what);
            }
        }
    }
}

// ---- the derived frame equals the lowered station's ---------------------

#[test]
fn a_placement_on_a_composite_corner_takes_the_incoming_segment() {
    let model = relations();
    // #82 on the corner, #89 4 um past it; both 1 m left.
    for expression in [82, 89] {
        let transform = assert_matches_station(&model, expression, SeamSide::Incoming);
        close(
            transform.origin,
            [10.0, 1.0, 0.0],
            "1 m left of the eastward leg",
        );
        close(transform.basis[0], [1.0, 0.0, 0.0], "the incoming tangent");
    }
    // 0.5 m along the incoming leg, past the corner.
    let transform = derived(&model, 1003, &ReferenceCurveEvaluator::default()).expect("derived");
    close(transform.origin, [10.5, 1.0, 0.0], "along the incoming leg");
}

#[test]
fn a_placement_off_the_joints_derives_the_stations_frame() {
    let model = relations();
    // On the tangent-continuous joint, on the arc, on the first leg.
    assert_matches_station(&model, 96, SeamSide::Incoming);
    let arc = assert_matches_station(&model, 103, SeamSide::Outgoing);
    // Halfway round the quarter circle about (0, 10), 1 m left (inwards).
    let (s, c) = std::f64::consts::FRAC_PI_4.sin_cos();
    close(arc.origin, [9.0 * c, 10.0 + 9.0 * s, 0.0], "on the arc");
    let leg = assert_matches_station(&model, 1001, SeamSide::Outgoing);
    close(leg.origin, [5.5, 1.0, 0.0], "on the first leg");
}

#[test]
fn a_placement_on_a_trimmed_curve_derives_the_stations_frame() {
    let model = relations();
    let transform = assert_matches_station(&model, 1000, SeamSide::Outgoing);
    // 5 m round the circle of radius 10 about (0, 10) from (10, 10).
    let angle = 0.5_f64;
    close(
        transform.origin,
        [9.0 * angle.cos(), 10.0 + 9.0 * angle.sin(), 0.0],
        "on the trimmed circle",
    );
}

#[test]
fn a_placement_on_segments_placed_at_stations_derives_the_stations_frame() {
    let model = relations();
    // #130 on the first segment; #137 on the joint at 20 sqrt(1.0004), 1 m
    // up.
    assert_matches_station(&model, 130, SeamSide::Outgoing);
    let joint = assert_matches_station(&model, 137, SeamSide::Incoming);
    // Along the 0.02 grade 3 m left of the gradient curve: station 30 of
    // the gradient curve, 3 m left, 1 m up the leaning section.
    let n = 1.0004_f64.sqrt();
    close(
        joint.origin,
        [30.0 - 0.02 / n, 3.0, 10.6 + 1.0 / n],
        "the kerb joint, 1 m up",
    );
}

// ---- refusals -------------------------------------------------------------

#[test]
fn a_composite_with_a_gap_is_refused_by_the_evaluator_by_name() {
    let model = relations();
    let error = derived(&model, 110, &ReferenceCurveEvaluator::default())
        .expect_err("the pieces do not meet");
    let GeometryError::Unsupported { detail, .. } = error else {
        panic!("expected Unsupported, got {error:?}");
    };
    assert!(detail.contains("do not meet"), "{detail}");
}

#[test]
fn a_parameter_along_a_composite_is_refused_by_name() {
    let model = relations();
    let error = derived(&model, 1002, &ReferenceCurveEvaluator::default())
        .expect_err("a parameter on a curve relation");
    let GeometryError::Unsupported {
        entity,
        type_name,
        detail,
    } = error
    else {
        panic!("expected Unsupported, got {error:?}");
    };
    assert_eq!(
        (entity, type_name.as_str()),
        (EntityId(29), "IFCCOMPOSITECURVE")
    );
    assert!(detail.contains("IfcParameterValue"), "{detail}");
}

// ---- #354: Verify compares against the incoming segment's frame ---------

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
fn verify_accepts_a_cache_computed_with_the_incoming_segment() {
    let model = relations();
    let evaluator = ReferenceCurveEvaluator::default();
    let world_cached = world(&model, "CACHE_PATH_INCOMING", &evaluator).expect("accepted");
    close(world_cached.origin, [10.5, 1.0, 0.0], "the cached corner");
    let uncached = world(&model, "UNCACHED_PATH_CORNER", &evaluator).expect("derived");
    assert_eq!(uncached, world_cached);
    let kerb = world(&model, "UNCACHED_KERB_JOINT", &evaluator).expect("derived");
    let expected = derived(&model, 137, &evaluator).expect("derived");
    close(kerb.origin, expected.origin, "the kerb joint");
}

#[test]
fn verify_refuses_a_cache_computed_with_the_outgoing_segment() {
    let model = relations();
    match world(
        &model,
        "CACHE_PATH_OUTGOING",
        &ReferenceCurveEvaluator::default(),
    ) {
        Err(GeometryError::CachedPlacementMismatch {
            cached,
            derived,
            distance,
            tolerance,
            ..
        }) => {
            close(cached, [9.0, 0.5, 0.0], "the stale cache");
            close(derived, [10.5, 1.0, 0.0], "the incoming frame");
            assert!(distance > 100.0 * tolerance, "{distance} vs {tolerance}");
        }
        other => panic!("expected CachedPlacementMismatch, got {other:?}"),
    }
}

// ---- an evaluator without curve paths is refused by name ----------------

/// Delegates every single-curve query, sided ones included, to the
/// reference evaluator and leaves the `path_*` queries to the contract's
/// defaults, which refuse with `CURVE_PATH_UNSUPPORTED`.
#[derive(Debug, Default)]
struct Pathless(ReferenceCurveEvaluator);

impl Backend for Pathless {
    fn descriptor(&self) -> BackendDescriptor {
        self.0.descriptor()
    }
}

impl CurveEvaluator for Pathless {
    fn distance_convention(&self, curve: &Curve3) -> DistanceConvention {
        self.0.distance_convention(curve)
    }
    fn point_at(&self, curve: &Curve3, at: CurveMeasure) -> GeomResult<Point3> {
        self.0.point_at(curve, at)
    }
    fn tangent_at(&self, curve: &Curve3, at: CurveMeasure) -> GeomResult<Vec3> {
        self.0.tangent_at(curve, at)
    }
    fn frame_at(&self, curve: &Curve3, at: CurveMeasure) -> GeomResult<Frame3> {
        self.0.frame_at(curve, at)
    }
    fn point_at_on(&self, curve: &Curve3, at: CurveMeasure, side: SeamSide) -> GeomResult<Point3> {
        self.0.point_at_on(curve, at, side)
    }
    fn tangent_at_on(&self, curve: &Curve3, at: CurveMeasure, side: SeamSide) -> GeomResult<Vec3> {
        self.0.tangent_at_on(curve, at, side)
    }
    fn frame_at_on(&self, curve: &Curve3, at: CurveMeasure, side: SeamSide) -> GeomResult<Frame3> {
        self.0.frame_at_on(curve, at, side)
    }
}

#[test]
fn an_evaluator_without_curve_paths_is_refused_naming_the_basis() {
    let model = relations();
    // On and off a joint of the plain composite, on the trimmed circle and
    // on the kerb, whose segments' stations it can frame.
    for (expression, basis) in [(82, 29), (103, 29), (1000, 19), (130, 75), (137, 75)] {
        let error = derived(&model, expression, &Pathless::default())
            .expect_err("never a frame read on one piece");
        assert!(error.is_unsupported(), "{error}");
        assert_eq!(error.entity(), Some(EntityId(expression)));
        let message = error.to_string();
        assert_eq!(
            error,
            GeometryError::CurvePathUnsupported {
                placement: EntityId(expression),
                basis: EntityId(basis),
            },
            "#{expression}"
        );
        assert!(message.contains("curve relation"), "{message}");
    }
    // Lowering a product with a cache refuses alike.
    assert!(matches!(
        world(&model, "CACHE_PATH_INCOMING", &Pathless::default()),
        Err(GeometryError::CurvePathUnsupported { .. })
    ));
    // Single-curve bases derive as before.
    let gradient = derived(&model, 69, &Pathless::default()).expect("a gradient curve basis");
    let reference = derived(&model, 69, &ReferenceCurveEvaluator::default()).expect("derived");
    assert_eq!(gradient, reference);
}
