//! Product placement: geometry must land where the file puts it.
//!
//! An IfcProduct carries an ObjectPlacement chain.
//! Lowering its items without that chain stacks every product at
//! the origin: the model renders, but it is a heap, not a building.

use ifc_geometry::lower::{
    lower_product_items, product_world_transform, select_shape_representation, LoweringSession,
    Tolerance,
};
use ifc_geometry::units;
use ifc_model::{Codec, Entity, EntityId, Model, Value};
use std::path::PathBuf;

fn reals(values: &[f64]) -> Value {
    Value::List(values.iter().copied().map(Value::Real).collect())
}

fn fixture(rel: &str) -> PathBuf {
    PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../test/fixtures/"
    ))
    .join(rel)
}

fn load(rel: &str) -> Model {
    ifc_step::StepCodec
        .read_path(&fixture(rel))
        .expect("fixture parses")
}

/// Centroid of a lowered product, in metres.
fn centroid(model: &Model, product: EntityId) -> [f64; 3] {
    let scale = units::resolve(model);
    let mut session = LoweringSession::new(model, &scale, Tolerance::building_scale());
    let root = lower_product_items(&mut session, product)
        .expect("product lowers")
        .expect("product has geometry");
    let lowered = session.finish(root).expect("graph closes");
    graph_centroid(&lowered.graph, lowered.root)
}

/// Average of every explicit position in the graph, with Instance frames applied.
///
/// A product is placed by transforms that sit ABOVE its geometry, so comparing
/// raw leaf points would miss the placement entirely. Walking from the roots
/// down and accumulating transforms is what makes the assertion meaningful.
fn graph_centroid(graph: &axiolid_model::GeometryGraph, root: axiolid_model::NodeId) -> [f64; 3] {
    let mut sum = [0.0f64; 3];
    let mut count = 0usize;
    let mut stack: Vec<(axiolid_model::NodeId, axiolid_core::Transform3)> =
        vec![(root, axiolid_core::Transform3::IDENTITY)];
    while let Some((id, frame)) = stack.pop() {
        let Some(node) = graph.get(id) else { continue };
        match node {
            axiolid_model::GeometryNode::Instance(instance) => {
                // An instance frame IS a position signal: product placement
                // lands here, above the shared geometry.
                let composed = frame * instance.transform;
                let t = composed.translation;
                sum[0] += t.x;
                sum[1] += t.y;
                sum[2] += t.z;
                count += 1;
                stack.push((instance.source, composed));
            }
            axiolid_model::GeometryNode::Collection(members) => {
                for m in members {
                    stack.push((*m, frame));
                }
            }
            axiolid_model::GeometryNode::BRep(brep) => {
                for v in brep.vertices() {
                    let p = frame.transform_point3(v.position);
                    sum[0] += p.x;
                    sum[1] += p.y;
                    sum[2] += p.z;
                    count += 1;
                }
            }
            axiolid_model::GeometryNode::SolidOperation(_) => {
                for r in node.references() {
                    stack.push((r, frame));
                }
            }
            axiolid_model::GeometryNode::Point3(p) => {
                let q = frame.transform_point3(*p);
                sum[0] += q.x;
                sum[1] += q.y;
                sum[2] += q.z;
                count += 1;
            }
            _ => {}
        }
    }
    assert!(count > 0, "graph carried no positions");
    [
        sum[0] / count as f64,
        sum[1] / count as f64,
        sum[2] / count as f64,
    ]
}

/// Four identical proxies at distinct placements land 10 m apart.
///
/// Ground truth from mapped_instances_multi_item.ifc: proxies #40/#47/#54/#61
/// hang off IfcLocalPlacements at (0,0,0), (10,0,0), (20,0,0), (30,0,0), and
/// the file is in metres. All four share ONE RepresentationMap, so before
/// placement they are geometrically identical and would coincide exactly.
#[test]
fn products_are_separated_by_their_placement_chains() {
    let model = load("ifclite-geometry/mapped_instances_multi_item.ifc");
    let scale = units::resolve(&model);
    // Compare the PRODUCT frame directly. A blended centroid also averages
    // the per-proxy mapped-item targets, which differ, masking the placement.
    let xs: Vec<f64> = [40u64, 47, 54, 61]
        .iter()
        .map(|id| {
            product_world_transform(&model, &scale, EntityId(*id))
                .expect("placement resolves")
                .origin[0]
        })
        .collect();

    assert_eq!(
        xs,
        vec![0.0, 10.0, 20.0, 30.0],
        "proxies sit 10 m apart in X"
    );

    // And the lowered geometry must actually move with it, not just the frame.
    let c0 = centroid(&model, EntityId(40));
    let c3 = centroid(&model, EntityId(61));
    assert!(
        c0 != c3,
        "identical source geometry must not coincide after placement"
    );
}

