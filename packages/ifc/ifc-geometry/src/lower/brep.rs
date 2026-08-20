//! Faceted B-rep lowering into exact topology.
//!
//! # Why topology and not a mesh
//!
//! An `IfcFacetedBrep` is a solid with planar faces, not a render mesh. It
//! carries face adjacency, bound nesting (holes), and void shells. Flattening
//! it to triangles at read time destroys exactly the information a boolean or
//! a volume query needs, and triangulation is a kernel decision. So this
//! builds `BRep<NodeId>` and leaves tessellation to `axiolid-tessellate`.
//!
//! # Sharing is the whole problem
//!
//! In `shared_point_faceted_brep.ifc`, 12 solids and 2028 faces are built
//! from ONE pool of 196 `IfcCartesianPoint` records. Every point is reused by
//! several faces, and every interior edge is shared by exactly two. Emitting
//! one vertex per polygon slot would produce 8112 vertices where 196 exist,
//! and no edge would ever be shared -- which silently turns a closed solid
//! into a pile of disconnected facets. Interning by `EntityId` (vertices) and
//! by unordered endpoint pair (edges) is what preserves the manifold.

use std::collections::BTreeMap;

use axiolid_core::Point3;
use axiolid_model::{GeometryNode, NodeId};
use axiolid_topology::{
    BRep, Edge, EdgeId, EdgeUse, Face, FaceBound, Loop, Orientation, Shell, ShellId, Solid, Vertex,
    VertexId,
};
use ifc_model::EntityId;

use crate::error::GeometryResult;
use crate::lower::session::LoweringSession;
use crate::resource::point::CartesianPoint;
use crate::resource::topology::{
    expect_type, ConnectedFaceSet, Face as FaceView, FaceBound as FaceBoundView, ManifoldSolidBrep,
    PolyLoop,
};
use crate::transform::Transform;

/// Chain kind reported when a brep nests too deeply or cycles.
const KIND: &str = "faceted brep";

/// Accumulates one solid's topology, interning shared vertices and edges.
///
/// Scoped to a single brep: two solids that quote the same points are still
/// independent bodies, so ids must not leak between them.
struct TopologyBuilder {
    brep: BRep<NodeId>,
    vertices: BTreeMap<EntityId, VertexId>,
    edges: BTreeMap<(usize, usize), EdgeId>,
}

impl TopologyBuilder {
    fn new() -> Self {
        Self {
            brep: BRep::default(),
            vertices: BTreeMap::new(),
            edges: BTreeMap::new(),
        }
    }

    /// Intern a vertex by source entity, so one point is one vertex.
    fn vertex(&mut self, point: EntityId, position: Point3) -> VertexId {
        if let Some(existing) = self.vertices.get(&point) {
            return *existing;
        }
        let id = self.brep.add_vertex(Vertex { position });
        self.vertices.insert(point, id);
        id
    }

    /// Intern an edge by its unordered endpoints and report its traversal sense.
    ///
    /// A closed manifold shares each edge between two faces that walk it in
    /// opposite directions. Keying on the sorted pair makes both walks find the
    /// same edge; the returned orientation records which way this use goes.
    fn edge(&mut self, start: VertexId, end: VertexId) -> EdgeUse {
        let (low, high) = (start.index(), end.index());
        let forward = low <= high;
        let key = if forward { (low, high) } else { (high, low) };
        let edge = *self.edges.entry(key).or_insert_with(|| {
            let (a, b) = if forward { (start, end) } else { (end, start) };
            self.brep.add_edge(Edge {
                start: a,
                end: b,
                curve: None,
            })
        });
        EdgeUse {
            edge,
            orientation: if forward {
                Orientation::Forward
            } else {
                Orientation::Reversed
            },
        }
    }
}

/// Lower one `IfcFacetedBrep` (or `WithVoids`) into a `BRep` node.
///
/// `frame` places the solid; brep coordinates are absolute in the file's
/// length unit, so the world frame is applied to each vertex here rather than
/// wrapped around the result. That keeps one body one node.
pub fn lower_faceted_brep_node(
    session: &mut LoweringSession<'_>,
    id: EntityId,
    frame: Transform,
) -> GeometryResult<NodeId> {
    if let Some(node) = session.memoized(id, KIND, frame) {
        return Ok(node);
    }
    session.enter(id, KIND)?;
    let result = build(session, id, frame);
    session.exit(id);
    let node = result?;
    session.memoize(id, KIND, frame, node);
    Ok(node)
}

