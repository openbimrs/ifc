//! Borrowed `IfcDocumentInformation` projection (IFC4 slots 0..16).
use crate::view::{
    borrowed_entity, optional_enum, optional_ref, optional_refs, optional_text, required_text,
    ClassificationView,
};
use crate::ClassificationResult;
borrowed_entity!(DocumentInformation, "IFCDOCUMENTINFORMATION");
impl<'m> DocumentInformation<'m> {
    /// The required `Identification` code of the document.
    pub fn identification(self) -> ClassificationResult<&'m str> {
        required_text(
            "IFCDOCUMENTINFORMATION",
            self.id(),
            self.entity(),
            0,
            "Identification",
        )
    }
    /// The required `Name` of the document.
    pub fn name(self) -> ClassificationResult<&'m str> {
        required_text(
            "IFCDOCUMENTINFORMATION",
            self.id(),
            self.entity(),
            1,
            "Name",
        )
    }
    /// The `Description` of the document, when authored.
    pub fn description(self) -> ClassificationResult<Option<&'m str>> {
        optional_text(
            "IFCDOCUMENTINFORMATION",
            self.id(),
            self.entity(),
            2,
            "Description",
        )
    }
    /// The `Location` (e.g. a URI) of the document, when authored.
    pub fn location(self) -> ClassificationResult<Option<&'m str>> {
        optional_text(
            "IFCDOCUMENTINFORMATION",
            self.id(),
            self.entity(),
            3,
            "Location",
        )
    }
    /// The `Purpose` the document serves, when authored.
    pub fn purpose(self) -> ClassificationResult<Option<&'m str>> {
        optional_text(
            "IFCDOCUMENTINFORMATION",
            self.id(),
            self.entity(),
            4,
            "Purpose",
        )
    }
    /// The `IntendedUse` of the document, when authored.
    pub fn intended_use(self) -> ClassificationResult<Option<&'m str>> {
        optional_text(
            "IFCDOCUMENTINFORMATION",
            self.id(),
            self.entity(),
            5,
            "IntendedUse",
        )
    }
    /// The `Scope` of the document, when authored.
    pub fn scope(self) -> ClassificationResult<Option<&'m str>> {
        optional_text(
            "IFCDOCUMENTINFORMATION",
            self.id(),
            self.entity(),
            6,
            "Scope",
        )
    }
    /// The `Revision` label of the document, when authored.
    pub fn revision(self) -> ClassificationResult<Option<&'m str>> {
        optional_text(
            "IFCDOCUMENTINFORMATION",
            self.id(),
            self.entity(),
            7,
            "Revision",
        )
    }
    /// Id of the `DocumentOwner` (an `IfcActorSelect`), when authored.
    pub fn document_owner_id(self) -> ClassificationResult<Option<ifc_model::EntityId>> {
        optional_ref(
            "IFCDOCUMENTINFORMATION",
            self.id(),
            self.entity(),
            8,
            "DocumentOwner",
        )
    }
    /// Ids of the `Editors` (`IfcActorSelect` members) who worked on the document, when authored.
    pub fn editors(self) -> ClassificationResult<Option<Vec<ifc_model::EntityId>>> {
        optional_refs(
            "IFCDOCUMENTINFORMATION",
            self.id(),
            self.entity(),
            9,
            "Editors",
        )
    }
    /// The `CreationTime` of the document, when authored.
    pub fn creation_time(self) -> ClassificationResult<Option<&'m str>> {
        optional_text(
            "IFCDOCUMENTINFORMATION",
            self.id(),
            self.entity(),
            10,
            "CreationTime",
        )
    }
    /// The `LastRevisionTime` of the document, when authored.
    pub fn last_revision_time(self) -> ClassificationResult<Option<&'m str>> {
        optional_text(
            "IFCDOCUMENTINFORMATION",
            self.id(),
            self.entity(),
            11,
            "LastRevisionTime",
        )
    }
    /// The `ElectronicFormat` (e.g. a MIME type) of the document, when authored.
    pub fn electronic_format(self) -> ClassificationResult<Option<&'m str>> {
        optional_text(
            "IFCDOCUMENTINFORMATION",
            self.id(),
            self.entity(),
            12,
            "ElectronicFormat",
        )
    }
    /// The `ValidFrom` date of the document, when authored.
    pub fn valid_from(self) -> ClassificationResult<Option<&'m str>> {
        optional_text(
            "IFCDOCUMENTINFORMATION",
            self.id(),
            self.entity(),
            13,
            "ValidFrom",
        )
    }
    /// The `ValidUntil` date of the document, when authored.
    pub fn valid_until(self) -> ClassificationResult<Option<&'m str>> {
        optional_text(
            "IFCDOCUMENTINFORMATION",
            self.id(),
            self.entity(),
            14,
            "ValidUntil",
        )
    }
    /// The `Confidentiality` enumerator of the document, when authored (`PUBLIC`, `RESTRICTED`, `CONFIDENTIAL`, `PERSONAL`, `USERDEFINED`, or `NOTDEFINED`).
    pub fn confidentiality(self) -> ClassificationResult<Option<&'m str>> {
        optional_enum(
            "IFCDOCUMENTINFORMATION",
            self.id(),
            self.entity(),
            15,
            "Confidentiality",
            &[
                "PUBLIC",
                "RESTRICTED",
                "CONFIDENTIAL",
                "PERSONAL",
                "USERDEFINED",
                "NOTDEFINED",
            ],
        )
    }
    /// The `Status` enumerator of the document, when authored (`DRAFT`, `FINAL`, `REVISION`, or `NOTDEFINED`).
    pub fn status(self) -> ClassificationResult<Option<&'m str>> {
        optional_enum(
            "IFCDOCUMENTINFORMATION",
            self.id(),
            self.entity(),
            16,
            "Status",
            &["DRAFT", "FINAL", "REVISION", "NOTDEFINED"],
        )
    }
}
impl<'m> ClassificationView<'m> {
    /// All `IfcDocumentInformation` instances in the model.
    pub fn documents(self) -> impl Iterator<Item = DocumentInformation<'m>> + 'm {
        self.model()
            .of_type("IFCDOCUMENTINFORMATION")
            .map(|(id, e)| DocumentInformation::from_known(id, e))
    }
}
