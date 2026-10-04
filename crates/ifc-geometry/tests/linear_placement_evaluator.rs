//! Product lowering that derives an `IfcLinearPlacement` through a
//! caller-supplied evaluator (#353), and checks a cached `CartesianPosition`
//! against the derivation (#354).
//!
//! The fixtures (`tools/gen_lowering_fixtures.py`) place 1 m blocks along an
//! `IfcGradientCurve`: a 100 m plan line along +X, a 0.02 grade from 10 m.
//! Expected values are computed here from that description, not read back
//! from the code under test: at 40 m the centreline is (40, 0, 10.8), the
//! tangent (1, 0, 0.02) / sqrt(1.0004); IFC4.3 puts a positive
//! `OffsetLateral` to the left, (0, 1, 0), and `OffsetVertical` and the
//! default `Axis` on up = tangent x left = (-0.02, 0, 1) / sqrt(1.0004).
//! The reference evaluator is a dev-dependency: the library links none.

#![cfg(feature = "compile")]

use axiolid_contracts::GeomResult;
use axiolid_core::{Frame3, Point3, Vec3};
use axiolid_curve::Curve3;
use axiolid_curve_evaluate_contract::{CurveEvaluator, CurveMeasure, DistanceConvention};
use axiolid_evaluate::ReferenceCurveEvaluator;
use ifc_geometry::constraint::placement::derive::cached_position_tolerance;
use ifc_geometry::lower::{
    lower_product_items, lower_product_net, LoweringSession, RepresentationPurpose,
};
use ifc_geometry::units::UnitScale;
use ifc_geometry::{
    product_representation_frame_with_evaluator, product_world_transform,
    product_world_transform_with_evaluator, units, CachedPositionPolicy, GeometryError, Transform,
};
use ifc_model::{Codec, Entity, EntityId, Model, Value};
use std::path::PathBuf;

const TOL: f64 = 1e-9;

fn fixture(name: &str) -> Model {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../test/fixtures/synthetic-lowering")
        .join(name);
    ifc_step::StepCodec
        .read_path(&path)
        .expect("fixture parses")
}

fn uncached() -> Model {
    fixture("linear_placement_uncached.ifc")
}

fn cache_check() -> Model {
    fixture("linear_placement_cache_check.ifc")
}

/// The proxy named `name`.
fn product(model: &Model, name: &str) -> EntityId {
    let found: Vec<EntityId> = model
        .of_type("IFCBUILDINGELEMENTPROXY")
        .filter(|(_, entity)| {
            matches!(entity.attributes.get(2), Some(Value::Text(text)) if &**text == name)
        })
        .map(|(id, _)| id)
        .collect();
    assert_eq!(found.len(), 1, "exactly one proxy named {name}");
    found[0]
}

fn norm() -> f64 {
    (1.0f64 + 0.02 * 0.02).sqrt()
}

fn tangent() -> [f64; 3] {
    [1.0 / norm(), 0.0, 0.02 / norm()]
}

const LEFT: [f64; 3] = [0.0, 1.0, 0.0];

fn up() -> [f64; 3] {
    [-0.02 / norm(), 0.0, 1.0 / norm()]
}

/// 40 m along, 2 m left.
const AT_40_LEFT_2: [f64; 3] = [40.0, 2.0, 10.8];

fn assert_vec(actual: [f64; 3], expected: [f64; 3], what: &str) {
    for axis in 0..3 {
        assert!(
            (actual[axis] - expected[axis]).abs() < TOL,
            "{what}: got {actual:?}, expected {expected:?}"
        );
    }
}

fn assert_derived_frame(world: &Transform) {
    assert_vec(world.origin, AT_40_LEFT_2, "origin");
    assert_vec(world.basis[0], tangent(), "x: the tangent");
    assert_vec(world.basis[1], LEFT, "y: the left");
    assert_vec(world.basis[2], up(), "z: up, perpendicular to the tangent");
}

fn with_evaluator(
    model: &Model,
    name: &str,
    cached: CachedPositionPolicy,
) -> Result<Transform, GeometryError> {
    let evaluator = ReferenceCurveEvaluator::default();
    product_world_transform_with_evaluator(
        model,
        &units::resolve(model),
        product(model, name),
        &evaluator,
        cached,
    )
}