/// A millimetre file scales the placement chain exactly once.
///
/// Local placements carry raw file coordinates, so a 3000 mm storey offset is
/// the literal 3000. The resolver composes the chain in file units and the
/// caller converts once; converting per link would cube the scale on a
/// three-deep chain and converting never leaves the model 1000x too big.
#[test]
fn a_millimetre_placement_chain_converts_exactly_once() {
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
            "IFCSIUNIT",
            vec![
                Value::Derived,
                Value::Enum("LENGTHUNIT".into()),
                Value::Enum("MILLI".into()),
                Value::Enum("METRE".into()),
            ],
        ),
    );
    model.insert(
        EntityId(4),
        Entity::new(
            "IFCUNITASSIGNMENT",
            vec![Value::List(vec![Value::Ref(EntityId(3))])],
        ),
    );
    // Site at origin, storey +3000 mm in Z, product +2000 mm in X under it.
    model.insert(
        EntityId(10),
        Entity::new(
            "IFCLOCALPLACEMENT",
            vec![Value::Null, Value::Ref(EntityId(2))],
        ),
    );
    model.insert(
        EntityId(11),
        Entity::new("IFCCARTESIANPOINT", vec![reals(&[0.0, 0.0, 3000.0])]),
    );
    model.insert(
        EntityId(12),
        Entity::new(
            "IFCAXIS2PLACEMENT3D",
            vec![Value::Ref(EntityId(11)), Value::Null, Value::Null],
        ),
    );
    model.insert(
        EntityId(13),
        Entity::new(
            "IFCLOCALPLACEMENT",
            vec![Value::Ref(EntityId(10)), Value::Ref(EntityId(12))],
        ),
    );

    model.insert(
        EntityId(14),
        Entity::new("IFCCARTESIANPOINT", vec![reals(&[2000.0, 0.0, 0.0])]),
    );
    model.insert(
        EntityId(15),
        Entity::new(
            "IFCAXIS2PLACEMENT3D",
            vec![Value::Ref(EntityId(14)), Value::Null, Value::Null],
        ),
    );
    model.insert(
        EntityId(16),
        Entity::new(
            "IFCLOCALPLACEMENT",
            vec![Value::Ref(EntityId(13)), Value::Ref(EntityId(15))],
        ),
    );

    model.insert(
        EntityId(18),
        Entity::new(
            "IFCUNITASSIGNMENT",
            vec![Value::List(vec![Value::Ref(EntityId(3))])],
        ),
    );
    model.insert(
        EntityId(19),
        Entity::new(
            "IFCPROJECT",
            vec![
                Value::Null,
                Value::Null,
                Value::Null,
                Value::Null,
                Value::Null,
                Value::Null,
                Value::Null,
                Value::Null,
                Value::Ref(EntityId(18)),
            ],
        ),
    );
    let scale = units::resolve(&model);
    model.insert(
        EntityId(17),
        Entity::new(
            "IFCWALL",
            vec![
                Value::Null,
                Value::Null,
                Value::Null,
                Value::Null,
                Value::Null,
                Value::Ref(EntityId(16)),
                Value::Null,
            ],
        ),
    );
    let world = product_world_transform(&model, &scale, EntityId(17)).expect("chain resolves");
    // 2000 mm in X under a 3000 mm Z storey: metres, converted once.
    assert!(
        (world.origin[0] - 2.0).abs() < 1e-12,
        "X: {:?}",
        world.origin
    );
    assert!(
        (world.origin[2] - 3.0).abs() < 1e-12,
        "Z: {:?}",
        world.origin
    );
}

/// Body is selected over Axis even when Axis is listed first.
///
/// Ground truth from issue_098_wall_W.ifc: wall #928204 points at shape
/// #928202, whose Representations list is (#928189 Axis/Curve2D, #928200
/// Body/SweptSolid) -- in that order. Taking the first entry yields a 2D
/// centreline instead of the wall solid, which renders as nothing.
#[test]
fn body_wins_over_an_earlier_axis_representation() {
    let model = load("ifclite-geometry/issue_098_wall_W.ifc");
    let chosen = select_shape_representation(&model, EntityId(928204))
        .expect("lookup succeeds")
        .expect("wall has a body");
    assert_eq!(
        chosen,
        EntityId(928200),
        "must pick Body, not the leading Axis"
    );
}

/// A product with no ObjectPlacement lowers at the identity, not an error.
///
/// ObjectPlacement is OPTIONAL in the schema. An unplaced product is legal and
/// means model-space coordinates, so refusing it would drop valid geometry.
#[test]
fn an_unplaced_product_resolves_to_the_identity() {
    let mut model = Model::new();
    model.insert(
        EntityId(1),
        Entity::new(
            "IFCSIUNIT",
            vec![
                Value::Derived,
                Value::Enum("LENGTHUNIT".into()),
                Value::Null,
                Value::Enum("METRE".into()),
            ],
        ),
    );
    let scale = units::resolve(&model);
    // A product record that exists but omits ObjectPlacement.
    model.insert(
        EntityId(50),
        Entity::new(
            "IFCBUILDINGELEMENTPROXY",
            vec![
                Value::Text("guid".into()),
                Value::Null,
                Value::Null,
                Value::Null,
                Value::Null,
                Value::Null,
                Value::Null,
                Value::Null,
                Value::Null,
            ],
        ),
    );
    let world = product_world_transform(&model, &scale, EntityId(50)).expect("legal");
    assert!(world.is_identity(1e-12), "unplaced product is model-space");
}
