//! Why validation could not run.
//!
//! Distinct from a [`Finding`](crate::report::Finding): a finding says the
//! *file* is wrong, this says the *validator* could not proceed. Conflating
//! them would let "we could not check this" be read as "this is fine".

use std::fmt;

/// Validation could not be performed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidateError {
    /// The file declares a schema this build has no tables for.
    ///
    /// Carries the token as written, so the caller can report it verbatim.
    UnknownSchema(String),
    /// The file declares no schema at all.
    NoSchemaDeclared,
}

impl fmt::Display for ValidateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownSchema(token) => {
                write!(f, "no schema tables for {token:?}")
            }
            Self::NoSchemaDeclared => f.write_str("the file declares no schema"),
        }
    }
}

impl std::error::Error for ValidateError {}
