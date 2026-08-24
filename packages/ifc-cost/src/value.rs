//! `IfcCostValue` — a monetary amount with a category.

use ifc_model::{Entity, EntityId};

/// A borrowed view of an `IfcCostValue` entity.
#[derive(Debug, Clone, Copy)]
pub struct CostValue<'m> {
    id: EntityId,
    entity: &'m Entity,
}

mod slot {
    /// `Name` (from `IfcAppliedValue`).
    pub const NAME: usize = 0;
    /// `AppliedValue`, typically wrapped in `IFCMONETARYMEASURE`.
    pub const APPLIED_VALUE: usize = 2;
}

impl<'m> CostValue<'m> {
    /// Wrap an entity known to be an `IfcCostValue`.
    pub fn new(id: EntityId, entity: &'m Entity) -> Self {
        Self { id, entity }
    }

    /// The entity id in the file.
    pub fn id(&self) -> EntityId {
        self.id
    }

    /// The value's name, e.g. `Estimate`.
    pub fn name(&self) -> Option<&'m str> {
        self.entity.text(slot::NAME)
    }

    /// The monetary amount.
    ///
    /// Unwraps the `IFCMONETARYMEASURE` wrapper: callers want the number, and
    /// requiring each of them to unwrap invites inconsistent handling.
    pub fn amount(&self) -> Option<f64> {
        self.entity
            .attribute(slot::APPLIED_VALUE)?
            .unwrap_typed()
            .as_f64()
    }
}
