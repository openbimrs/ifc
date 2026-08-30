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
///
/// The inheritance chain is `IfcRoot` -> `IfcObjectDefinition` -> `IfcObject`
/// -> `IfcControl` -> `IfcCostItem`, which contributes, in order:
///
/// ```text
/// 0 GlobalId        IfcRoot
/// 1 OwnerHistory    IfcRoot
/// 2 Name            IfcRoot
/// 3 Description     IfcRoot
/// 4 ObjectType      IfcObject      <- contributes a slot
/// 5 Identification  IfcControl
/// 6 PredefinedType  IfcCostItem
/// 7 CostValues      IfcCostItem
/// 8 CostQuantities  IfcCostItem
/// ```
///
/// `IfcObjectDefinition` adds only INVERSE attributes, which are not stored
/// positionally. Verified against IFC4 EXPRESS and cross-checked by writing
/// the entity with IfcOpenShell and reading back its attribute order.
mod slot {
    /// `GlobalId` (from `IfcRoot`).
    pub const GLOBAL_ID: usize = 0;
    /// `Name` (from `IfcRoot`).
    pub const NAME: usize = 2;
    /// `Description` (from `IfcRoot`).
    pub const DESCRIPTION: usize = 3;
    /// `Identification` (from `IfcControl`).
    pub const IDENTIFICATION: usize = 5;
    /// `PredefinedType`.
    pub const PREDEFINED_TYPE: usize = 6;
    /// `CostValues`.
    pub const COST_VALUES: usize = 7;
    /// `CostQuantities`.
    pub const COST_QUANTITIES: usize = 8;
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

    /// The description.
    pub fn description(&self) -> Option<&'m str> {
        self.entity.text(slot::DESCRIPTION)
    }

    /// The predefined type token, e.g. `MATERIAL`, without its dots.
    pub fn predefined_type(&self) -> Option<&'m str> {
        match self.entity.attribute(slot::PREDEFINED_TYPE)? {
            Value::Enum(e) => Some(e),
            _ => None,
        }
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