/// Centre of the lowered block, in metres: the mean of the graph's Instance
/// translations and points, with Instance frames applied (the block's
/// Instance carries its centre).
fn lowered_centre(model: &Model, name: &str) -> Result<[f64; 3], GeometryError> {
    let evaluator = ReferenceCurveEvaluator::default();
    let scale = units::resolve(model);
    let mut session = LoweringSession::new(model, &scale).with_curve_evaluator(&evaluator);
    let root = lower_product_items(&mut session, product(model, name))?.expect("a Body");
    let lowered = session.finish(root)?;
    let graph = &lowered.graph;
    let mut sum = [0.0f64; 3];
    let mut count = 0usize;
    let mut stack = vec![(lowered.root, axiolid_core::Transform3::IDENTITY)];
    while let Some((id, frame)) = stack.pop() {
        match graph.get(id).expect("node exists") {
            axiolid_model::GeometryNode::Instance(instance) => {
                let composed = frame * instance.transform;
                let t = composed.translation;
                sum = [sum[0] + t.x, sum[1] + t.y, sum[2] + t.z];
                count += 1;
                stack.push((instance.source, composed));
            }
            axiolid_model::GeometryNode::Collection(members) => {
                stack.extend(members.iter().map(|member| (*member, frame)));
            }
            _ => {}
        }
    }
    assert!(count > 0, "the block is placed by an Instance");
    Ok(sum.map(|value| value / count as f64))
}

/// The block's centre for a product framed as derived: half a metre up its
/// own z from the placement origin.
fn derived_block_centre() -> [f64; 3] {
    std::array::from_fn(|i| AT_40_LEFT_2[i] + 0.5 * up()[i])
}

// ---- #353: an uncached placement derived through the evaluator ----------

#[test]
fn linear_placement_uncached_lowers_to_the_derived_position() {
    let model = uncached();
    let centre = lowered_centre(&model, "LENGTH").expect("derived through the evaluator");
    assert_vec(centre, derived_block_centre(), "block centre");
}

#[test]
fn linear_placement_uncached_world_transform_is_the_derived_frame() {
    let model = uncached();
    let world = with_evaluator(&model, "LENGTH", CachedPositionPolicy::Verify)
        .expect("derived through the evaluator");
    assert_derived_frame(&world);

    let evaluator = ReferenceCurveEvaluator::default();
    let frame = product_representation_frame_with_evaluator(
        &model,
        &units::resolve(&model),
        product(&model, "LENGTH"),
        RepresentationPurpose::Body,
        &evaluator,
        CachedPositionPolicy::Verify,
    )
    .expect("derived")
    .expect("a Body");
    assert_derived_frame(&frame);
}

#[test]
fn linear_placement_uncached_without_an_evaluator_is_refused_as_before() {
    let model = uncached();
    let scale = units::resolve(&model);
    let mut session = LoweringSession::new(&model, &scale);
    assert!(!session.derives_linear_placements());
    let error = lower_product_items(&mut session, product(&model, "LENGTH"))
        .expect_err("the cache-only path has nothing to read");
    let GeometryError::Unsupported { detail, .. } = error else {
        panic!("expected Unsupported, got {error:?}");
    };
    assert!(detail.contains("CartesianPosition"), "{detail}");
}

/// `LENGTH` with its `DistanceAlong` turned into `IfcParameterValue(0.4)`,
/// in memory: a committed fixture keeps only items that lower.
fn with_parameter_value() -> Model {
    let mut model = uncached();
    let expression = model
        .of_type("IFCPOINTBYDISTANCEEXPRESSION")
        .map(|(id, _)| id)
        .min()
        .expect("LENGTH's expression comes first");
    let mut entity = model.get(expression).expect("exists").clone();
    entity.attributes[0] = Value::Typed {
        type_name: "IFCPARAMETERVALUE".into(),
        value: Box::new(Value::Real(0.4)),
    };
    model.insert(expression, entity);
    model
}

#[test]
fn linear_placement_uncached_parameter_value_is_refused_by_name() {
    let model = with_parameter_value();
    let error = lowered_centre(&model, "LENGTH").expect_err("undefined parameter (#347)");
    let GeometryError::Unsupported {
        type_name, detail, ..
    } = &error
    else {
        panic!("expected Unsupported, got {error:?}");
    };
    assert_eq!(type_name, "IFCGRADIENTCURVE");
    assert!(detail.contains("IfcParameterValue"), "{detail}");
}

