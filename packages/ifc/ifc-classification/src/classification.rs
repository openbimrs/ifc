//! `IfcClassification` and `IfcClassificationReference`.
//!
//! References nest: a reference may point at a parent reference rather than
//! the classification root, forming a facet path.
//!
//! Implementation is tracked in the adjacent `PLAN.md`.

//! ## Internal split
//!
//! - `system.rs`: IfcClassification.
//! - `reference.rs`: hierarchical references.

mod reference;
mod system;
