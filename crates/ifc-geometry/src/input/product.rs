//! Product shape and placement links.
//!
//! # Why a view and not a walk
//!
//! IfcProduct is abstract: walls, proxies, sites and 200 other
//! subtypes carry ObjectPlacement and Representation at the same
//! two absolute slots, inherited from IfcProduct. Reading them by
//! slot here means the caller never enumerates subtypes, and a
//! schema query decides what counts as a product.

use ifc_model::{Entity, EntityId, Model};

use crate::slots::Slots;

/// Absolute slots on IfcProduct, inherited by every subtype.
pub mod slot {
    /// IfcRoot.GlobalId .. IfcObject.ObjectType occupy slots 0..4.
    /// IfcProduct adds its own two after them.
    #[cfg_attr(not(feature = "lowering"), allow(dead_code))]
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
    ///
    /// Only `lower` consumes this today. `input` is private, so a kernel-free
    /// build sees no caller; that is a visibility artifact, not a boundary --
    /// reading slot 5 is a plain IfcProduct slot read either way.
    #[cfg_attr(not(feature = "lowering"), allow(dead_code))]
    pub fn object_placement(&self) -> Option<EntityId> {
        self.slots.opt_ref(slot::OBJECT_PLACEMENT)
    }

    /// The shape definition, absent for products with no geometry.
    pub fn representation(&self) -> Option<EntityId> {
        self.slots.opt_ref(slot::REPRESENTATION)
    }
}

/// Products in the model that carry geometry, in stable id order.
///
/// Kernel-free on purpose: asking which entities have a shape is a slot read,
/// so this answers the same under `--no-default-features` as it does with the
/// full kernel linked. `tests/kernel_free_build.rs` holds that line.
///
/// IfcProduct is abstract with hundreds of subtypes, so enumerating names
/// would rot. A product is recognised structurally instead: it has a
/// Representation in slot 6 pointing at an IfcProductRepresentation.
pub fn geometric_products(model: &Model) -> Vec<EntityId> {
    let mut found: Vec<EntityId> = model
        .iter()
        .filter(|(id, entity)| {
            let product = Product::new(*id, entity);
            product.representation().is_some_and(|shape| {
                model.get(shape).is_some_and(|e| {
                    e.type_name.contains("PRODUCTREPRESENTATION")
                        || e.type_name.contains("PRODUCTDEFINITIONSHAPE")
                })
            })
        })
        .map(|(id, _)| id)
        .collect();
    found.sort();
    found
}
