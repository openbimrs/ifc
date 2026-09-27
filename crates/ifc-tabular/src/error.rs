//! Refusals raised while staging tabular records, and read failures.

use ifc_model::EntityId;
use thiserror::Error;

/// Why a tabular record was refused.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum TabularError {
    /// A row carries a different cell count than the first row (WR1).
    ///
    /// Ragged tables parse and render; the damage appears only when a
    /// reader indexes a column the short row does not have.
    #[error("{entity}: row {row} has {found} cells, the first row has {expected}")]
    RaggedRow {
        /// The entity being staged.
        entity: &'static str,
        /// Zero-based index of the offending row.
        row: usize,
        /// Cells in the first row.
        expected: usize,
        /// Cells in this row.
        found: usize,
    },
    /// More than one row is marked as a heading (WR2).
    #[error("{entity}: {found} heading rows, at most one is allowed")]
    TooManyHeadings {
        /// The entity being staged.
        entity: &'static str,
        /// How many rows claimed to be headings.
        found: usize,
    },
    /// A required list was empty.
    ///
    /// The schema spells these `LIST [1:?]`, so an empty list is not a
    /// sparse record but a malformed one.
    #[error("{entity}.{attribute} requires at least one entry")]
    EmptyList {
        /// The entity being staged.
        entity: &'static str,
        /// The attribute that came up empty.
        attribute: &'static str,
    },
    /// A required label was absent or whitespace-only.
    ///
    /// A blank string satisfies EXISTS while naming nothing, so it is
    /// refused rather than written.
    #[error("{entity}.{attribute} requires a non-blank value")]
    BlankRequired {
        /// The entity being staged.
        entity: &'static str,
        /// The attribute that was blank.
        attribute: &'static str,
    },
    /// A numeric attribute was not finite.
    #[error("{entity}.{attribute} requires a finite value")]
    NotFinite {
        /// The entity being staged.
        entity: &'static str,
        /// The offending attribute.
        attribute: &'static str,
    },
}

/// Result alias for tabular staging.
pub type TabularResult<T> = Result<T, TabularError>;

/// Why a read could not produce a view at all.
///
/// Defects INSIDE a table or series are not errors: they are reported as
/// [`crate::TabularIssue`]s on the view, beside what did decode.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum TabularReadError {
    /// No record has this id.
    #[error("entity {id} does not exist")]
    UnknownEntity {
        /// The requested id.
        id: EntityId,
    },
    /// The record is not the kind the read asked for.
    #[error("{id} is {actual}, expected {expected}")]
    WrongEntityType {
        /// The requested id.
        id: EntityId,
        /// The type or types the read accepts.
        expected: &'static str,
        /// The record's type.
        actual: String,
    },
    /// The declared schema is not one this view decodes.
    #[error("{schema} is not supported; tabular reads decode IFC4 and IFC4X3")]
    UnsupportedSchema {
        /// The schema's name.
        schema: String,
    },
}

/// Result alias for tabular reads.
pub type TabularReadResult<T> = Result<T, TabularReadError>;
