//! Recording: capture the exact inverse of a run of model writes.
//!
//! While a recording is open, every primitive write (`insert`, `push`,
//! `set_attribute`, `set_attributes`, `retype`, `remove`, `header_mut`) keeps
//! the step that undoes it. Ending the recording yields an [`Inverse`].
//!
//! Recordings nest. The inner recording's inverse is returned to its caller
//! and its steps also join the enclosing recording, so the outer inverse
//! undoes everything the inner one did and more.
//!
//! Outside a recording nothing is captured and a write pays one emptiness
//! check.

use crate::model::Model;
use crate::mutation::inverse::Inverse;
use crate::mutation::step::Step;

/// The writes of one open recording.
#[derive(Debug, Clone)]
pub(crate) struct Segment {
    /// Undoing steps in the order the writes happened.
    pub(crate) steps: Vec<Step>,
    /// `max_id` when the recording began.
    pub(crate) max_id: u64,
    /// The revision when the recording began.
    pub(crate) revision: u64,
}

impl Model {
    /// Start capturing the inverse of every write that follows.
    ///
    /// Nestable: each call needs its own [`Model::end_recording`].
    pub fn begin_recording(&mut self) {
        let segment = Segment {
            steps: Vec::new(),
            max_id: self.max_id(),
            revision: self.revision(),
        };
        self.recording_mut().push(segment);
    }

    /// Stop the innermost recording and return the inverse of its writes,
    /// or `None` when no recording is open.
    ///
    /// The inner steps are also kept by the enclosing recording, if any.
    pub fn end_recording(&mut self) -> Option<Inverse> {
        let segment = self.recording_mut().pop()?;
        if let Some(outer) = self.recording_mut().last_mut() {
            outer.steps.extend(segment.steps.iter().cloned());
        }
        let mut steps = segment.steps;
        steps.reverse();
        Some(Inverse::new(
            steps,
            segment.max_id,
            segment.revision,
            self.revision(),
        ))
    }

    /// Whether a recording is open.
    pub fn is_recording(&self) -> bool {
        !self.recording_is_empty()
    }
}
