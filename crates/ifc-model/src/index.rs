//! Type buckets and reverse-reference indices.
//!
//! `ids_of_type` is served by an index built during insertion, because "every
//! entity of this type" is the most common query any consumer makes. The
//! reverse index is built on demand instead -- see `reverse.rs` for why.
//!
//! ## Internal split
//!
//! - `reverse.rs`: target-to-referrer and slot reverse index.
//! - `maintained.rs`: the reverse and GlobalId indexes a model keeps once
//!   built, and the write footprints that update them.

mod maintained;
mod reverse;

pub(crate) use maintained::{Derived, Footprint};
pub use reverse::{Referrer, ReverseIndex};
