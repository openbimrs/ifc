//! Derived linear placements on a tangent discontinuity take the incoming
//! tangent (#409).
//!
//! IFC4.3 ADD2 8.9.3.48.3: "If DistanceAlong coincides with a point of
//! tangential discontinuity (within precision limits), then the tangent of
//! the previous segment governs." The station lowering reads such a point
//! from the incoming side (#346); a placement derived through a
//! `CurveEvaluator` must agree with it. Each case is compared with the
//! lowered station's frame, resolved through `axiolid-reference`, and with
//! the outgoing frame it must differ from.
//!
//! Fixtures: `synthetic-lowering/station_seams_ifc4x3.ifc` (a gradient
//! curve whose grade breaks from 0.02 to -0.01 at 40 m, and an L polyline
//! turning left at 10 m) and `station_relations_ifc4x3.ifc` (the same
//! gradient curve, whose break is the joint of its `Segments`, segments
//! placed at stations along it, and a plain composite curve). The model
//! precision is 1e-5 m. Records a case needs beyond the fixture are
//! appended inline.

#![cfg(feature = "compile")]

use axiolid_contracts::{Backend, BackendDescriptor, GeomResult};
use axiolid_core::{Frame3, Point3, Vec3};
use axiolid_curve::Curve3;
use axiolid_curve_evaluate_contract::{CurveEvaluator, CurveMeasure, DistanceConvention, SeamSide};
use axiolid_evaluate::ReferenceCurveEvaluator;
use axiolid_model::{GeometryNode, StationOffsets};
use axiolid_reference::station::{station_section3_on, SectionFrame};
use ifc_alignment::AlignmentUnits;
use ifc_geometry::constraint::placement::derive::derive_placement_transform;
use ifc_geometry::lower::{lower_point_by_distance_node, LoweringSession};
use ifc_geometry::{
    product_world_transform_with_evaluator, units, CachedPositionPolicy, GeometryError, Transform,
};
use ifc_model::{Codec, EntityId, Model, Value};
use std::path::PathBuf;

const EPS: f64 = 1e-9;

/// `name` from `synthetic-lowering/`, with `extra` records appended.
fn fixture(name: &str, extra: &str) -> Model {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../test/fixtures/synthetic-lowering")
        .join(name);
    let text = std::fs::read_to_string(&path).expect("fixture reads");
    let text = text.replacen(
        "ENDSEC;\nEND-ISO-10303-21;",
        &format!("{extra}\nENDSEC;\nEND-ISO-10303-21;"),
        1,
    );
    assert!(text.contains(extra), "{name}: records appended");
    ifc_step::StepCodec
        .read_bytes(text.as_bytes())
        .unwrap_or_else(|e| panic!("{name}: {e:?}"))
}

/// Beyond the station-seams fixture: a point 0.1 mm past the grade break
/// (ten times the precision, so off it), a polyline parameter on the
/// corner, and products placed by linear placements on both seams whose
/// caches were computed with the incoming and the outgoing tangent.
const SEAMS_EXTRA: &str =
    "#1000=IFCPOINTBYDISTANCEEXPRESSION(IFCLENGTHMEASURE(40.0001),$,2.,$,#39);
#1001=IFCPOINTBYDISTANCEEXPRESSION(IFCPARAMETERVALUE(1.),1.,$,0.5,#43);
#1002=IFCAXIS2PLACEMENTLINEAR(#72,$,$);
#1003=IFCCARTESIANPOINT((39.9600079976008,0.,12.799600119960015));
#1004=IFCAXIS2PLACEMENT3D(#1003,$,$);
#1005=IFCLINEARPLACEMENT($,#1002,#1004);
#1006=IFCBUILDINGELEMENTPROXY('0Y0f2uCbr0gfm5cmtZmh01',$,'CACHE_GRADE_INCOMING',$,$,#1005,$,$,$);
#1007=IFCCARTESIANPOINT((40.019999000074996,0.,12.799900007499376));
#1008=IFCAXIS2PLACEMENT3D(#1007,$,$);
#1009=IFCLINEARPLACEMENT($,#1002,#1008);
#1010=IFCBUILDINGELEMENTPROXY('0Y0f2uCbr0gfm5cmtZmh02',$,'CACHE_GRADE_OUTGOING',$,$,#1009,$,$,$);
#1011=IFCAXIS2PLACEMENTLINEAR(#86,$,$);
#1012=IFCCARTESIANPOINT((10.5,1.,0.));
#1013=IFCAXIS2PLACEMENT3D(#1012,$,$);
#1014=IFCLINEARPLACEMENT($,#1011,#1013);
#1015=IFCBUILDINGELEMENTPROXY('0Y0f2uCbr0gfm5cmtZmh03',$,'CACHE_CORNER_INCOMING',$,$,#1014,$,$,$);
#1016=IFCCARTESIANPOINT((9.,0.5,0.));
#1017=IFCAXIS2PLACEMENT3D(#1016,$,$);
#1018=IFCLINEARPLACEMENT($,#1011,#1017);
#1019=IFCBUILDINGELEMENTPROXY('0Y0f2uCbr0gfm5cmtZmh04',$,'CACHE_CORNER_OUTGOING',$,$,#1018,$,$,$);
#1020=IFCAXIS2PLACEMENTLINEAR(#93,$,$);
#1021=IFCLINEARPLACEMENT($,#1020,$);
#1022=IFCBUILDINGELEMENTPROXY('0Y0f2uCbr0gfm5cmtZmh05',$,'UNCACHED_CORNER_NEAR',$,$,#1021,$,$,$);";

