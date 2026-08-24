//! Type buckets and reverse-reference indices.
//!
//! `all_of_type("IfcWall")` and 'who references me' must both be O(1)-ish; a
//! linear scan over millions of entities per query is the naive trap.
//!
//! Implementation is tracked in `../PLAN.md`.

//! ## Internal split
//!
//! - `type_index.rs`: existing type-name lookup ownership.
//! - `reverse.rs`: target-to-referrer and slot reverse index.
//! - `builder.rs`: derived index construction and rebuild.

mod builder;
mod reverse;
mod type_index;