#[test]
fn linear_placement_uncached_explicit_axes_compose_in_the_curve_frame() {
    // Axis (0, 0, 1) is the curve's up; RefDirection (0, 1, 0) its left.
    // So local x = left, z = up, y = z x x = up x left = -tangent.
    let model = uncached();
    let world = with_evaluator(&model, "AXES", CachedPositionPolicy::Verify).expect("derived");
    assert_vec(world.origin, AT_40_LEFT_2, "origin");
    assert_vec(world.basis[0], LEFT, "x: RefDirection, the left");
    assert_vec(
        world.basis[1],
        tangent().map(|v| -v),
        "y: back along the tangent",
    );
    assert_vec(world.basis[2], up(), "z: Axis, up");
}

/// Delegates to the reference evaluator but claims 3D arc length on every
/// curve, where IFC states plan distance along an alignment.
#[derive(Debug, Default)]
struct ArcLengthEverywhere(ReferenceCurveEvaluator);

impl axiolid_contracts::Backend for ArcLengthEverywhere {
    fn descriptor(&self) -> axiolid_contracts::BackendDescriptor {
        self.0.descriptor()
    }
}

impl CurveEvaluator for ArcLengthEverywhere {
    fn distance_convention(&self, _curve: &Curve3) -> DistanceConvention {
        DistanceConvention::ArcLength3d
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
fn linear_placement_uncached_an_evaluator_measuring_another_distance_is_refused() {
    let model = uncached();
    let error = product_world_transform_with_evaluator(
        &model,
        &units::resolve(&model),
        product(&model, "LENGTH"),
        &ArcLengthEverywhere::default(),
        CachedPositionPolicy::Verify,
    )
    .expect_err("a 3D arc length is not the plan distance IFC states");
    let GeometryError::Unsupported { detail, .. } = error else {
        panic!("expected Unsupported, got {error:?}");
    };
    assert!(detail.contains("different distance"), "{detail}");
}

// ---- #354: a cached position checked against the derivation ------------

#[test]
fn linear_placement_cache_check_a_matching_cache_lowers_to_the_derived_frame() {
    let model = cache_check();
    let world = with_evaluator(&model, "MATCH", CachedPositionPolicy::Verify)
        .expect("the cache agrees with the expression");
    // The linear expression is authoritative: the cache's plumb axes are not
    // what is returned, and are not compared.
    assert_derived_frame(&world);
    let centre = lowered_centre(&model, "MATCH").expect("lowers");
    assert_vec(centre, derived_block_centre(), "block centre");
}

#[test]
fn linear_placement_cache_check_within_the_declared_precision_lowers() {
    // 4e-6 m off, under the declared Precision of 1e-5 m.
    let model = cache_check();
    let world = with_evaluator(&model, "NEAR", CachedPositionPolicy::Verify).expect("within");
    assert_derived_frame(&world);
}

fn mismatch(error: GeometryError) -> ([f64; 3], [f64; 3], f64, f64) {
    match error {
        GeometryError::CachedPlacementMismatch {
            cached,
            derived,
            distance,
            tolerance,
            ..
        } => (cached, derived, distance, tolerance),
        other => panic!("expected CachedPlacementMismatch, got {other:?}"),
    }
}

#[test]
fn linear_placement_cache_check_just_outside_the_declared_precision_is_refused() {
    // 5e-5 m off: five times the declared Precision.
    let model = cache_check();
    let error = with_evaluator(&model, "OUTSIDE", CachedPositionPolicy::Verify)
        .expect_err("beyond the model's tolerance");
    let (cached, _, distance, tolerance) = mismatch(error);
    assert_vec(cached, [40.00005, 2.0, 10.8], "cached");
    assert!((distance - 5e-5).abs() < 1e-9, "distance {distance}");
    assert!((tolerance - 1e-5).abs() < 1e-12, "tolerance {tolerance}");
}

#[test]
fn linear_placement_cache_check_a_stale_cache_is_refused_naming_both_positions() {
    let model = cache_check();
    let stale = product(&model, "STALE");
    let error = with_evaluator(&model, "STALE", CachedPositionPolicy::Verify)
        .expect_err("a stale cache is not a position");
    let placement = error.entity().expect("names the placement");
    assert!(model
        .get(placement)
        .expect("exists")
        .is_type("IFCLINEARPLACEMENT"));
    let message = error.to_string();
    let (cached, derived, distance, _) = mismatch(error);
    assert_vec(cached, [60.0, 2.0, 11.2], "cached");
    assert_vec(derived, AT_40_LEFT_2, "derived");
    assert!((distance - (400.0f64 + 0.16).sqrt()).abs() < 1e-9);
    assert!(
        message.contains("60") && message.contains("40"),
        "{message}"
    );

    // Lowering refuses the same way, the net path included.
    let error = lowered_centre(&model, "STALE").expect_err("lowering checks too");
    mismatch(error);
    let evaluator = ReferenceCurveEvaluator::default();
    let scale = units::resolve(&model);
    let mut session = LoweringSession::new(&model, &scale).with_curve_evaluator(&evaluator);
    mismatch(lower_product_net(&mut session, stale).expect_err("net lowering checks too"));
}

#[test]
fn linear_placement_cache_check_trust_uses_the_cache_as_it_is() {
    let model = cache_check();
    let world = with_evaluator(&model, "STALE", CachedPositionPolicy::Trust).expect("trusted");
    assert_vec(world.origin, [60.0, 2.0, 11.2], "the cached origin");
    assert_vec(world.basis[2], [0.0, 0.0, 1.0], "the cached plumb axis");

    // The session builder carries the policy.
    let evaluator = ReferenceCurveEvaluator::default();
    let scale = units::resolve(&model);
    let mut session = LoweringSession::new(&model, &scale)
        .with_curve_evaluator(&evaluator)
        .with_cached_position_policy(CachedPositionPolicy::Trust);
    assert!(session.derives_linear_placements());
    lower_product_items(&mut session, product(&model, "STALE"))
        .expect("trusted cache lowers")
        .expect("a Body");
}

#[test]
fn linear_placement_cache_check_without_an_evaluator_is_unchanged() {
    // The cache-only entry points read the cache, stale or not.
    let model = cache_check();
    let world = product_world_transform(&model, &units::resolve(&model), product(&model, "STALE"))
        .expect("the cache-only path");
    assert_vec(world.origin, [60.0, 2.0, 11.2], "the cached origin");
}

// ---- the tolerance follows the declared precision and length unit -------

fn model_with_context(precision: Value) -> Model {
    let mut model = Model::new();
    model.insert(
        EntityId(1),
        Entity::new(
            "IFCCARTESIANPOINT",
            vec![Value::List(vec![Value::Real(0.0); 3])],
        ),
    );
    model.insert(
        EntityId(2),
        Entity::new(
            "IFCAXIS2PLACEMENT3D",
            vec![Value::Ref(EntityId(1)), Value::Null, Value::Null],
        ),
    );
    model.insert(
        EntityId(3),
        Entity::new(
            "IFCGEOMETRICREPRESENTATIONCONTEXT",
            vec![
                Value::Null,
                Value::Text("Model".into()),
                Value::Integer(3),
                precision,
                Value::Ref(EntityId(2)),
                Value::Null,
            ],
        ),
    );
    model
}

fn millimetres() -> UnitScale {
    UnitScale {
        length_to_metres: 0.001,
        angle_to_radians: 1.0,
    }
}

#[test]
fn the_cache_tolerance_is_the_declared_precision_in_metres() {
    let model = model_with_context(Value::Real(0.01));
    let tolerance = cached_position_tolerance(&model, &millimetres()).expect("valid");
    assert!(
        (tolerance - 1e-5).abs() < 1e-18,
        "0.01 mm is 1e-5 m: {tolerance}"
    );
}

#[test]
fn without_a_declared_precision_the_cache_tolerance_is_ifcs_default() {
    let model = model_with_context(Value::Null);
    let tolerance = cached_position_tolerance(&model, &millimetres()).expect("valid");
    assert!(
        (tolerance - 1e-8).abs() < 1e-20,
        "1e-5 mm is 1e-8 m: {tolerance}"
    );
}

#[test]
fn an_unusable_declared_precision_is_refused() {
    let model = model_with_context(Value::Real(-1.0));
    let error = cached_position_tolerance(&model, &millimetres()).expect_err("negative");
    assert!(matches!(error, GeometryError::Degenerate { entity, .. } if entity == EntityId(3)));
}
