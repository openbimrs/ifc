//! Built element and distribution occurrence classes.
//!
//! A type definition says what a pump *is*; an occurrence says that
//! one particular pump sits here, in this system, with this tag. This
//! crate authors the occurrence side and enforces the link between
//! the two: the schema pairs each occurrence class with exactly one
//! type class, and a mismatch is refused rather than written.
//!
//! The catalogue in [`table`] is generated from
//! `references/ifc-spec/ifc4x3-add2/IFC4X3_ADD2.exp` by
//! `scripts/gen-occurrences.py`; [`create`] and
//! [`create_with_owner_history`] are the writers. They write the model's
//! declared release, laid out from its own table, not the catalogue's
//! IFC4X3 row.

mod authoring;
mod release;
pub mod table;

pub use authoring::{
    create, create_with_owner_history, OccurrenceDraft, OccurrenceError, OccurrenceResult,
};
