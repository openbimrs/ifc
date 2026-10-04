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
//! Reading and writing caller-provided C buffers cannot be done without
//! `unsafe`, so this crate's unsafe surface is its whole export set. It is
//! not the only unsafe code in the workspace: the opt-in memory-mapped read
//! (`ifc_step::StepReader::read_path_mapped`, wrapped by
//! `openbim-ifc-binding-core` and `openbim-ifc-py`) is an `unsafe fn` too,
//! because another process may change a mapped file. Here, each dereference
//! sits in `buffer` or directly beside its null/length check, with a
//! `SAFETY` comment.
//!
//! The conventions above mirror Axiolid's ADR 0040 on purpose. A divergence
//! needs its reason recorded in the module docs or an ADR. ABI 0.1 symbols
//! are frozen once released: a change adds `openbim_ifc_v0_2_` exports rather
//! than altering a `v0_1_` one.

#![deny(unsafe_op_in_unsafe_fn)]

mod attributes;
mod buffer;
mod checks;
mod domains;
mod edits;
mod errors;
mod header;
mod model;
mod open;
mod options;
mod registry;
mod status;
pub mod tape;
mod xml;

pub use attributes::*;
pub use checks::*;
pub use domains::*;
pub use edits::*;
pub use errors::*;
pub use header::*;
pub use model::*;
pub use open::*;
pub use options::*;
pub use status::{OpenbimIfcStatus, OpenbimIfcVersion};
pub use tape::OpenbimIfcValueNode;
pub use xml::*;

/// Opt-in allocator (feature `rusty_alloc`, off by default). It only covers
/// this library's Rust allocations; the host's `malloc` is untouched, and
/// no Rust allocation crosses the ABI, so the host never frees one. See
/// #49 for the measurements.
#[cfg(test)]
mod capability_tests;
#[cfg(test)]
mod domain_tests;

#[cfg(feature = "rusty_alloc")]
#[global_allocator]
static ALLOCATOR: rusty_alloc_api::RustyAlloc = rusty_alloc_api::RustyAlloc;
