//! `ifc-validate` — schema conformance checking.
//!
//! # Why this is its own crate
//!
//! The EXPRESS schema is not just a type list: IFC4 defines **47 functions and
//! 2 global rules**, plus per-entity WHERE clauses and inverse-attribute
//! cardinalities. A file can parse cleanly and still be invalid, and knowing
//! *which* is the difference between "this tool is broken" and "this file is
//! broken."
//!
//! Separating validation from parsing keeps the parser fast: the hot path never
//! pays for rules a consumer did not ask for.
//!
//! # Scope
//!
//! - WHERE-rule evaluation against the schema
//! - Cardinality and inverse-attribute integrity
//! - GUID validity, uniqueness, and the compressed-GUID encoding
//! - Reference integrity: no dangling `#id`, no type mismatches
//! - Header conformance (`FILE_SCHEMA` versus actual content)
//!
//! # Design note
//!
//! Findings are **structured diagnostics with severity**, not a boolean. Real
//! models routinely carry hundreds of minor violations; a validator that
//! answers only pass/fail is unusable on production data. This mirrors the
//! fixture corpus, whose files are named `pass-*` and `fail-*` precisely
//! because both are expected outcomes.
