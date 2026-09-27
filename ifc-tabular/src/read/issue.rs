//! What a read found wrong with a table or time series.

use ifc_model::EntityId;

/// One defect found while reading a table or time series.
///
/// A read never drops what it cannot decode: a malformed slot, a dangling
/// reference, or a violated WHERE rule becomes an issue on the returned
/// view, and whatever did decode is still returned beside it. `entity` is
/// always the record the defect sits on, which may be a row, column or
/// value record rather than the container that was asked for.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum TabularIssue {
    /// The record's slot count differs from its schema declaration.
    ///
    /// Slots are still read by position, so a short record reports its
    /// missing required attributes as well.
    Arity {
        /// The record.
        entity: EntityId,
        /// Its STEP type name.
        type_name: &'static str,
        /// Slots the schema declares.
        expected: usize,
        /// Slots the record carries.
        found: usize,
    },
    /// A required attribute is null or absent.
    Missing {
        /// The record.
        entity: EntityId,
        /// The attribute.
        attribute: &'static str,
    },
    /// A slot holds a value its declaration does not admit.
    Malformed {
        /// The record.
        entity: EntityId,
        /// The attribute.
        attribute: &'static str,
        /// The value found, in `Debug` form.
        found: String,
    },
    /// A `LIST [1:?]` attribute is present but empty.
    EmptyList {
        /// The record.
        entity: EntityId,
        /// The attribute.
        attribute: &'static str,
    },
    /// A reference names a record the model does not hold.
    Dangling {
        /// The record holding the reference.
        entity: EntityId,
        /// The attribute.
        attribute: &'static str,
        /// The missing target.
        target: EntityId,
    },
    /// A reference names a record of the wrong type.
    WrongReferenceType {
        /// The record holding the reference.
        entity: EntityId,
        /// The attribute.
        attribute: &'static str,
        /// The target.
        target: EntityId,
        /// The type the declaration requires.
        expected: &'static str,
        /// The type the target has.
        actual: String,
    },
    /// `IfcTable.WR1`: a row's cell count differs from the first row's.
    RaggedRow {
        /// The table.
        table: EntityId,
        /// The offending row.
        row: EntityId,
        /// Its zero-based position in `Rows`.
        index: usize,
        /// Cells in `Rows[1]`.
        expected: usize,
        /// Cells in this row.
        found: usize,
    },
    /// `IfcTable.WR2`: more than one row has `IsHeading` TRUE.
    TooManyHeadings {
        /// The table.
        table: EntityId,
        /// How many rows claim to be the heading.
        found: usize,
    },
}
