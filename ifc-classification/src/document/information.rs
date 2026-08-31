//! Borrowed `IfcDocumentInformation` projection (IFC4 slots 0..16).
use crate::view::{
    borrowed_entity, optional_enum, optional_ref, optional_refs, optional_text, required_text,
    ClassificationView,
};
use crate::ClassificationResult;
borrowed_entity!(DocumentInformation, "IFCDOCUMENTINFORMATION");
impl<'m> DocumentInformation<'m> {
    pub fn identification(self) -> ClassificationResult<&'m str> {
        required_text(
            "IFCDOCUMENTINFORMATION",
            self.id(),
            self.entity(),
            0,
            "Identification",
        )
    }
    pub fn name(self) -> ClassificationResult<&'m str> {
        required_text(
            "IFCDOCUMENTINFORMATION",
            self.id(),
            self.entity(),
            1,
            "Name",
        )
    }
    pub fn description(self) -> ClassificationResult<Option<&'m str>> {
        optional_text(
            "IFCDOCUMENTINFORMATION",
            self.id(),
            self.entity(),
            2,
            "Description",
        )
    }
    pub fn location(self) -> ClassificationResult<Option<&'m str>> {
        optional_text(
            "IFCDOCUMENTINFORMATION",
            self.id(),
            self.entity(),
            3,
            "Location",
        )
    }
    pub fn purpose(self) -> ClassificationResult<Option<&'m str>> {
        optional_text(
            "IFCDOCUMENTINFORMATION",
            self.id(),
            self.entity(),
            4,
            "Purpose",
        )
    }
    pub fn intended_use(self) -> ClassificationResult<Option<&'m str>> {
        optional_text(
            "IFCDOCUMENTINFORMATION",
            self.id(),
            self.entity(),
            5,
            "IntendedUse",
        )
    }
    pub fn scope(self) -> ClassificationResult<Option<&'m str>> {
        optional_text(
            "IFCDOCUMENTINFORMATION",
            self.id(),
            self.entity(),
            6,
            "Scope",
        )
    }
    pub fn revision(self) -> ClassificationResult<Option<&'m str>> {
        optional_text(
            "IFCDOCUMENTINFORMATION",
            self.id(),
            self.entity(),
            7,
            "Revision",
        )
    }
    pub fn document_owner_id(self) -> ClassificationResult<Option<ifc_model::EntityId>> {
        optional_ref(
            "IFCDOCUMENTINFORMATION",
            self.id(),
            self.entity(),
            8,
            "DocumentOwner",
        )
    }
    pub fn editors(self) -> ClassificationResult<Option<Vec<ifc_model::EntityId>>> {
        optional_refs(
            "IFCDOCUMENTINFORMATION",
            self.id(),
            self.entity(),
            9,
            "Editors",
        )
    }
    pub fn creation_time(self) -> ClassificationResult<Option<&'m str>> {
        optional_text(
            "IFCDOCUMENTINFORMATION",
            self.id(),
            self.entity(),
            10,
            "CreationTime",
        )
    }
    pub fn last_revision_time(self) -> ClassificationResult<Option<&'m str>> {
        optional_text(
            "IFCDOCUMENTINFORMATION",
            self.id(),
            self.entity(),
            11,
            "LastRevisionTime",
        )
    }
    pub fn electronic_format(self) -> ClassificationResult<Option<&'m str>> {
        optional_text(
            "IFCDOCUMENTINFORMATION",
            self.id(),
            self.entity(),
            12,
            "ElectronicFormat",
        )
    }
    pub fn valid_from(self) -> ClassificationResult<Option<&'m str>> {
        optional_text(
            "IFCDOCUMENTINFORMATION",
            self.id(),
            self.entity(),
            13,
            "ValidFrom",
        )
    }
    pub fn valid_until(self) -> ClassificationResult<Option<&'m str>> {
        optional_text(
            "IFCDOCUMENTINFORMATION",
            self.id(),
            self.entity(),
            14,
            "ValidUntil",
        )
    }
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
    pub fn documents(self) -> impl Iterator<Item = DocumentInformation<'m>> + 'm {
        self.model()
            .of_type("IFCDOCUMENTINFORMATION")
            .map(|(id, e)| DocumentInformation::from_known(id, e))
    }
}
