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
    /// The file's header names no schema this crate bundles, or (from a
    /// domain view, #123) a release the view is not verified for, such as
    /// IFC4X1 for property sets or IFC2X3 for georeferencing.
    UnsupportedSchema(String),
    /// A file could not be opened or read.
    Io(String),
    /// No ifcXML XSD profile is named by the given token (only `IFC4` and
    /// `IFC4X3_ADD2` have one).
    UnsupportedProfile(String),
    /// The operation needs a binding feature this build left out, e.g.
    /// `validate` in a size-trimmed browser package.
    FeatureDisabled(&'static str),
    /// A domain view (#123) refused the file's data: it contradicts the
    /// schema, is ambiguous where the schema allows one answer, or cannot
    /// prove an exact answer (a lenient read's skipped records).
    InvalidModel(String),
    /// A domain view (#123) followed a reference to an entity the file does
    /// not contain.
    MissingReference(String),
    /// A domain view (#123) stopped at its traversal budget: a cycle, or
    /// nesting deeper than it follows.
    BudgetExceeded(String),
    /// A domain view (#123) met a construct it does not interpret, such as
    /// a coordinate operation with no project-to-map form; refused rather
    /// than approximated.
    Unsupported(String),
    /// A domain query (#123) named an entity of a type the query does not
    /// accept, such as the property sets of a cartesian point.
    WrongEntityType(String),
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
            Self::InvalidModel(_) => "invalid-model",
            Self::MissingReference(_) => "missing-reference",
            Self::BudgetExceeded(_) => "budget-exceeded",
            Self::Unsupported(_) => "unsupported",
            Self::WrongEntityType(_) => "wrong-entity-type",
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
            // The detail is the header token, or names why it is refused:
            // not bundled in this build, or not read by a domain view.
            Self::UnsupportedSchema(detail) => write!(f, "unsupported schema: {detail:?}"),
            Self::Io(detail) => write!(f, "cannot read file: {detail}"),
            Self::UnsupportedProfile(token) => write!(
                f,
                "no ifcXML XSD profile for {token:?} (IFC4 or IFC4X3_ADD2)"
            ),
            Self::FeatureDisabled(feature) => {
                write!(f, "this build leaves out the `{feature}` feature")
            }
            Self::InvalidModel(detail) => write!(f, "invalid model: {detail}"),
            Self::MissingReference(detail) => write!(f, "missing reference: {detail}"),
            Self::BudgetExceeded(detail) => write!(f, "budget exceeded: {detail}"),
            Self::Unsupported(detail) => write!(f, "unsupported: {detail}"),
            Self::WrongEntityType(detail) => write!(f, "wrong entity type: {detail}"),
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
        "invalid-model",
        "missing-reference",
        "budget-exceeded",
        "unsupported",
        "wrong-entity-type",
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
            BindingError::InvalidModel(String::new()),
            BindingError::MissingReference(String::new()),
            BindingError::BudgetExceeded(String::new()),
            BindingError::Unsupported(String::new()),
            BindingError::WrongEntityType(String::new()),
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
                | BindingError::FeatureDisabled(_)
                | BindingError::InvalidModel(_)
                | BindingError::MissingReference(_)
                | BindingError::BudgetExceeded(_)
                | BindingError::Unsupported(_)
                | BindingError::WrongEntityType(_) => {}
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
