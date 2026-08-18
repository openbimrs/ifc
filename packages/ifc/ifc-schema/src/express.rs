//! Parser for the official EXPRESS (`.exp`) schema files.
//!
//! Reads `references/ifc-spec/<version>/*.exp` and produces the tables in
//! [`crate::entity`] and [`crate::types`]. This runs offline — the output is
//! committed, so a normal build never parses EXPRESS and consumers never need
//! the spec files present.
//!
//! # Scope
//!
//! Only the declarations we act on: `ENTITY`, `TYPE`, `SUBTYPE OF`, attribute
//! lists, `OPTIONAL`, `DERIVE`. The `WHERE` rules and `FUNCTION` bodies (47 of
//! them in IFC4) are a separate concern belonging to `ifc-validate`.
//!
//! Not yet implemented — Stage 1 in `docs/ROADMAP.md`.
