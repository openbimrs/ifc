//! Borrowed `IfcDocumentReference` projection.
use crate::view::{borrowed_entity, optional_ref, optional_text, ClassificationView};
use crate::{ClassificationError, ClassificationResult};
borrowed_entity!(DocumentReference, "IFCDOCUMENTREFERENCE");
impl<'m> DocumentReference<'m> {
    /// The `Location` (e.g. a URI) of the referenced document, when authored.
    pub fn location(self) -> ClassificationResult<Option<&'m str>> {
        optional_text(
            "IFCDOCUMENTREFERENCE",
            self.id(),
            self.entity(),
            0,
            "Location",
        )
    }
    /// The `Identification` code of the referenced document, when authored.
    pub fn identification(self) -> ClassificationResult<Option<&'m str>> {
        optional_text(
            "IFCDOCUMENTREFERENCE",
            self.id(),
            self.entity(),
            1,
            "Identification",
        )
    }
    /// The `Name` of the referenced document; mutually exclusive with `referenced_document_id`.
    pub fn name(self) -> ClassificationResult<Option<&'m str>> {
        optional_text("IFCDOCUMENTREFERENCE", self.id(), self.entity(), 2, "Name")
    }
    /// The `Description` of the referenced document, when authored.
    pub fn description(self) -> ClassificationResult<Option<&'m str>> {
        optional_text(
            "IFCDOCUMENTREFERENCE",
            self.id(),
            self.entity(),
            3,
            "Description",
        )
    }
    /// Id of the `ReferencedDocument` (an `IfcDocumentInformation`); mutually exclusive with `name`.
    pub fn referenced_document_id(self) -> ClassificationResult<Option<ifc_model::EntityId>> {
        optional_ref(
            "IFCDOCUMENTREFERENCE",
            self.id(),
            self.entity(),
            4,
            "ReferencedDocument",
        )
    }
    /// Checks `IfcExternalReference.WR1` (at least one of `Location`, `Identification`, `Name` stated) and the local `WR1` rule that exactly one of `Name` and `ReferencedDocument` is stated.
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
    /// All `IfcDocumentReference` instances in the model.
    pub fn document_references(self) -> impl Iterator<Item = DocumentReference<'m>> + 'm {
        self.model()
            .of_type("IFCDOCUMENTREFERENCE")
            .map(|(id, e)| DocumentReference::from_known(id, e))
    }
}
