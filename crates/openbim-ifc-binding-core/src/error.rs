//! Errors raised by every binding.
//!
//! Each error carries a stable machine-readable [`BindingError::code`] and a
//! human message. The codes are shared by all hosts (ADR 0013), so a
//! JavaScript `err.code`, a Python `err.code` and a C status all name the same
//! failure the same way.
//!
//! Every code is therefore part of the public contract of all three
//! bindings: a code may be added, never renamed or reused for a different
//! failure. The `tests` module below pins the released set.

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
    /// A file could not be opened or read.
    Io(String),
    /// No ifcXML XSD profile is named by the given token (only `IFC4` and
    /// `IFC4X3_ADD2` have one).
    UnsupportedProfile(String),
    /// The operation needs a binding feature this build left out, e.g.
    /// `validate` in a size-trimmed browser package.
    FeatureDisabled(&'static str),
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
            Self::Io(_) => "io",
            Self::UnsupportedProfile(_) => "unsupported-profile",
            Self::FeatureDisabled(_) => "feature-disabled",
        }
    }
}

impl fmt::Display for BindingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // The detail names its format: `STEP: ...` or `ifcXML: ...`.
            Self::Parse(detail) => write!(f, "cannot parse {detail}"),
            Self::Write(detail) => write!(f, "cannot write {detail}"),
            Self::MissingEntity(id) => write!(f, "no entity #{id}"),
            Self::InvalidValue(detail) => write!(f, "invalid IFC value: {detail}"),
            Self::OutOfRange(detail) => write!(f, "out of range: {detail}"),
            Self::UnsupportedSchema(token) => {
                write!(f, "no bundled schema for {token:?} in this build")
            }
            Self::Io(detail) => write!(f, "cannot read file: {detail}"),
            Self::UnsupportedProfile(token) => write!(
                f,
                "no ifcXML XSD profile for {token:?} (IFC4 or IFC4X3_ADD2)"
            ),
            Self::FeatureDisabled(feature) => {
                write!(f, "this build leaves out the `{feature}` feature")
            }
        }
    }
}

impl std::error::Error for BindingError {}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::BindingError;

    /// Released codes in declaration order. Append a new code; never edit or
    /// remove one, because hosts match on these strings.
    const RELEASED_CODES: &[&str] = &[
        "parse",
        "write",
        "missing-entity",
        "invalid-value",
        "out-of-range",
        "unsupported-schema",
        "io",
        "unsupported-profile",
        "feature-disabled",
    ];

    /// One value of every variant, in declaration order.
    fn one_of_each() -> Vec<BindingError> {
        let all = vec![
            BindingError::Parse(String::new()),
            BindingError::Write(String::new()),
            BindingError::MissingEntity(0),
            BindingError::InvalidValue(String::new()),
            BindingError::OutOfRange(String::new()),
            BindingError::UnsupportedSchema(String::new()),
            BindingError::Io(String::new()),
            BindingError::UnsupportedProfile(String::new()),
            BindingError::FeatureDisabled(""),
        ];
        // Exhaustive on purpose: a new variant does not compile until it is
        // listed above, so its code cannot escape the snapshot.
        for error in &all {
            match error {
                BindingError::Parse(_)
                | BindingError::Write(_)
                | BindingError::MissingEntity(_)
                | BindingError::InvalidValue(_)
                | BindingError::OutOfRange(_)
                | BindingError::UnsupportedSchema(_)
                | BindingError::Io(_)
                | BindingError::UnsupportedProfile(_)
                | BindingError::FeatureDisabled(_) => {}
            }
        }
        all
    }

    #[test]
    fn released_codes_are_never_renamed_or_reused() {
        let codes: Vec<&str> = one_of_each().iter().map(BindingError::code).collect();
        assert!(
            codes.len() >= RELEASED_CODES.len(),
            "a BindingError variant was removed: {codes:?}"
        );
        assert_eq!(
            &codes[..RELEASED_CODES.len()],
            RELEASED_CODES,
            "a released BindingError code changed; add a new code instead"
        );
        let unique: BTreeSet<&str> = codes.iter().copied().collect();
        assert_eq!(
            unique.len(),
            codes.len(),
            "two variants share a code: {codes:?}"
        );
    }
}
