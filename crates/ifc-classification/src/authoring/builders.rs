//! Constructors and builder setters for the drafts of the parent module,
//! kept apart so the writer stays under the 800-line limit.

#[allow(clippy::wildcard_imports)]
use super::*;

impl<'a> ClassificationDraft<'a> {
    /// Starts a draft from its required fields; every other field is unset.
    #[must_use]
    pub fn new(name: &'a str) -> Self {
        Self {
            source: None,
            edition: None,
            edition_date: None,
            name,
            description: None,
            location: None,
            reference_tokens: None,
        }
    }

    /// Sets `source`: `Source` publishing organization, when stated. Required
    /// by IFC2X3.
    #[must_use]
    pub fn source(mut self, value: &'a str) -> Self {
        self.source = Some(value);
        self
    }

    /// Sets `edition`: `Edition` identifier of the classification, when stated.
    /// Required by IFC2X3.
    #[must_use]
    pub fn edition(mut self, value: &'a str) -> Self {
        self.edition = Some(value);
        self
    }

    /// Sets `edition_date`: `EditionDate` (IFC date string), when stated.
    /// IFC2X3 types it as an `IfcCalendarDate` record, so text is refused
    /// there.
    #[must_use]
    pub fn edition_date(mut self, value: &'a str) -> Self {
        self.edition_date = Some(value);
        self
    }

    /// Sets `description`: `Description` of the classification, when stated.
    /// IFC4 onwards.
    #[must_use]
    pub fn description(mut self, value: &'a str) -> Self {
        self.description = Some(value);
        self
    }

    /// Sets `location`: `Location` (URI) of the classification, when stated:
    /// written to `Specification` in IFC4X3. IFC4 onwards.
    #[must_use]
    pub fn location(mut self, value: &'a str) -> Self {
        self.location = Some(value);
        self
    }

    /// Sets `reference_tokens`: `ReferenceTokens` delimiter set; must be non-
    /// empty when given. IFC4 onwards.
    #[must_use]
    pub fn reference_tokens(mut self, value: &'a [&'a str]) -> Self {
        self.reference_tokens = Some(value);
        self
    }
}

impl<'a> ClassificationReferenceDraft<'a> {
    /// Starts an empty draft; every field is unset.
    #[must_use]
    pub fn new() -> Self {
        Self {
            location: None,
            identification: None,
            name: None,
            referenced_source: None,
            description: None,
            sort: None,
        }
    }

    /// Sets `location`: `Location` (URI) of the reference; at least one of
    /// location/identification/name must be given.
    #[must_use]
    pub fn location(mut self, value: &'a str) -> Self {
        self.location = Some(value);
        self
    }

    /// Sets `identification`: `Identification` code within the classification,
    /// when stated (`ItemReference` in IFC2X3).
    #[must_use]
    pub fn identification(mut self, value: &'a str) -> Self {
        self.identification = Some(value);
        self
    }

    /// Sets `name`: `Name` of the referenced item, when stated.
    #[must_use]
    pub fn name(mut self, value: &'a str) -> Self {
        self.name = Some(value);
        self
    }

    /// Sets `referenced_source`: `ReferencedSource`: the parent
    /// `IfcClassification` or `IfcClassificationReference` (only an
    /// `IfcClassification` in IFC2X3), when stated.
    #[must_use]
    pub fn referenced_source(mut self, value: EntityId) -> Self {
        self.referenced_source = Some(value);
        self
    }

    /// Sets `description`: `Description` of the reference, when stated. IFC4
    /// onwards.
    #[must_use]
    pub fn description(mut self, value: &'a str) -> Self {
        self.description = Some(value);
        self
    }

    /// Sets `sort`: `Sort` order token, when stated. IFC4 onwards.
    #[must_use]
    pub fn sort(mut self, value: &'a str) -> Self {
        self.sort = Some(value);
        self
    }
}

impl<'a> DocumentDraft<'a> {
    /// Starts a draft from its required fields; every other field is unset.
    #[must_use]
    pub fn new(identification: &'a str, name: &'a str) -> Self {
        Self {
            identification,
            name,
            description: None,
            location: None,
            purpose: None,
            intended_use: None,
            scope: None,
            revision: None,
            document_owner: None,
            editors: None,
            creation_time: None,
            last_revision_time: None,
            electronic_format: None,
            valid_from: None,
            valid_until: None,
            confidentiality: None,
            status: None,
        }
    }

    /// Sets `description`: `Description` of the document, when stated.
    #[must_use]
    pub fn description(mut self, value: &'a str) -> Self {
        self.description = Some(value);
        self
    }

