//! Boundary representation: vertices, edges, loops, faces and shells.
//!
//! The topology types are thin -- most are one or two references -- so
//! the writer's value is in the two things a careless one gets wrong.
//!
//! # `IfcOrientedEdge` restates nothing
//!
//! It inherits `EdgeStart` and `EdgeEnd` from `IfcEdge` but **derives**
//! both from the edge it orients. A derived attribute is written `*` in
//! STEP, not `$`: the first says "the supertype computes this", the
//! second says "this is absent". Both decode to a missing value in
//! most readers, so only the serialized text tells them apart -- and a
//! file using `$` is not conforming.
//!
//! # Orientation is not decoration
//!
//! `IfcFaceBound.Orientation` and `IfcOrientedEdge.Orientation` decide
//! which side of a face is solid and which way a loop runs. Writing the
//! wrong boolean produces a file that parses, validates, and denotes an
//! inside-out solid.
//!
//! # What this module does not check
//!
//! Shell closure, loop planarity and face orientation consistency all
//! need an evaluator. A caller can build an `IfcClosedShell` that is
//! not closed and nothing here objects: that is the boundary of what a
//! kernel-free writer can honestly assert, and it is documented rather
//! than silently implied.

use ifc_model::{Entity, EntityId, Transaction, Value};

use crate::error::GeometryError;
use crate::resource::topology::slot;

use super::{invalid, refs};

/// Stage an `IfcVertexPoint`.
pub fn vertex_point(tx: &mut Transaction, geometry: EntityId) -> EntityId {
    let mut attrs = vec![Value::Null; 1];
    attrs[slot::VERTEX_GEOMETRY] = Value::Ref(geometry);
    tx.create(Entity::new("IFCVERTEXPOINT", attrs))
}

/// Stage an `IfcEdge` between two vertices.
pub fn edge(tx: &mut Transaction, start: EntityId, end: EntityId) -> EntityId {
    let mut attrs = vec![Value::Null; 2];
    attrs[slot::EDGE_START] = Value::Ref(start);
    attrs[slot::EDGE_END] = Value::Ref(end);
    tx.create(Entity::new("IFCEDGE", attrs))
}

/// Stage an `IfcEdgeCurve`: an edge with curve geometry.
///
/// `same_sense` says whether the edge runs along the curve's own
/// direction. It is not optional and it is not cosmetic.
pub fn edge_curve(
    tx: &mut Transaction,
    start: EntityId,
    end: EntityId,
    geometry: EntityId,
    same_sense: bool,
) -> EntityId {
    let mut attrs = vec![Value::Null; 4];
    attrs[slot::EDGE_START] = Value::Ref(start);
    attrs[slot::EDGE_END] = Value::Ref(end);
    attrs[slot::EDGE_GEOMETRY] = Value::Ref(geometry);
    attrs[slot::EDGE_SAME_SENSE] = Value::Bool(same_sense);
    tx.create(Entity::new("IFCEDGECURVE", attrs))
}

/// Stage an `IfcOrientedEdge` over an existing edge.
///
/// The inherited `EdgeStart` and `EdgeEnd` are written `Value::Derived`
/// -- `*` in STEP -- because the schema derives them from
/// `EdgeElement`. Writing `$` there would claim they are absent, which
/// is a different and non-conforming statement. See the module note.
pub fn oriented_edge(tx: &mut Transaction, element: EntityId, orientation: bool) -> EntityId {
    let mut attrs = vec![Value::Null; 4];
    attrs[slot::EDGE_START] = Value::Derived;
    attrs[slot::EDGE_END] = Value::Derived;
    attrs[slot::EDGE_ELEMENT] = Value::Ref(element);
    attrs[slot::EDGE_ORIENTATION] = Value::Bool(orientation);
    tx.create(Entity::new("IFCORIENTEDEDGE", attrs))
}

/// Stage an `IfcSubedge`.
///
/// Unlike an oriented edge, a subedge *does* state its own vertices:
/// they are a genuine restriction of the parent, not a derivation.
pub fn subedge(tx: &mut Transaction, start: EntityId, end: EntityId, parent: EntityId) -> EntityId {
    let mut attrs = vec![Value::Null; 3];
    attrs[slot::EDGE_START] = Value::Ref(start);
    attrs[slot::EDGE_END] = Value::Ref(end);
    attrs[slot::PARENT_EDGE] = Value::Ref(parent);
    tx.create(Entity::new("IFCSUBEDGE", attrs))
}

