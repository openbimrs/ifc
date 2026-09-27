//! Borrowed `IfcMaterial` identity.

use crate::view::{borrowed_entity, optional_text, required_text, MaterialView};
use crate::MaterialResult;

borrowed_entity!(Material, "IFCMATERIAL");

impl<'m> Material<'m> {
    /// `IfcMaterial.Name`. Required; a malformed record is an error, not
    /// an absent value. Declared by every release.
    pub fn name(self) -> MaterialResult<&'m str> {
        required_text(
            "IFCMATERIAL",
            self.id(),
            self.entity(),
            self.slot("Name")?,
            "Name",
        )
    }

    /// `IfcMaterial.Description`, if given.
    ///
    /// # Errors
    ///
    /// [`crate::MaterialError::NotInSchema`] for IFC2X3, whose `IfcMaterial`
    /// declares `Name` only.
    pub fn description(self) -> MaterialResult<Option<&'m str>> {
        optional_text(
            "IFCMATERIAL",
            self.id(),
            self.entity(),
            self.slot("Description")?,
            "Description",
        )
    }

    /// `IfcMaterial.Category`, if given.
    ///
    /// # Errors
    ///
    /// [`crate::MaterialError::NotInSchema`] for IFC2X3, whose `IfcMaterial`
    /// declares `Name` only.
    pub fn category(self) -> MaterialResult<Option<&'m str>> {
        optional_text(
            "IFCMATERIAL",
            self.id(),
            self.entity(),
            self.slot("Category")?,
            "Category",
        )
    }
}

impl<'m> MaterialView<'m> {
    /// Iterates every `IfcMaterial` instance in the model.
    pub fn materials(self) -> impl Iterator<Item = Material<'m>> + 'm {
        let release = self.release();
        self.model()
            .of_type("IFCMATERIAL")
            .map(move |(id, entity)| Material::from_known(id, entity, release))
    }
}
