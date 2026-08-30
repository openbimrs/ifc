//! Malformed schedule data and traversal refusals.
//!
//! These describe a file that states something contradictory, not a failure of
//! this crate to read it. They are returned alongside results so one bad task
//! does not hide an otherwise readable schedule.

pub use crate::sequence::SequenceCycle;
pub use crate::task::TaskTimeAnomaly;
