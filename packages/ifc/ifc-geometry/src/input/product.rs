//! Product shape and placement links.
//!
//! # Why a view and not a walk
//!
//! IfcProduct is abstract: walls, proxies, sites and 200 other
//! subtypes carry ObjectPlacement and Representation at the same
//! two absolute slots, inherited from IfcProduct. Reading them by
//! slot here means the caller never enumerates subtypes, and a
//! schema query decides what counts as a product.

use ifc_model::{Entity, EntityId};

use crate::slots::Slots;

/// Absolute slots on IfcProduct, inherited by every subtype.
pub mod slot {
    /// IfcRoot.GlobalId .. IfcObject.ObjectType occupy slots 0..4.
    /// IfcProduct adds its own two after them.
    pub const OBJECT_PLACEMENT: usize = 5;
    /// The IfcProductRepresentation for this product.
    pub const REPRESENTATION: usize = 6;
}

/// One IfcProduct occurrence: where it sits and what it looks like.
#[derive(Debug, Clone, Copy)]
pub struct Product<'m> {
    slots: Slots<'m>,
}

impl<'m> Product<'m> {
    /// Wrap an entity assumed to be an IfcProduct subtype.
    pub fn new(id: EntityId, entity: &'m Entity) -> Self {
        Self {
            slots: Slots::new(id, entity),
        }
    }

    /// The placement chain root, absent for model-space products.
    pub fn object_placement(&self) -> Option<EntityId> {
        self.slots.opt_ref(slot::OBJECT_PLACEMENT)
    }

    /// The shape definition, absent for products with no geometry.
    pub fn representation(&self) -> Option<EntityId> {
        self.slots.opt_ref(slot::REPRESENTATION)
    }
}
