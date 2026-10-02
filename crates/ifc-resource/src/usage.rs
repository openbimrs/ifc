//! Resource time, quantity and levelling.
//!
//! ## Internal split
//!
//! - `time.rs`: usage time.
//! - `quantity.rs`: usage quantities.
//! - `measure.rs`: `IfcMeasureWithUnit`, the IFC2X3 `BaseQuantity` form.

mod measure;
mod quantity;
mod time;

use ifc_model::EntityId;

use crate::error::ResourceResult;
use crate::view::ResourceView;

pub use measure::MeasureWithUnit;
pub use quantity::{ComplexQuantity, SimpleQuantity, SimpleQuantityValue};
pub use time::ResourceTime;

impl<'m, 's> ResourceView<'m, 's> {
    /// Projects an `IfcPhysicalSimpleQuantity` by entity id.
    pub fn simple_quantity(&self, id: EntityId) -> ResourceResult<SimpleQuantity<'m, 's>> {
        SimpleQuantity::from_record(self.record(id, "IfcPhysicalSimpleQuantity")?)
    }

    /// Projects an `IfcMeasureWithUnit` by entity id: the IFC2X3
    /// `BaseQuantity` form (see
    /// [`ConstructionResource::base_quantity_measure`](crate::ConstructionResource::base_quantity_measure)).
    pub fn measure_with_unit(&self, id: EntityId) -> ResourceResult<MeasureWithUnit<'m, 's>> {
        Ok(MeasureWithUnit::from_record(
            self.record(id, "IfcMeasureWithUnit")?,
        ))
    }

    /// Projects an `IfcPhysicalComplexQuantity` by entity id.
    pub fn complex_quantity(&self, id: EntityId) -> ResourceResult<ComplexQuantity<'m, 's>> {
        ComplexQuantity::from_record(self.record(id, "IfcPhysicalComplexQuantity")?)
    }
}
