//! Record-aligned partitioning for the parallel scan.
//!
//! # The pitfall this module exists to prevent
//!
//! Splitting a STEP file at arbitrary byte offsets corrupts the parse: an
//! offset can land inside a quoted string that contains `;`, or mid-record.
//! Partition boundaries must be **resynced forward to the next record start**
//! (a `#` followed by digits then `=`, at a position not inside a string).
//!
//! The validation for this is a total-count check: parsing with 1 partition
//! and with N partitions must yield identical entity counts. That test is
//! cheap and catches every misalignment.
//!
//! Not yet implemented -- Stage 1 in `../PLAN.md`.

/// A byte range covering whole records, safe to scan independently.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Partition {
    /// Inclusive start offset, aligned to a record start.
    pub start: usize,
    /// Exclusive end offset, aligned to a record start (or EOF).
    pub end: usize,
}
