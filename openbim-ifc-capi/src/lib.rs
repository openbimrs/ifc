//! Versioned C ABI for the IFC facade (ADR 0013, #38).
//!
//! Every export is prefixed `openbim_ifc_v0_1_` and follows the conventions
//! of Axiolid's C ABI (its ADR 0040), so a native host can use both the same
//! way:
//!
//! - Models are opaque non-zero `u64` handles, never Rust addresses. A stale
//!   or twice-destroyed handle is [`OpenbimIfcStatus::InvalidHandle`].
//! - The caller owns every output buffer. Each such call writes the size it
//!   needs to `out_required` first; a null buffer with capacity 0 is the size
//!   query, and a short buffer is [`OpenbimIfcStatus::BufferTooSmall`]. No
//!   Rust allocation crosses the boundary, so there is no free function.
//! - Every export runs inside `catch_unwind`; a panic becomes
//!   [`OpenbimIfcStatus::Panic`] and never unwinds into C.
//! - C integers are validated as integers, never read as Rust enums.
//!
//! Attribute values cross as a *value tape*: a flat pre-order array of
//! [`OpenbimIfcValueNode`] plus one byte buffer holding every string. See
//! [`tape`] for the encoding, which carries the same kinds as the other
//! bindings so `$`/`*`, `.U.`/`.F.` and typed wrappers stay distinct.
//!
//! This is the one crate in the IFC workspace that uses `unsafe`: reading and
//! writing caller-provided C buffers cannot be done otherwise. Each
//! dereference sits in `buffer` or directly beside its null/length check,
//! with a `SAFETY` comment.

#![deny(unsafe_op_in_unsafe_fn)]

mod buffer;
mod errors;
mod model;
mod registry;
mod status;
pub mod tape;

pub use errors::*;
pub use model::*;
pub use status::{OpenbimIfcStatus, OpenbimIfcVersion};
pub use tape::OpenbimIfcValueNode;

/// Opt-in allocator (feature `mimalloc`, off by default). It only covers
/// this library's Rust allocations; the host's `malloc` is untouched, and
/// no Rust allocation crosses the ABI, so the host never frees one. See
/// #49 for the measurements and the pure-Rust follow-up.
#[cfg(feature = "mimalloc")]
#[global_allocator]
static ALLOCATOR: mimalloc::MiMalloc = mimalloc::MiMalloc;
