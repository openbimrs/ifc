//! Borrowed `IfcDocumentReference` projection.
use crate::view::{borrowed_entity, optional_ref, optional_text, ClassificationView};
use crate::{ClassificationError, ClassificationResult};
borrowed_entity!(DocumentReference, "IFCDOCUMENTREFERENCE");
impl<'m> DocumentReference<'m> {
    pub fn location(self) -> ClassificationResult<Option<&'m str>> {
        optional_text(
            "IFCDOCUMENTREFERENCE",
            self.id(),
            self.entity(),
            0,
            "Location",
        )
    }
    pub fn identification(self) -> ClassificationResult<Option<&'m str>> {
        optional_text(
            "IFCDOCUMENTREFERENCE",
            self.id(),
            self.entity(),
            1,
            "Identification",
        )
    }
    pub fn name(self) -> ClassificationResult<Option<&'m str>> {
        optional_text("IFCDOCUMENTREFERENCE", self.id(), self.entity(), 2, "Name")
    }
    pub fn description(self) -> ClassificationResult<Option<&'m str>> {
        optional_text(
            "IFCDOCUMENTREFERENCE",
            self.id(),
            self.entity(),
            3,
            "Description",
        )
    }
    pub fn referenced_document_id(self) -> ClassificationResult<Option<ifc_model::EntityId>> {
        optional_ref(
            "IFCDOCUMENTREFERENCE",
            self.id(),
            self.entity(),
            4,
            "ReferencedDocument",
        )
    }
    pub fn validate(self) -> ClassificationResult<()> {
        if self.location()?.is_none() && self.identification()?.is_none() && self.name()?.is_none()
        {
            return Err(ClassificationError::InvalidValue {
                entity: "IFCDOCUMENTREFERENCE",
                id: self.id(),
                attribute: "IfcExternalReference.WR1",
                value: "Location, Identification, and Name are all unstated".into(),
            });
        }
        if self.name()?.is_some() ^ self.referenced_document_id()?.is_some() {
            Ok(())
        } else {
            Err(ClassificationError::InvalidValue {
                entity: "IFCDOCUMENTREFERENCE",
                id: self.id(),
                attribute: "WR1",
                value: "exactly one of Name and ReferencedDocument must be stated".into(),
            })
        }
    }
}
impl<'m> ClassificationView<'m> {
    pub fn document_references(self) -> impl Iterator<Item = DocumentReference<'m>> + 'm {
        self.model()
            .of_type("IFCDOCUMENTREFERENCE")
            .map(|(id, e)| DocumentReference::from_known(id, e))
    }
}
