//! `IfcTopologyResource`: the faceted-B-rep entity family.
//!
//! # Why these are views
//!
//! One `IfcClosedShell` in the corpus holds 169 faces, each with bounds and
//! a loop of shared points. Materializing every level eagerly would copy the
//! same 196-point pool 12 times. These borrow the model and resolve on
//! demand, so the lowerer decides what to intern.
//!
//! Slot indices follow STEP inheritance: a subtype's own attributes start
//! after every supertype attribute.

use ifc_model::{Entity, EntityId, Model};

use crate::error::{GeometryError, GeometryResult};
use crate::slots::Slots;

/// Attribute positions for the topology entities.
pub mod slot {
    /// `IfcPolyLoop.Polygon`
    pub const POLYGON: usize = 0;
    /// `IfcFaceBound.Bound`
    pub const BOUND: usize = 0;
    /// `IfcFaceBound.Orientation`
    pub const ORIENTATION: usize = 1;
    /// `IfcFace.Bounds`
    pub const BOUNDS: usize = 0;
    /// `IfcConnectedFaceSet.CfsFaces`
    pub const CFS_FACES: usize = 0;
    /// `IfcManifoldSolidBrep.Outer`
    pub const OUTER: usize = 0;
    /// `IfcFacetedBrepWithVoids.Voids`
    pub const VOIDS: usize = 1;
    /// `IfcVertexPoint.VertexGeometry`
    pub const VERTEX_GEOMETRY: usize = 0;
    /// `IfcEdge.EdgeStart`
    pub const EDGE_START: usize = 0;
    /// `IfcEdge.EdgeEnd`
    pub const EDGE_END: usize = 1;
    /// `IfcEdgeCurve.EdgeGeometry`
    pub const EDGE_GEOMETRY: usize = 2;
    /// `IfcEdgeCurve.SameSense`
    pub const EDGE_SAME_SENSE: usize = 3;
    /// `IfcOrientedEdge.EdgeElement`; slots 0-1 are the inherited, unset
    /// `IfcEdge` vertices, written `*` in a STEP file.
    pub const EDGE_ELEMENT: usize = 2;
    /// `IfcOrientedEdge.Orientation`
    pub const EDGE_ORIENTATION: usize = 3;
    /// `IfcEdgeLoop.EdgeList`
    pub const EDGE_LIST: usize = 0;
    /// `IfcFaceSurface.FaceSurface`
    pub const FACE_SURFACE: usize = 1;
    /// `IfcFaceSurface.SameSense`
    pub const FACE_SAME_SENSE: usize = 2;
}

/// `IfcPolyLoop`: a closed wire given as an ordered point list.
#[derive(Debug, Clone, Copy)]
pub struct PolyLoop<'m> {
    slots: Slots<'m>,
}

impl<'m> PolyLoop<'m> {
    /// Wrap an entity assumed to be an `IfcPolyLoop`.
    pub fn new(id: EntityId, entity: &'m Entity) -> Self {
        Self {
            slots: Slots::new(id, entity),
        }
    }

    /// The entity id.
    pub fn id(&self) -> EntityId {
        self.slots.id()
    }

    /// The polygon point references in file order.
    ///
    /// The schema requires at least three unique points; a shorter list
    /// bounds no area and is rejected rather than silently skipped.
    pub fn polygon(&self) -> GeometryResult<Vec<EntityId>> {
        let points = self.slots.req_ref_list(slot::POLYGON, "Polygon")?;
        if points.len() < 3 {
            return Err(self.slots.degenerate(format!(
                "polygon has {} points; a loop needs at least 3",
                points.len()
            )));
        }
        Ok(points)
    }
}

/// `IfcFaceBound` and its `IfcFaceOuterBound` subtype.
#[derive(Debug, Clone, Copy)]
pub struct FaceBound<'m> {
    slots: Slots<'m>,
}

