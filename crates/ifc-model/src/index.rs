//! Type buckets and reverse-reference indices.
//!
//! `ids_of_type` is served by an index built during insertion, because "every
//! entity of this type" is the most common query any consumer makes. The
//! reverse index is built on demand instead -- see `reverse.rs` for why.
//!
//! ## Internal split
//!
//! - `reverse.rs`: target-to-referrer and slot reverse index.

mod reverse;

pub use reverse::{Referrer, ReverseIndex};
