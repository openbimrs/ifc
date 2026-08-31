//! Borrowed `IfcClassification` projection (IFC4 slots 0..6).
use crate::view::{
    borrowed_entity, optional_text, optional_texts, required_text, ClassificationView,
};
use crate::ClassificationResult;
borrowed_entity!(ClassificationSystem, "IFCCLASSIFICATION");
impl<'m> ClassificationSystem<'m> {
    pub fn source(self) -> ClassificationResult<Option<&'m str>> {
        optional_text("IFCCLASSIFICATION", self.id(), self.entity(), 0, "Source")
    }
    pub fn edition(self) -> ClassificationResult<Option<&'m str>> {
        optional_text("IFCCLASSIFICATION", self.id(), self.entity(), 1, "Edition")
    }
    pub fn edition_date(self) -> ClassificationResult<Option<&'m str>> {
        optional_text(
            "IFCCLASSIFICATION",
            self.id(),
            self.entity(),
            2,
            "EditionDate",
        )
    }
    pub fn name(self) -> ClassificationResult<&'m str> {
        required_text("IFCCLASSIFICATION", self.id(), self.entity(), 3, "Name")
    }
    pub fn description(self) -> ClassificationResult<Option<&'m str>> {
        optional_text(
            "IFCCLASSIFICATION",
            self.id(),
            self.entity(),
            4,
            "Description",
        )
    }
    pub fn location(self) -> ClassificationResult<Option<&'m str>> {
        optional_text("IFCCLASSIFICATION", self.id(), self.entity(), 5, "Location")
    }
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
    pub fn systems(self) -> impl Iterator<Item = ClassificationSystem<'m>> + 'm {
        self.model()
            .of_type("IFCCLASSIFICATION")
            .map(|(id, e)| ClassificationSystem::from_known(id, e))
    }
}
