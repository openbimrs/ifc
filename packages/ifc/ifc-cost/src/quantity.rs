//! Quantities a cost is computed against.
//!
//! `IfcCostItem.CostQuantities` points at `IfcPhysicalQuantity` subtypes
//! (`IfcQuantityVolume`, `IfcQuantityArea`, ...). The generic reading lives
//! here; the full quantity model belongs to `ifc-properties`, and this module
//! deliberately does not duplicate it.

use ifc_model::{Entity, EntityId};

/// A borrowed view of a physical quantity referenced by a cost item.
#[derive(Debug, Clone, Copy)]
pub struct CostQuantity<'m> {
    id: EntityId,
    entity: &'m Entity,
}

impl<'m> CostQuantity<'m> {
    /// Wrap a quantity entity.
    pub fn new(id: EntityId, entity: &'m Entity) -> Self {
        Self { id, entity }
    }

    /// The entity id in the file.
    pub fn id(&self) -> EntityId {
        self.id
    }

    /// The quantity name, always attribute 0 on `IfcPhysicalSimpleQuantity`.
    pub fn name(&self) -> Option<&'m str> {
        self.entity.text(0)
    }

    /// The numeric value.
    ///
    /// The slot differs per subtype (`IfcQuantityVolume.VolumeValue` is 3,
    /// `IfcQuantityCount.CountValue` is 3 as well), so this scans for the
    /// first numeric attribute after the unit rather than hard-coding a
    /// position per type.
    pub fn value(&self) -> Option<f64> {
        self.entity
            .attributes
            .iter()
            .skip(2)
            .find_map(|v| v.unwrap_typed().as_f64())
    }
}