impl<'m> FaceBound<'m> {
    /// Wrap an entity assumed to be an `IfcFaceBound` subtype.
    pub fn new(id: EntityId, entity: &'m Entity) -> Self {
        Self {
            slots: Slots::new(id, entity),
        }
    }

    /// The entity id.
    pub fn id(&self) -> EntityId {
        self.slots.id()
    }

    /// The bounding `IfcLoop`.
    pub fn bound(&self) -> GeometryResult<EntityId> {
        self.slots.req_ref(slot::BOUND, "Bound")
    }

    /// Whether the loop orientation agrees with the face normal.
    ///
    /// `.F.` means the sense is reversed: the loop must be traversed backwards
    /// to bound the face correctly. Defaulting a missing value to `true` would
    /// silently flip such a face inside out, so absence is an error.
    pub fn orientation(&self) -> GeometryResult<bool> {
        self.slots.req_bool(slot::ORIENTATION, "Orientation")
    }

    /// Whether this is the outer bound rather than a hole.
    pub fn is_outer(&self) -> bool {
        self.slots
            .type_name()
            .eq_ignore_ascii_case("IFCFACEOUTERBOUND")
    }
}

/// `IfcFace`: a bounded region, possibly with holes.
#[derive(Debug, Clone, Copy)]
pub struct Face<'m> {
    slots: Slots<'m>,
}

impl<'m> Face<'m> {
    /// Wrap an entity assumed to be an `IfcFace` subtype.
    pub fn new(id: EntityId, entity: &'m Entity) -> Self {
        Self {
            slots: Slots::new(id, entity),
        }
    }

    /// The entity id.
    pub fn id(&self) -> EntityId {
        self.slots.id()
    }

    /// The bound references. The schema requires at least one.
    pub fn bounds(&self) -> GeometryResult<Vec<EntityId>> {
        let bounds = self.slots.req_ref_list(slot::BOUNDS, "Bounds")?;
        if bounds.is_empty() {
            return Err(self.slots.degenerate("face has no bounds"));
        }
        Ok(bounds)
    }
}

/// `IfcConnectedFaceSet` and its `IfcClosedShell`/`IfcOpenShell` subtypes.
#[derive(Debug, Clone, Copy)]
pub struct ConnectedFaceSet<'m> {
    slots: Slots<'m>,
}

impl<'m> ConnectedFaceSet<'m> {
    /// Wrap an entity assumed to be an `IfcConnectedFaceSet` subtype.
    pub fn new(id: EntityId, entity: &'m Entity) -> Self {
        Self {
            slots: Slots::new(id, entity),
        }
    }

    /// The entity id.
    pub fn id(&self) -> EntityId {
        self.slots.id()
    }

    /// The member face references.
    pub fn faces(&self) -> GeometryResult<Vec<EntityId>> {
        let faces = self.slots.req_ref_list(slot::CFS_FACES, "CfsFaces")?;
        if faces.is_empty() {
            return Err(self.slots.degenerate("face set has no faces"));
        }
        Ok(faces)
    }

    /// Whether the source asserts this shell is closed.
    ///
    /// Only `IfcClosedShell` carries that guarantee. Reporting an open shell
    /// as closed would let a downstream boolean assume a valid interior.
    pub fn is_closed(&self) -> bool {
        self.slots
            .type_name()
            .eq_ignore_ascii_case("IFCCLOSEDSHELL")
    }
}

/// `IfcManifoldSolidBrep` and its `IfcFacetedBrep`/`WithVoids` subtypes.
#[derive(Debug, Clone, Copy)]
pub struct ManifoldSolidBrep<'m> {
    slots: Slots<'m>,
}

impl<'m> ManifoldSolidBrep<'m> {
    /// Wrap an entity assumed to be an `IfcManifoldSolidBrep` subtype.
    pub fn new(id: EntityId, entity: &'m Entity) -> Self {
        Self {
            slots: Slots::new(id, entity),
        }
    }

    /// The entity id.
    pub fn id(&self) -> EntityId {
        self.slots.id()
    }

