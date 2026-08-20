//! Mapped-item instancing lowered against real fixtures.
//!
//! # Why instancing must not be flattened
//!
//! A furniture family placed 400 times is one geometry definition and 400
//! transforms. Copying the geometry per occurrence turns a 5 MB model into a
//! 2 GB one. `axiolid_model::Instance` exists to preserve that sharing, and these
//! tests assert the sharing survives lowering.
//!
//! Expected values are read directly out of the fixture files, quoted at each
//! test, so a reviewer can check them against the source records.

use axiolid_model::{GeometryNode, SolidOperation};
use ifc_geometry::lower::{
    lower_mapped_item_node, lower_representation_item, LoweringSession, SessionLimits, Tolerance,
};
use ifc_geometry::transform::Transform;
use ifc_geometry::units;
use ifc_model::{Codec, Entity, EntityId, Model, Value};
use ifc_step::StepCodec;
use std::path::PathBuf;

fn fixture(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../test/fixtures/ifclite-geometry")
        .join(rel)
}

fn model(name: &str) -> Model {
    StepCodec.read_path(&fixture(name)).expect("fixture parses")
}

fn reals(values: &[f64]) -> Value {
    Value::List(values.iter().copied().map(Value::Real).collect())
}

/// Follow `Instance` links down to the first non-instance node.
fn resolve(graph: &axiolid_model::GeometryGraph, mut id: axiolid_model::NodeId) -> &GeometryNode {
    loop {
        match graph.get(id).expect("node resolves") {
            GeometryNode::Instance(instance) => id = instance.source,
            other => return other,
        }
    }
}

fn count_kind(graph: &axiolid_model::GeometryGraph, pick: fn(&GeometryNode) -> bool) -> usize {
    graph.iter().filter(|(_, node)| pick(node)).count()
}

/// Four occurrences of one shared map produce four instances over one subtree.
///
/// Ground truth from `mapped_instances_multi_item.ifc`:
///
/// ```text
/// #18= IFCREPRESENTATIONMAP(#17,#16)          <- one shared map
/// #34= IFCMAPPEDITEM(#18,#27)   target (5,0,0)
/// #41= IFCMAPPEDITEM(#18,#29)   target (0,5,0)
/// #48= IFCMAPPEDITEM(#18,#31)   target (5,5,0)
/// #55= IFCMAPPEDITEM(#18,#33)   target (0,0,7)
/// ```
///
/// The map body holds two extrusions, so a flattening implementation would
/// build eight solids. Instancing must build two and reuse them four times.
#[test]
fn occurrences_of_one_map_share_a_single_lowered_subtree() {
    let model = model("mapped_instances_multi_item.ifc");
    let scale = units::resolve(&model);
    let mut session = LoweringSession::new(&model, &scale, Tolerance::building_scale());

    let items = [EntityId(34), EntityId(41), EntityId(48), EntityId(55)];
    let roots: Vec<_> = items
        .iter()
        .map(|id| {
            lower_mapped_item_node(&mut session, *id, Transform::identity())
                .expect("each occurrence lowers")
        })
        .collect();

    let collection = session
        .node(GeometryNode::Collection(roots.clone()))
        .expect("roots share one graph");
    let lowered = session.finish(collection).expect("session finishes");

    // Distinct occurrences are distinct nodes.
    let unique: std::collections::BTreeSet<_> = roots.iter().collect();
    assert_eq!(unique.len(), 4, "four occurrences must be four nodes");

    // But the geometry underneath is shared: exactly two extrusions total,
    // not eight, because the map body was lowered once.
    let extrusions = count_kind(&lowered.graph, |node| {
        matches!(
            node,
            GeometryNode::SolidOperation(SolidOperation::Extrusion { .. })
        )
    });
    assert_eq!(
        extrusions, 2,
        "the shared map body must be lowered once, not once per occurrence"
    );
}

/// A mapped item is an `Instance`, never a flattened copy.
#[test]
fn a_mapped_item_lowers_to_an_instance_node() {
    let model = model("mapped_instances_multi_item.ifc");
    let scale = units::resolve(&model);
    let mut session = LoweringSession::new(&model, &scale, Tolerance::building_scale());

    let root = lower_mapped_item_node(&mut session, EntityId(34), Transform::identity())
        .expect("the occurrence lowers");
    let lowered = session.finish(root).expect("session finishes");

    assert!(
        matches!(
            lowered.graph.get(lowered.root).expect("root resolves"),
            GeometryNode::Instance(_)
        ),
        "a mapped item must preserve instancing"
    );
}

