//! STEP read options, in a host-independent form.
//!
//! Mirrors the facade's `ParseOptions`, which is `#[non_exhaustive]` and so
//! cannot be built field by field outside its crate. Each host fills this
//! record from its own idiom (a JS object, Python keywords, C flag bits)
//! and [`ParseOptions::to_facade`] is the one place it becomes the facade's.

use crate::BindingError;

/// What a STEP read does with a data record it cannot parse.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum OnMalformed {
    /// Fail the read. The default: an authoring tool that silently drops
    /// entities corrupts the file it edits.
    #[default]
    Abort,
    /// Skip the record, report it as a diagnostic, and keep reading.
    Skip,
}

impl OnMalformed {
    /// The host-facing spelling: `abort` or `skip`.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Abort => "abort",
            Self::Skip => "skip",
        }
    }

    /// Parse a host-facing spelling; anything else is `invalid-value`.
    pub fn parse(text: &str) -> Result<Self, BindingError> {
        match text {
            "abort" => Ok(Self::Abort),
            "skip" => Ok(Self::Skip),
            other => Err(BindingError::InvalidValue(format!(
                "onMalformed must be \"abort\" or \"skip\", not {other:?}"
            ))),
        }
    }
}

/// How a STEP read treats damaged or non-conforming input.
///
/// The default is [`Self::strict`]. Whatever a lenient read recovers from is
/// reported, one entry each, by `IfcModel::diagnostics`; header structure
/// stays fatal either way.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ParseOptions {
    /// Policy for unparsable data records.
    pub on_malformed: OnMalformed,
    /// Report duplicate instance ids and references to ids no record
    /// defines, as diagnostics. Nothing is dropped or rewritten.
    pub check_references: bool,
    /// Read a real written without its decimal point (`1E-05`) as the real
    /// it spells, with a diagnostic, instead of refusing the file.
    pub accept_real_without_point: bool,
}

impl ParseOptions {
    /// Any malformed record fails the read.
    pub const fn strict() -> Self {
        Self {
            on_malformed: OnMalformed::Abort,
            check_references: false,
            accept_real_without_point: false,
        }
    }

    /// The facade's lenient preset: skip malformed records and accept reals
    /// without a decimal point, each reported as a diagnostic.
    pub const fn lenient() -> Self {
        Self::from_facade(ifc::ParseOptions::lenient())
    }

    const fn from_facade(options: ifc::ParseOptions) -> Self {
        Self {
            on_malformed: match options.on_malformed_record {
                ifc::OnMalformed::Skip => OnMalformed::Skip,
                ifc::OnMalformed::Abort => OnMalformed::Abort,
            },
            check_references: options.check_references,
            accept_real_without_point: options.accept_real_without_point,
        }
    }

    /// The facade's options.
    pub(crate) const fn to_facade(self) -> ifc::ParseOptions {
        ifc::ParseOptions::strict()
            .on_malformed_record(match self.on_malformed {
                OnMalformed::Abort => ifc::OnMalformed::Abort,
                OnMalformed::Skip => ifc::OnMalformed::Skip,
            })
            .check_references(self.check_references)
            .accept_real_without_point(self.accept_real_without_point)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_presets_match_the_facade() {
        assert_eq!(ParseOptions::default(), ParseOptions::strict());
        assert_eq!(
            ParseOptions::strict().to_facade(),
            ifc::ParseOptions::strict()
        );
        assert_eq!(
            ParseOptions::lenient().to_facade(),
            ifc::ParseOptions::lenient()
        );
        assert_eq!(ParseOptions::lenient().on_malformed, OnMalformed::Skip);
    }

    #[test]
    fn the_policy_spelling_round_trips_and_refuses_anything_else() {
        for policy in [OnMalformed::Abort, OnMalformed::Skip] {
            assert_eq!(OnMalformed::parse(policy.as_str()), Ok(policy));
        }
        assert!(matches!(
            OnMalformed::parse("Skip"),
            Err(BindingError::InvalidValue(_))
        ));
    }
}
