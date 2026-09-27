//! What a transaction refuses to do, and why.
//!
//! # Conflicts are found BEFORE anything is written
//!
//! Every variant here is produced during preflight, against a projected view
//! of what the model would look like if the whole transaction applied. None
//! of them can be raised mid-apply, because apply runs only once preflight
//! returned clean -- which is what makes the commit atomic without needing an
//! undo log.

use crate::value::EntityId;

/// A reason a transaction cannot be committed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Conflict {
    /// The model changed since the transaction was opened.
    ///
    /// Optimistic concurrency: two editors read the same model, both plan
    /// edits, and the second to commit is working from a view that no longer
    /// exists. Rejecting is the only safe answer -- the second editor's
    /// preflight was computed against state that has since moved.
    StaleRevision {
        /// The revision the transaction was opened against.
        expected: u64,
        /// The revision the model is actually at.
        found: u64,
    },
    /// An edit names an entity that does not exist and is not being created.
    MissingTarget {
        /// Position of the offending edit within the transaction.
        edit: usize,
        /// The id it named.
        id: EntityId,
    },
    /// An insert would overwrite an entity that already exists.
    ///
    /// [`crate::Model::insert`] deliberately replaces, because a codec
    /// re-reading a file must be able to. A transaction refuses instead: an
    /// author who did not mean to destroy an entity gets told, rather than
    /// discovering it later in a diff.
    IdAlreadyExists {
        /// Position of the offending edit.
        edit: usize,
        /// The id it tried to occupy.
        id: EntityId,
    },
    /// An edit writes a reference to an entity that will not exist.
    ///
    /// Checked against the PROJECTED model, so referencing an entity the same
    /// transaction creates is fine, and referencing one it removes is not.
    DanglingReference {
        /// Position of the offending edit.
        edit: usize,
        /// The entity that would hold the bad reference.
        from: EntityId,
        /// The attribute slot it would sit in.
        slot: usize,
        /// The target that would not exist.
        target: EntityId,
    },
    /// A removal would leave a surviving entity pointing at nothing.
    ///
    /// [`crate::Model::remove`] permits this and documents it; a transaction
    /// does not. Deleting a storey that walls still reference produces a file
    /// that parses and is wrong, which is worse than a refused edit.
    ///
    /// The fix is to include the referrers' updates in the same transaction,
    /// which is exactly what the projected check allows.
    RemovalWouldDangle {
        /// Position of the offending removal.
        edit: usize,
        /// The entity being removed.
        removed: EntityId,
        /// A surviving entity that still references it.
        referrer: EntityId,
        /// The slot the surviving reference sits in.
        slot: usize,
    },
}

impl std::fmt::Display for Conflict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::StaleRevision { expected, found } => write!(
                f,
                "model moved from revision {expected} to {found} since the transaction opened"
            ),
            Self::MissingTarget { edit, id } => {
                write!(f, "edit {edit} names #{} which does not exist", id.0)
            }
            Self::IdAlreadyExists { edit, id } => {
                write!(f, "edit {edit} would overwrite existing #{}", id.0)
            }
            Self::DanglingReference {
                edit,
                from,
                slot,
                target,
            } => write!(
                f,
                "edit {edit}: #{}[{slot}] would reference #{} which will not exist",
                from.0, target.0
            ),
            Self::RemovalWouldDangle {
                edit,
                removed,
                referrer,
                slot,
            } => write!(
                f,
                "edit {edit}: removing #{} leaves #{}[{slot}] dangling",
                removed.0, referrer.0
            ),
        }
    }
}

impl std::error::Error for Conflict {}
