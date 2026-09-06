//! Resource time, quantity and levelling.
//!
//! ## Internal split
//!
//! - `time.rs`: usage time.
//! - `quantity.rs`: usage quantities.

mod quantity;
mod time;

use ifc_model::EntityId;

use crate::error::ResourceResult;
use crate::view::ResourceView;

pub use quantity::{ComplexQuantity, SimpleQuantity, SimpleQuantityValue};
pub use time::ResourceTime;

impl<'m, 's> ResourceView<'m, 's> {
    pub fn simple_quantity(&self, id: EntityId) -> ResourceResult<SimpleQuantity<'m, 's>> {
        SimpleQuantity::from_record(self.record(id, "IfcPhysicalSimpleQuantity")?)
    }

    pub fn complex_quantity(&self, id: EntityId) -> ResourceResult<ComplexQuantity<'m, 's>> {
        ComplexQuantity::from_record(self.record(id, "IfcPhysicalComplexQuantity")?)
    }
}