/// The instance transform is `frame * target * origin`, in that order.
///
/// Ground truth from `mapped_instances_multi_item.ifc`:
///
/// ```text
/// #34= IFCMAPPEDITEM(#18,#27)
/// #27= IFCCARTESIANTRANSFORMATIONOPERATOR3D($,$,#26,$,$)
/// #26= IFCCARTESIANPOINT((5.,0.,0.))      <- MappingTarget
/// #18= IFCREPRESENTATIONMAP(#17,#16)
/// #17= IFCAXIS2PLACEMENT3D(#1,$,$)        <- MappingOrigin, at the origin
/// ```
///
/// The file declares METRE, so numbers pass through unscaled. With an outer
/// frame translating +2 in Y the composed origin must be (5,2,0): getting the
/// order backwards yields a different point, which is the whole reason this
/// asserts on numbers rather than on structure.
#[test]
fn the_instance_transform_composes_frame_then_target_then_origin() {
    let model = model("mapped_instances_multi_item.ifc");
    let scale = units::resolve(&model);
    let mut session = LoweringSession::new(&model, &scale, Tolerance::building_scale());

    let frame = Transform::translation([0.0, 2.0, 0.0]);
    let root = lower_mapped_item_node(&mut session, EntityId(34), frame).expect("lowers");
    let lowered = session.finish(root).expect("session finishes");

    let GeometryNode::Instance(instance) = lowered.graph.get(lowered.root).expect("root") else {
        panic!("expected an instance");
    };
    let origin = instance.transform.translation.to_array();
    assert!(
        (origin[0] - 5.0).abs() < 1e-9 && (origin[1] - 2.0).abs() < 1e-9 && origin[2].abs() < 1e-9,
        "expected target (5,0,0) under frame +2Y to land at (5,2,0), got {origin:?}"
    );
}

/// Nested maps compose transforms multiplicatively through both levels.
///
/// Ground truth from `nested_mapped_item.ifc` (declares MILLI metre, so the
/// unit scale converts mm to m):
///
/// ```text
/// #31= IFCMAPPEDITEM(#21,#30)      outer occurrence
/// #30= IFCCARTESIANTRANSFORMATIONOPERATOR3D($,$,#29,1.,$)
/// #29= IFCCARTESIANPOINT((0.,5000.,0.))    outer target: +5 m in Y
/// #21= IFCREPRESENTATIONMAP(#2,#20)        origin at (0,0,0)
/// #20= IFCSHAPEREPRESENTATION(...,(#16,#19))
/// #19= IFCMAPPEDITEM(#14,#18)      nested occurrence
/// #18= IFCCARTESIANTRANSFORMATIONOPERATOR3D($,$,#17,2.,$)
/// #17= IFCCARTESIANPOINT((10000.,0.,0.))   inner target: +10 m in X, scale 2
/// ```
///
/// The inner cube must therefore land at (10, 5, 0) metres with a scale of 2:
/// the outer +5Y and the inner +10X both apply. A single-level implementation
/// that stops at the outer map puts it at (0,5,0) and fails here.
#[test]
fn nested_maps_compose_through_every_level() {
    let model = model("nested_mapped_item.ifc");
    let scale = units::resolve(&model);
    let mut session = LoweringSession::new(&model, &scale, Tolerance::building_scale());

    let root = lower_mapped_item_node(&mut session, EntityId(31), Transform::identity())
        .expect("the nested occurrence lowers");
    let lowered = session.finish(root).expect("session finishes");

    // The outer instance wraps the outer map body, which is a collection of
    // the outer cube plus the nested instance.
    let GeometryNode::Instance(outer) = lowered.graph.get(lowered.root).expect("root") else {
        panic!("expected an outer instance");
    };
    let outer_origin = outer.transform.translation.to_array();
    assert!(
        (outer_origin[1] - 5.0).abs() < 1e-9,
        "outer target is +5000 mm in Y = 5 m, got {outer_origin:?}"
    );

    // Find the nested instance: an Instance whose own source is reached from
    // the outer map's collection.
    let GeometryNode::Collection(members) = resolve(&lowered.graph, outer.source) else {
        panic!("outer map body must be a collection of its two items");
    };
    // Both members are Instances: the outer cube's own placement and the
    // nested mapped item. They are told apart by what they instance -- a
    // mapped item always wraps a representation Collection, whereas a swept
    // solid's placement wraps the SolidOperation itself.
    let nested = members
        .iter()
        .find_map(|member| match lowered.graph.get(*member).expect("member") {
            GeometryNode::Instance(inner)
                if matches!(
                    lowered.graph.get(inner.source).expect("instance source"),
                    GeometryNode::Collection(_)
                ) =>
            {
                Some(inner)
            }
            _ => None,
        })
        .expect("the outer map contains a nested mapped item");

    let inner_origin = nested.transform.translation.to_array();
    assert!(
        (inner_origin[0] - 10.0).abs() < 1e-9,
        "inner target is +10000 mm in X = 10 m, got {inner_origin:?}"
    );

    // Scale 2 on the inner operator must survive as a scaled basis.
    let x_axis = nested.transform.matrix3.x_axis.to_array();
    let magnitude = (x_axis[0] * x_axis[0] + x_axis[1] * x_axis[1] + x_axis[2] * x_axis[2]).sqrt();
    assert!(
        (magnitude - 2.0).abs() < 1e-9,
        "inner operator declares Scale 2., got basis magnitude {magnitude}"
    );
}

