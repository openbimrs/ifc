//! Borrowed `IfcDocumentInformation` projection, read by attribute name in
//! the bound release.
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
            self.text_slot("Identification")?,
            "Identification",
        )
    }
    /// The required `Name` of the document.
    pub fn name(self) -> ClassificationResult<&'m str> {
        required_text(
            "IFCDOCUMENTINFORMATION",
            self.id(),
            self.entity(),
            self.text_slot("Name")?,
            "Name",
        )
    }
    /// The `Description` of the document, when authored.
    pub fn description(self) -> ClassificationResult<Option<&'m str>> {
        optional_text(
            "IFCDOCUMENTINFORMATION",
            self.id(),
            self.entity(),
            self.text_slot("Description")?,
            "Description",
        )
    }
    /// The `Location` (e.g. a URI) of the document, when authored.
    pub fn location(self) -> ClassificationResult<Option<&'m str>> {
        optional_text(
            "IFCDOCUMENTINFORMATION",
            self.id(),
            self.entity(),
            self.text_slot("Location")?,
            "Location",
        )
    }
    /// The `Purpose` the document serves, when authored.
    pub fn purpose(self) -> ClassificationResult<Option<&'m str>> {
        optional_text(
            "IFCDOCUMENTINFORMATION",
            self.id(),
            self.entity(),
            self.text_slot("Purpose")?,
            "Purpose",
        )
    }
    /// The `IntendedUse` of the document, when authored.
    pub fn intended_use(self) -> ClassificationResult<Option<&'m str>> {
        optional_text(
            "IFCDOCUMENTINFORMATION",
            self.id(),
            self.entity(),
            self.text_slot("IntendedUse")?,
            "IntendedUse",
        )
    }
    /// The `Scope` of the document, when authored.
    pub fn scope(self) -> ClassificationResult<Option<&'m str>> {
        optional_text(
            "IFCDOCUMENTINFORMATION",
            self.id(),
            self.entity(),
            self.text_slot("Scope")?,
            "Scope",
        )
    }
    /// The `Revision` label of the document, when authored.
    pub fn revision(self) -> ClassificationResult<Option<&'m str>> {
        optional_text(
            "IFCDOCUMENTINFORMATION",
            self.id(),
            self.entity(),
            self.text_slot("Revision")?,
            "Revision",
        )
    }
    /// Id of the `DocumentOwner` (an `IfcActorSelect`), when authored.
    pub fn document_owner_id(self) -> ClassificationResult<Option<ifc_model::EntityId>> {
        optional_ref(
            "IFCDOCUMENTINFORMATION",
            self.id(),
            self.entity(),
            self.slot("DocumentOwner")?,
            "DocumentOwner",
        )
    }
    /// Ids of the `Editors` (`IfcActorSelect` members) who worked on the document, when authored.
    pub fn editors(self) -> ClassificationResult<Option<Vec<ifc_model::EntityId>>> {
        optional_refs(
            "IFCDOCUMENTINFORMATION",
            self.id(),
            self.entity(),
            self.slot("Editors")?,
            "Editors",
        )
    }
    /// The `CreationTime` of the document, when authored.
    pub fn creation_time(self) -> ClassificationResult<Option<&'m str>> {
        optional_text(
            "IFCDOCUMENTINFORMATION",
            self.id(),
            self.entity(),
            self.text_slot("CreationTime")?,
            "CreationTime",
        )
    }
    /// The `LastRevisionTime` of the document, when authored.
    pub fn last_revision_time(self) -> ClassificationResult<Option<&'m str>> {
        optional_text(
            "IFCDOCUMENTINFORMATION",
            self.id(),
            self.entity(),
            self.text_slot("LastRevisionTime")?,
            "LastRevisionTime",
        )
    }
    /// The `ElectronicFormat` (e.g. a MIME type) of the document, when authored.
    pub fn electronic_format(self) -> ClassificationResult<Option<&'m str>> {
        optional_text(
            "IFCDOCUMENTINFORMATION",
            self.id(),
            self.entity(),
            self.text_slot("ElectronicFormat")?,
            "ElectronicFormat",
        )
    }
    /// The `ValidFrom` date of the document, when authored.
    pub fn valid_from(self) -> ClassificationResult<Option<&'m str>> {
        optional_text(
            "IFCDOCUMENTINFORMATION",
            self.id(),
            self.entity(),
            self.text_slot("ValidFrom")?,
            "ValidFrom",
        )
    }
    /// The `ValidUntil` date of the document, when authored.
    pub fn valid_until(self) -> ClassificationResult<Option<&'m str>> {
        optional_text(
            "IFCDOCUMENTINFORMATION",
            self.id(),
            self.entity(),
            self.text_slot("ValidUntil")?,
            "ValidUntil",
        )
    }
    /// The `Confidentiality` enumerator of the document, when authored: one
    /// of the bound release's `IfcDocumentConfidentialityEnum` values
    /// (`PUBLIC`, `RESTRICTED`, `CONFIDENTIAL`, `PERSONAL`, `USERDEFINED`,
    /// `NOTDEFINED` in IFC2X3, IFC4 and IFC4X3).
    pub fn confidentiality(self) -> ClassificationResult<Option<&'m str>> {
        self.enumerated("Confidentiality")
    }
    /// The `Status` enumerator of the document, when authored: one of the
    /// bound release's `IfcDocumentStatusEnum` values (`DRAFT`, `FINALDRAFT`,
    /// `FINAL`, `REVISION`, `NOTDEFINED` in IFC2X3, IFC4 and IFC4X3).
    pub fn status(self) -> ClassificationResult<Option<&'m str>> {
        self.enumerated("Status")
    }
    fn enumerated(self, attribute: &'static str) -> ClassificationResult<Option<&'m str>> {
        const ENTITY: &str = "IFCDOCUMENTINFORMATION";
        let allowed = self.release().enumerators(ENTITY, self.id(), attribute)?;
        optional_enum(
            ENTITY,
            self.id(),
            self.entity(),
            self.slot(attribute)?,
            attribute,
            &allowed,
        )
    }
}
impl<'m> ClassificationView<'m> {
    /// All `IfcDocumentInformation` instances in the model.
    pub fn documents(self) -> impl Iterator<Item = DocumentInformation<'m>> + 'm {
        self.model()
            .of_type("IFCDOCUMENTINFORMATION")
            .map(move |(id, e)| DocumentInformation::from_known(id, e, self.release()))
    }
}