/// Stage an `IfcPolyLoop` through an ordered point list.
///
/// # Errors
///
/// Refuses fewer than three points: `LIST [3:?]`, because two points
/// bound no area. The closing point is implicit -- repeating the first
/// point at the end creates a zero-length edge, so this writer takes
/// the list as the schema defines it and does not close it for you.
pub fn poly_loop(tx: &mut Transaction, polygon: &[EntityId]) -> Result<EntityId, GeometryError> {
    const T: &str = "IFCPOLYLOOP";
    if polygon.len() < 3 {
        return Err(invalid(
            T,
            "Polygon",
            format!("expected at least 3 points, got {}", polygon.len()),
        ));
    }
    // LIST [3:?] OF UNIQUE: a repeated vertex is a zero-length edge, and
    // a loop that closes by repeating its first point is the commonest
    // way to write one. The closure is implied, never stated.
    for (i, point) in polygon.iter().enumerate() {
        if polygon[..i].contains(point) {
            return Err(invalid(
                T,
                "Polygon",
                "the point list is UNIQUE; a loop closes implicitly, so the \
                 first point must not be repeated at the end",
            ));
        }
    }
    let mut attrs = vec![Value::Null; 1];
    attrs[slot::POLYGON] = refs(polygon);
    Ok(tx.create(Entity::new(T, attrs)))
}

/// Stage an `IfcEdgeLoop` over oriented edges.
///
/// # Errors
///
/// Refuses an empty edge list. Whether the edges actually form a closed
/// circuit is not checked: that needs to follow vertex identity through
/// the chain, which is evaluation.
pub fn edge_loop(tx: &mut Transaction, edges: &[EntityId]) -> Result<EntityId, GeometryError> {
    const T: &str = "IFCEDGELOOP";
    if edges.is_empty() {
        return Err(invalid(T, "EdgeList", "expected at least one edge"));
    }
    let mut attrs = vec![Value::Null; 1];
    attrs[slot::EDGE_LIST] = refs(edges);
    Ok(tx.create(Entity::new(T, attrs)))
}

/// Stage an `IfcVertexLoop`: a degenerate loop at a single vertex.
///
/// Legal, and occasionally meaningful as a cone apex.
pub fn vertex_loop(tx: &mut Transaction, vertex: EntityId) -> EntityId {
    tx.create(Entity::new("IFCVERTEXLOOP", vec![Value::Ref(vertex)]))
}

/// Stage an `IfcFaceBound`.
///
/// `orientation` false means the loop runs opposite to the face's own
/// sense. Getting it wrong turns a hole into a boundary.
pub fn face_bound(tx: &mut Transaction, bound: EntityId, orientation: bool) -> EntityId {
    bound_entity(tx, "IFCFACEBOUND", bound, orientation)
}

/// Stage an `IfcFaceOuterBound`: the bound enclosing the face's area.
///
/// A face has at most one of these. The distinction from a plain
/// `IfcFaceBound` is the entity type, not an attribute.
pub fn face_outer_bound(tx: &mut Transaction, bound: EntityId, orientation: bool) -> EntityId {
    bound_entity(tx, "IFCFACEOUTERBOUND", bound, orientation)
}

/// The two slots both bound types share.
fn bound_entity(
    tx: &mut Transaction,
    type_name: &'static str,
    bound: EntityId,
    orientation: bool,
) -> EntityId {
    let mut attrs = vec![Value::Null; 2];
    attrs[slot::BOUND] = Value::Ref(bound);
    attrs[slot::ORIENTATION] = Value::Bool(orientation);
    tx.create(Entity::new(type_name, attrs))
}

/// Stage an `IfcFace` from its bounds.
///
/// # Errors
///
/// Refuses an empty bound set: `SET [1:?]`.
pub fn face(tx: &mut Transaction, bounds: &[EntityId]) -> Result<EntityId, GeometryError> {
    const T: &str = "IFCFACE";
    if bounds.is_empty() {
        return Err(invalid(T, "Bounds", "expected at least one bound"));
    }
    let mut attrs = vec![Value::Null; 1];
    attrs[slot::BOUNDS] = refs(bounds);
    Ok(tx.create(Entity::new(T, attrs)))
}

/// Stage an `IfcFaceSurface` or `IfcAdvancedFace`.
///
/// The two differ only in type name: an advanced face promises its
/// surface is one of the analytic or B-spline forms, which is a claim
/// about the referenced entity rather than about this record.
///
/// # Errors
///
/// Refuses an empty bound set.
pub fn face_surface(
    tx: &mut Transaction,
    bounds: &[EntityId],
    surface: EntityId,
    same_sense: bool,
    advanced: bool,
) -> Result<EntityId, GeometryError> {
    let type_name = if advanced {
        "IFCADVANCEDFACE"
    } else {
        "IFCFACESURFACE"
    };
    if bounds.is_empty() {
        return Err(invalid(type_name, "Bounds", "expected at least one bound"));
    }
    let mut attrs = vec![Value::Null; 3];
    attrs[slot::BOUNDS] = refs(bounds);
    attrs[slot::FACE_SURFACE] = Value::Ref(surface);
    attrs[slot::FACE_SAME_SENSE] = Value::Bool(same_sense);
    Ok(tx.create(Entity::new(type_name, attrs)))
}

