//! Structured findings: severity, entity, rule, message.
//!
//! A validation result must be machine-readable so `ids` can consume it and a
//! CI gate can act on it.
//!
//! Implementation is tracked in `../PLAN.md`.

//! ## Internal split
//!
//! - `finding.rs`: stable structured finding.
//! - `path.rs`: entity, attribute, and rule paths.
//! - `summary.rs`: deterministic report summaries.

mod finding;
mod path;
mod summary;