/// A cyclic mapping graph is reported, not followed until the stack dies.
///
/// Ground truth from `nested_mapped_item_cycle.ifc`:
///
/// ```text
/// #18= IFCMAPPEDITEM(#17,#14)
/// #17= IFCREPRESENTATIONMAP(#2,#16)
/// #16= IFCSHAPEREPRESENTATION(...,(#12,#15))
/// #15= IFCMAPPEDITEM(#20,#14)
/// #20= IFCREPRESENTATIONMAP(#2,#19)
/// #19= IFCSHAPEREPRESENTATION(...,(#18))   <- back to #18
/// ```
///
/// The IFC spec pushes cycle prevention to the application layer, so real
/// exporters do emit these. Detecting beats overflowing.
#[test]
fn a_cyclic_mapping_graph_is_reported() {
    let model = model("nested_mapped_item_cycle.ifc");
    let scale = units::resolve(&model);
    let mut session = LoweringSession::new(&model, &scale, Tolerance::building_scale());

    let error = lower_mapped_item_node(&mut session, EntityId(18), Transform::identity())
        .expect_err("a cyclic mapping graph must not lower");

    assert!(
        error.to_string().contains("cyclic"),
        "expected a cyclic-chain report, got: {error}"
    );
    assert!(
        error.entity().is_some(),
        "the report must name the entity it gave up on"
    );
}

/// The depth budget bounds a deep-but-acyclic mapping chain.
#[test]
fn a_mapping_chain_stops_at_the_depth_limit() {
    let model = model("nested_mapped_item.ifc");
    let scale = units::resolve(&model);
    let mut session = LoweringSession::with_limits(
        &model,
        &scale,
        Tolerance::building_scale(),
        SessionLimits { max_depth: 1 },
    );

    let error = lower_mapped_item_node(&mut session, EntityId(31), Transform::identity())
        .expect_err("depth 1 cannot reach the nested map");
    assert!(
        error.to_string().contains("depth") || error.to_string().contains("cyclic"),
        "expected a bounded-walk report, got: {error}"
    );
}

/// The dispatcher routes IFCMAPPEDITEM instead of reporting it unsupported.
#[test]
fn the_dispatcher_routes_mapped_items() {
    let model = model("mapped_instances_multi_item.ifc");
    let scale = units::resolve(&model);
    let mut session = LoweringSession::new(&model, &scale, Tolerance::building_scale());

    let node = lower_representation_item(&mut session, EntityId(34), Transform::identity())
        .expect("the dispatcher must lower a mapped item");
    let lowered = session.finish(node).expect("session finishes");
    assert!(matches!(
        lowered.graph.get(lowered.root).expect("root"),
        GeometryNode::Instance(_)
    ));
}

/// A representation with no items still lowers, as an empty collection.
///
/// Ground truth: `#40= IFCSHAPEREPRESENTATION(#5,'Body','SweptSolid',())` in
/// `nested_mapped_item.ifc` exists precisely to exercise this path. An empty
/// body is valid IFC and must not be an error.
#[test]
fn an_empty_representation_lowers_to_an_empty_collection() {
    let model = model("nested_mapped_item.ifc");
    let scale = units::resolve(&model);
    let mut session = LoweringSession::new(&model, &scale, Tolerance::building_scale());

    let node = ifc_geometry::lower::lower_representation(&mut session, EntityId(40))
        .expect("an empty representation is valid");
    let lowered = session.finish(node).expect("session finishes");
    assert!(matches!(
        lowered.graph.get(lowered.root).expect("root"),
        GeometryNode::Collection(members) if members.is_empty()
    ));
}

