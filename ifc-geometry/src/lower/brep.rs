//! Faceted B-rep lowering into exact topology.
//!
//! # Why topology and not a mesh
//!
//! An `IfcFacetedBrep` is a solid with planar faces, not a render mesh. It
//! carries face adjacency, bound nesting (holes), and void shells. Flattening
//! it to triangles at read time destroys exactly the information a boolean or
//! a volume query needs, and triangulation is a kernel decision. So this
//! builds `BRep<NodeId>` and leaves tessellation to the
//! `axiolid-tessellation-contract` operation boundary.
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
use crate::lower::curve::lower_curve_node;
use crate::lower::session::LoweringSession;
use crate::lower::surface::lower_surface_node;
use crate::resource::point::CartesianPoint;
use crate::resource::topology::{
    expect_type, ConnectedFaceSet, EdgeCurve as EdgeCurveView, EdgeLoop as EdgeLoopView,
    Face as FaceView, FaceBound as FaceBoundView, FaceSurface as FaceSurfaceView,
    ManifoldSolidBrep, OrientedEdge as OrientedEdgeView, PolyLoop, VertexPoint as VertexPointView,
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
    /// Advanced-brep edges, interned by source entity rather than by endpoint
    /// pair: two edges may share endpoints yet follow different curves.
    curved_edges: BTreeMap<EntityId, EdgeUse<NodeId>>,
}

