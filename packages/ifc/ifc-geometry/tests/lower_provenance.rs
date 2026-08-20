//! Source provenance is a side table from neutral graph nodes to IFC entities.
//!
//! The geometry graph stays format-neutral. Attribution travels beside it so
//! diagnostics and downstream inspection can answer which IFC entity produced
//! a node without teaching `axiolid-model` about IFC identifiers.

use axiolid_model::{GeometryNode, SolidOperation};
use ifc_geometry::lower::{lower_extruded_area_solid_node, LoweringSession, Tolerance};
use ifc_geometry::transform::Transform;
use ifc_geometry::units;
use ifc_model::{Codec, EntityId, Model, Value};
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

fn first_extrusion(model: &Model) -> EntityId {
    *model
        .ids_of_type("IFCEXTRUDEDAREASOLID")
        .first()
        .expect("fixture has an extrusion")
}

fn swept_area(model: &Model, extrusion: EntityId) -> EntityId {
    match model
        .get(extrusion)
        .expect("extrusion resolves")
        .attributes
        .first()
    {
        Some(Value::Ref(profile)) => *profile,
        other => panic!("expected swept-area reference, got {other:?}"),
    }
}

#[test]
fn a_real_lowered_subtree_names_the_ifc_entity_for_each_node() {
    let model = wall_model();
    let scale = units::resolve(&model);
    let extrusion = first_extrusion(&model);
    let profile_entity = swept_area(&model, extrusion);
    let mut session = LoweringSession::new(&model, &scale, Tolerance::building_scale());

    let root = lower_extruded_area_solid_node(&mut session, extrusion, Transform::identity())
        .expect("extrusion lowers");
    let lowered = session.finish(root).expect("session finishes");

    assert_eq!(lowered.provenance.source(root), Some(extrusion));
    let GeometryNode::Instance(instance) = lowered.graph.get(root).expect("root resolves") else {
        panic!("placed extrusion has an instance root");
    };
    assert_eq!(
        lowered.provenance.source(instance.source),
        Some(extrusion),
        "the operation is emitted by the extrusion entity"
    );
    let GeometryNode::SolidOperation(SolidOperation::Extrusion { profile, .. }) = lowered
        .graph
        .get(instance.source)
        .expect("operation resolves")
    else {
        panic!("instance source is an extrusion operation");
    };
    assert_eq!(
        lowered.provenance.source(*profile),
        Some(profile_entity),
        "a shared profile keeps its own source entity"
    );
    assert!(
        lowered.provenance.iter().all(|(_, entity)| entity.0 != 0),
        "real IFC lowering must not invent an EntityId(0) provenance source"
    );
    assert_eq!(
        lowered.provenance.len(),
        lowered.graph.iter().count(),
        "every node in this production subtree has an IFC source"
    );
}

#[test]
fn implicit_nodes_follow_the_innermost_active_entity_not_id_order() {
    let model = Model::new();
    let scale = units::resolve(&model);
    let mut session = LoweringSession::new(&model, &scale, Tolerance::building_scale());
    let outer = EntityId(99);
    let inner = EntityId(1);

    session.enter(outer, "outer").expect("outer enters");
    let outer_before = session
        .node(GeometryNode::Point3(axiolid_core::Point3::ZERO))
        .expect("outer node");
    session.enter(inner, "inner").expect("inner enters");
    let inner_node = session
        .node(GeometryNode::Point3(axiolid_core::Point3::ZERO))
        .expect("inner node");
    session.exit(inner);
    let outer_after = session
        .node(GeometryNode::Point3(axiolid_core::Point3::ZERO))
        .expect("outer resumes");
    session.exit(outer);
    let root = session
        .node_for(
            EntityId(77),
            GeometryNode::Collection(vec![outer_before, inner_node, outer_after]),
        )
        .expect("collection root");
    let lowered = session.finish(root).expect("session finishes");

    assert_eq!(lowered.provenance.source(outer_before), Some(outer));
    assert_eq!(lowered.provenance.source(inner_node), Some(inner));
    assert_eq!(lowered.provenance.source(outer_after), Some(outer));
    assert_eq!(lowered.provenance.source(root), Some(EntityId(77)));
}

#[test]
fn caller_synthesized_unscoped_nodes_have_no_fake_ifc_source() {
    let model = Model::new();
    let scale = units::resolve(&model);
    let mut session = LoweringSession::new(&model, &scale, Tolerance::building_scale());

    let root = session
        .node(GeometryNode::Point3(axiolid_core::Point3::ZERO))
        .expect("unscoped node inserts");
    let lowered = session.finish(root).expect("session finishes");

    assert_eq!(lowered.provenance.source(root), None);
    assert!(lowered.provenance.is_empty());
}

#[test]
fn memoized_lowering_reuses_the_original_provenance_entry() {
    let model = wall_model();
    let scale = units::resolve(&model);
    let extrusion = first_extrusion(&model);
    let mut session = LoweringSession::new(&model, &scale, Tolerance::building_scale());

    let first = lower_extruded_area_solid_node(&mut session, extrusion, Transform::identity())
        .expect("first lowering");
    let provenance_after_first = session.provenance().len();
    let second = lower_extruded_area_solid_node(&mut session, extrusion, Transform::identity())
        .expect("memoized lowering");

    assert_eq!(first, second);
    assert_eq!(session.provenance().len(), provenance_after_first);
    assert_eq!(session.provenance().source(second), Some(extrusion));
}