fn build(
    session: &mut LoweringSession<'_>,
    id: EntityId,
    frame: Transform,
) -> GeometryResult<NodeId> {
    let entity = session.entity(id, id)?;
    let view = ManifoldSolidBrep::new(id, entity);
    let outer_ref = view.outer()?;
    let void_refs = view.voids()?;

    let mut builder = TopologyBuilder::new();
    let outer = shell(session, &mut builder, id, outer_ref, frame)?;
    let mut voids = Vec::with_capacity(void_refs.len());
    for void_ref in void_refs {
        voids.push(shell(session, &mut builder, id, void_ref, frame)?);
    }
    builder.brep.add_solid(Solid { outer, voids });
    session.node_for(id, GeometryNode::BRep(builder.brep))
}

/// Lower one shell and every face it holds.
fn shell(
    session: &mut LoweringSession<'_>,
    builder: &mut TopologyBuilder,
    referrer: EntityId,
    id: EntityId,
    frame: Transform,
) -> GeometryResult<ShellId> {
    let entity = expect_type(
        session.model(),
        referrer,
        id,
        &["IFCCLOSEDSHELL", "IFCOPENSHELL", "IFCCONNECTEDFACESET"],
        "IfcConnectedFaceSet",
    )?;
    let view = ConnectedFaceSet::new(id, entity);
    let closed = view.is_closed();
    let mut faces = Vec::new();
    for face_ref in view.faces()? {
        let face_id = face(session, builder, id, face_ref, frame)?;
        faces.push((face_id, Orientation::Forward));
    }
    Ok(builder.brep.add_shell(Shell { faces, closed }))
}

/// Lower one face and all of its bounds.
///
/// A planar facet needs no support surface: the loop's points define the plane
/// exactly. `Face::surface` stays `None` rather than inventing a fitted plane
/// that could disagree with the vertices.
fn face(
    session: &mut LoweringSession<'_>,
    builder: &mut TopologyBuilder,
    referrer: EntityId,
    id: EntityId,
    frame: Transform,
) -> GeometryResult<axiolid_topology::FaceId> {
    let entity = expect_type(
        session.model(),
        referrer,
        id,
        &["IFCFACE", "IFCFACESURFACE", "IFCADVANCEDFACE"],
        "IfcFace",
    )?;
    let view = FaceView::new(id, entity);
    let mut bounds = Vec::new();
    for bound_ref in view.bounds()? {
        bounds.push(bound(session, builder, id, bound_ref, frame)?);
    }
    Ok(builder.brep.add_face(Face {
        surface: None,
        bounds,
        orientation: Orientation::Forward,
    }))
}

/// Lower one face bound into a loop plus its orientation flags.
fn bound(
    session: &mut LoweringSession<'_>,
    builder: &mut TopologyBuilder,
    referrer: EntityId,
    id: EntityId,
    frame: Transform,
) -> GeometryResult<FaceBound> {
    let entity = expect_type(
        session.model(),
        referrer,
        id,
        &["IFCFACEBOUND", "IFCFACEOUTERBOUND"],
        "IfcFaceBound",
    )?;
    let view = FaceBoundView::new(id, entity);
    let loop_id = poly_loop(session, builder, id, view.bound()?, frame)?;
    Ok(FaceBound {
        loop_id,
        orientation: if view.orientation()? {
            Orientation::Forward
        } else {
            Orientation::Reversed
        },
        outer: view.is_outer(),
    })
}

/// Lower one `IfcPolyLoop` into interned vertices and edges.
///
/// The polygon is implicitly closed: the schema lists N points and the closing
/// edge from the last back to the first is implied. Emitting only N-1 edges
/// leaves the loop open and every downstream closure check fails.
fn poly_loop(
    session: &mut LoweringSession<'_>,
    builder: &mut TopologyBuilder,
    referrer: EntityId,
    id: EntityId,
    frame: Transform,
) -> GeometryResult<axiolid_topology::LoopId> {
    let entity = expect_type(
        session.model(),
        referrer,
        id,
        &["IFCPOLYLOOP"],
        "IfcPolyLoop",
    )?;
    let view = PolyLoop::new(id, entity);
    let points = view.polygon()?;

    let mut vertices = Vec::with_capacity(points.len());
    for point_ref in &points {
        let point_entity = session.entity(id, *point_ref)?;
        let raw = CartesianPoint::new(*point_ref, point_entity).coordinates_3d()?;
        let scaled = raw.map(|value| session.units().length(value));
        let placed = frame.apply(scaled);
        vertices.push(builder.vertex(*point_ref, Point3::from_array(placed)));
    }

    let mut edges = Vec::with_capacity(vertices.len());
    for (index, start) in vertices.iter().enumerate() {
        let end = vertices[(index + 1) % vertices.len()];
        if *start == end {
            continue;
        }
        edges.push(builder.edge(*start, end));
    }
    if edges.len() < 3 {
        return Err(session.degenerate(
            id,
            "IFCPOLYLOOP",
            format!("loop collapses to {} distinct edges", edges.len()),
        ));
    }
    Ok(builder.brep.add_loop(Loop { edges }))
}
