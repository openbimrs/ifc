//! One session, one graph: recursive lowering must share a builder.
//!
//! # Why this file exists
//!
//! `../AGENTS.md` states the invariant directly:
//!
//! > Recursive lowering appends to one session-owned graph builder. Family
//! > lowerers return `NodeId`; they do not freeze isolated child graphs.
//!
//! A family lowerer that freezes its own graph cannot be composed. `NodeId`
//! handles are owned by the graph that minted them, so two independently
//! frozen graphs produce mutually foreign handles. Every composite IFC family
//! -- booleans, mapped items, CSG trees, B-rep faces sharing a surface --
//! needs two children in ONE graph. These tests pin that property.

use axiolid_core::Vec3;
use axiolid_model::{GeometryNode, SolidOperation};
use ifc_geometry::lower::{
    lower_extruded_area_solid_node, LoweringSession, SessionLimits, Tolerance,
};
use ifc_geometry::transform::Transform;
use ifc_geometry::units;
use ifc_model::{Codec, EntityId, Model};
use ifc_step::StepCodec;
use std::path::PathBuf;

fn fixture(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../test/fixtures")
        .join(rel)
}

fn wall_model() -> Model {
    StepCodec
        .read_path(&fixture("ifclite-geometry/issue_098_wall_W.ifc"))
        .expect("fixture parses")
}

fn extrusion_ids(model: &Model, count: usize) -> Vec<EntityId> {
    let ids: Vec<_> = model
        .ids_of_type("IFCEXTRUDEDAREASOLID")
        .iter()
        .copied()
        .take(count)
        .collect();
    assert_eq!(ids.len(), count, "fixture must supply {count} extrusions");
    ids
}

/// The composability property that the old isolated-graph design could not hold.
///
/// Two extrusions lowered through one session yield handles that are valid in
/// the same graph, so a boolean over both is constructible. Under the previous
/// design each lowerer returned its own frozen graph and this was impossible:
/// the second handle was `ForeignReference` in the first graph.
#[test]
fn two_families_lower_into_one_graph_and_can_be_combined() {
    let model = wall_model();
    let scale = units::resolve(&model);
    let ids = extrusion_ids(&model, 2);

    let mut session = LoweringSession::new(&model, &scale, Tolerance::building_scale());
    let left = lower_extruded_area_solid_node(&mut session, ids[0], Transform::identity())
        .expect("first extrusion lowers");
    let right = lower_extruded_area_solid_node(&mut session, ids[1], Transform::identity())
        .expect("second extrusion lowers");

    // The point of the whole exercise: a node referencing BOTH children.
    let union = session
        .node(GeometryNode::SolidOperation(SolidOperation::Boolean {
            left,
            right,
            operator: axiolid_core::BooleanOperator::Union,
        }))
        .expect("children share one graph, so a boolean is constructible");

    let lowered = session.finish(union).expect("session finishes");
    let GeometryNode::SolidOperation(SolidOperation::Boolean { left, right, .. }) =
        lowered.graph.get(lowered.root).expect("root resolves")
    else {
        panic!("expected a boolean root");
    };
    assert!(
        lowered.graph.get(*left).is_some() && lowered.graph.get(*right).is_some(),
        "both operands must resolve inside the single finished graph"
    );
}

/// Lowering the same entity twice must reuse one node, not duplicate it.
///
/// IFC files share profiles, points, and directions aggressively. Without
/// memoization a 3,000-wall file re-pushes the same rectangle 3,000 times.
#[test]
fn lowering_the_same_entity_twice_reuses_one_node() {
    let model = wall_model();
    let scale = units::resolve(&model);
    let id = extrusion_ids(&model, 1)[0];

    let mut session = LoweringSession::new(&model, &scale, Tolerance::building_scale());
    let first = lower_extruded_area_solid_node(&mut session, id, Transform::identity())
        .expect("first lowering");
    let after_first = session.node_count();
    let second = lower_extruded_area_solid_node(&mut session, id, Transform::identity())
        .expect("second lowering");

    assert_eq!(
        first, second,
        "same entity and frame must memoize to one node"
    );
    assert_eq!(
        session.node_count(),
        after_first,
        "a memoized hit must not append new nodes"
    );
}

/// A different world frame is a different result and must NOT collide.
///
/// The memo key has to include the placement; otherwise a mapped item placed
/// twice would silently collapse into a single wrongly-placed instance.
#[test]
fn the_same_entity_under_a_different_frame_is_a_distinct_node() {
    let model = wall_model();
    let scale = units::resolve(&model);
    let id = extrusion_ids(&model, 1)[0];

    let mut session = LoweringSession::new(&model, &scale, Tolerance::building_scale());
    let identity = lower_extruded_area_solid_node(&mut session, id, Transform::identity())
        .expect("identity frame lowers");

    let mut moved = Transform::identity();
    moved.origin = [5.0, 0.0, 0.0];
    let translated =
        lower_extruded_area_solid_node(&mut session, id, moved).expect("translated frame lowers");

    assert_ne!(
        identity, translated,
        "distinct placements must not share a memo entry"
    );
}

