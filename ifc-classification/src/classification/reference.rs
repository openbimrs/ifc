//! Borrowed `IfcClassificationReference` projection.
use crate::view::{borrowed_entity, optional_ref, optional_text, ClassificationView};
use crate::{ClassificationError, ClassificationResult};
borrowed_entity!(ClassificationReference, "IFCCLASSIFICATIONREFERENCE");
impl<'m> ClassificationReference<'m> {
    /// The `Location` (e.g. a URI) of the referenced classification item, when authored.
    pub fn location(self) -> ClassificationResult<Option<&'m str>> {
        optional_text(
            "IFCCLASSIFICATIONREFERENCE",
            self.id(),
            self.entity(),
            0,
            "Location",
        )
    }
    /// The `Identification` code of the referenced classification item, when authored.
    pub fn identification(self) -> ClassificationResult<Option<&'m str>> {
        optional_text(
            "IFCCLASSIFICATIONREFERENCE",
            self.id(),
            self.entity(),
            1,
            "Identification",
        )
    }
    /// The `Name` of the referenced classification item, when authored.
    pub fn name(self) -> ClassificationResult<Option<&'m str>> {
        optional_text(
            "IFCCLASSIFICATIONREFERENCE",
            self.id(),
            self.entity(),
            2,
            "Name",
        )
    }
    /// Id of the `ReferencedSource` — the parent `IfcClassification` or `IfcClassificationReference` in the hierarchy — when authored.
    pub fn referenced_source_id(self) -> ClassificationResult<Option<ifc_model::EntityId>> {
        optional_ref(
            "IFCCLASSIFICATIONREFERENCE",
            self.id(),
            self.entity(),
            3,
            "ReferencedSource",
        )
    }
    /// The `Description` of the referenced classification item, when authored.
    pub fn description(self) -> ClassificationResult<Option<&'m str>> {
        optional_text(
            "IFCCLASSIFICATIONREFERENCE",
            self.id(),
            self.entity(),
            4,
            "Description",
        )
    }
    /// The `Sort` order of the referenced classification item, when authored.
    pub fn sort(self) -> ClassificationResult<Option<&'m str>> {
        optional_text(
            "IFCCLASSIFICATIONREFERENCE",
            self.id(),
            self.entity(),
            5,
            "Sort",
        )
    }
    /// Checks the `IfcExternalReference.WR1` rule: at least one of `Location`, `Identification`, or `Name` must be stated.
    pub fn validate(self) -> ClassificationResult<()> {
        if self.location()?.is_some() || self.identification()?.is_some() || self.name()?.is_some()
        {
            Ok(())
        } else {
            Err(ClassificationError::InvalidValue {
                entity: "IFCCLASSIFICATIONREFERENCE",
                id: self.id(),
                attribute: "WR1",
                value: "Location, Identification, and Name are all unstated".into(),
            })
        }
    }
}
impl<'m> ClassificationView<'m> {
    /// All `IfcClassificationReference` instances in the model.
    pub fn references(self) -> impl Iterator<Item = ClassificationReference<'m>> + 'm {
        self.model()
            .of_type("IFCCLASSIFICATIONREFERENCE")
            .map(|(id, e)| ClassificationReference::from_known(id, e))
    }
}
