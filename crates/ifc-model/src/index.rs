//! Type buckets and reverse-reference indices.
//!
//! `ids_of_type` is served by an index built during insertion, because "every
//! entity of this type" is the most common query any consumer makes. The
//! reverse index is built on demand instead -- see `reverse.rs` for why.
//!
//! ## Internal split
//!
//! - `type_index.rs`: existing type-name lookup ownership.
//! - `reverse.rs`: target-to-referrer and slot reverse index.
//! - `builder.rs`: derived index construction and rebuild.

mod builder;
mod reverse;
mod type_index;

pub use reverse::{Referrer, ReverseIndex};
