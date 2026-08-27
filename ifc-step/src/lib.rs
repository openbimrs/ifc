//! `ifc-step` — IFC adaptation over generic ISO 10303-21 syntax.
//!
//! Generic tokenization, escaping, records, parameters, sections, parsing,
//! writing, partitioning, spans, diagnostics, and event sinks live in
//! [`openbim_step`]. This crate owns only conversion between that generic
//! substrate and [`ifc_model::Model`] plus IFC codec detection and I/O policy.
//!
//! | Module | Role |
//! | --- | --- |
//! | [`codec`] | [`StepCodec`]/[`StepReader`]: detection, parse policy, I/O |
//! | [`error`] | STEP-specific failure modes |
//! | `parser` | Generic exchange to IFC record model |
//! | `writer` | IFC record model to physical file |

pub mod codec;
pub mod error;
mod parser;
mod writer;

pub use codec::{StepCodec, StepReader};
pub use error::StepError;
pub use openbim_step::is_step_file;
/// Malformed-record policy, re-exported so consumers need not depend on
/// `openbim-step` directly to configure a reader.
pub use openbim_step::{OnMalformed, ParseOptions};