    /// Sets `location`: `Location` (URI) of the document, when stated. IFC4
    /// onwards.
    #[must_use]
    pub fn location(mut self, value: &'a str) -> Self {
        self.location = Some(value);
        self
    }

    /// Sets `purpose`: `Purpose` of the document, when stated.
    #[must_use]
    pub fn purpose(mut self, value: &'a str) -> Self {
        self.purpose = Some(value);
        self
    }

    /// Sets `intended_use`: `IntendedUse` of the document, when stated.
    #[must_use]
    pub fn intended_use(mut self, value: &'a str) -> Self {
        self.intended_use = Some(value);
        self
    }

    /// Sets `scope`: `Scope` of the document, when stated.
    #[must_use]
    pub fn scope(mut self, value: &'a str) -> Self {
        self.scope = Some(value);
        self
    }

    /// Sets `revision`: `Revision` identifier, when stated.
    #[must_use]
    pub fn revision(mut self, value: &'a str) -> Self {
        self.revision = Some(value);
        self
    }

    /// Sets `document_owner`: `DocumentOwner`: an `IfcActorSelect`, when
    /// stated.
    #[must_use]
    pub fn document_owner(mut self, value: EntityId) -> Self {
        self.document_owner = Some(value);
        self
    }

    /// Sets `editors`: `Editors`: non-empty unique set of `IfcActorSelect` ids,
    /// when stated.
    #[must_use]
    pub fn editors(mut self, value: &'a [EntityId]) -> Self {
        self.editors = Some(value);
        self
    }

    /// Sets `creation_time`: `CreationTime` (IFC date-time string), when
    /// stated. IFC2X3 types it as an `IfcDateAndTime` record, so text is
    /// refused there.
    #[must_use]
    pub fn creation_time(mut self, value: &'a str) -> Self {
        self.creation_time = Some(value);
        self
    }

    /// Sets `last_revision_time`: `LastRevisionTime` (IFC date-time string),
    /// when stated. A record in IFC2X3, as `creation_time`.
    #[must_use]
    pub fn last_revision_time(mut self, value: &'a str) -> Self {
        self.last_revision_time = Some(value);
        self
    }

    /// Sets `electronic_format`: `ElectronicFormat`, when stated. IFC2X3 types
    /// it as an `IfcDocumentElectronicFormat` record, so text is refused there.
    #[must_use]
    pub fn electronic_format(mut self, value: &'a str) -> Self {
        self.electronic_format = Some(value);
        self
    }

    /// Sets `valid_from`: `ValidFrom` (IFC date string), when stated. An
    /// `IfcCalendarDate` record in IFC2X3.
    #[must_use]
    pub fn valid_from(mut self, value: &'a str) -> Self {
        self.valid_from = Some(value);
        self
    }

    /// Sets `valid_until`: `ValidUntil` (IFC date string), when stated. An
    /// `IfcCalendarDate` record in IFC2X3.
    #[must_use]
    pub fn valid_until(mut self, value: &'a str) -> Self {
        self.valid_until = Some(value);
        self
    }

    /// Sets `confidentiality`: `Confidentiality` enumerator; must be one of the
    /// release's `IfcDocumentConfidentialityEnum` values.
    #[must_use]
    pub fn confidentiality(mut self, value: &'a str) -> Self {
        self.confidentiality = Some(value);
        self
    }

    /// Sets `status`: `Status` enumerator; must be one of the release's
    /// `IfcDocumentStatusEnum` values.
    #[must_use]
    pub fn status(mut self, value: &'a str) -> Self {
        self.status = Some(value);
        self
    }
}

impl<'a> DocumentReferenceDraft<'a> {
    /// Starts an empty draft; every field is unset.
    #[must_use]
    pub fn new() -> Self {
        Self {
            location: None,
            identification: None,
            name: None,
            description: None,
            referenced_document: None,
        }
    }

    /// Sets `location`: `Location` (URI) of the reference; at least one of
    /// location/identification/name must be given.
    #[must_use]
    pub fn location(mut self, value: &'a str) -> Self {
        self.location = Some(value);
        self
    }

    /// Sets `identification`: `Identification` code, when stated
    /// (`ItemReference` in IFC2X3).
    #[must_use]
    pub fn identification(mut self, value: &'a str) -> Self {
        self.identification = Some(value);
        self
    }

    /// Sets `name`: `Name`; exactly one of `name` and `referenced_document`
    /// must be given.
    #[must_use]
    pub fn name(mut self, value: &'a str) -> Self {
        self.name = Some(value);
        self
    }