/// Composition order is target-then-origin, proven with both non-identity.
///
/// Every corpus fixture happens to use an identity MappingOrigin, which makes
/// `target o origin` and `origin o target` numerically identical and hides an
/// inverted composition. This model gives the two frames DIFFERENT
/// translations and a scaling target, so the orders disagree:
///
/// ```text
///   MappingOrigin  = translate (1, 0, 0)
///   MappingTarget  = translate (0, 10, 0), Scale 3
///
///   target o origin = (0,10,0) + 3*(1,0,0) = (3, 10, 0)   <- correct
///   origin o target = (1, 0,0) + 1*(0,10,0) = (1, 10, 0)  <- inverted
/// ```
///
/// The scale is what separates them: the outer frame must scale the inner
/// translation, which only happens when the target is applied outermost.
#[test]
fn composition_applies_the_target_outside_the_origin() {
    let mut model = Model::new();

    // Geometry: a 1x1 rectangle extruded 1 unit, at the map's own origin.
    model.insert(
        EntityId(1),
        Entity::new("IFCCARTESIANPOINT", vec![reals(&[0.0, 0.0])]),
    );
    model.insert(
        EntityId(2),
        Entity::new(
            "IFCAXIS2PLACEMENT2D",
            vec![Value::Ref(EntityId(1)), Value::Null],
        ),
    );
    model.insert(
        EntityId(3),
        Entity::new(
            "IFCRECTANGLEPROFILEDEF",
            vec![
                Value::Enum("AREA".into()),
                Value::Null,
                Value::Ref(EntityId(2)),
                Value::Real(1.0),
                Value::Real(1.0),
            ],
        ),
    );
    model.insert(
        EntityId(4),
        Entity::new("IFCCARTESIANPOINT", vec![reals(&[0.0, 0.0, 0.0])]),
    );
    model.insert(
        EntityId(5),
        Entity::new(
            "IFCAXIS2PLACEMENT3D",
            vec![Value::Ref(EntityId(4)), Value::Null, Value::Null],
        ),
    );
    model.insert(
        EntityId(6),
        Entity::new("IFCDIRECTION", vec![reals(&[0.0, 0.0, 1.0])]),
    );
    model.insert(
        EntityId(7),
        Entity::new(
            "IFCEXTRUDEDAREASOLID",
            vec![
                Value::Ref(EntityId(3)),
                Value::Ref(EntityId(5)),
                Value::Ref(EntityId(6)),
                Value::Real(1.0),
            ],
        ),
    );
    model.insert(
        EntityId(8),
        Entity::new(
            "IFCSHAPEREPRESENTATION",
            vec![
                Value::Null,
                Value::Text("Body".into()),
                Value::Text("SweptSolid".into()),
                Value::List(vec![Value::Ref(EntityId(7))]),
            ],
        ),
    );

    // MappingOrigin at (1, 0, 0) -- deliberately NOT the identity.
    model.insert(
        EntityId(9),
        Entity::new("IFCCARTESIANPOINT", vec![reals(&[1.0, 0.0, 0.0])]),
    );
    model.insert(
        EntityId(10),
        Entity::new(
            "IFCAXIS2PLACEMENT3D",
            vec![Value::Ref(EntityId(9)), Value::Null, Value::Null],
        ),
    );
    model.insert(
        EntityId(11),
        Entity::new(
            "IFCREPRESENTATIONMAP",
            vec![Value::Ref(EntityId(10)), Value::Ref(EntityId(8))],
        ),
    );

    // MappingTarget at (0, 10, 0) with Scale 3.
    model.insert(
        EntityId(12),
        Entity::new("IFCCARTESIANPOINT", vec![reals(&[0.0, 10.0, 0.0])]),
    );
    model.insert(
        EntityId(13),
        Entity::new(
            "IFCCARTESIANTRANSFORMATIONOPERATOR3D",
            vec![
                Value::Null,
                Value::Null,
                Value::Ref(EntityId(12)),
                Value::Real(3.0),
                Value::Null,
            ],
        ),
    );
    model.insert(
        EntityId(14),
        Entity::new(
            "IFCMAPPEDITEM",
            vec![Value::Ref(EntityId(11)), Value::Ref(EntityId(13))],
        ),
    );

    let scale = units::resolve(&model);
    let mut session = LoweringSession::new(&model, &scale, Tolerance::building_scale());
    let root = lower_mapped_item_node(&mut session, EntityId(14), Transform::identity())
        .expect("the mapped item lowers");
    let lowered = session.finish(root).expect("session finishes");

    let GeometryNode::Instance(instance) = lowered.graph.get(lowered.root).expect("root") else {
        panic!("expected an instance root");
    };
    let origin = instance.transform.translation.to_array();
    assert!(
        (origin[0] - 3.0).abs() < 1e-9 && (origin[1] - 10.0).abs() < 1e-9,
        "target must apply outside the origin: expected (3, 10, 0), got {origin:?}"
    );
}

