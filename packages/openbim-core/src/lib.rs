//! `openbim-core` — the vocabulary every openBIM standard shares.
//!
//! # What belongs here
//!
//! **Domain** concepts used by more than one standard — not XML, not ZIP.
//! Encoding substrate lives in `openbim-codec-xml` / `openbim-codec-zip`, one layer below, so
//! that `packages/` can use it too without ever depending on `openbim/`.
//!
//! Three things earn their place:
//!
//! - [`Outcome`] — the applicable/not-applicable trichotomy. IDS *produces*
//!   it, BCF *consumes* it. Defined once so the two agree.
//! - [`ElementRef`] — "this element, in this document". BCF viewpoints and
//!   ICDD linksets are both cross-document references and would otherwise
//!   invent incompatible shapes for the same idea.
//! - [`Detected`] — a version *and how it was determined*, including explicit
//!   disagreement.
//!
//! If something is needed by exactly one standard, it belongs in that
//! standard's crate instead. If this crate ever holds only re-exports of
//! `wire-*`, delete it — that would prove there was no shared domain.
//!
//! # Status
//!
//! **Scaffold.** The types here are real and tested, but no standard is
//! implemented yet. This crate is published so the `openbim-*` name family is
//! coherent from the first release.

#![forbid(unsafe_code)]

pub mod detected;
pub mod element_ref;
pub mod outcome;

pub use detected::Detected;
pub use element_ref::ElementRef;
pub use outcome::Outcome;
