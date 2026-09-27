//! Mutation: checked edits to entities already in the model.
//!
//! ## Internal split
//!
//! - `edit.rs`: schema-agnostic edit operations (`set_attribute`, `retype`,
//!   `remove`) — the mutable counterpart to `insert`/`push`.
//! - `transaction.rs`: preflight and atomic commit.
//! - `conflict.rs`: ID/reference/index conflict diagnostics.
//!
//! `edit.rs` adds inherent `Model` methods; `transaction.rs` owns the batched
//! authoring contract. Both are re-exported at the crate root.

mod conflict;
mod edit;
mod transaction;

pub use conflict::Conflict;
pub use transaction::{Applied, Edit, Transaction};
