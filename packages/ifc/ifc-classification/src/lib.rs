//! `ifc-classification` — classification, documents, and external references.
//!
//! # Why this is its own crate
//!
//! IFC4 has **12 entities** here, and they carry disproportionate practical
//! weight: national delivery specifications are written in terms of
//! classification (Uniclass, OmniClass, DIN 276, IFC-SB), and the single most
//! common model-checking question in practice is *"is every element
//! classified?"*.
//!
//! # Scope
//!
//! - `IfcClassification` and reference hierarchies
//! - Document information, references, and relationships
//! - Library information and references
//! - The `IfcExternalReferenceRelationship` association machinery
//!
//! # Pitfall
//!
//! Classification arrives both as a *reference* (an identifier plus a source)
//! and, in imported models, sometimes flattened into a property set. A checker
//! that only looks at one of the two under-reports.
