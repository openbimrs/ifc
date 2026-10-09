//! `polygon-extruder`: a small, real `MeshCompiler` written outside the crate.
//!
//! This is the caller-supplied backend of the comparison. It names only the
//! published Axiolid contracts and the neutral graph, never an
//! `ifc-geometry` internal, so it builds under the contracts-only `compile`
//! feature with no reference engine linked.
//!
//! It compiles a deliberately narrow, straight-edged subset, and compiles
//! it itself:
//!
//! * `Collection` and `Instance` (mapped items, placements), composing the
//!   transforms and flipping triangle winding under a mirror;
//! * `TriMesh` leaves, copied and placed;
//! * `PolygonMesh` leaves, and `BRep` leaves whose faces are plain planar
//!   polygons (no face surface, no edge curve, as `IfcFacetedBrep` lowers),
//!   each face triangulated in its own plane, holes included;
//! * linear `Extrusion` of a straight-edged profile: an unrounded
//!   `Rectangle` (solid or hollow), a `Contour` of line segments with or
//!   without holes, or either under a `Derived` 2D transform; caps
//!   triangulated, sides as quads;
//! * `Primitive::Block`, centred on its origin as the neutral model defines it.
//!
//! Everything else -- curved profiles and edges, booleans, half-spaces,
//! revolutions, sweeps along curves -- is refused with
//! `GeomError::UnsupportedInput` naming the node family. Geometry it admits
//! but cannot build (a zero depth, a profile with no area, a
//! self-intersecting face) is refused as `GeomError::Degenerate`. It never
//! approximates and never drops a node.

use axiolid_contracts::{
    Backend, BackendDescriptor, BackendId, ExecutionOptions, ExecutionTarget, GeomError,
    GeomResult, Operation,
};
use axiolid_core::{Point2, Point3, Transform3, Vec3};
use axiolid_curve::Curve2;
use axiolid_mesh::{PolygonMesh, TriMesh};
use axiolid_mesh_compile_contract::MeshCompiler;
use axiolid_model::{GeometryGraph, GeometryNode, NodeId, SolidOperation};
use axiolid_primitive::Primitive;
use axiolid_profile::{Contour, Profile};
use axiolid_topology::{BRep, Orientation};

use crate::triangulate::{orient, signed_area, triangulate};

/// Graph nodes visited before refusing as over budget. Mapped items can
/// multiply work; a budget refusal is a typed answer, never a hang.
const NODE_BUDGET: usize = 1_000_000;

/// The caller-supplied kernel. Stateless, so one instance serves a run.
#[derive(Debug, Default, Clone, Copy)]
pub struct PolygonExtruder;

impl PolygonExtruder {
    /// Its identity, as it appears in the comparison table.
    pub const ID: BackendId = BackendId::new("polygon-extruder");
}

impl Backend for PolygonExtruder {
    fn descriptor(&self) -> BackendDescriptor {
        BackendDescriptor::new(Self::ID, ExecutionTarget::PortableCpu)
    }
}

impl MeshCompiler for PolygonExtruder {
    fn compile_mesh(
        &self,
        graph: &GeometryGraph,
        root: NodeId,
        options: &ExecutionOptions,
    ) -> GeomResult<TriMesh> {
        let tolerance = options.tolerance().linear();
        let mut out = MeshBuilder::default();
        let mut stack = vec![(root, Transform3::IDENTITY)];
        let mut visited = 0usize;
        while let Some((id, transform)) = stack.pop() {
            options.check_cancelled()?;
            visited += 1;
            if visited > NODE_BUDGET {
                return Err(GeomError::BudgetExceeded {
                    resource: "graph nodes",
                });
            }
            let node = graph
                .get(id)
                .ok_or_else(|| GeomError::InvalidInput(format!("{id:?} is not in the graph")))?;
            match node {
                GeometryNode::Collection(children) => {
                    stack.extend(children.iter().rev().map(|child| (*child, transform)));
                }
                GeometryNode::Instance(instance) => {
                    stack.push((instance.source, transform * instance.transform));
                }
                GeometryNode::TriMesh(mesh) => out.append_mesh(mesh, transform)?,
                GeometryNode::PolygonMesh(mesh) => {
                    out.append_polygon_mesh(mesh, transform, tolerance)?;
                }
                GeometryNode::BRep(brep) => out.append_brep(brep, transform, tolerance)?,
                GeometryNode::SolidOperation(SolidOperation::Extrusion {
                    profile,
                    direction,
                    depth,
                }) => {
                    let region = Region::of_profile(graph, *profile, tolerance)?;
                    out.append_extrusion(&region, *direction, *depth, transform, tolerance)?;
                }
                GeometryNode::Primitive(Primitive::Block { x, y, z }) => {
                    out.append_block(*x, *y, *z, transform)?;
                }
                other => return Err(unsupported(family(other))),
            }
        }
        Ok(out.finish())
    }
}