impl TopologyBuilder {
    fn new() -> Self {
        Self {
            brep: BRep::default(),
            vertices: BTreeMap::new(),
            edges: BTreeMap::new(),
            curved_edges: BTreeMap::new(),
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
    fn edge(&mut self, start: VertexId, end: VertexId) -> EdgeUse<NodeId> {
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
            // A faceted loop has no parametric curve: IfcPolyLoop gives
            // vertices only. A pcurve would have to be invented.
            pcurve: None,
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

/// Lower one shell or connected face set as a standalone `BRep` node.
///
/// A surface model's members are shells, which are not representation items
/// and so cannot go through the item dispatcher. Each becomes its own BRep
/// carrying exactly one shell and NO solid: a surface model asserts no
/// volume, and adding a `Solid` here would manufacture one.
pub fn lower_shell_node(
    session: &mut LoweringSession<'_>,
    referrer: EntityId,
    id: EntityId,
    frame: Transform,
) -> GeometryResult<NodeId> {
    if let Some(node) = session.memoized(id, KIND, frame) {
        return Ok(node);
    }
    session.enter(id, KIND)?;
    let mut builder = TopologyBuilder::new();
    let result = shell(session, &mut builder, referrer, id, frame);
    session.exit(id);
    result?;
    let node = session.node_for(id, GeometryNode::BRep(builder.brep))?;
    session.memoize(id, KIND, frame, node);
    Ok(node)
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
    // An IfcFaceSurface names the surface its boundary lies on, and SameSense
    // says whether the face agrees with that surface's normal. A plain
    // IfcFace has neither: its loop points define the plane exactly, so the
    // handle stays None rather than fitting a plane that could disagree.
    let (surface, orientation) = if session.type_name(id)?.eq_ignore_ascii_case("IFCFACE") {
        (None, Orientation::Forward)
    } else {
        let surface_view = FaceSurfaceView::new(id, entity);
        let node = lower_surface_node(session, surface_view.face_surface()?, frame)?;
        let sense = if surface_view.same_sense() {
            Orientation::Forward
        } else {
            Orientation::Reversed
        };
        (Some(node), sense)
    };
    Ok(builder.brep.add_face(Face {
        surface,
        bounds,
        orientation,
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
    let bound_ref = view.bound()?;
    // A face bound is a Loop, which is either a point list or an edge list.
    // Advanced breps use the latter, so dispatch rather than assuming.
    let loop_id = if session
        .type_name(bound_ref)?
        .eq_ignore_ascii_case("IFCEDGELOOP")
    {
        edge_loop(session, builder, id, bound_ref, frame)?
    } else {
        poly_loop(session, builder, id, bound_ref, frame)?
    };
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

/// Lower one `IfcEdgeLoop` into interned vertices and shared edges.
///
/// Unlike a poly loop this is already a list of edges, and the sharing is
/// explicit: several oriented edges point at one `IfcEdgeCurve`. Interning by
/// the underlying edge entity is what keeps the two faces meeting at a seam
/// attached to the SAME topological edge.
fn edge_loop(
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
        &["IFCEDGELOOP"],
        "IfcEdgeLoop",
    )?;
    let view = EdgeLoopView::new(id, entity);
    let mut edges = Vec::new();
    for oriented_ref in view.edge_list()? {
        edges.push(oriented_edge(session, builder, id, oriented_ref, frame)?);
    }
    Ok(builder.brep.add_loop(Loop { edges }))
}

/// Resolve one `IfcOrientedEdge` to a use of a shared, interned edge.
///
/// The oriented edge's own flag composes with the sense the interned edge was
/// first created in: an edge stored reversed and then used reversed runs
/// forward. Dropping either flag half-flips seams and the shell stops closing.
fn oriented_edge(
    session: &mut LoweringSession<'_>,
    builder: &mut TopologyBuilder,
    referrer: EntityId,
    id: EntityId,
    frame: Transform,
) -> GeometryResult<EdgeUse<NodeId>> {
    let entity = expect_type(
        session.model(),
        referrer,
        id,
        &["IFCORIENTEDEDGE"],
        "IfcOrientedEdge",
    )?;
    let view = OrientedEdgeView::new(id, entity);
    let base = edge_curve(session, builder, id, view.edge_element()?, frame)?;
    if view.orientation() {
        Ok(base)
    } else {
        Ok(EdgeUse {
            edge: base.edge,
            orientation: flip(base.orientation),
            pcurve: base.pcurve,
        })
    }
}

/// Intern one `IfcEdgeCurve` (or plain `IfcEdge`) and lower its support curve.
///
/// Keyed by entity id, not by endpoint pair: two edges can share endpoints and
/// still be different curves (the two halves of a full circle, for instance),
/// so the geometric key used for poly loops would wrongly merge them.
fn edge_curve(
    session: &mut LoweringSession<'_>,
    builder: &mut TopologyBuilder,
    referrer: EntityId,
    id: EntityId,
    frame: Transform,
) -> GeometryResult<EdgeUse<NodeId>> {
    if let Some(existing) = builder.curved_edges.get(&id) {
        return Ok(*existing);
    }
    let entity = expect_type(
        session.model(),
        referrer,
        id,
        &["IFCEDGE", "IFCEDGECURVE"],
        "IfcEdge",
    )?;
    let view = EdgeCurveView::new(id, entity);
    let start = topological_vertex(session, builder, id, view.start()?, frame)?;
    let end = topological_vertex(session, builder, id, view.end()?, frame)?;
    let (curve, pcurve) = match view.edge_geometry() {
        Some(curve_ref) if session.type_name(curve_ref)? == "IFCPCURVE" => {
            (None, Some(lower_curve_node(session, curve_ref, frame)?))
        }
        Some(curve_ref) => (Some(lower_curve_node(session, curve_ref, frame)?), None),
        None => (None, None),
    };
    // SameSense false means the edge runs against its curve. Record that as
    // the stored edge's orientation rather than swapping the vertices, so the
    // curve handle and the vertex order stay consistent with the file.
    let orientation = if view.same_sense() {
        Orientation::Forward
    } else {
        Orientation::Reversed
    };
    let edge = builder.brep.add_edge(Edge { start, end, curve });
    // IFC permits EdgeGeometry itself to be an IfcPCurve. Keep that
    // parameter-space relation on the use; placing it in Edge.curve would
    // mislabel it as the edge's 3D carrier geometry.
    let use_ = EdgeUse {
        edge,
        orientation,
        pcurve,
    };
    builder.curved_edges.insert(id, use_);
    Ok(use_)
}

/// Intern an `IfcVertexPoint`, reusing the vertex when several edges meet.
fn topological_vertex(
    session: &mut LoweringSession<'_>,
    builder: &mut TopologyBuilder,
    referrer: EntityId,
    id: EntityId,
    frame: Transform,
) -> GeometryResult<VertexId> {
    let entity = expect_type(
        session.model(),
        referrer,
        id,
        &["IFCVERTEXPOINT"],
        "IfcVertexPoint",
    )?;
    let point_ref = VertexPointView::new(id, entity).vertex_geometry()?;
    let point_entity = session.entity(id, point_ref)?;
    let raw = CartesianPoint::new(point_ref, point_entity).coordinates_3d()?;
    let scaled = raw.map(|value| session.units().length(value));
    let placed = frame.apply(scaled);
    Ok(builder.vertex(id, Point3::from_array(placed)))
}

/// Reverse a traversal sense.
fn flip(orientation: Orientation) -> Orientation {
    match orientation {
        Orientation::Forward => Orientation::Reversed,
        Orientation::Reversed => Orientation::Forward,
    }
}
