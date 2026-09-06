//! Inventory capability: `IfcInventory` metadata and group membership.
//!
//! ## Internal split
//!
//! - `definition.rs`: inventory metadata.
//! - `items.rs`: contained asset links.

mod definition;
mod items;

use crate::error::ResourceResult;
use crate::view::ResourceView;
use ifc_model::EntityId;

pub use definition::Inventory;

impl<'m, 's> ResourceView<'m, 's> {
    pub fn inventory(&self, id: EntityId) -> ResourceResult<Inventory<'m, 's>> {
        Inventory::from_record(self.record(id, "IfcInventory")?)
    }
}
