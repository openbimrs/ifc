//! Mutation: checked edits to entities already in the model.
//!
//! ## Internal split
//!
//! - `edit.rs`: schema-agnostic edit operations (`set_attribute`, `retype`,
//!   `remove`) — the mutable counterpart to `insert`/`push`.
//! - `transaction.rs`: preflight and atomic commit.
//! - `conflict.rs`: ID/reference/index conflict diagnostics.
//!
//! - `step.rs`: crate-private primitive writes stored as their own inverse.
//! - `record.rs`: `begin_recording` / `end_recording` on `Model`.
//! - `inverse.rs`: `Inverse`, the recorded undo of a run of writes.
//! - `history.rs`: `History`, a bounded undo and redo stack of inverses.
//!
//! `edit.rs` and `record.rs` add inherent `Model` methods; `transaction.rs`
//! owns the batched authoring contract. The public items are re-exported at
//! the crate root.

mod conflict;
mod edit;
mod history;
mod inverse;
mod record;
mod step;
mod transaction;

pub use conflict::Conflict;
pub use history::{History, HistoryError};
pub use inverse::{Inverse, InverseError, Restored};
pub(crate) use record::Segment;
pub(crate) use step::Step;
pub use transaction::{Applied, Edit, Transaction};