/// The typed refusal for a node family this kernel does not compile.
fn unsupported(input: &'static str) -> GeomError {
    GeomError::UnsupportedInput {
        backend: PolygonExtruder::ID,
        operation: Operation::GraphCompilation,
        input,
    }
}

/// A stable name for a node family, for the refusal.
fn family(node: &GeometryNode) -> &'static str {
    match node {
        GeometryNode::SolidOperation(operation) => match operation {
            SolidOperation::Boolean { .. } => "SolidOperation::Boolean",
            SolidOperation::BoundedHalfSpace { .. } => "SolidOperation::BoundedHalfSpace",
            SolidOperation::Revolution { .. } => "SolidOperation::Revolution",
            SolidOperation::TaperedExtrusion { .. } => "SolidOperation::TaperedExtrusion",
            SolidOperation::SweptDisk { .. } => "SolidOperation::SweptDisk",
            SolidOperation::FixedReferenceSweep { .. } => "SolidOperation::FixedReferenceSweep",
            SolidOperation::SurfaceCurveSweep { .. } => "SolidOperation::SurfaceCurveSweep",
            _ => "SolidOperation (other)",
        },
        GeometryNode::Primitive(_) => "Primitive (other than Block)",
        GeometryNode::HalfSpace(_) => "HalfSpace",
        GeometryNode::Surface(_) | GeometryNode::SurfaceRelation(_) => "Surface",
        GeometryNode::Curve2(_) | GeometryNode::Curve3(_) | GeometryNode::CurveRelation(_) => {
            "Curve"
        }
        GeometryNode::BoundingBox(_) => "BoundingBox",
        GeometryNode::Profile(_) | GeometryNode::OpenProfile(_) => "bare Profile",
        _ => "node family",
    }
}

/// A planar region: its points and its rings of point indices, the outer
/// ring first and counter-clockwise, holes clockwise.
struct Region {
    points: Vec<Point2>,
    rings: Vec<Vec<usize>>,
}

impl Region {
    /// Build from raw rings, dropping repeated and closing points and
    /// refusing a ring that encloses no area.
    fn new(raw: Vec<Vec<Point2>>, tolerance: f64) -> GeomResult<Self> {
        let mut points = Vec::new();
        let mut rings = Vec::with_capacity(raw.len());
        for mut ring in raw {
            ring.dedup_by(|a, b| a.distance(*b) <= tolerance);
            while ring.len() > 1 && ring[0].distance(ring[ring.len() - 1]) <= tolerance {
                ring.pop();
            }
            if ring.len() < 3 {
                return Err(GeomError::Degenerate(
                    "polygon ring has fewer than three distinct vertices".into(),
                ));
            }
            let start = points.len();
            points.extend(ring);
            rings.push((start..points.len()).collect::<Vec<_>>());
        }
        for ring in &rings {
            if signed_area(&points, ring).abs() <= tolerance * tolerance {
                return Err(GeomError::Degenerate(
                    "polygon ring encloses no area".into(),
                ));
            }
        }
        let rings = orient(&points, rings);
        Ok(Self { points, rings })
    }

    fn of_profile(graph: &GeometryGraph, profile: NodeId, tolerance: f64) -> GeomResult<Self> {
        let Some(GeometryNode::Profile(profile)) = graph.get(profile) else {
            return Err(GeomError::InvalidInput(
                "an extrusion's profile operand is not a Profile node".into(),
            ));
        };
        Self::new(profile_rings(profile, tolerance)?, tolerance)
    }

    fn triangles(&self, tolerance: f64) -> GeomResult<Vec<[usize; 3]>> {
        triangulate(&self.points, &self.rings, tolerance * tolerance)
    }
}

