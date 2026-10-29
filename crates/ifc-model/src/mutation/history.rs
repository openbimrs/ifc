//! A bounded undo and redo history over [`Inverse`]s.
//!
//! One entry is one tracked change: a batch of authoring operations, a
//! property edit, a single `set_attribute`. [`History::track`] runs the
//! change and keeps its inverse; [`History::undo`] and [`History::redo`]
//! move entries between the two stacks.
//!
//! # Staleness
//!
//! The history is valid for one model revision. Anything that changes the
//! model outside the history (an untracked write) makes [`History::undo`] and
//! [`History::redo`] return [`HistoryError::StaleRevision`] and change
//! nothing, because the entries no longer describe the model. The next
//! [`History::push`] or [`History::track`] whose change began at another
//! revision clears both stacks first.
//!
//! The bound counts entries, not bytes: removed entities stay in memory until
//! their entry is dropped.
//!
//! A [`Model::preview`] is not a change; the history stays valid across it.
//!
//! ```
//! use ifc_model::{Entity, History, Model};
//!
//! let mut model = Model::new();
//! let mut history = History::new(8);
//! let id = history.track(&mut model, |m| m.push(Entity::new("IFCWALL", vec![])));
//! assert!(model.contains(id));
//!
//! history.undo(&mut model).unwrap();
//! assert!(model.is_empty());
//! history.redo(&mut model).unwrap();
//! assert!(model.contains(id));
//! ```

use std::collections::VecDeque;

use crate::model::Model;
use crate::mutation::inverse::{Inverse, Restored};

/// A bounded undo and redo history. See the module docs.
#[derive(Debug, Clone, Default)]
pub struct History {
    undo: VecDeque<Inverse>,
    redo: Vec<Inverse>,
    limit: usize,
    /// The model revision both stacks describe; `Some` exactly when a stack
    /// is not empty.
    revision: Option<u64>,
}

/// Why an undo or redo did not happen.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum HistoryError {
    /// The undo stack is empty.
    #[error("nothing to undo")]
    NothingToUndo,
    /// The redo stack is empty.
    #[error("nothing to redo")]
    NothingToRedo,
    /// The model changed outside the history.
    #[error("model moved from revision {expected} to {found} outside the history")]
    StaleRevision {
        /// The revision the history describes.
        expected: u64,
        /// The model's revision.
        found: u64,
    },
}

impl History {
    /// A history keeping at most `limit` undo entries; 0 disables it.
    #[must_use]
    pub fn new(limit: usize) -> Self {
        Self {
            limit,
            ..Self::default()
        }
    }

    /// The most undo entries kept.
    #[must_use]
    pub fn limit(&self) -> usize {
        self.limit
    }

    /// Change the bound, dropping the oldest undo entries and the farthest
    /// redo entries beyond it.
    pub fn set_limit(&mut self, limit: usize) {
        self.limit = limit;
        self.trim();
    }

    /// Entries [`History::undo`] can step back through.
    #[must_use]
    pub fn undo_len(&self) -> usize {
        self.undo.len()
    }

    /// Entries [`History::redo`] can step forward through.
    #[must_use]
    pub fn redo_len(&self) -> usize {
        self.redo.len()
    }

    /// Whether [`History::undo`] has an entry.
    #[must_use]
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    /// Whether [`History::redo`] has an entry.
    #[must_use]
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// Drop every entry.
    pub fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
        self.revision = None;
    }

    /// Keep `inverse` as the newest undo entry and clear redo.
    ///
    /// Ignored when the history is disabled or the inverse is empty. If the
    /// inverse began at a revision the history does not describe, the old
    /// entries are dropped first.
    pub fn push(&mut self, inverse: Inverse) {
        if self.limit == 0 || inverse.is_empty() {
            return;
        }
        if self.revision.is_some_and(|r| r != inverse.before()) {
            self.clear();
        }
        self.redo.clear();
        self.revision = Some(inverse.revision());
        self.undo.push_back(inverse);
        self.trim();
    }

    /// Run `change` on `model` and keep the inverse of what it wrote as one
    /// entry.
    ///
    /// Whatever `change` wrote is kept whether it returned `Ok` or `Err`;
    /// nothing is rolled back. With a limit of 0 the change runs unrecorded.
    pub fn track<T>(&mut self, model: &mut Model, change: impl FnOnce(&mut Model) -> T) -> T {
        if self.limit == 0 {
            return change(model);
        }
        let (value, inverse) = Inverse::record(model, change);
        self.push(inverse);
        value
    }

    /// What [`History::undo`] would change, without changing it.
    pub fn peek_undo(&self, model: &Model) -> Result<Restored, HistoryError> {
        let entry = self.undo.back().ok_or(HistoryError::NothingToUndo)?;
        self.check(model)?;
        Ok(entry.summary(model))
    }

    /// What [`History::redo`] would change, without changing it.
    pub fn peek_redo(&self, model: &Model) -> Result<Restored, HistoryError> {
        let entry = self.redo.last().ok_or(HistoryError::NothingToRedo)?;
        self.check(model)?;
        Ok(entry.summary(model))
    }

    /// Undo the newest entry.
    pub fn undo(&mut self, model: &mut Model) -> Result<Restored, HistoryError> {
        if self.undo.is_empty() {
            return Err(HistoryError::NothingToUndo);
        }
        self.check(model)?;
        let mut entry = self.undo.pop_back().expect("checked non-empty");
        entry.rebase(model.revision());
        let (restored, redo) = entry
            .apply(model)
            .expect("the entry was rebased to the current revision");
        if !redo.is_empty() {
            self.redo.push(redo);
        }
        self.settle(model);
        Ok(restored)
    }

    /// Redo the most recently undone entry.
    pub fn redo(&mut self, model: &mut Model) -> Result<Restored, HistoryError> {
        if self.redo.is_empty() {
            return Err(HistoryError::NothingToRedo);
        }
        self.check(model)?;
        let mut entry = self.redo.pop().expect("checked non-empty");
        entry.rebase(model.revision());
        let (restored, undo) = entry
            .apply(model)
            .expect("the entry was rebased to the current revision");
        if !undo.is_empty() {
            self.undo.push_back(undo);
        }
        self.settle(model);
        Ok(restored)
    }

    fn check(&self, model: &Model) -> Result<(), HistoryError> {
        match self.revision {
            Some(expected) if expected != model.revision() => Err(HistoryError::StaleRevision {
                expected,
                found: model.revision(),
            }),
            _ => Ok(()),
        }
    }

    /// After a move between the stacks: follow the model's new revision.
    fn settle(&mut self, model: &Model) {
        self.trim();
        if self.undo.is_empty() && self.redo.is_empty() {
            self.revision = None;
        } else {
            self.revision = Some(model.revision());
        }
    }

    fn trim(&mut self) {
        while self.undo.len() > self.limit {
            self.undo.pop_front();
        }
        if self.redo.len() > self.limit {
            let excess = self.redo.len() - self.limit;
            self.redo.drain(..excess);
        }
        if self.undo.is_empty() && self.redo.is_empty() {
            self.revision = None;
        }
    }
}