/// Which shell kind a face set forms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellKind {
    /// `IfcClosedShell`: claims to bound a volume.
    ///
    /// The claim is not verified here -- see the module note on what a
    /// kernel-free writer cannot check.
    Closed,
    /// `IfcOpenShell`: a surface patch that does not enclose anything.
    Open,
    /// `IfcConnectedFaceSet`: connected faces, neither open nor closed.
    Connected,
}

impl ShellKind {
    /// The entity type name.
    fn type_name(self) -> &'static str {
        match self {
            Self::Closed => "IFCCLOSEDSHELL",
            Self::Open => "IFCOPENSHELL",
            Self::Connected => "IFCCONNECTEDFACESET",
        }
    }
}

/// Stage a shell over a set of faces.
///
/// # Errors
///
/// Refuses an empty face set: `SET [1:?]` on all three types.
pub fn shell(
    tx: &mut Transaction,
    kind: ShellKind,
    faces: &[EntityId],
) -> Result<EntityId, GeometryError> {
    let type_name = kind.type_name();
    if faces.is_empty() {
        return Err(invalid(type_name, "CfsFaces", "expected at least one face"));
    }
    let mut attrs = vec![Value::Null; 1];
    attrs[slot::CFS_FACES] = refs(faces);
    Ok(tx.create(Entity::new(type_name, attrs)))
}

/// Which brep flavour to author.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrepKind {
    /// `IfcFacetedBrep`: every face is planar and polygonally bounded.
    Faceted,
    /// `IfcAdvancedBrep`: faces may carry analytic or B-spline surfaces.
    Advanced,
}

/// Stage a manifold solid brep, with or without voids.
///
/// Passing an empty `voids` slice authors the plain form
/// (`IfcFacetedBrep` / `IfcAdvancedBrep`); passing voids authors the
/// `WithVoids` subtype. The schema declares `Voids` as `SET [1:?]`, so
/// there is no such thing as a `WithVoids` brep with no voids -- the
/// two are different entity types, not one type with an empty set.
pub fn manifold_solid_brep(
    tx: &mut Transaction,
    kind: BrepKind,
    outer: EntityId,
    voids: &[EntityId],
) -> EntityId {
    let type_name = match (kind, voids.is_empty()) {
        (BrepKind::Faceted, true) => "IFCFACETEDBREP",
        (BrepKind::Faceted, false) => "IFCFACETEDBREPWITHVOIDS",
        (BrepKind::Advanced, true) => "IFCADVANCEDBREP",
        (BrepKind::Advanced, false) => "IFCADVANCEDBREPWITHVOIDS",
    };
    let mut attrs = vec![Value::Null; if voids.is_empty() { 1 } else { 2 }];
    attrs[slot::OUTER] = Value::Ref(outer);
    if !voids.is_empty() {
        attrs[slot::VOIDS] = refs(voids);
    }
    tx.create(Entity::new(type_name, attrs))
}

/// Stage an `IfcShellBasedSurfaceModel`.
///
/// A surface model, not a solid: the shells describe skin, and nothing
/// claims they enclose a volume.
///
/// # Errors
///
/// Refuses an empty shell set.
pub fn shell_based_surface_model(
    tx: &mut Transaction,
    shells: &[EntityId],
) -> Result<EntityId, GeometryError> {
    const T: &str = "IFCSHELLBASEDSURFACEMODEL";
    if shells.is_empty() {
        return Err(invalid(T, "SbsmBoundary", "expected at least one shell"));
    }
    Ok(tx.create(Entity::new(T, vec![refs(shells)])))
}

/// Stage an `IfcFaceBasedSurfaceModel`.
///
/// # Errors
///
/// Refuses an empty face set.
pub fn face_based_surface_model(
    tx: &mut Transaction,
    face_sets: &[EntityId],
) -> Result<EntityId, GeometryError> {
    const T: &str = "IFCFACEBASEDSURFACEMODEL";
    if face_sets.is_empty() {
        return Err(invalid(T, "FbsmFaces", "expected at least one face set"));
    }
    Ok(tx.create(Entity::new(T, vec![refs(face_sets)])))
}