/// A straight-edged profile's rings, outer first, in profile coordinates.
fn profile_rings(profile: &Profile, tolerance: f64) -> GeomResult<Vec<Vec<Point2>>> {
    let rectangle = |x: f64, y: f64| {
        let (hx, hy) = (x / 2.0, y / 2.0);
        vec![
            Point2::new(-hx, -hy),
            Point2::new(hx, -hy),
            Point2::new(hx, hy),
            Point2::new(-hx, hy),
        ]
    };
    match profile {
        Profile::Rectangle(r) => {
            let rounded = |radius: Option<f64>| radius.is_some_and(|r| r != 0.0);
            if rounded(r.outer_radius) || rounded(r.inner_radius) {
                return Err(unsupported("Rectangle profile with rounded corners"));
            }
            let mut rings = vec![rectangle(r.x, r.y)];
            if let Some(t) = r.thickness {
                rings.push(rectangle(r.x - 2.0 * t, r.y - 2.0 * t));
            }
            Ok(rings)
        }
        Profile::Contour(contour) => std::iter::once(&contour.outer)
            .chain(&contour.holes)
            .map(|ring| straight_contour(ring, tolerance))
            .collect(),
        Profile::Derived { basis, transform } => Ok(profile_rings(basis, tolerance)?
            .into_iter()
            .map(|ring| {
                ring.into_iter()
                    .map(|p| transform.transform_point2(p))
                    .collect()
            })
            .collect()),
        Profile::Circle(_) => Err(unsupported("Circle profile")),
        Profile::Ellipse(_) => Err(unsupported("Ellipse profile")),
        Profile::Section(_) => Err(unsupported("parameterised Section profile")),
        Profile::Composite(_) => Err(unsupported("Composite profile")),
        Profile::CenterLine(_) => Err(unsupported("CenterLine profile")),
        _ => Err(unsupported("Profile (other)")),
    }
}

/// The start points of a contour of line segments, in traversal order,
/// refusing a gap between consecutive segments rather than bridging it.
fn straight_contour(contour: &Contour, tolerance: f64) -> GeomResult<Vec<Point2>> {
    let mut ends = Vec::with_capacity(contour.segments.len());
    for segment in &contour.segments {
        let Curve2::Line(line) = &segment.curve else {
            return Err(unsupported("curved Contour segment"));
        };
        let at = |t: f64| line.origin + line.direction * t;
        let (a, b) = (at(segment.domain.start), at(segment.domain.end));
        ends.push(if segment.same_sense { (a, b) } else { (b, a) });
    }
    for (index, (_, end)) in ends.iter().enumerate() {
        let next = ends[(index + 1) % ends.len()].0;
        if end.distance(next) > tolerance {
            return Err(GeomError::InvalidInput(format!(
                "contour segment {index} ends {} from where the next starts",
                end.distance(next)
            )));
        }
    }
    let points: Vec<Point2> = ends.into_iter().map(|(start, _)| start).collect();
    if points.iter().all(|point| point.is_finite()) {
        Ok(points)
    } else {
        Err(GeomError::InvalidInput(
            "contour has a non-finite vertex".into(),
        ))
    }
}

/// Accumulates placed triangles into one mesh.
#[derive(Default)]
struct MeshBuilder {
    positions: Vec<Point3>,
    indices: Vec<u32>,
}

impl MeshBuilder {
    fn finish(self) -> TriMesh {
        TriMesh::new(self.positions, self.indices)
    }

    /// Append placed vertices, returning the index of the first.
    fn push_points(
        &mut self,
        points: impl IntoIterator<Item = Point3>,
        transform: Transform3,
    ) -> GeomResult<u32> {
        let base = index(self.positions.len())?;
        self.positions
            .extend(points.into_iter().map(|p| transform.transform_point3(p)));
        index(self.positions.len())?;
        Ok(base)
    }

    /// Append triangles over local indices, flipping them when `flip`.
    fn push_triangles(
        &mut self,
        base: u32,
        triangles: &[[usize; 3]],
        flip: bool,
    ) -> GeomResult<()> {
        for [a, b, c] in triangles {
            let (b, c) = if flip { (c, b) } else { (b, c) };
            for local in [a, b, c] {
                self.indices.push(base + index(*local)?);
            }
        }
        Ok(())
    }

