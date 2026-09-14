//! Malformed schedule data and traversal refusals.
//!
//! These describe a file that states something contradictory, not a failure of
//! this crate to read it. They are returned alongside results so one bad task
//! does not hide an otherwise readable schedule.

pub use crate::sequence::SequenceCycle;
pub use crate::task::TaskTimeAnomaly;

/// An authored value rejected before it reached the model.
///
/// Distinct from the anomalies above: those describe a file already
/// written, this describes a draft refused so the file never says it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScheduleAuthoringError {
    /// An attribute value is not valid for its slot.
    InvalidValue {
        /// The entity type being authored.
        entity: &'static str,
        /// The attribute that failed.
        attribute: &'static str,
        /// What was expected instead.
        expected: &'static str,
    },
}

impl std::fmt::Display for ScheduleAuthoringError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidValue {
                entity,
                attribute,
                expected,
            } => write!(f, "{entity}.{attribute}: expected {expected}"),
        }
    }
}

impl std::error::Error for ScheduleAuthoringError {}
