//! Refusals raised while staging tabular records.

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
