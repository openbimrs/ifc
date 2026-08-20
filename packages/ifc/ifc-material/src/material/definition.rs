//! Borrowed `IfcMaterial` identity.

use crate::view::{borrowed_entity, optional_text, required_text, MaterialView};
use crate::MaterialResult;

borrowed_entity!(Material, "IFCMATERIAL");

impl<'m> Material<'m> {
    pub fn name(self) -> MaterialResult<&'m str> {
        required_text("IFCMATERIAL", self.id(), self.entity(), 0, "Name")
    }

    pub fn description(self) -> MaterialResult<Option<&'m str>> {
        optional_text("IFCMATERIAL", self.id(), self.entity(), 1, "Description")
    }

    pub fn category(self) -> MaterialResult<Option<&'m str>> {
        optional_text("IFCMATERIAL", self.id(), self.entity(), 2, "Category")
    }
}

impl<'m> MaterialView<'m> {
    pub fn materials(self) -> impl Iterator<Item = Material<'m>> + 'm {
        self.model()
            .of_type("IFCMATERIAL")
            .map(|(id, entity)| Material::from_known(id, entity))
    }
}