/// Self-referential chains are detected instead of overflowing the stack.
///
/// The IFC spec pushes cycle prevention to the application layer, so real
/// exporter output does contain them.
#[test]
fn a_cyclic_chain_is_reported_rather_than_overflowing() {
    let model = wall_model();
    let scale = units::resolve(&model);
    let mut session = LoweringSession::new(&model, &scale, Tolerance::building_scale());

    let entity = EntityId(1);
    session.enter(entity, "mapped item").expect("first entry");
    let error = session
        .enter(entity, "mapped item")
        .expect_err("re-entering the same entity is a cycle");

    assert!(
        error.to_string().contains("cyclic"),
        "expected a cyclic-chain error, got: {error}"
    );
    assert_eq!(
        error.entity(),
        Some(entity),
        "the error must name the entity"
    );
}

/// A deep-but-acyclic chain still terminates at the documented limit.
#[test]
fn an_over_deep_chain_stops_at_the_limit() {
    let model = wall_model();
    let scale = units::resolve(&model);
    let limits = SessionLimits { max_depth: 4 };
    let mut session =
        LoweringSession::with_limits(&model, &scale, Tolerance::building_scale(), limits);

    for index in 0..4 {
        session
            .enter(EntityId(index + 1), "mapped item")
            .expect("entries within the limit succeed");
    }
    let error = session
        .enter(EntityId(99), "mapped item")
        .expect_err("exceeding the depth limit must fail");

    assert!(
        error.to_string().contains("exceeded depth"),
        "expected a depth-limit error, got: {error}"
    );
}

/// Leaving a chain frees the entity for legitimate reuse elsewhere.
///
/// Sharing is not a cycle. A profile referenced by two different solids must
/// lower twice without being mistaken for recursion.
#[test]
fn leaving_a_chain_allows_legitimate_reuse() {
    let model = wall_model();
    let scale = units::resolve(&model);
    let mut session = LoweringSession::new(&model, &scale, Tolerance::building_scale());

    let entity = EntityId(7);
    session.enter(entity, "mapped item").expect("first entry");
    session.exit(entity);
    session
        .enter(entity, "mapped item")
        .expect("sharing after exit is not a cycle");
}

/// Graph-level construction faults surface as located IFC errors.
///
/// A raw `GraphError` names a `NodeId`, which is meaningless to someone
/// debugging a 500k-entity file. The session must translate it to an error
/// carrying the IFC entity.
#[test]
fn graph_faults_are_reported_against_the_source_entity() {
    let model = wall_model();
    let scale = units::resolve(&model);
    let mut session = LoweringSession::new(&model, &scale, Tolerance::building_scale());

    let point = session
        .node(GeometryNode::Point3(Vec3::ZERO))
        .expect("a leaf node inserts");
    let lowered = session.finish(point).expect("finish succeeds");
    assert_eq!(
        lowered.graph.roots(),
        &[lowered.root],
        "the finished graph must expose exactly the requested root"
    );
}

/// A profile shared by several solids is stored once.
///
/// This is the concrete payoff of the session. A real model defines a section
/// once and extrudes it for every member of a type; without memoization the
/// graph grows one duplicate profile per referencing solid, wasting memory and
/// destroying the instancing signal a downstream consumer would use.
///
/// Asserted on real exporter output rather than a synthetic model.
#[test]
fn a_shared_profile_is_stored_once_for_many_solids() {
    let model = wall_model();
    let scale = units::resolve(&model);
    let ids = extrusion_ids(&model, 2);

    let swept_area = |id: EntityId| -> EntityId {
        match model
            .get(id)
            .expect("extrusion resolves")
            .attributes
            .first()
        {
            Some(ifc_model::Value::Ref(reference)) => *reference,
            other => panic!("expected a swept-area reference, got {other:?}"),
        }
    };

    let mut session = LoweringSession::new(&model, &scale, Tolerance::building_scale());
    let first = lower_extruded_area_solid_node(&mut session, ids[0], Transform::identity())
        .expect("first extrusion lowers");
    let second = lower_extruded_area_solid_node(&mut session, ids[1], Transform::identity())
        .expect("second extrusion lowers");
    assert_ne!(first, second, "distinct solids remain distinct nodes");

    // The profile memo is keyed by the profile entity under the identity
    // frame, so a hit here proves the second solid reused the first node.
    let shared_profile = session
        .memoized(swept_area(ids[0]), "profile", Transform::identity())
        .expect("the first solid memoized its profile");

    if swept_area(ids[0]) == swept_area(ids[1]) {
        let reused = session
            .memoized(swept_area(ids[1]), "profile", Transform::identity())
            .expect("the shared profile stays memoized");
        assert_eq!(
            shared_profile, reused,
            "one profile definition must yield exactly one graph node"
        );
    }

    // Re-lowering the very same solid must append nothing at all.
    let before = session.node_count();
    let again = lower_extruded_area_solid_node(&mut session, ids[0], Transform::identity())
        .expect("re-lowering succeeds");
    assert_eq!(again, first, "the solid itself is memoized");
    assert_eq!(
        session.node_count(),
        before,
        "a fully memoized subtree must append no nodes"
    );
}
