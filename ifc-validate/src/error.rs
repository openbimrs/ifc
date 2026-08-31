//! Why validation could not run.
//!
//! Distinct from a [`Finding`](crate::report::Finding): a finding says the
//! *file* is wrong, this says the *validator* could not proceed. Conflating
//! them would let "we could not check this" be read as "this is fine".

use std::fmt;

/// Validation could not be performed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidateError {
    /// The `FILE_SCHEMA` token is not a schema version this crate recognises.
    ///
    /// Carries the token as written, so the caller can report it verbatim.
    UnknownSchema(String),
    /// A recognised schema version for which this build bundles no tables.
    ///
    /// Separate from [`Self::UnknownSchema`] because the remedy differs: an
    /// unknown token usually means a malformed or non-IFC file, while an
    /// unbundled schema is a known gap in this build. Reporting both as
    /// "unknown" would hide which one a user is hitting.
    UnbundledSchema(String),
    /// The file declares no schema at all.
    NoSchemaDeclared,
}

impl fmt::Display for ValidateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownSchema(token) => {
                write!(f, "unrecognised schema token {token:?}")
            }
            Self::UnbundledSchema(token) => {
                write!(f, "no schema tables bundled for {token:?}")
            }
            Self::NoSchemaDeclared => f.write_str("the file declares no schema"),
        }
    }
}

impl std::error::Error for ValidateError {}

#[cfg(test)]
mod tests {
    use super::*;

    /// The two refusals must not read alike: one is a bad token, the other a
    /// known schema this build cannot check.
    #[test]
    fn the_two_schema_refusals_are_distinguishable() {
        let unknown = ValidateError::UnknownSchema("STEP".into()).to_string();
        let unbundled = ValidateError::UnbundledSchema("IFC_FUTURE_PROFILE".into()).to_string();
        assert_ne!(unknown, unbundled);
        assert!(unbundled.contains("IFC_FUTURE_PROFILE"));
        assert!(unknown.contains("STEP"));
    }
}
