//! Borrowed `IfcLibraryInformation` projection.
use crate::view::{
    borrowed_entity, optional_ref, optional_text, required_text, ClassificationView,
};
use crate::ClassificationResult;
borrowed_entity!(LibraryInformation, "IFCLIBRARYINFORMATION");
impl<'m> LibraryInformation<'m> {
    pub fn name(self) -> ClassificationResult<&'m str> {
        required_text("IFCLIBRARYINFORMATION", self.id(), self.entity(), 0, "Name")
    }
    pub fn version(self) -> ClassificationResult<Option<&'m str>> {
        optional_text(
            "IFCLIBRARYINFORMATION",
            self.id(),
            self.entity(),
            1,
            "Version",
        )
    }
    pub fn publisher_id(self) -> ClassificationResult<Option<ifc_model::EntityId>> {
        optional_ref(
            "IFCLIBRARYINFORMATION",
            self.id(),
            self.entity(),
            2,
            "Publisher",
        )
    }
    pub fn version_date(self) -> ClassificationResult<Option<&'m str>> {
        optional_text(
            "IFCLIBRARYINFORMATION",
            self.id(),
            self.entity(),
            3,
            "VersionDate",
        )
    }
    pub fn location(self) -> ClassificationResult<Option<&'m str>> {
        optional_text(
            "IFCLIBRARYINFORMATION",
            self.id(),
            self.entity(),
            4,
            "Location",
        )
    }
    pub fn description(self) -> ClassificationResult<Option<&'m str>> {
        optional_text(
            "IFCLIBRARYINFORMATION",
            self.id(),
            self.entity(),
            5,
            "Description",
        )
    }
}
impl<'m> ClassificationView<'m> {
    pub fn libraries(self) -> impl Iterator<Item = LibraryInformation<'m>> + 'm {
        self.model()
            .of_type("IFCLIBRARYINFORMATION")
            .map(|(id, e)| LibraryInformation::from_known(id, e))
    }
}
