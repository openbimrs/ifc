//! UNIQUE declarations and global identity checks.
//!
//! Duplicate IFC GUIDs are common in merged models. Preserve every duplicate in
//! the model, then report all conflicting entity IDs deterministically. The
//! `pass-*` / `fail-duplicated-guids*` validation fixtures are the first proof.
//!
//! Follow `AGENTS.md` and `PLAN.md` in this directory. Keep this module
//! crate-private until it owns a deliberate public contract.
