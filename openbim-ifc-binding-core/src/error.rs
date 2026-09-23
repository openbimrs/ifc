//! Errors raised by every binding.
//!
//! Each error carries a stable machine-readable [`BindingError::code`] and a
//! human message. The codes are shared by all hosts (ADR 0013), so a
//! JavaScript `err.code`, a Python `err.code` and a C status all name the same
//! failure the same way.

use std::fmt;

/// Why a binding call failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BindingError {
    /// The STEP input could not be parsed.
    Parse(String),
    /// The model could not be serialized.
    Write(String),
    /// No entity has the given id.
    MissingEntity(u64),
    /// A host value did not follow the tagged value encoding.
    InvalidValue(String),
    /// An id or index was outside the range the model can represent.
    OutOfRange(String),
    /// The file's header names no schema this crate bundles.
    UnsupportedSchema(String),
}

impl BindingError {
    /// Stable code for programmatic handling.
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Parse(_) => "parse",
            Self::Write(_) => "write",
            Self::MissingEntity(_) => "missing-entity",
            Self::InvalidValue(_) => "invalid-value",
            Self::OutOfRange(_) => "out-of-range",
            Self::UnsupportedSchema(_) => "unsupported-schema",
        }
    }
}

impl fmt::Display for BindingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse(detail) => write!(f, "cannot parse STEP: {detail}"),
            Self::Write(detail) => write!(f, "cannot write STEP: {detail}"),
            Self::MissingEntity(id) => write!(f, "no entity #{id}"),
            Self::InvalidValue(detail) => write!(f, "invalid IFC value: {detail}"),
            Self::OutOfRange(detail) => write!(f, "out of range: {detail}"),
            Self::UnsupportedSchema(token) => {
                write!(
                    f,
                    "no bundled schema for {token:?}; expected IFC2X3, IFC4 or IFC4X3"
                )
            }
        }
    }
}

impl std::error::Error for BindingError {}
