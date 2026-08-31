//! Borrowed `IfcClassificationReference` projection.
use crate::view::{borrowed_entity, optional_ref, optional_text, ClassificationView};
use crate::{ClassificationError, ClassificationResult};
borrowed_entity!(ClassificationReference, "IFCCLASSIFICATIONREFERENCE");
impl<'m> ClassificationReference<'m> {
    pub fn location(self) -> ClassificationResult<Option<&'m str>> {
        optional_text(
            "IFCCLASSIFICATIONREFERENCE",
            self.id(),
            self.entity(),
            0,
            "Location",
        )
    }
    pub fn identification(self) -> ClassificationResult<Option<&'m str>> {
        optional_text(
            "IFCCLASSIFICATIONREFERENCE",
            self.id(),
            self.entity(),
            1,
            "Identification",
        )
    }
    pub fn name(self) -> ClassificationResult<Option<&'m str>> {
        optional_text(
            "IFCCLASSIFICATIONREFERENCE",
            self.id(),
            self.entity(),
            2,
            "Name",
        )
    }
    pub fn referenced_source_id(self) -> ClassificationResult<Option<ifc_model::EntityId>> {
        optional_ref(
            "IFCCLASSIFICATIONREFERENCE",
            self.id(),
            self.entity(),
            3,
            "ReferencedSource",
        )
    }
    pub fn description(self) -> ClassificationResult<Option<&'m str>> {
        optional_text(
            "IFCCLASSIFICATIONREFERENCE",
            self.id(),
            self.entity(),
            4,
            "Description",
        )
    }
    pub fn sort(self) -> ClassificationResult<Option<&'m str>> {
        optional_text(
            "IFCCLASSIFICATIONREFERENCE",
            self.id(),
            self.entity(),
            5,
            "Sort",
        )
    }
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
    pub fn references(self) -> impl Iterator<Item = ClassificationReference<'m>> + 'm {
        self.model()
            .of_type("IFCCLASSIFICATIONREFERENCE")
            .map(|(id, e)| ClassificationReference::from_known(id, e))
    }
}
