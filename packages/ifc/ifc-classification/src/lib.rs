//! `ifc-classification` -- Classification systems, document references and external libraries.
//!
//!
//! 12 entities in IFC4. This is how Uniclass, OmniClass and national systems
//! attach to elements, which is what most compliance checking keys on.
//!
//! # Module map
//!
//! | Module | Role |
//! |---|---|
//! | `classification` | `IfcClassification` and `IfcClassificationReference` |
//! | `document` | `IfcDocumentInformation` and document references |
//! | `library` | `IfcLibraryInformation` external library links |
//! | `assignment` | Binding a classification or document to objects |
//! | `error` | Why a classification lookup failed |
//!
//! # Status
//!
//! Scaffold -- modules are reserved with intent, not implemented. See
//! `../PLAN.md` for the stage that fills them.

mod assignment;
mod classification;
mod document;
mod error;
mod library;

mod query;