    /// The outer boundary shell.
    pub fn outer(&self) -> GeometryResult<EntityId> {
        self.slots.req_ref(slot::OUTER, "Outer")
    }

    /// Interior void shells, empty unless this is an `IfcFacetedBrepWithVoids`.
    ///
    /// Voids are what make a brick a hollow block. Dropping them yields a
    /// solid that is visually identical from outside and wrong by volume, so
    /// the attribute is read whenever the subtype declares it.
    pub fn voids(&self) -> GeometryResult<Vec<EntityId>> {
        if !self
            .slots
            .type_name()
            .eq_ignore_ascii_case("IFCFACETEDBREPWITHVOIDS")
        {
            return Ok(Vec::new());
        }
        let voids = self.slots.req_ref_list(slot::VOIDS, "Voids")?;
        if voids.is_empty() {
            return Err(self
                .slots
                .degenerate("IfcFacetedBrepWithVoids declares no voids"));
        }
        Ok(voids)
    }
}

/// Resolve an entity and confirm it belongs to an expected type family.
pub fn expect_type<'m>(
    model: &'m Model,
    referrer: EntityId,
    id: EntityId,
    accepted: &[&str],
    expected: &'static str,
) -> GeometryResult<&'m Entity> {
    let entity = model.get(id).ok_or(GeometryError::MissingEntity {
        referrer,
        missing: id,
    })?;
    if accepted
        .iter()
        .any(|name| entity.type_name.eq_ignore_ascii_case(name))
    {
        return Ok(entity);
    }
    Err(GeometryError::WrongEntityType {
        entity: id,
        actual: entity.type_name.to_string(),
        expected,
    })
}

/// `IfcVertexPoint`: a topological vertex carrying its geometric point.
#[derive(Debug, Clone, Copy)]
pub struct VertexPoint<'m> {
    slots: Slots<'m>,
}

impl<'m> VertexPoint<'m> {
    /// Wrap an entity assumed to be an `IfcVertexPoint`.
    pub fn new(id: EntityId, entity: &'m Entity) -> Self {
        Self {
            slots: Slots::new(id, entity),
        }
    }

    /// The entity id.
    pub fn id(&self) -> EntityId {
        self.slots.id()
    }

    /// The `IfcCartesianPoint` this vertex sits on.
    pub fn vertex_geometry(&self) -> GeometryResult<EntityId> {
        self.slots.req_ref(slot::VERTEX_GEOMETRY, "VertexGeometry")
    }
}

/// `IfcEdge` and its `IfcEdgeCurve` subtype: a bounded piece of a curve.
///
/// `EdgeStart`/`EdgeEnd` are `IfcVertex` references, not points. An
/// `IfcEdgeCurve` adds the supporting curve and a sense flag saying whether
/// the edge runs along the curve or against it.
#[derive(Debug, Clone, Copy)]
pub struct EdgeCurve<'m> {
    slots: Slots<'m>,
}

impl<'m> EdgeCurve<'m> {
    /// Wrap an entity assumed to be an `IfcEdge` or `IfcEdgeCurve`.
    pub fn new(id: EntityId, entity: &'m Entity) -> Self {
        Self {
            slots: Slots::new(id, entity),
        }
    }

    /// The entity id.
    pub fn id(&self) -> EntityId {
        self.slots.id()
    }

    /// The start vertex reference.
    pub fn start(&self) -> GeometryResult<EntityId> {
        self.slots.req_ref(slot::EDGE_START, "EdgeStart")
    }

    /// The end vertex reference.
    pub fn end(&self) -> GeometryResult<EntityId> {
        self.slots.req_ref(slot::EDGE_END, "EdgeEnd")
    }

    /// The supporting curve, absent on a plain `IfcEdge`.
    ///
    /// A plain edge is straight between its vertices, so `None` is a complete
    /// description rather than a missing value.
    pub fn edge_geometry(&self) -> Option<EntityId> {
        self.slots.opt_ref(slot::EDGE_GEOMETRY)
    }

