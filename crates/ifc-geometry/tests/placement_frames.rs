//! Linear placements composed with the frame their basis curve is stated
//! in (#357), and placements relative to linear placements (#363).
//!
//! The fixture (`tools/gen_lowering_fixtures.py`,
//! `linear_placement_alignment_frame.ifc`) carries the #353 centreline -- a
//! 100 m plan line along +X, a 0.02 grade from 10 m -- on an `IfcAlignment`
//! placed off identity: the site at (100, 0, 0), the alignment at
//! (400, 300, 20) relative to it with its x along world +Y. Expected values
//! are computed here from that description: the alignment's frame maps a
//! local `(x, y, z)` to `(500 - y, 300 + x, 20 + z)`, so the station 40 m
//! along, 2 m left, `(40, 2, 10.8)`, is `(498, 340, 30.8)`. The context's
//! `WorldCoordinateSystem` adds `(10, 20, 0)` to lowered geometry only.

#![cfg(feature = "lowering")]

use ifc_geometry::lower::{lower_product_items, LoweringSession};
use ifc_geometry::{product_world_transform, units, GeometryError, Transform};
use ifc_model::{Codec, Entity, EntityId, Model, Value};
use std::path::PathBuf;

const TOL: f64 = 1e-9;

fn model() -> Model {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../test/fixtures/synthetic-lowering/linear_placement_alignment_frame.ifc");
    ifc_step::StepCodec
        .read_path(&path)
        .expect("fixture parses")
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

/// The `ObjectPlacement` of `id`.
fn placement_of(model: &Model, id: EntityId) -> EntityId {
    model
        .get(id)
        .and_then(|entity| entity.attributes.get(5))
        .and_then(Value::as_ref_id)
        .expect("placed")
}

fn alignment_placement(model: &Model) -> EntityId {
    let (alignment, _) = model.of_type("IFCALIGNMENT").next().expect("an alignment");
    placement_of(model, alignment)
}

fn site_placement(model: &Model) -> EntityId {
    let (site, _) = model.of_type("IFCSITE").next().expect("a site");
    placement_of(model, site)
}

/// Replace attribute `slot` of `id`.
fn set(model: &mut Model, id: EntityId, slot: usize, value: Value) {
    let mut entity = model.get(id).expect("exists").clone();
    entity.attributes[slot] = value;
    model.insert(id, entity);
}

/// The alignment's frame applied to a local point.
fn aligned(p: [f64; 3]) -> [f64; 3] {
    [500.0 - p[1], 300.0 + p[0], 20.0 + p[2]]
}

/// The alignment's frame applied to a local direction.
fn aligned_dir(v: [f64; 3]) -> [f64; 3] {
    [-v[1], v[0], v[2]]
}

const STATION: [f64; 3] = [498.0, 340.0, 30.8];
const WCS: [f64; 3] = [10.0, 20.0, 0.0];

fn norm() -> f64 {
    (1.0f64 + 0.02 * 0.02).sqrt()
}

/// The derived station frame `(tangent, left, up)`, in world axes.
fn derived_axes() -> [[f64; 3]; 3] {
    [
        aligned_dir([1.0 / norm(), 0.0, 0.02 / norm()]),
        aligned_dir([0.0, 1.0, 0.0]),
        aligned_dir([-0.02 / norm(), 0.0, 1.0 / norm()]),
    ]
}

/// The cached station frame: the alignment's own, plumb.
fn cached_axes() -> [[f64; 3]; 3] {
    [
        aligned_dir([1.0, 0.0, 0.0]),
        aligned_dir([0.0, 1.0, 0.0]),
        [0.0, 0.0, 1.0],
    ]
}

fn add(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn scale(v: [f64; 3], s: f64) -> [f64; 3] {
    v.map(|c| c * s)
}

fn assert_vec(actual: [f64; 3], expected: [f64; 3], what: &str) {
    for axis in 0..3 {
        assert!(
            (actual[axis] - expected[axis]).abs() < TOL,
            "{what}: got {actual:?}, expected {expected:?}"
        );
    }
}

fn assert_frame(world: &Transform, origin: [f64; 3], axes: [[f64; 3]; 3]) {
    assert_vec(world.origin, origin, "origin");
    for (i, axis) in axes.iter().enumerate() {
        assert_vec(world.basis[i], *axis, &format!("axis {i}"));
    }
}

fn cache_only(model: &Model, name: &str) -> Result<Transform, GeometryError> {
    product_world_transform(model, &units::resolve(model), product(model, name))
}

/// Centre of the lowered block: the mean Instance translation, with
/// Instance frames composed.
fn centre_of(lowered: &ifc_geometry::lower::LoweredGeometry) -> [f64; 3] {
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
    sum.map(|value| value / count as f64)
}

fn lowered_centre_cache_only(model: &Model, name: &str) -> Result<[f64; 3], GeometryError> {
    let scale = units::resolve(model);
    let mut session = LoweringSession::new(model, &scale);
    let root = lower_product_items(&mut session, product(model, name))?.expect("a Body");
    Ok(centre_of(&session.finish(root)?))
}

// ---- #357: the cache path composes with the alignment's frame ------------

#[test]
fn alignment_frame_cache_path_places_the_product_in_the_alignment_frame() {
    let model = model();
    for name in ["CACHED", "CACHED_IMPLIED"] {
        let world = cache_only(&model, name).expect("the cache composes with the alignment");
        assert_frame(&world, STATION, cached_axes());
    }
    // The CartesianPosition alone is (40, 2, 10.8): far from the world
    // position, which is what reading it as world coordinates produced.
    assert_vec(aligned([40.0, 2.0, 10.8]), STATION, "the expected station");
}

#[test]
fn alignment_frame_cache_path_lowers_with_the_wcs_applied_once() {
    let model = model();
    let centre = lowered_centre_cache_only(&model, "CACHED").expect("lowers");
    assert_vec(
        centre,
        add(add(STATION, WCS), [0.0, 0.0, 0.5]),
        "block centre",
    );
}

#[test]
fn alignment_frame_uncached_without_an_evaluator_is_still_refused() {
    let model = model();
    let error = cache_only(&model, "REL_TO").expect_err("nothing to read");
    assert!(error.is_unsupported(), "{error}");
}

#[test]
fn alignment_frame_a_conflicting_placement_rel_to_is_refused() {
    let mut model = model();
    let cached = placement_of(&model, product(&model, "CACHED"));
    let site = site_placement(&model);
    set(&mut model, cached, 0, Value::Ref(site));
    let error = cache_only(&model, "CACHED").expect_err("two frames");
    assert!(
        matches!(
            error,
            GeometryError::PlacementRelToConflict { placement, stated, implied }
                if placement == cached && stated == site && implied == alignment_placement(&model)
        ),
        "{error:?}"
    );
}

#[test]
fn alignment_frame_an_equal_copy_of_the_alignment_placement_is_accepted() {
    let mut model = model();
    let original = alignment_placement(&model);
    let copy = model.push(model.get(original).expect("exists").clone());
    let cached = placement_of(&model, product(&model, "CACHED"));
    set(&mut model, cached, 0, Value::Ref(copy));
    let world = cache_only(&model, "CACHED").expect("the same frame, stated twice");
    assert_frame(&world, STATION, cached_axes());
}

// ---- #363: placements relative to a linear placement ----------------------

#[test]
fn alignment_frame_a_bracket_relative_to_a_cached_linear_placement_composes() {
    let model = model();
    let world = cache_only(&model, "BRACKET_CACHED").expect("composes");
    // (1, 0, 3) in the cached station frame.
    let origin = add(STATION, add(cached_axes()[0], scale(cached_axes()[2], 3.0)));
    assert_frame(&world, origin, cached_axes());
    let centre = lowered_centre_cache_only(&model, "BRACKET_CACHED").expect("lowers");
    assert_vec(
        centre,
        add(add(origin, WCS), [0.0, 0.0, 0.5]),
        "bracket centre",
    );
}

#[test]
fn alignment_frame_a_mixed_cycle_through_a_linear_placement_is_refused() {
    // The alignment placed relative to the bracket that sits on a station
    // of that very alignment.
    let mut model = model();
    let alignment = alignment_placement(&model);
    let bracket = placement_of(&model, product(&model, "BRACKET_CACHED"));
    set(&mut model, alignment, 0, Value::Ref(bracket));
    let error = cache_only(&model, "BRACKET_CACHED").expect_err("cyclic");
    assert!(
        matches!(error, GeometryError::CyclicChain { .. }),
        "{error:?}"
    );
}

#[test]
fn alignment_frame_the_depth_limit_counts_a_mixed_chain() {
    // 64 local placements stacked on the cached linear placement, which
    // sits on the alignment, site: deeper than the limit.
    let mut model = model();
    let mut parent = placement_of(&model, product(&model, "CACHED"));
    let axes = model.push(Entity::new(
        "IFCAXIS2PLACEMENT3D",
        vec![Value::Ref(EntityId(1)), Value::Null, Value::Null],
    ));
    for _ in 0..64 {
        parent = model.push(Entity::new(
            "IFCLOCALPLACEMENT",
            vec![Value::Ref(parent), Value::Ref(axes)],
        ));
    }
    let mut resolver = ifc_geometry::constraint::local::PlacementResolver::new();
    let error = resolver
        .world_transform(&model, parent)
        .expect_err("too deep");
    assert!(
        matches!(error, GeometryError::ChainTooDeep { limit: 64, .. }),
        "{error:?}"
    );
}

// ---- the derived path ------------------------------------------------------

#[cfg(feature = "compile")]
mod derived {
    use super::*;
    use axiolid_evaluate::ReferenceCurveEvaluator;
    use ifc_geometry::{product_world_transform_with_evaluator, CachedPositionPolicy};

    fn with_evaluator(
        model: &Model,
        name: &str,
        cached: CachedPositionPolicy,
    ) -> Result<Transform, GeometryError> {
        product_world_transform_with_evaluator(
            model,
            &units::resolve(model),
            product(model, name),
            &ReferenceCurveEvaluator::default(),
            cached,
        )
    }

    fn lowered_centre(model: &Model, name: &str) -> Result<[f64; 3], GeometryError> {
        let evaluator = ReferenceCurveEvaluator::default();
        let scale = units::resolve(model);
        let mut session = LoweringSession::new(model, &scale).with_curve_evaluator(&evaluator);
        let root = lower_product_items(&mut session, product(model, name))?.expect("a Body");
        Ok(centre_of(&session.finish(root)?))
    }

    #[test]
    fn alignment_frame_derived_path_places_the_product_in_the_alignment_frame() {
        let model = model();
        for name in ["REL_TO", "IMPLIED"] {
            let world = with_evaluator(&model, name, CachedPositionPolicy::Verify)
                .expect("derived in the alignment's frame");
            assert_frame(&world, STATION, derived_axes());
            let centre = lowered_centre(&model, name).expect("lowers");
            let expected = add(add(STATION, WCS), scale(derived_axes()[2], 0.5));
            assert_vec(centre, expected, "block centre");
        }
    }

    #[test]
    fn alignment_frame_verify_accepts_a_correct_cache_off_identity() {
        let model = model();
        for name in ["CACHED", "CACHED_IMPLIED"] {
            let world = with_evaluator(&model, name, CachedPositionPolicy::Verify)
                .expect("the cache agrees once both compose with the alignment");
            // The linear expression is authoritative.
            assert_frame(&world, STATION, derived_axes());
        }
        let trusted = with_evaluator(&model, "CACHED", CachedPositionPolicy::Trust).expect("ok");
        assert_frame(&trusted, STATION, cached_axes());
    }

    #[test]
    fn alignment_frame_verify_reports_a_stale_cache_in_world_coordinates() {
        let mut model = model();
        let cached = placement_of(&model, product(&model, "CACHED"));
        let position = model
            .get(cached)
            .and_then(|e| e.attributes.get(2))
            .and_then(Value::as_ref_id)
            .expect("cached");
        let point = model
            .get(position)
            .and_then(|e| e.attributes.first())
            .and_then(Value::as_ref_id)
            .expect("a location");
        // 60 m along instead of 40.
        set(
            &mut model,
            point,
            0,
            Value::List(vec![Value::Real(60.0), Value::Real(2.0), Value::Real(10.8)]),
        );
        let error =
            with_evaluator(&model, "CACHED", CachedPositionPolicy::Verify).expect_err("stale");
        let GeometryError::CachedPlacementMismatch {
            cached, derived, ..
        } = error
        else {
            panic!("expected CachedPlacementMismatch, got {error:?}");
        };
        assert_vec(cached, aligned([60.0, 2.0, 10.8]), "cached, in world");
        assert_vec(derived, STATION, "derived, in world");
    }

    #[test]
    fn alignment_frame_a_bracket_relative_to_a_derived_linear_placement_composes() {
        let model = model();
        let world = with_evaluator(&model, "BRACKET_DERIVED", CachedPositionPolicy::Verify)
            .expect("composes");
        let [tangent, _, up] = derived_axes();
        let origin = add(STATION, add(tangent, scale(up, 3.0)));
        assert_frame(&world, origin, derived_axes());
        let centre = lowered_centre(&model, "BRACKET_DERIVED").expect("lowers");
        assert_vec(
            centre,
            add(add(origin, WCS), scale(up, 0.5)),
            "bracket centre",
        );
    }

    #[test]
    fn alignment_frame_derived_conflict_is_refused_before_deriving() {
        let mut model = model();
        let rel_to = placement_of(&model, product(&model, "REL_TO"));
        let site = site_placement(&model);
        set(&mut model, rel_to, 0, Value::Ref(site));
        let error =
            with_evaluator(&model, "REL_TO", CachedPositionPolicy::Verify).expect_err("two frames");
        assert!(
            matches!(error, GeometryError::PlacementRelToConflict { .. }),
            "{error:?}"
        );
    }
}
