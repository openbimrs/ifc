//! Borrowed `IfcClassification` projection (IFC4 slots 0..6).
use crate::view::{
    borrowed_entity, optional_text, optional_texts, required_text, ClassificationView,
};
use crate::ClassificationResult;
borrowed_entity!(ClassificationSystem, "IFCCLASSIFICATION");
impl<'m> ClassificationSystem<'m> {
    /// The `Source` (organization) that publishes this classification system, when authored.
    pub fn source(self) -> ClassificationResult<Option<&'m str>> {
        optional_text("IFCCLASSIFICATION", self.id(), self.entity(), 0, "Source")
    }
    /// The `Edition` label of this classification system, when authored.
    pub fn edition(self) -> ClassificationResult<Option<&'m str>> {
        optional_text("IFCCLASSIFICATION", self.id(), self.entity(), 1, "Edition")
    }
    /// The `EditionDate` of this classification system, when authored.
    pub fn edition_date(self) -> ClassificationResult<Option<&'m str>> {
        optional_text(
            "IFCCLASSIFICATION",
            self.id(),
            self.entity(),
            2,
            "EditionDate",
        )
    }
    /// The required `Name` of this classification system.
    pub fn name(self) -> ClassificationResult<&'m str> {
        required_text("IFCCLASSIFICATION", self.id(), self.entity(), 3, "Name")
    }
    /// The `Description` of this classification system, when authored.
    pub fn description(self) -> ClassificationResult<Option<&'m str>> {
        optional_text(
            "IFCCLASSIFICATION",
            self.id(),
            self.entity(),
            4,
            "Description",
        )
    }
    /// The `Location` (e.g. a URI) of this classification system, when authored.
    pub fn location(self) -> ClassificationResult<Option<&'m str>> {
        optional_text("IFCCLASSIFICATION", self.id(), self.entity(), 5, "Location")
    }
    /// The `ReferenceTokens` used to compose classification identifications, when authored.
    pub fn reference_tokens(self) -> ClassificationResult<Option<Vec<&'m str>>> {
        optional_texts(
            "IFCCLASSIFICATION",
            self.id(),
            self.entity(),
            6,
            "ReferenceTokens",
        )
    }
}
impl<'m> ClassificationView<'m> {
    /// All `IfcClassification` instances in the model.
    pub fn systems(self) -> impl Iterator<Item = ClassificationSystem<'m>> + 'm {
        self.model()
            .of_type("IFCCLASSIFICATION")
            .map(|(id, e)| ClassificationSystem::from_known(id, e))
    }
}
