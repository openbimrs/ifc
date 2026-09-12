//! Borrowed `IfcLibraryReference` projection.
use crate::view::{borrowed_entity, optional_ref, optional_text, ClassificationView};
use crate::{ClassificationError, ClassificationResult};
borrowed_entity!(LibraryReference, "IFCLIBRARYREFERENCE");
impl<'m> LibraryReference<'m> {
    /// The `Location` (e.g. a URI) of the referenced library item, when authored.
    pub fn location(self) -> ClassificationResult<Option<&'m str>> {
        optional_text(
            "IFCLIBRARYREFERENCE",
            self.id(),
            self.entity(),
            0,
            "Location",
        )
    }
    /// The `Identification` code of the referenced library item, when authored.
    pub fn identification(self) -> ClassificationResult<Option<&'m str>> {
        optional_text(
            "IFCLIBRARYREFERENCE",
            self.id(),
            self.entity(),
            1,
            "Identification",
        )
    }
    /// The `Name` of the referenced library item, when authored.
    pub fn name(self) -> ClassificationResult<Option<&'m str>> {
        optional_text("IFCLIBRARYREFERENCE", self.id(), self.entity(), 2, "Name")
    }
    /// The `Description` of the referenced library item, when authored.
    pub fn description(self) -> ClassificationResult<Option<&'m str>> {
        optional_text(
            "IFCLIBRARYREFERENCE",
            self.id(),
            self.entity(),
            3,
            "Description",
        )
    }
    /// The `Language` of the referenced library item, when authored.
    pub fn language(self) -> ClassificationResult<Option<&'m str>> {
        optional_text(
            "IFCLIBRARYREFERENCE",
            self.id(),
            self.entity(),
            4,
            "Language",
        )
    }
    /// Id of the `ReferencedLibrary` (an `IfcLibraryInformation`), when authored.
    pub fn referenced_library_id(self) -> ClassificationResult<Option<ifc_model::EntityId>> {
        optional_ref(
            "IFCLIBRARYREFERENCE",
            self.id(),
            self.entity(),
            5,
            "ReferencedLibrary",
        )
    }
    /// Checks the `IfcExternalReference.WR1` rule: at least one of `Location`, `Identification`, or `Name` must be stated.
    pub fn validate(self) -> ClassificationResult<()> {
        if self.location()?.is_some() || self.identification()?.is_some() || self.name()?.is_some()
        {
            Ok(())
        } else {
            Err(ClassificationError::InvalidValue {
                entity: "IFCLIBRARYREFERENCE",
                id: self.id(),
                attribute: "WR1",
                value: "Location, Identification, and Name are all unstated".into(),
            })
        }
    }
}
impl<'m> ClassificationView<'m> {
    /// All `IfcLibraryReference` instances in the model.
    pub fn library_references(self) -> impl Iterator<Item = LibraryReference<'m>> + 'm {
        self.model()
            .of_type("IFCLIBRARYREFERENCE")
            .map(|(id, e)| LibraryReference::from_known(id, e))
    }
}
