//! Borrowed `IfcMaterial` identity.

use crate::view::{borrowed_entity, optional_text, required_text, MaterialView};
use crate::MaterialResult;

borrowed_entity!(Material, "IFCMATERIAL");

impl<'m> Material<'m> {
    /// `IfcMaterial.Name`. Required; a malformed record is an error, not
    /// an absent value.
    pub fn name(self) -> MaterialResult<&'m str> {
        required_text("IFCMATERIAL", self.id(), self.entity(), 0, "Name")
    }

    /// `IfcMaterial.Description`, if given.
    pub fn description(self) -> MaterialResult<Option<&'m str>> {
        optional_text("IFCMATERIAL", self.id(), self.entity(), 1, "Description")
    }

    /// `IfcMaterial.Category`, if given.
    pub fn category(self) -> MaterialResult<Option<&'m str>> {
        optional_text("IFCMATERIAL", self.id(), self.entity(), 2, "Category")
    }
}

impl<'m> MaterialView<'m> {
    /// Iterates every `IfcMaterial` instance in the model.
    pub fn materials(self) -> impl Iterator<Item = Material<'m>> + 'm {
        self.model()
            .of_type("IFCMATERIAL")
            .map(|(id, entity)| Material::from_known(id, entity))
    }
}