    /// Does the edge run along the curve's own direction?
    ///
    /// Defaults to true when absent. A false flag reverses the edge relative
    /// to its curve, which matters for any parameterised traversal.
    pub fn same_sense(&self) -> bool {
        self.slots.opt_bool(slot::EDGE_SAME_SENSE).unwrap_or(true)
    }
}

/// `IfcOrientedEdge`: a reuse of an edge, possibly reversed.
///
/// This is the entity that makes edge sharing explicit. Two faces meeting at
/// one edge each hold an oriented edge pointing at the SAME `IfcEdgeCurve`,
/// with opposite `Orientation`. Resolving through to the underlying edge is
/// what preserves the manifold; treating each use as its own edge silently
/// disconnects the solid.
#[derive(Debug, Clone, Copy)]
pub struct OrientedEdge<'m> {
    slots: Slots<'m>,
}

impl<'m> OrientedEdge<'m> {
    /// Wrap an entity assumed to be an `IfcOrientedEdge`.
    pub fn new(id: EntityId, entity: &'m Entity) -> Self {
        Self {
            slots: Slots::new(id, entity),
        }
    }

    /// The underlying edge this use points at.
    pub fn edge_element(&self) -> GeometryResult<EntityId> {
        self.slots.req_ref(slot::EDGE_ELEMENT, "EdgeElement")
    }

    /// Does this use run along the underlying edge, or against it?
    pub fn orientation(&self) -> bool {
        self.slots.opt_bool(slot::EDGE_ORIENTATION).unwrap_or(true)
    }
}

/// `IfcEdgeLoop`: a closed wire given as a list of oriented edges.
///
/// The curved counterpart of `IfcPolyLoop`. Unlike a poly loop the closure
/// is explicit: the last edge's end vertex is the first edge's start, and no
/// implied closing segment is added.
#[derive(Debug, Clone, Copy)]
pub struct EdgeLoop<'m> {
    slots: Slots<'m>,
}

impl<'m> EdgeLoop<'m> {
    /// Wrap an entity assumed to be an `IfcEdgeLoop`.
    pub fn new(id: EntityId, entity: &'m Entity) -> Self {
        Self {
            slots: Slots::new(id, entity),
        }
    }

    /// The oriented edges in traversal order.
    ///
    /// A loop needs at least one edge; an empty list bounds nothing and is
    /// rejected rather than producing a face with no boundary.
    pub fn edge_list(&self) -> GeometryResult<Vec<EntityId>> {
        let edges = self.slots.req_ref_list(slot::EDGE_LIST, "EdgeList")?;
        if edges.is_empty() {
            return Err(self.slots.degenerate("edge loop has no edges"));
        }
        Ok(edges)
    }
}

/// `IfcFaceSurface` and its `IfcAdvancedFace` subtype.
///
/// Adds a support surface and a sense flag to `IfcFace`. `SameSense` says
/// whether the face normal agrees with the surface normal; ignoring it yields
/// an inside-out face that still passes every structural check.
#[derive(Debug, Clone, Copy)]
pub struct FaceSurface<'m> {
    slots: Slots<'m>,
}

impl<'m> FaceSurface<'m> {
    /// Wrap an entity assumed to be an `IfcFaceSurface` or `IfcAdvancedFace`.
    pub fn new(id: EntityId, entity: &'m Entity) -> Self {
        Self {
            slots: Slots::new(id, entity),
        }
    }

    /// The supporting surface reference.
    pub fn face_surface(&self) -> GeometryResult<EntityId> {
        self.slots.req_ref(slot::FACE_SURFACE, "FaceSurface")
    }

    /// Does the face normal agree with the surface normal?
    pub fn same_sense(&self) -> bool {
        self.slots.opt_bool(slot::FACE_SAME_SENSE).unwrap_or(true)
    }
}
