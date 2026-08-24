//! `IfcCostSchedule` — the document containing cost items.

use ifc_model::{Entity, EntityId};

/// A borrowed view of an `IfcCostSchedule` entity.
#[derive(Debug, Clone, Copy)]
pub struct CostSchedule<'m> {
    id: EntityId,
    entity: &'m Entity,
}

mod slot {
    /// `GlobalId` (from `IfcRoot`).
    pub const GLOBAL_ID: usize = 0;
    /// `Name` (from `IfcRoot`).
    pub const NAME: usize = 2;
    /// `PredefinedType`, e.g. `.BUDGET.`
    pub const PREDEFINED_TYPE: usize = 8;
}

impl<'m> CostSchedule<'m> {
    /// Wrap an entity known to be an `IfcCostSchedule`.
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

    /// The schedule name.
    pub fn name(&self) -> Option<&'m str> {
        self.entity.text(slot::NAME)
    }

    /// The predefined type token, e.g. `BUDGET`, without its dots.
    pub fn predefined_type(&self) -> Option<&'m str> {
        match self.entity.attribute(slot::PREDEFINED_TYPE)? {
            ifc_model::Value::Enum(e) => Some(e),
            _ => None,
        }
    }
}
