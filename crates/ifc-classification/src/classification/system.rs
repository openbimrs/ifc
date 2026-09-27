//! Borrowed `IfcClassification` projection (IFC4 slots 0..6).
use crate::view::{
    borrowed_entity, optional_text, optional_texts, required_text, ClassificationView,
};
use crate::ClassificationResult;
borrowed_entity!(ClassificationSystem, "IFCCLASSIFICATION");
impl<'m> ClassificationSystem<'m> {
    /// The `Source` (organization) that publishes this classification system, when authored.
    pub fn source(self) -> ClassificationResult<Option<&'m str>> {
        optional_text(
            "IFCCLASSIFICATION",
            self.id(),
            self.entity(),
            self.text_slot("Source")?,
            "Source",
        )
    }
    /// The `Edition` label of this classification system, when authored.
    pub fn edition(self) -> ClassificationResult<Option<&'m str>> {
        optional_text(
            "IFCCLASSIFICATION",
            self.id(),
            self.entity(),
            self.text_slot("Edition")?,
            "Edition",
        )
    }
    /// The `EditionDate` of this classification system, when authored.
    pub fn edition_date(self) -> ClassificationResult<Option<&'m str>> {
        optional_text(
            "IFCCLASSIFICATION",
            self.id(),
            self.entity(),
            self.text_slot("EditionDate")?,
            "EditionDate",
        )
    }
    /// The required `Name` of this classification system.
    pub fn name(self) -> ClassificationResult<&'m str> {
        required_text(
            "IFCCLASSIFICATION",
            self.id(),
            self.entity(),
            self.text_slot("Name")?,
            "Name",
        )
    }
    /// The `Description` of this classification system, when authored.
    pub fn description(self) -> ClassificationResult<Option<&'m str>> {
        optional_text(
            "IFCCLASSIFICATION",
            self.id(),
            self.entity(),
            self.text_slot("Description")?,
            "Description",
        )
    }
    /// The `Location` (e.g. a URI) of this classification system, when authored.
    pub fn location(self) -> ClassificationResult<Option<&'m str>> {
        optional_text(
            "IFCCLASSIFICATION",
            self.id(),
            self.entity(),
            self.text_slot("Location")?,
            "Location",
        )
    }
    /// The `ReferenceTokens` used to compose classification identifications, when authored.
    pub fn reference_tokens(self) -> ClassificationResult<Option<Vec<&'m str>>> {
        optional_texts(
            "IFCCLASSIFICATION",
            self.id(),
            self.entity(),
            self.text_slot("ReferenceTokens")?,
            "ReferenceTokens",
        )
    }
}
impl<'m> ClassificationView<'m> {
    /// All `IfcClassification` instances in the model.
    pub fn systems(self) -> impl Iterator<Item = ClassificationSystem<'m>> + 'm {
        self.model()
            .of_type("IFCCLASSIFICATION")
            .map(move |(id, e)| ClassificationSystem::from_known(id, e, self.release()))
    }
}
