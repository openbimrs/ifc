#![cfg(feature = "lowering")]
//! `IfcSubedge` lowering: an edge carved from a parent edge.
//!
//! # Why an in-memory fixture
//!
//! No surveyed corpus carries an `IfcSubedge`. These models are the smallest
//! shapes that force the decisions the traversal has to make: inheriting the
//! parent's carrier curve, keeping the subedge's own narrower extent, and
//! walking a parent chain more than one hop deep.

use axiolid_model::GeometryNode;
use ifc_geometry::lower::{lower_representation_item, LoweringSession};
use ifc_geometry::transform::Transform;
use ifc_geometry::units;
use ifc_model::{Entity, EntityId, Model, Value};

fn r(id: u64) -> Value {
    Value::Ref(EntityId(id))
}

fn n(v: f64) -> Value {
    Value::Real(v)
}

fn ent(type_name: &str, attributes: Vec<Value>) -> Entity {
    Entity::new(type_name, attributes)
}

/// A one-face brep whose first oriented edge names a subedge.
///
/// `parent_chain` inserts an intermediate subedge, so the carrier curve is
/// two hops away rather than one.
fn model(parent_chain: bool) -> Model {
    let mut m = Model::new();
    // Points, then the vertices that carry them.
    for (id, x, y) in [
        (1u64, 0.0, 0.0),
        (2, 10.0, 0.0),
        (3, 0.0, 4.0),
        (4, 3.0, 0.0),
    ] {
        m.insert(
            EntityId(id),
            ent(
                "IFCCARTESIANPOINT",
                vec![Value::List(vec![n(x), n(y), n(0.0)])],
            ),
        );
        m.insert(EntityId(id + 10), ent("IFCVERTEXPOINT", vec![r(id)]));
    }

    // The parent edge spans the full width; its curve is the carrier.
    m.insert(
        EntityId(30),
        ent(
            "IFCCARTESIANPOINT",
            vec![Value::List(vec![n(0.0), n(0.0), n(0.0)])],
        ),
    );
    m.insert(
        EntityId(31),
        ent(
            "IFCDIRECTION",
            vec![Value::List(vec![n(1.0), n(0.0), n(0.0)])],
        ),
    );
    m.insert(EntityId(32), ent("IFCVECTOR", vec![r(31), n(1.0)]));
    m.insert(EntityId(33), ent("IFCLINE", vec![r(30), r(32)]));
    m.insert(
        EntityId(34),
        ent("IFCEDGECURVE", vec![r(11), r(12), r(33), Value::Bool(true)]),
    );

    // The subedge carves 0..3 out of the parent's 0..10 span.
    let subedge_parent = if parent_chain {
        // An intermediate subedge, so the carrier is two hops up.
        m.insert(EntityId(35), ent("IFCSUBEDGE", vec![r(11), r(12), r(34)]));
        35
    } else {
        34
    };
    m.insert(
        EntityId(36),
        ent("IFCSUBEDGE", vec![r(11), r(14), r(subedge_parent)]),
    );

    // Two ordinary edges close the triangle.
    m.insert(
        EntityId(37),
        ent(
            "IFCEDGECURVE",
            vec![r(14), r(13), Value::Null, Value::Bool(true)],
        ),
    );
    m.insert(
        EntityId(38),
        ent(
            "IFCEDGECURVE",
            vec![r(13), r(11), Value::Null, Value::Bool(true)],
        ),
    );

    for (id, edge) in [(40u64, 36u64), (41, 37), (42, 38)] {
        m.insert(
            EntityId(id),
            ent(
                "IFCORIENTEDEDGE",
                vec![Value::Null, Value::Null, r(edge), Value::Bool(true)],
            ),
        );
    }
    m.insert(
        EntityId(50),
        ent("IFCEDGELOOP", vec![Value::List(vec![r(40), r(41), r(42)])]),
    );
    m.insert(
        EntityId(51),
        ent("IFCFACEOUTERBOUND", vec![r(50), Value::Bool(true)]),
    );
    m.insert(EntityId(52), ent("IFCFACE", vec![Value::List(vec![r(51)])]));
    m.insert(
        EntityId(53),
        ent("IFCCLOSEDSHELL", vec![Value::List(vec![r(52)])]),
    );
    m.insert(EntityId(54), ent("IFCFACETEDBREP", vec![r(53)]));
    m
}

fn brep(model: &Model) -> axiolid_topology::BRep<axiolid_model::NodeId> {
    let scale = units::resolve(model);
    let mut session = LoweringSession::new(model, &scale);
    let node = lower_representation_item(&mut session, EntityId(54), Transform::identity())
        .unwrap_or_else(|e| panic!("the brep must lower: {e}"));
    let lowered = session.finish(node).expect("session finishes");
    match lowered.graph.get(lowered.root).expect("root node") {
        GeometryNode::BRep(b) => b.clone(),
        other => panic!("expected a BRep, got {other:?}"),
    }
}

/// The subedge carries the parent's curve, not a fresh or absent one.
///
/// This is the whole point of the traversal: geometry lives on the parent,
/// extent lives on the subedge. Reading the curve off the subedge's own
/// slots would find nothing, and the carved edge would lose its carrier.
#[test]
fn a_subedge_inherits_the_parent_edge_curve() {
    let m = model(false);
    let brep = brep(&m);
    // Only the parent edge was given a curve; the two closing edges have none.
    // The subedge is the edge spanning vertices (0,0,0)-(3,0,0).
    let carved = brep
        .edges()
        .iter()
        .find(|e| {
            let s = brep.vertices()[e.start.index()].position.to_array();
            let t = brep.vertices()[e.end.index()].position.to_array();
            s == [0.0, 0.0, 0.0] && t == [3.0, 0.0, 0.0]
        })
        .expect("the subedge keeps its own narrower extent");
    assert!(
        carved.curve.is_some(),
        "the subedge must carry the parent's curve, not an absent one"
    );
}

/// The carved extent is the subedge's, never the parent's full span.
///
/// Taking the parent's vertices would silently widen the edge from 0..3 to
/// 0..10. The parent is interned in its own right (it is a real edge the
/// subedge refers to), so the check is that the *loop's* edge is the narrow
/// one -- not that 10.0 is absent from the brep entirely.
#[test]
fn a_subedge_keeps_its_own_extent_not_the_parents() {
    let m = model(false);
    let brep = brep(&m);
    let loop_ = &brep.loops()[0];
    let first = &loop_.edges[0];
    let edge = &brep.edges()[first.edge.index()];
    assert_eq!(
        brep.vertices()[edge.end.index()].position.to_array(),
        [3.0, 0.0, 0.0],
        "the loop must walk the carved subedge, not the parent's full span"
    );
}
/// A subedge of a subedge still finds the carrier curve.
///
/// IFC types `ParentEdge` as `IfcEdge`, so a chain is legal. Stopping at the
/// first hop would leave the deeper subedge with no geometry.
#[test]
fn a_subedge_chain_walks_to_the_carrier_curve() {
    let m = model(true);
    let brep = brep(&m);
    let carved = brep
        .edges()
        .iter()
        .find(|e| {
            let s = brep.vertices()[e.start.index()].position.to_array();
            let t = brep.vertices()[e.end.index()].position.to_array();
            s == [0.0, 0.0, 0.0] && t == [3.0, 0.0, 0.0]
        })
        .expect("the deeper subedge still carves 0..3");
    assert!(
        carved.curve.is_some(),
        "a two-hop parent chain must still reach the carrier curve"
    );
}
