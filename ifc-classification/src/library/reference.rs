//! Borrowed `IfcLibraryReference` projection.
use crate::view::{borrowed_entity, optional_ref, optional_text, ClassificationView};
use crate::{ClassificationError, ClassificationResult};
borrowed_entity!(LibraryReference, "IFCLIBRARYREFERENCE");
impl<'m> LibraryReference<'m> {
    pub fn location(self) -> ClassificationResult<Option<&'m str>> {
        optional_text(
            "IFCLIBRARYREFERENCE",
            self.id(),
            self.entity(),
            0,
            "Location",
        )
    }
    pub fn identification(self) -> ClassificationResult<Option<&'m str>> {
        optional_text(
            "IFCLIBRARYREFERENCE",
            self.id(),
            self.entity(),
            1,
            "Identification",
        )
    }
    pub fn name(self) -> ClassificationResult<Option<&'m str>> {
        optional_text("IFCLIBRARYREFERENCE", self.id(), self.entity(), 2, "Name")
    }
    pub fn description(self) -> ClassificationResult<Option<&'m str>> {
        optional_text(
            "IFCLIBRARYREFERENCE",
            self.id(),
            self.entity(),
            3,
            "Description",
        )
    }
    pub fn language(self) -> ClassificationResult<Option<&'m str>> {
        optional_text(
            "IFCLIBRARYREFERENCE",
            self.id(),
            self.entity(),
            4,
            "Language",
        )
    }
    pub fn referenced_library_id(self) -> ClassificationResult<Option<ifc_model::EntityId>> {
        optional_ref(
            "IFCLIBRARYREFERENCE",
            self.id(),
            self.entity(),
            5,
            "ReferencedLibrary",
        )
    }
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
    pub fn library_references(self) -> impl Iterator<Item = LibraryReference<'m>> + 'm {
        self.model()
            .of_type("IFCLIBRARYREFERENCE")
            .map(|(id, e)| LibraryReference::from_known(id, e))
    }
}