    fn append_mesh(&mut self, mesh: &TriMesh, transform: Transform3) -> GeomResult<()> {
        mesh.validate_structure()
            .map_err(|error| GeomError::InvalidInput(format!("triangle mesh leaf: {error}")))?;
        let base = self.push_points(mesh.positions.iter().copied(), transform)?;
        let triangles: Vec<[usize; 3]> = mesh
            .triangles()
            .map(|[a, b, c]| [a as usize, b as usize, c as usize])
            .collect();
        self.push_triangles(base, &triangles, mirrors(transform))
    }

    fn append_polygon_mesh(
        &mut self,
        mesh: &PolygonMesh,
        transform: Transform3,
        tolerance: f64,
    ) -> GeomResult<()> {
        let base = self.push_points(mesh.positions.iter().copied(), transform)?;
        let mut triangles = Vec::new();
        for face in &mesh.faces {
            let rings: Vec<Vec<usize>> = std::iter::once(&face.outer)
                .chain(&face.holes)
                .map(|ring| ring.iter().map(|&i| i as usize).collect())
                .collect();
            triangles.extend(face_triangles(&mesh.positions, &rings, tolerance)?);
        }
        self.push_triangles(base, &triangles, mirrors(transform))
    }

    fn append_brep(
        &mut self,
        brep: &BRep<NodeId>,
        transform: Transform3,
        tolerance: f64,
    ) -> GeomResult<()> {
        let positions: Vec<Point3> = brep.vertices().iter().map(|v| v.position).collect();
        let base = self.push_points(positions.iter().copied(), transform)?;
        let missing = || GeomError::InvalidInput("B-rep handle out of range".into());
        let mut triangles = Vec::new();
        for shell in brep.shells() {
            for (face_id, in_shell) in &shell.faces {
                let face = brep.faces().get(face_id.index()).ok_or_else(missing)?;
                if face.surface.is_some() {
                    return Err(unsupported("BRep face on an explicit surface"));
                }
                // The outer bound first, then the holes.
                let mut bounds: Vec<_> = face.bounds.iter().collect();
                bounds.sort_by_key(|bound| !bound.outer);
                let mut rings = Vec::with_capacity(bounds.len());
                for bound in bounds {
                    let edges = &brep
                        .loops()
                        .get(bound.loop_id.index())
                        .ok_or_else(missing)?
                        .edges;
                    let mut ring = Vec::with_capacity(edges.len());
                    for used in edges {
                        let edge = brep.edges().get(used.edge.index()).ok_or_else(missing)?;
                        if edge.curve.is_some() {
                            return Err(unsupported("BRep edge on a curve"));
                        }
                        let start = match used.orientation {
                            Orientation::Forward => edge.start,
                            Orientation::Reversed => edge.end,
                        };
                        ring.push(start.index());
                    }
                    if bound.orientation == Orientation::Reversed {
                        ring.reverse();
                    }
                    rings.push(ring);
                }
                let reversed = (face.orientation == Orientation::Reversed)
                    != (*in_shell == Orientation::Reversed);
                let mut found = face_triangles(&positions, &rings, tolerance)?;
                if reversed {
                    found.iter_mut().for_each(|t| t.swap(1, 2));
                }
                triangles.extend(found);
            }
        }
        self.push_triangles(base, &triangles, mirrors(transform))
    }

    fn append_extrusion(
        &mut self,
        region: &Region,
        direction: Vec3,
        depth: f64,
        transform: Transform3,
        tolerance: f64,
    ) -> GeomResult<()> {
        if !(depth.is_finite() && depth > 0.0) {
            return Err(GeomError::Degenerate(format!("extrusion depth {depth}")));
        }
        let offset = direction
            .try_normalize()
            .ok_or_else(|| GeomError::Degenerate("extrusion direction has no length".into()))?
            * depth;
        if offset.z.abs() <= tolerance {
            return Err(GeomError::Degenerate(
                "extrusion direction lies in the profile plane".into(),
            ));
        }
        let n = region.points.len();
        let bottom = region.points.iter().map(|p| Point3::new(p.x, p.y, 0.0));
        let top = bottom.clone().map(|p| p + offset);
        let base = self.push_points(bottom.chain(top), transform)?;

        // Built as if extruding up (+Z): the bottom cap faces down, the top
        // cap up, and each side quad outward, the outer ring running
        // counter-clockwise and every hole clockwise.
        let cap = region.triangles(tolerance)?;
        let mut triangles = Vec::with_capacity(2 * cap.len() + 2 * n);
        triangles.extend(cap.iter().map(|[a, b, c]| [*a, *c, *b]));
        triangles.extend(cap.iter().map(|[a, b, c]| [a + n, b + n, c + n]));
        for ring in &region.rings {
            for (k, &i) in ring.iter().enumerate() {
                let j = ring[(k + 1) % ring.len()];
                triangles.push([i, j, j + n]);
                triangles.push([i, j + n, i + n]);
            }
        }
        // Extruding down mirrors that solid through the profile plane.
        let downward = offset.z < 0.0;
        self.push_triangles(base, &triangles, downward != mirrors(transform))
    }