/// One representation reached by two DIFFERENT mapped items is lowered once.
///
/// The multi-occurrence fixture memoizes at the mapped-item level, which hides
/// whether the representation itself is shared. Here two distinct items point
/// at the same map with different targets, so the item memo cannot fire and
/// only the representation memo can prevent a duplicate subtree.
#[test]
fn one_representation_reached_twice_is_lowered_once() {
    let model = model("mapped_instances_multi_item.ifc");
    let scale = units::resolve(&model);
    let mut session = LoweringSession::new(&model, &scale, Tolerance::building_scale());

    // #34 and #41 are different IfcMappedItems over the same map #18.
    let first = lower_mapped_item_node(&mut session, EntityId(34), Transform::identity())
        .expect("first occurrence lowers");
    let after_first = session.node_count();
    let second = lower_mapped_item_node(&mut session, EntityId(41), Transform::identity())
        .expect("second occurrence lowers");

    assert_ne!(
        first, second,
        "different targets must yield different instances"
    );
    assert_eq!(
        session.node_count(),
        after_first + 1,
        "the second occurrence must add ONLY its Instance node, reusing the \
         shared representation subtree"
    );

    let lowered = session.finish(second).expect("session finishes");
    let (GeometryNode::Instance(a), GeometryNode::Instance(b)) = (
        lowered.graph.get(first).expect("first"),
        lowered.graph.get(second).expect("second"),
    ) else {
        panic!("both occurrences must be instances");
    };
    assert_eq!(
        a.source, b.source,
        "both occurrences must instance the SAME representation node"
    );
}

/// A mapped item that reaches itself is caught by the item-level guard.
///
/// `lower_representation` also maintains the active chain, so a cycle running
/// through a representation is caught even without the item guard. This model
/// closes the loop at the ITEM: the map's representation lists the very item
/// being lowered, so only the guard inside `lower_mapped_item_node` can stop
/// it before the graph builder recurses forever.
#[test]
fn a_self_referencing_mapped_item_is_caught_at_the_item_level() {
    let mut model = Model::new();
    model.insert(
        EntityId(1),
        Entity::new("IFCCARTESIANPOINT", vec![reals(&[0.0, 0.0, 0.0])]),
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
            "IFCCARTESIANTRANSFORMATIONOPERATOR3D",
            vec![
                Value::Null,
                Value::Null,
                Value::Ref(EntityId(1)),
                Value::Null,
                Value::Null,
            ],
        ),
    );
    // #5's representation contains #6, and #6 maps back through #5.
    model.insert(
        EntityId(4),
        Entity::new(
            "IFCSHAPEREPRESENTATION",
            vec![
                Value::Null,
                Value::Text("Body".into()),
                Value::Text("MappedRepresentation".into()),
                Value::List(vec![Value::Ref(EntityId(6))]),
            ],
        ),
    );
    model.insert(
        EntityId(5),
        Entity::new(
            "IFCREPRESENTATIONMAP",
            vec![Value::Ref(EntityId(2)), Value::Ref(EntityId(4))],
        ),
    );
    model.insert(
        EntityId(6),
        Entity::new(
            "IFCMAPPEDITEM",
            vec![Value::Ref(EntityId(5)), Value::Ref(EntityId(3))],
        ),
    );

    let scale = units::resolve(&model);
    let mut session = LoweringSession::new(&model, &scale, Tolerance::building_scale());
    let error = lower_mapped_item_node(&mut session, EntityId(6), Transform::identity())
        .expect_err("a self-referencing mapped item must not lower");
    assert!(
        error.to_string().contains("cyclic"),
        "expected the cycle guard, got: {error}"
    );
    assert_eq!(
        error.entity(),
        Some(EntityId(6)),
        "the report must name the item that closed the loop"
    );
}
