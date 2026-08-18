//! One entity instance: id, type, attribute slots.
//!
//! The dense handle plus typed attribute access. Attributes stay lazily
//! interpreted -- a model has tens of millions of slots and most are never read.
//!
//! Not yet implemented -- see `docs/ROADMAP.md`.
