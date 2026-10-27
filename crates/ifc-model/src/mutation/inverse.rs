//! The recorded inverse of a run of model writes, applied exactly.
//!
//! # Why not an inverse transaction
//!
//! A `Create` appends to the end of file order and of its type bucket, so an
//! inverse built from the staged edits could never put a removed entity back
//! where it was, nor restore the id allocator. An [`Inverse`] therefore holds
//! the primitive steps the model itself recorded (`step.rs`), including those
//! positions. Applying one restores file order, type-index order, attribute
//! counts, the header and the next free id, so a codec writes the same bytes.
//!
//! # Revisions
//!
//! The revision is never restored: an undo is a new change and bumps it, or a
//! transaction opened before the undo could commit against different state.
//! An inverse therefore applies only to the revision it was recorded up to,
//! and applying it returns the inverse of the application (redo).
//!
//! Revisions are process-unique (see `Model::revision`), so an inverse is
//! also refused by a model that is not the one it was recorded on, or a clone
//! of it that has since diverged. A clone taken at the recorded revision
//! accepts it, which is correct: it holds the same state.

use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};

use ahash::AHashMap;

use crate::model::Model;
use crate::mutation::step::Step;
use crate::value::EntityId;

/// The recorded undo of a run of writes. See the module docs.
#[derive(Debug, Clone, PartialEq)]
pub struct Inverse {
    /// Undoing steps, already in the order they apply.
    steps: Vec<Step>,
    /// `max_id` as it was before the writes.
    max_id: u64,
    /// The revision when recording began.
    before: u64,
    /// The revision the inverse applies to.
    after: u64,
}

/// The net effect of applying an [`Inverse`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct Restored {
    /// Absent before the application, present after.
    pub created: Vec<EntityId>,
    /// Present before the application, absent after.
    pub removed: Vec<EntityId>,
    /// Present both times and touched.
    pub modified: Vec<EntityId>,
    /// Whether the header was replaced.
    pub header: bool,
}

/// Why an [`Inverse`] was not applied.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum InverseError {
    /// The model changed since the inverse was recorded.
    #[error("model moved from revision {expected} to {found} since the change was recorded")]
    StaleRevision {
        /// The revision the inverse applies to.
        expected: u64,
        /// The model's revision.
        found: u64,
    },
}

impl Inverse {
    pub(crate) fn new(steps: Vec<Step>, max_id: u64, before: u64, after: u64) -> Self {
        Self {
            steps,
            max_id,
            before,
            after,
        }
    }

    /// Run `change` on `model` and return its result with the inverse of
    /// every write it made.
    ///
    /// If `change` panics the recording is closed before the panic resumes.
    pub fn record<T>(model: &mut Model, change: impl FnOnce(&mut Model) -> T) -> (T, Inverse) {
        let depth = model.recording_depth();
        model.begin_recording();
        let outcome = catch_unwind(AssertUnwindSafe(|| change(model)));
        // A change that began a recording and did not end it must not make
        // this call return the wrong inverse.
        model.unwind_recording(depth + 1);
        assert!(
            model.recording_depth() == depth + 1,
            "the change ended a recording it did not begin"
        );
        let inverse = model
            .end_recording()
            .expect("the recording opened above is still open");
        match outcome {
            Ok(value) => (value, inverse),
            Err(panic) => resume_unwind(panic),
        }
    }

    /// Whether the writes changed nothing.
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    /// Number of primitive steps.
    pub fn len(&self) -> usize {
        self.steps.len()
    }

    /// The model revision this inverse applies to.
    pub fn revision(&self) -> u64 {
        self.after
    }

    pub(crate) fn before(&self) -> u64 {
        self.before
    }

    /// Re-targets the inverse at `revision`; the caller has checked the model
    /// is at that revision and the entry still describes it.
    pub(crate) fn rebase(&mut self, revision: u64) {
        self.after = revision;
    }

    /// The net effect of applying this inverse to `model`, computed without
    /// applying it.
    #[must_use]
    pub fn summary(&self, model: &Model) -> Restored {
        // id -> (present before, present now), in order of first touch.
        let mut seen: AHashMap<EntityId, (bool, bool)> = AHashMap::new();
        let mut order: Vec<EntityId> = Vec::new();
        let mut restored = Restored::default();
        for step in &self.steps {
            let (id, now) = match step {
                Step::Header(_) => {
                    restored.header = true;
                    continue;
                }
                Step::Remove { id } => (*id, Some(false)),
                Step::Insert { id, .. } => (*id, Some(true)),
                Step::Replace { id, .. } | Step::Slot { id, .. } | Step::Retype { id, .. } => {
                    (*id, None)
                }
            };
            let entry = seen.entry(id).or_insert_with(|| {
                order.push(id);
                let present = model.contains(id);
                (present, present)
            });
            if let Some(now) = now {
                entry.1 = now;
            }
        }
        for id in order {
            match seen[&id] {
                (false, true) => restored.created.push(id),
                (true, false) => restored.removed.push(id),
                (true, true) => restored.modified.push(id),
                (false, false) => {}
            }
        }
        restored
    }

    /// Restore the model to the state before the recorded writes.
    ///
    /// Returns what changed and the inverse of this application, which redoes
    /// the writes. A model that moved on since recording is refused and left
    /// untouched.
    pub fn apply(self, model: &mut Model) -> Result<(Restored, Inverse), InverseError> {
        if model.revision() != self.after {
            return Err(InverseError::StaleRevision {
                expected: self.after,
                found: model.revision(),
            });
        }
        let restored = self.summary(model);
        let before = model.revision();
        let prior_max_id = model.max_id();
        let recording = model.is_recording();
        let mut opposite = Vec::with_capacity(self.steps.len());
        for step in self.steps {
            let undone = step.apply(model);
            if recording {
                model.record(undone.clone());
            }
            opposite.push(undone);
        }
        model.set_max_id(self.max_id);
        if !opposite.is_empty() {
            model.bump_revision();
        }
        opposite.reverse();
        let redo = Inverse::new(opposite, prior_max_id, before, model.revision());
        Ok((restored, redo))
    }
}
