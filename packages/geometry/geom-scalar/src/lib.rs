#![forbid(unsafe_code)]

//! Portable scalar reference implementation: the correctness oracle (ADR 0012).
//!
//! No intrinsics, no threading, no feature gates. Every optimized backend is
//! validated by differential test against this crate, so it must stay readable
//! and obviously correct in preference to being fast.

pub mod expansion;
pub mod orientation;
pub mod polygon;

pub use expansion::{two_diff, two_product, two_sum};
pub use orientation::{orient2d, orient2d_filter};
pub use polygon::{ring_orientation, signed_area2, triangulate_simple};
