//! Horizontal segments: line, arc, spiral transitions.
//!
//!
//! Implementation is tracked in `../PLAN.md`.

//! ## Internal split
//!
//! - `layout.rs`: segment order and continuity.
//! - `segment.rs`: line/arc/transition parameters.

mod layout;
mod segment;

mod transition;
