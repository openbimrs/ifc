//! Mutation: checked edits to entities already in the model.
//!
//! ## Internal split
//!
//! - `edit.rs`: schema-agnostic edit operations (`set_attribute`, `retype`,
//!   `remove`) — the mutable counterpart to `insert`/`push`.
//! - `transaction.rs`: preflight and atomic commit. Planned; not yet owned.
//! - `conflict.rs`: ID/reference/index conflict diagnostics. Planned; not yet
//!   owned.
//!
//! This module is private; its only public surface is the inherent `Model`
//! methods `edit.rs` adds, re-exported at the crate root alongside every
//! other `Model` capability.

mod conflict;
mod edit;
mod transaction;
