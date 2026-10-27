//! Undo and redo for every mutator (#465).
//!
//! History is off by default: [`IfcModel::set_history_limit`] turns it on
//! with a bound on the number of undo entries. While it is on, each public
//! mutator is one entry: the exact inverse of every primitive write it made,
//! recorded by `ifc::History::track`. [`IfcModel::undo`] restores the model
//! to the state before the last entry (entities under their ids, at their
//! file positions, the header included) and [`IfcModel::redo`] reverses an
//! undo. A mutator that is refused and changed nothing records nothing; a
//! dry run records nothing.
//!
//! Undoing the creation of an entity frees its id again, so a later call may
//! reuse it. Undo and redo are new changes: the model revision moves on.
//!
//! A change made outside the history (a mutator while the limit was 0, or a
//! write through the facade) makes the stacks stale: `undo` and `redo` are
//! refused with `history-stale` and change nothing, and the next recorded
//! change drops the stale entries. An empty stack is `history-empty`.

use crate::record::{Field, Record, ToRecord};
use crate::{BindingError, IfcModel};

/// How much history a model holds: [`IfcModel::history`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HistoryState {
    /// The bound on undo entries; `0` records nothing.
    pub limit: usize,
    /// Entries [`IfcModel::undo`] can step back through.
    pub undo: usize,
    /// Entries [`IfcModel::redo`] can step forward through.
    pub redo: usize,
}

/// What an undo or redo restores (or would restore): ids in order of
/// first occurrence in the change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryChange {
    /// Entities absent before the application, present after it.
    pub created: Vec<u64>,
    /// Entities present before the application, absent after it.
    pub removed: Vec<u64>,
    /// Entities present both times whose content changed.
    pub modified: Vec<u64>,
    /// Whether the file header changed.
    pub header: bool,
}

impl ToRecord for HistoryState {
    fn to_record(&self) -> Record {
        Record::new(
            "HistoryState",
            vec![
                ("limit", Field::Count(self.limit)),
                ("undo", Field::Count(self.undo)),
                ("redo", Field::Count(self.redo)),
            ],
        )
    }
}

impl ToRecord for HistoryChange {
    fn to_record(&self) -> Record {
        Record::new(
            "HistoryChange",
            vec![
                ("created", Field::ids(self.created.iter().copied())),
                ("removed", Field::ids(self.removed.iter().copied())),
                ("modified", Field::ids(self.modified.iter().copied())),
                ("header", Field::Bool(self.header)),
            ],
        )
    }
}

impl From<ifc::Restored> for HistoryChange {
    fn from(restored: ifc::Restored) -> Self {
        let ids = |ids: Vec<ifc::EntityId>| ids.into_iter().map(|id| id.0).collect();
        Self {
            created: ids(restored.created),
            removed: ids(restored.removed),
            modified: ids(restored.modified),
            header: restored.header,
        }
    }
}

fn refused(error: ifc::HistoryError) -> BindingError {
    match error {
        ifc::HistoryError::NothingToUndo => BindingError::HistoryEmpty("nothing to undo".into()),
        ifc::HistoryError::NothingToRedo => BindingError::HistoryEmpty("nothing to redo".into()),
        ifc::HistoryError::StaleRevision { .. } => BindingError::HistoryStale(error.to_string()),
        other => BindingError::Unsupported(other.to_string()),
    }
}

impl IfcModel {
    /// Run a mutator so that, when history is on, what it changes becomes
    /// one entry. Nested calls (`author` inside `create_entity`) join the
    /// outer entry; a panic drops the history, whose stacks no longer
    /// describe the model.
    pub(crate) fn tracked<T>(&mut self, change: impl FnOnce(&mut Self) -> T) -> T {
        if self.history.limit() == 0 || self.inner.is_recording() {
            return change(self);
        }
        self.inner.begin_recording();
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| change(&mut *self)));
        let inverse = self.inner.end_recording();
        match outcome {
            Ok(value) => {
                if let Some(inverse) = inverse {
                    self.history.push(inverse);
                }
                value
            }
            Err(panic) => {
                self.history.clear();
                std::panic::resume_unwind(panic)
            }
        }
    }

    /// Bound the undo entries kept; `0` turns history off and drops what
    /// is held. Lowering the limit drops the oldest entries.
    pub fn set_history_limit(&mut self, limit: usize) {
        self.history.set_limit(limit);
        if limit == 0 {
            self.history.clear();
        }
    }

    /// The history bound and how many entries each direction holds.
    pub fn history(&self) -> HistoryState {
        HistoryState {
            limit: self.history.limit(),
            undo: self.history.undo_len(),
            redo: self.history.redo_len(),
        }
    }

    /// What [`Self::undo`] would change, without changing anything.
    ///
    /// Refused with `history-empty` when there is nothing to undo and
    /// `history-stale` when the model changed outside the history.
    pub fn peek_undo(&self) -> Result<HistoryChange, BindingError> {
        self.history
            .peek_undo(&self.inner)
            .map(Into::into)
            .map_err(refused)
    }

    /// What [`Self::redo`] would change, without changing anything.
    pub fn peek_redo(&self) -> Result<HistoryChange, BindingError> {
        self.history
            .peek_redo(&self.inner)
            .map(Into::into)
            .map_err(refused)
    }

    /// Restore the model to the state before the last recorded change.
    /// A refusal changes nothing.
    pub fn undo(&mut self) -> Result<HistoryChange, BindingError> {
        self.history
            .undo(&mut self.inner)
            .map(Into::into)
            .map_err(refused)
    }

    /// Reverse the last [`Self::undo`]. A refusal changes nothing.
    pub fn redo(&mut self) -> Result<HistoryChange, BindingError> {
        self.history
            .redo(&mut self.inner)
            .map(Into::into)
            .map_err(refused)
    }

    /// Drop every undo and redo entry, keeping the limit.
    pub fn clear_history(&mut self) {
        self.history.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::Tagged;

    #[test]
    fn a_write_outside_the_history_makes_it_stale() {
        let mut model = IfcModel::empty();
        model.set_history_limit(4);
        let id = model
            .add("IFCLABEL", vec![Tagged::Text("a".into())])
            .unwrap();
        // A write the history did not see: the facade, not a tracked call.
        model.inner.push(ifc::Entity::new("IFCLABEL", Vec::new()));
        let stale = model.undo().unwrap_err();
        assert_eq!(stale.code(), "history-stale");
        assert_eq!(model.peek_undo().unwrap_err().code(), "history-stale");
        assert_eq!(model.history().undo, 1, "a refusal keeps the entries");
        assert!(model.type_of(id).is_ok());

        // The next recorded change drops the stale entries.
        model
            .add("IFCLABEL", vec![Tagged::Text("b".into())])
            .unwrap();
        assert_eq!(model.history().undo, 1);
        model.undo().unwrap();
        assert_eq!(model.undo().unwrap_err().code(), "history-empty");
    }

    #[test]
    fn a_panic_drops_the_history() {
        let mut model = IfcModel::empty();
        model.set_history_limit(4);
        model
            .add("IFCLABEL", vec![Tagged::Text("a".into())])
            .unwrap();
        let panicked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            model.tracked(|this| {
                this.add("IFCLABEL", vec![Tagged::Text("b".into())])
                    .unwrap();
                panic!("boom");
            })
        }));
        assert!(panicked.is_err());
        assert_eq!(model.history().undo, 0);
        assert!(!model.inner.is_recording());
    }
}