    /// Sets `description`: `Description` of the reference, when stated. IFC4
    /// onwards.
    #[must_use]
    pub fn description(mut self, value: &'a str) -> Self {
        self.description = Some(value);
        self
    }

    /// Sets `referenced_document`: `ReferencedDocument`: an
    /// `IfcDocumentInformation`; exactly one of `name` and this must be given.
    /// IFC4 onwards: IFC2X3 links the other way, so there `name` is required.
    #[must_use]
    pub fn referenced_document(mut self, value: EntityId) -> Self {
        self.referenced_document = Some(value);
        self
    }
}

impl<'a> LibraryDraft<'a> {
    /// Starts a draft from its required fields; every other field is unset.
    #[must_use]
    pub fn new(name: &'a str) -> Self {
        Self {
            name,
            version: None,
            publisher: None,
            version_date: None,
            location: None,
            description: None,
        }
    }

    /// Sets `version`: `Version` identifier, when stated.
    #[must_use]
    pub fn version(mut self, value: &'a str) -> Self {
        self.version = Some(value);
        self
    }

    /// Sets `publisher`: `Publisher`: an `IfcActorSelect` (only an
    /// `IfcOrganization` in IFC2X3), when stated.
    #[must_use]
    pub fn publisher(mut self, value: EntityId) -> Self {
        self.publisher = Some(value);
        self
    }

    /// Sets `version_date`: `VersionDate` (IFC date-time string), when stated.
    /// An `IfcCalendarDate` record in IFC2X3.
    #[must_use]
    pub fn version_date(mut self, value: &'a str) -> Self {
        self.version_date = Some(value);
        self
    }

    /// Sets `location`: `Location` (URI) of the library, when stated. IFC4
    /// onwards.
    #[must_use]
    pub fn location(mut self, value: &'a str) -> Self {
        self.location = Some(value);
        self
    }

    /// Sets `description`: `Description` of the library, when stated. IFC4
    /// onwards.
    #[must_use]
    pub fn description(mut self, value: &'a str) -> Self {
        self.description = Some(value);
        self
    }
}

impl<'a> LibraryReferenceDraft<'a> {
    /// Starts an empty draft; every field is unset.
    #[must_use]
    pub fn new() -> Self {
        Self {
            location: None,
            identification: None,
            name: None,
            description: None,
            language: None,
            referenced_library: None,
        }
    }

    /// Sets `location`: `Location` (URI) of the reference; at least one of
    /// location/identification/name must be given.
    #[must_use]
    pub fn location(mut self, value: &'a str) -> Self {
        self.location = Some(value);
        self
    }

    /// Sets `identification`: `Identification` code within the library, when
    /// stated (`ItemReference` in IFC2X3).
    #[must_use]
    pub fn identification(mut self, value: &'a str) -> Self {
        self.identification = Some(value);
        self
    }

    /// Sets `name`: `Name` of the referenced item, when stated.
    #[must_use]
    pub fn name(mut self, value: &'a str) -> Self {
        self.name = Some(value);
        self
    }

    /// Sets `description`: `Description` of the reference, when stated. IFC4
    /// onwards.
    #[must_use]
    pub fn description(mut self, value: &'a str) -> Self {
        self.description = Some(value);
        self
    }

    /// Sets `language`: `Language` of the referenced content, when stated. IFC4
    /// onwards.
    #[must_use]
    pub fn language(mut self, value: &'a str) -> Self {
        self.language = Some(value);
        self
    }

    /// Sets `referenced_library`: `ReferencedLibrary`: an
    /// `IfcLibraryInformation`, when stated. IFC4 onwards.
    #[must_use]
    pub fn referenced_library(mut self, value: EntityId) -> Self {
        self.referenced_library = Some(value);
        self
    }
}

impl<'a> AssociationDraft<'a> {
    /// Starts a draft from its required fields; every other field is unset.
    #[must_use]
    pub fn new(global_id: &'a str, related_objects: &'a [EntityId]) -> Self {
        Self {
            global_id,
            name: None,
            description: None,
            related_objects,
        }
    }

    /// Sets `name`: `Name` of the relationship, when stated.
    #[must_use]
    pub fn name(mut self, value: &'a str) -> Self {
        self.name = Some(value);
        self
    }

    /// Sets `description`: `Description` of the relationship, when stated.
    #[must_use]
    pub fn description(mut self, value: &'a str) -> Self {
        self.description = Some(value);
        self
    }
}
