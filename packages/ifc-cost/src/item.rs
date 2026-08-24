//! `IfcCostItem` — one line in a cost schedule.
//!
//! Attribute positions come from the IFC4 schema in
//! `references/ifc-spec/ifc4-add2-tc1/IFC4.exp`. They are read defensively:
//! real files are routinely short a trailing optional attribute, so every
//! accessor returns an `Option` rather than indexing blindly.

use ifc_model::{Entity, EntityId, Value};

/// A borrowed view of an `IfcCostItem` entity.
#[derive(Debug, Clone, Copy)]
pub struct CostItem<'m> {
    id: EntityId,
    entity: &'m Entity,
}

/// `IfcCostItem` attribute slots, from `IfcRoot` down.
mod slot {
    /// `GlobalId` (from `IfcRoot`).
    pub const GLOBAL_ID: usize = 0;
    /// `Name` (from `IfcRoot`).
    pub const NAME: usize = 2;
    /// `Identification`.
    pub const IDENTIFICATION: usize = 4;
    /// `CostValues`.
    pub const COST_VALUES: usize = 5;
    /// `CostQuantities`.
    pub const COST_QUANTITIES: usize = 6;
}

impl<'m> CostItem<'m> {
    /// Wrap an entity known to be an `IfcCostItem`.
    pub fn new(id: EntityId, entity: &'m Entity) -> Self {
        Self { id, entity }
    }

    /// The entity id in the file.
    pub fn id(&self) -> EntityId {
        self.id
    }

    /// The `GlobalId` string.
    pub fn global_id(&self) -> Option<&'m str> {
        self.entity.text(slot::GLOBAL_ID)
    }

    /// The human-readable name.
    pub fn name(&self) -> Option<&'m str> {
        self.entity.text(slot::NAME)
    }

    /// The user-facing identification code.
    pub fn identification(&self) -> Option<&'m str> {
        self.entity.text(slot::IDENTIFICATION)
    }

    /// Ids of the `IfcCostValue`s attached to this item.
    pub fn value_refs(&self) -> Vec<EntityId> {
        refs_in(self.entity.attribute(slot::COST_VALUES))
    }

    /// Ids of the quantities this cost is computed against.
    pub fn quantity_refs(&self) -> Vec<EntityId> {
        refs_in(self.entity.attribute(slot::COST_QUANTITIES))
    }
}

/// Collect entity references from an optional aggregate attribute.
fn refs_in(value: Option<&Value>) -> Vec<EntityId> {
    let mut out = Vec::new();
    if let Some(v) = value {
        v.for_each_ref(&mut |id| out.push(id));
    }
    out
}