    fn append_block(&mut self, x: f64, y: f64, z: f64, transform: Transform3) -> GeomResult<()> {
        if ![x, y, z].iter().all(|v| v.is_finite() && *v > 0.0) {
            return Err(GeomError::Degenerate(format!(
                "block extent {x} x {y} x {z}"
            )));
        }
        let (hx, hy) = (x / 2.0, y / 2.0);
        let ring = vec![
            Point2::new(-hx, -hy),
            Point2::new(hx, -hy),
            Point2::new(hx, hy),
            Point2::new(-hx, hy),
        ];
        // A block is its base rectangle extruded by z from z/2 below centre.
        let lowered = transform * Transform3::from_translation(Vec3::new(0.0, 0.0, -z / 2.0));
        self.append_extrusion(&Region::new(vec![ring], 0.0)?, Vec3::Z, z, lowered, 0.0)
    }
}

/// Triangulate one planar face given as rings of indices into `positions`
/// (outer first), keeping its winding: triangles face where the outer ring's
/// Newell normal points. Returns index triples into `positions`.
fn face_triangles(
    positions: &[Point3],
    rings: &[Vec<usize>],
    tolerance: f64,
) -> GeomResult<Vec<[usize; 3]>> {
    let corner = |i: usize| {
        positions
            .get(i)
            .copied()
            .ok_or_else(|| GeomError::InvalidInput(format!("face vertex {i} is out of range")))
    };
    let outer = rings
        .first()
        .filter(|ring| ring.len() >= 3)
        .ok_or_else(|| GeomError::Degenerate("face has fewer than three corners".into()))?;
    // Newell's normal, about the first corner so a face far from the origin
    // (survey coordinates) keeps its precision.
    let origin = corner(outer[0])?;
    let mut normal = Vec3::ZERO;
    for k in 0..outer.len() {
        let (a, b) = (corner(outer[k])?, corner(outer[(k + 1) % outer.len()])?);
        normal += (a - origin).cross(b - origin);
    }
    if normal.length() <= tolerance * tolerance {
        return Err(GeomError::Degenerate("face encloses no area".into()));
    }
    let normal = normal.normalize();
    let u = normal.any_orthonormal_vector();
    let v = normal.cross(u);
    // Project every ring's corners into the face plane, local index space.
    let mut points = Vec::new();
    let mut global = Vec::new();
    let mut local_rings = Vec::with_capacity(rings.len());
    for ring in rings {
        let start = points.len();
        for &i in ring {
            let d = corner(i)? - origin;
            points.push(Point2::new(d.dot(u), d.dot(v)));
            global.push(i);
        }
        local_rings.push((start..points.len()).collect::<Vec<_>>());
    }
    // `orient` keeps the outer ring counter-clockwise about the normal, which
    // it already is, and turns the holes clockwise.
    let local_rings = orient(&points, local_rings);
    let triangles = triangulate(&points, &local_rings, tolerance * tolerance)?;
    Ok(triangles
        .into_iter()
        .map(|[a, b, c]| [global[a], global[b], global[c]])
        .collect())
}

/// Whether `transform` reverses orientation, so triangle winding must flip
/// to keep the outward side outward.
fn mirrors(transform: Transform3) -> bool {
    transform.matrix3.determinant() < 0.0
}

fn index(value: usize) -> GeomResult<u32> {
    u32::try_from(value).map_err(|_| GeomError::BudgetExceeded {
        resource: "u32 vertex index",
    })
}