/// Beyond the station-relations fixture: points on and 4 um before the
/// joint of the gradient curve's segments at 40 m.
const RELATIONS_EXTRA: &str =
    "#1000=IFCPOINTBYDISTANCEEXPRESSION(IFCLENGTHMEASURE(40.),$,2.,$,#64);
#1001=IFCPOINTBYDISTANCEEXPRESSION(IFCLENGTHMEASURE(39.999996),$,2.,$,#64);";

fn seams() -> Model {
    fixture("station_seams_ifc4x3.ifc", SEAMS_EXTRA)
}

fn relations() -> Model {
    fixture("station_relations_ifc4x3.ifc", RELATIONS_EXTRA)
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

/// The lowered station of `expression`: its basis, distance, side and
/// offsets.
fn lowered(model: &Model, expression: u64) -> (Curve3, f64, SeamSide, StationOffsets) {
    let scale = units::resolve(model);
    let mut session = LoweringSession::new(model, &scale);
    let root =
        lower_point_by_distance_node(&mut session, EntityId(expression), Transform::identity())
            .expect("the station lowers");
    let lowered = session.finish(root).expect("a valid graph");
    let (station, side) = match lowered.graph.get(lowered.root) {
        Some(GeometryNode::CurveStation(station)) => (*station, SeamSide::Outgoing),
        Some(GeometryNode::OrientedCurveStation(oriented)) => {
            assert!(oriented.orientation.is_base(), "#{expression}: unturned");
            (oriented.station, oriented.seam)
        }
        other => panic!("#{expression}: a station, got {other:?}"),
    };
    let Some(GeometryNode::Curve3(curve)) = lowered.graph.get(station.basis) else {
        panic!("#{expression}: an atomic 3D basis");
    };
    (
        curve.clone(),
        station.station.distance,
        side,
        station.station.offsets,
    )
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

fn point(p: Point3) -> [f64; 3] {
    [p.x, p.y, p.z]
}

/// `transform` is `section` with `offsets` applied: the IFC frame
/// `(tangent, left, up)` at the offset point.
fn assert_frame(
    transform: &Transform,
    section: &SectionFrame,
    offsets: StationOffsets,
    what: &str,
) {
    let placed = section.place(offsets.lateral, offsets.vertical, offsets.longitudinal);
    close(transform.origin, point(placed), &format!("{what}: origin"));
    close(
        transform.basis[0],
        array(section.tangent),
        &format!("{what}: x, the tangent"),
    );
    close(
        transform.basis[1],
        array(section.lateral),
        &format!("{what}: y, the left"),
    );
    close(
        transform.basis[2],
        array(section.up),
        &format!("{what}: z, up"),
    );
}

/// The derived placement of `expression` equals its lowered station's
/// frame, on side `side` at `seam`, and differs from the other side's.
fn assert_matches_station(model: &Model, expression: u64, seam: f64, side: SeamSide, what: &str) {
    let evaluator = ReferenceCurveEvaluator::default();
    let transform = derived(model, expression, &evaluator).expect(what);
    let (curve, distance, lowered_side, offsets) = lowered(model, expression);
    assert_eq!(lowered_side, side, "{what}: the station's side");
    assert_eq!(distance, seam, "{what}: the station's distance");
    let section = station_section3_on(&curve, distance, side).expect("resolves");
    assert_frame(&transform, &section, offsets, what);

    let other = match side {
        SeamSide::Incoming => SeamSide::Outgoing,
        _ => SeamSide::Incoming,
    };
    let other = station_section3_on(&curve, distance, other).expect("resolves");
    let gap = (0..3)
        .map(|i| (transform.basis[0][i] - array(other.tangent)[i]).abs())
        .fold(0.0, f64::max);
    if side == SeamSide::Incoming {
        assert!(gap > 1e-3, "{what}: not the outgoing tangent");
    }
}

// ---- the derived frame equals the lowered station's ---------------------

#[test]
fn a_placement_on_a_grade_break_takes_the_incoming_grade() {
    let model = seams();
    // #72 on the break, #79 4 um past it: within the 1e-5 m precision.
    for (expression, what) in [(72, "GRADE_BREAK_AT"), (79, "GRADE_BREAK_NEAR")] {
        assert_matches_station(&model, expression, 40.0, SeamSide::Incoming, what);
    }
    // Closed form: up on the incoming 0.02 grade leans back.
    let transform = derived(&model, 79, &ReferenceCurveEvaluator::default()).expect("derived");
    let n = 1.0004_f64.sqrt();
    close(
        transform.origin,
        [40.0 - 0.04 / n, 0.0, 10.8 + 2.0 / n],
        "2 m up the incoming grade",
    );
}

#[test]
fn a_placement_on_a_corner_takes_the_incoming_leg() {
    let model = seams();
    // #86 on the corner, #93 4 um before it.
    for (expression, what) in [(86, "CORNER_AT"), (93, "CORNER_NEAR")] {
        assert_matches_station(&model, expression, 10.0, SeamSide::Incoming, what);
        let transform =
            derived(&model, expression, &ReferenceCurveEvaluator::default()).expect("derived");
        // 1 m left of and 0.5 m along the eastward leg.
        close(transform.origin, [10.5, 1.0, 0.0], what);
    }
}

#[test]
fn a_placement_beyond_the_precision_of_a_seam_reads_the_outgoing_side() {
    // 0.1 mm past the break: off the seam for IFC and for the station.
    let model = seams();
    assert_matches_station(&model, 1000, 40.0001, SeamSide::Outgoing, "past the break");
    let transform = derived(&model, 1000, &ReferenceCurveEvaluator::default()).expect("derived");
    let m = 1.0001_f64.sqrt();
    close(
        transform.basis[0],
        [1.0 / m, 0.0, -0.01 / m],
        "the outgoing grade",
    );
}

#[test]
fn a_polyline_parameter_on_a_corner_takes_the_incoming_leg() {
    // Parameter 1 is the corner vertex (one unit per segment). The station
    // lowering refuses a parameter, so the expected frame is the corner's.
    let model = seams();
    let transform = derived(&model, 1001, &ReferenceCurveEvaluator::default())
        .expect("a parameter on the corner derives");
    let (curve, ..) = lowered(&model, 86);
    let section = station_section3_on(&curve, 10.0, SeamSide::Incoming).expect("resolves");
    assert_frame(
        &transform,
        &section,
        StationOffsets::new(1.0, 0.0, 0.5),
        "parameter 1",
    );
}

#[test]
fn a_placement_on_the_joint_of_a_gradient_curves_segments_takes_the_incoming_segment() {
    let model = relations();
    for (expression, what) in [(1000, "on the joint"), (1001, "4 um before the joint")] {
        assert_matches_station(&model, expression, 40.0, SeamSide::Incoming, what);
    }
}

#[test]
fn segments_placed_at_stations_off_a_seam_derive_the_stations_frame() {
    // The kerb's segments stand at 10 m and 30 m, 3 m left (#69, #72).
    let model = relations();
    for (expression, at) in [(69, 10.0), (72, 30.0)] {
        assert_matches_station(
            &model,
            expression,
            at,
            SeamSide::Outgoing,
            "segment station",
        );
    }
}

#[test]
fn a_placement_on_a_plain_composite_curve_is_refused_by_name_as_before() {
    // The station lowering reads the joint at 10 m of a composite of
    // polylines and an arc as a curve relation; the evaluator contract
    // evaluates one curve, so the derivation refuses that basis by name.
    let model = relations();
    let error = derived(&model, 145, &ReferenceCurveEvaluator::default())
        .expect_err("a composite curve basis");
    let GeometryError::Unsupported {
        type_name, detail, ..
    } = error
    else {
        panic!("expected Unsupported, got {error:?}");
    };
    assert_eq!(type_name, "IFCCOMPOSITECURVE");
    assert!(detail.contains("basis curve"), "{detail}");
}

// ---- #354: Verify compares against the incoming frame -------------------

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
fn verify_accepts_a_cache_computed_with_the_incoming_tangent() {
    let model = seams();
    let evaluator = ReferenceCurveEvaluator::default();
    for (name, expression) in [("CACHE_GRADE_INCOMING", 72), ("CACHE_CORNER_INCOMING", 86)] {
        let world = world(&model, name, &evaluator).expect(name);
        let expected = derived(&model, expression, &evaluator).expect("derived");
        close(world.origin, expected.origin, name);
        close(world.basis[0], expected.basis[0], name);
    }
    // An uncached placement 4 um before the corner derives the corner's frame.
    let world = world(&model, "UNCACHED_CORNER_NEAR", &evaluator).expect("derived");
    close(world.origin, [10.5, 1.0, 0.0], "uncached near the corner");
}

#[test]
fn verify_refuses_a_cache_computed_with_the_outgoing_tangent() {
    let model = seams();
    let evaluator = ReferenceCurveEvaluator::default();
    for (name, cached) in [
        (
            "CACHE_GRADE_OUTGOING",
            [40.019999000074996, 0.0, 12.799900007499376],
        ),
        ("CACHE_CORNER_OUTGOING", [9.0, 0.5, 0.0]),
    ] {
        match world(&model, name, &evaluator) {
            Err(GeometryError::CachedPlacementMismatch {
                cached: found,
                distance,
                tolerance,
                ..
            }) => {
                close(found, cached, name);
                assert!(
                    distance > 100.0 * tolerance,
                    "{name}: {distance} vs {tolerance}"
                );
            }
            other => panic!("{name}: expected CachedPlacementMismatch, got {other:?}"),
        }
    }
}

// ---- an evaluator without seam sides is refused by name -----------------

/// Delegates the side-less queries to the reference evaluator and leaves
/// the sided ones to the contract's defaults, which refuse `Incoming`.
#[derive(Debug, Default)]
struct Sideless(ReferenceCurveEvaluator);

impl Backend for Sideless {
    fn descriptor(&self) -> BackendDescriptor {
        self.0.descriptor()
    }
}

impl CurveEvaluator for Sideless {
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
}

#[test]
fn an_evaluator_without_seam_sides_is_refused_naming_the_seam() {
    let model = seams();
    for (expression, basis, seam) in [
        (72, 39, 40.0),
        (79, 39, 40.0),
        (93, 43, 10.0),
        (1001, 43, 10.0),
    ] {
        let error = derived(&model, expression, &Sideless::default())
            .expect_err("never the outgoing frame");
        assert!(error.is_unsupported(), "{error}");
        assert_eq!(error.entity(), Some(EntityId(expression)));
        let message = error.to_string();
        let GeometryError::SeamSideUnsupported {
            placement,
            basis: found,
            distance,
        } = error
        else {
            panic!("#{expression}: expected SeamSideUnsupported, got {error:?}");
        };
        assert_eq!(
            (placement, found, distance),
            (EntityId(expression), EntityId(basis), seam)
        );
        assert!(message.contains("tangent discontinuity"), "{message}");
    }
    // Off a seam it derives as before, and lowering refuses on one alike.
    let off = derived(&model, 1000, &Sideless::default()).expect("off a seam");
    let reference = derived(&model, 1000, &ReferenceCurveEvaluator::default()).expect("derived");
    assert_eq!(off, reference);
    assert!(matches!(
        world(&model, "CACHE_GRADE_INCOMING", &Sideless::default()),
        Err(GeometryError::SeamSideUnsupported { .. })
    ));
}

// ---- #355 against kernel#242's documented axes ----------------------------

/// kernel#242 documents the evaluator's frame as `x` tangent, `y` up, `z`
/// right. The derived IFC frame reads only `x`, so it is `(x, -z, y)` of
/// it, on a seam's incoming side and off a seam alike.
#[test]
fn the_derived_frame_is_the_evaluators_x_minus_z_y() {
    let evaluator = ReferenceCurveEvaluator::default();
    for (model, expression) in [
        (seams(), 72),
        (seams(), 1000),
        (seams(), 86),
        (relations(), 69),
    ] {
        let transform = derived(&model, expression, &evaluator).expect("derived");
        let (curve, distance, side, _) = lowered(&model, expression);
        let frame = evaluator
            .frame_at_on(&curve, CurveMeasure::Distance(distance), side)
            .expect("the evaluator frames it");
        let what = format!("#{expression}");
        close(transform.basis[0], array(frame.x), &format!("{what}: x"));
        close(
            transform.basis[1],
            array(-frame.z),
            &format!("{what}: left = -z"),
        );
        close(
            transform.basis[2],
            array(frame.y),
            &format!("{what}: up = y"),
        );
    }
}
