//! Borrowed IFC2X3/IFC4/IFC4X3 classification, document, library and association semantics.
//!
//! Views borrow [`ifc_model::Model`] and read every record against the
//! release its header declares (see [`classification_schema`]): IFC2X3 TC1,
//! IFC4 ADD2 TC1 or IFC4X3 ADD2. Slot positions, selects, domains and
//! enumerations come from that release's bundled table, looked up by
//! attribute name. A header declaring several schemas, or one with no
//! bundled table, is refused rather than read as IFC4; a header declaring
//! none (an in-memory model) binds IFC4.
//!
//! Authoring helpers stage records on a caller-owned
//! [`ifc_model::Transaction`] in the same release's layout, refusing values
//! and entities the release cannot hold. No query performs external I/O.

mod assignment;
mod authoring;
mod classification;
mod document;
mod error;
mod external_relationship;
mod library;
mod query;
mod release;
mod view;

pub use assignment::{ClassificationAssignment, DocumentAssignment, LibraryAssignment};
pub use authoring::{
    associate_classification, associate_classification_with_owner_history, associate_document,
    associate_document_with_owner_history, associate_library, associate_library_with_owner_history,
    create_classification, create_classification_in, create_classification_reference,
    create_document, create_document_reference, create_library, create_library_reference,
    relate_documents, AssociationDraft, ClassificationDraft, ClassificationReferenceDraft,
    DocumentDraft, DocumentReferenceDraft, LibraryDraft, LibraryReferenceDraft,
};
pub use classification::{ClassificationNotation, ClassificationReference, ClassificationSystem};
pub use document::{DocumentInformation, DocumentReference};
pub use error::{ClassificationError, ClassificationResult};
pub use external_relationship::{
    create_external_reference_relationship, ExternalReferenceRelationship,
    ExternalReferenceRelationshipDraft,
};
/// The IFC release a classification read binds to (re-exported from `ifc-schema`).
pub use ifc_schema::SchemaVersion;
pub use library::{LibraryInformation, LibraryReference};
pub use query::{ClassificationHierarchy, EffectiveClassifications};
pub use release::classification_schema;
pub use view::ClassificationView;
