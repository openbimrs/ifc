//! Preview: run a change, look at it, and put the model back exactly.
//!
//! A preview records the change like [`Inverse::record`](crate::Inverse::record)
//! does, then applies the recorded steps backwards instead of returning them.
//! Entities, file order, type-bucket order, attribute counts, header, id
//! allocation and diagnostics are put back, the maintained indexes follow
//! through the same steps, and so does the revision.
//!
//! ## Why restoring the revision is sound
//!
//! Revisions are process-unique (ADR 0022): each names one state of one model
//! lineage. The preview puts back exactly the state revision `R` named, so
//! `R` names exactly one state again. The revisions drawn inside the change
//! are never reissued (the process-wide counter is not touched), so a cache
//! keyed by one of them can only miss, never hit a different state. An
//! `Inverse`, `Transaction` or `History` made before the preview targets `R`
//! and stays valid; one made inside the change after a write targets a
//! revision nothing has any more and is refused as stale.
//!
//! The rollback never records: the enclosing recording, if any, gains no
//! steps, because the writes and their undoing cancel.

use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};

use crate::model::Model;

impl Model {
    /// Run `change` on this model, return its result, and put the model back
    /// exactly as it was, revision included.
    ///
    /// Use it to look at a change without making it, for example to mesh what
    /// a pointer move would create. Unlike [`Inverse::apply`](crate::Inverse::apply),
    /// which is a new change and draws a new revision, a preview leaves no
    /// trace: a [`History`](crate::History), [`Transaction`](crate::Transaction)
    /// or [`Inverse`](crate::Inverse) made before it stays valid.
    ///
    /// The cost is what the change writes, not the size of the model. Put
    /// back are the entities and their order, the type buckets, the header,
    /// the id allocator, the diagnostics, the maintained indexes and the
    /// revision. The revisions drawn inside the change are never handed out
    /// again, and the ids it created are free again. Effects outside the
    /// model are not rolled back, and a `Transaction` or `Inverse` made
    /// inside the change after a write is stale afterwards.
    ///
    /// Previews nest, and may run inside [`Inverse::record`](crate::Inverse::record)
    /// or `History::track`, whose recording keeps none of it.
    ///
    /// # Panics
    ///
    /// If `change` panics the model is rolled back and the revision restored
    /// before the panic resumes. Panics if `change` ended or replaced the
    /// recording the preview began (the model is then left as `change` left
    /// it). If a rollback step itself panics (only through a broken
    /// [`EntitySource`](crate::EntitySource)) the original payload is lost
    /// and the model is left partly rolled back.
    ///
    /// # Examples
    ///
    /// ```
    /// use ifc_model::{Entity, History, Model};
    ///
    /// let mut model = Model::new();
    /// let mut history = History::new(8);
    /// history.track(&mut model, |m| {
    ///     m.push(Entity::new("IFCWALL", Vec::new()));
    /// });
    /// history.undo(&mut model).unwrap();
    /// let revision = model.revision();
    ///
    /// let seen = model.preview(|m| {
    ///     m.push(Entity::new("IFCWALL", Vec::new()));
    ///     m.len()
    /// });
    /// assert_eq!(seen, 1);
    /// assert!(model.is_empty());
    /// assert_eq!(model.revision(), revision);
    /// assert!(history.redo(&mut model).is_ok());
    /// ```
    pub fn preview<R>(&mut self, change: impl FnOnce(&mut Model) -> R) -> R {
        let depth = self.recording_depth();
        let revision = self.revision();
        let diagnostics = self.diagnostics().len();
        self.begin_recording();
        let outcome = catch_unwind(AssertUnwindSafe(|| change(self)));
        self.unwind_recording(depth + 1);
        assert!(
            self.recording_depth() == depth + 1
                && self
                    .recording_mut()
                    .last()
                    .is_some_and(|segment| segment.revision == revision),
            "the change ended a recording it did not begin"
        );
        let segment = self.recording_mut().pop().expect("checked above");
        // Not merged into an enclosing recording: the writes and their
        // rollback cancel.
        for step in segment.steps.into_iter().rev() {
            let _ = step.apply(self);
        }
        self.set_max_id(segment.max_id);
        self.restore_revision(segment.revision);
        self.truncate_diagnostics(diagnostics);
        match outcome {
            Ok(value) => value,
            Err(panic) => resume_unwind(panic),
        }
    }
}
