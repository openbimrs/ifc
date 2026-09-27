//! World-space bounds read straight off a lowered graph, without tessellating.
//!
//! Only leaves whose extent is exact as written are bounded here: triangle
//! and polygon meshes (their vertices are the shape), authored bounding
//! boxes, and the straight-edged solids in [`solid`] -- a linear extrusion
//! of a polygonal profile and a block, bounded from their vertices. Mesh
//! positions are already in world coordinates, because lowering bakes
//! placement into them. An `Instance` above a leaf applies its transform to
//! the leaf's points, never to an intermediate box, so the result stays
//! tight.
//!
//! Any other leaf — a curved profile or surface, a boolean — returns
//! `None`, and the caller falls back to the compiled mesh. Refusing is
//! cheaper than guessing: a boolean difference can only shrink its operand,
//! so taking the operand's box would over-state the result.

use axiolid_core::{Aabb, Point3, Transform3};
use axiolid_model::{GeometryGraph, GeometryNode, NodeId, SolidOperation};
use axiolid_primitive::Primitive;

mod solid;

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
            GeometryNode::SolidOperation(SolidOperation::Extrusion {
                profile,
                direction,
                depth,
            }) => {
                let vertices = solid::extrusion(graph, *profile, *direction, *depth)?;
                extend(&mut aabb, &vertices, transform);
            }
            GeometryNode::Primitive(Primitive::Block { x, y, z }) => {
                extend(&mut aabb, &solid::block(*x, *y, *z)?, transform);
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

    /// An extrusion that builds no solid falls back, so the compiler names
    /// the defect instead of this returning a flat or empty box.
    #[test]
    fn a_degenerate_extrusion_is_not_bounded_here() {
        use axiolid_core::Vec3;
        use axiolid_profile::{Profile, RectangleProfile};

        let mut builder = GeometryGraphBuilder::default();
        let profile = builder
            .push(GeometryNode::Profile(Profile::Rectangle(
                RectangleProfile {
                    x: 2.0,
                    y: 1.0,
                    thickness: None,
                    outer_radius: None,
                    inner_radius: None,
                },
            )))
            .unwrap();
        let mut extrusion = |direction: Vec3, depth: f64| {
            builder
                .push(GeometryNode::SolidOperation(SolidOperation::Extrusion {
                    profile,
                    direction,
                    depth,
                }))
                .unwrap()
        };
        let sound = extrusion(Vec3::Z, 3.0);
        let flat = extrusion(Vec3::Z, 0.0);
        let aimless = extrusion(Vec3::ZERO, 3.0);
        let graph = builder.finish(vec![sound, flat, aimless]).unwrap();

        let aabb = exact(&graph, sound).expect("a rectangle extrusion is exact");
        assert_eq!(aabb.min, Point3::new(-1.0, -0.5, 0.0));
        assert_eq!(aabb.max, Point3::new(1.0, 0.5, 3.0));
        assert_eq!(exact(&graph, flat), None);
        assert_eq!(exact(&graph, aimless), None);
    }
}
