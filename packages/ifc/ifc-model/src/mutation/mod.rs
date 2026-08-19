//! Mutation capability scaffold.

//! ## Internal split
//!
//! - `edit.rs`: schema-agnostic edit operations.
//! - `transaction.rs`: preflight and atomic commit.
//! - `conflict.rs`: ID/reference/index conflict diagnostics.

mod conflict;
mod edit;
mod transaction;
