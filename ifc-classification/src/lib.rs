//! Borrowed IFC4 classification, document, library and association semantics.
//!
//! Views borrow [`ifc_model::Model`]; authoring helpers stage records on a
//! caller-owned [`ifc_model::Transaction`]. No query performs external I/O.

mod assignment;
mod authoring;
mod classification;
mod document;
mod error;
mod library;
mod query;
mod view;

pub use assignment::{ClassificationAssignment, DocumentAssignment, LibraryAssignment};
pub use authoring::{
    associate_classification, associate_document, associate_library, create_classification,
    create_classification_reference, create_document, create_document_reference, create_library,
    create_library_reference, AssociationDraft, ClassificationDraft, ClassificationReferenceDraft,
    DocumentDraft, DocumentReferenceDraft, LibraryDraft, LibraryReferenceDraft,
};
pub use classification::{ClassificationReference, ClassificationSystem};
pub use document::{DocumentInformation, DocumentReference};
pub use error::{ClassificationError, ClassificationResult};
pub use library::{LibraryInformation, LibraryReference};
pub use query::{ClassificationHierarchy, EffectiveClassifications};
pub use view::ClassificationView;
