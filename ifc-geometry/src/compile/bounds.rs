//! World-space bounds read straight off a lowered graph, without tessellating.
//!
//! Only leaves whose extent is exact as written are bounded here: triangle
//! and polygon meshes (their vertices are the shape) and authored bounding
//! boxes. Their positions are already in world coordinates, because lowering
//! bakes placement into them. An `Instance` above a leaf applies its
//! transform to the leaf's points, never to an intermediate box, so the
//! result stays tight.
//!
//! Any other leaf — an extrusion, a curved surface, a boolean — returns
//! `None`, and the caller falls back to the compiled mesh. Refusing is
//! cheaper than guessing: an extrusion's profile bound is exact, but a
//! boolean difference can only shrink its operand, so taking the operand's
//! box would over-state the result.

use axiolid_core::{Aabb, Point3, Transform3};
use axiolid_model::{GeometryGraph, GeometryNode, NodeId};

/// Nodes visited before giving up. Lowering never builds a cycle, but a
/// mapped item repeated many times can multiply work; falling back to the
/// mesh path is always available.
const NODE_BUDGET: usize = 1_000_000;

/// The exact world bounds under `root`, or `None` when some reachable leaf
/// is not exactly boundable without tessellation.
pub(super) fn exact(graph: &GeometryGraph, root: NodeId) -> Option<Aabb> {
    let mut aabb = Aabb::empty();
    let mut stack = vec![(root, Transform3::IDENTITY)];
    let mut visited = 0usize;
    while let Some((id, transform)) = stack.pop() {
        visited += 1;
        if visited > NODE_BUDGET {
            return None;
        }
        match graph.get(id)? {
            GeometryNode::TriMesh(mesh) => extend(&mut aabb, &mesh.positions, transform),
            GeometryNode::PolygonMesh(mesh) => extend(&mut aabb, &mesh.positions, transform),
            GeometryNode::BoundingBox(authored) => {
                extend(&mut aabb, &corners(authored), transform);
            }
            GeometryNode::Instance(instance) => {
                stack.push((instance.source, transform * instance.transform));
            }
            GeometryNode::Collection(children) => {
                stack.extend(children.iter().map(|child| (*child, transform)));
            }
            _ => return None,
        }
    }
    Some(aabb)
}

fn extend(aabb: &mut Aabb, points: &[Point3], transform: Transform3) {
    for point in points {
        aabb.extend(transform.transform_point3(*point));
    }
}

/// The eight corners, so a rotated box is bounded by where its corners land.
fn corners(aabb: &Aabb) -> [Point3; 8] {
    let (a, b) = (aabb.min, aabb.max);
    [
        Point3::new(a.x, a.y, a.z),
        Point3::new(b.x, a.y, a.z),
        Point3::new(a.x, b.y, a.z),
        Point3::new(b.x, b.y, a.z),
        Point3::new(a.x, a.y, b.z),
        Point3::new(b.x, a.y, b.z),
        Point3::new(a.x, b.y, b.z),
        Point3::new(b.x, b.y, b.z),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use axiolid_mesh::TriMesh;
    use axiolid_model::{GeometryGraphBuilder, Instance};

    fn mesh(points: &[[f64; 3]]) -> GeometryNode {
        let positions = points.iter().map(|p| Point3::from_array(*p)).collect();
        GeometryNode::TriMesh(TriMesh::new(positions, vec![0, 1, 2]))
    }

    #[test]
    fn an_instance_transforms_points_not_a_box() {
        // A thin diagonal triangle rotated 45°: bounding its points after
        // rotation is tighter than rotating its local box.
        let mut builder = GeometryGraphBuilder::default();
        let leaf = builder
            .push(mesh(&[[0.0, 0.0, 0.0], [1.0, 1.0, 0.0], [1.0, 1.0, 0.0]]))
            .unwrap();
        let rotate = Transform3::from_rotation_z(-std::f64::consts::FRAC_PI_4);
        let root = builder
            .push(GeometryNode::Instance(Instance {
                source: leaf,
                transform: rotate,
            }))
            .unwrap();
        let graph = builder.finish(vec![root]).unwrap();
        let aabb = exact(&graph, root).expect("meshes are exact");
        assert!((aabb.max.x - 2f64.sqrt()).abs() < 1e-12);
        assert!(aabb.max.y.abs() < 1e-12 && aabb.min.y.abs() < 1e-12);
    }

    #[test]
    fn a_collection_unions_its_members_and_a_foreign_leaf_refuses() {
        let mut builder = GeometryGraphBuilder::default();
        let a = builder
            .push(mesh(&[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]))
            .unwrap();
        let b = builder
            .push(GeometryNode::BoundingBox(Aabb {
                min: Point3::new(-1.0, 0.0, 2.0),
                max: Point3::new(0.0, 3.0, 4.0),
            }))
            .unwrap();
        let root = builder.push(GeometryNode::Collection(vec![a, b])).unwrap();
        let point = builder.push(GeometryNode::Point3(Point3::ZERO)).unwrap();
        let mixed = builder
            .push(GeometryNode::Collection(vec![a, point]))
            .unwrap();
        let graph = builder.finish(vec![root, mixed]).unwrap();
        let aabb = exact(&graph, root).unwrap();
        assert_eq!(aabb.min, Point3::new(-1.0, 0.0, 0.0));
        assert_eq!(aabb.max, Point3::new(1.0, 3.0, 4.0));
        assert_eq!(exact(&graph, mixed), None);
    }
}
