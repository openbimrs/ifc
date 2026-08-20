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
