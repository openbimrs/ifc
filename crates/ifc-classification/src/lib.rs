//! Borrowed IFC2X3/IFC4 classification, document, library and association semantics.
//!
//! Views borrow [`ifc_model::Model`] and read every record against the
//! release its header declares (see [`classification_schema`]): slot
//! positions, selects and domains come from that release's bundled table.
//! Authoring helpers stage IFC4 records on a caller-owned
//! [`ifc_model::Transaction`]. No query performs external I/O.

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
    associate_classification, associate_document, associate_library, create_classification,
    create_classification_reference, create_document, create_document_reference, create_library,
    create_library_reference, relate_documents, AssociationDraft, ClassificationDraft,
    ClassificationReferenceDraft, DocumentDraft, DocumentReferenceDraft, LibraryDraft,
    LibraryReferenceDraft,
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
