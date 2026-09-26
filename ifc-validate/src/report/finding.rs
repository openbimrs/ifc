//! What a validator says when something is wrong.
//!
//! # Severity is about conformance, not about how annoyed you should be
//!
//! [`Severity::Error`] means the file violates the schema: a required
//! attribute is absent, a reference points at nothing, a GUID is duplicated.
//! [`Severity::Warning`] means the file is legal but suspicious.
//! [`Severity::Unsupported`] means *this validator did not check* -- the rule
//! exists in the schema and is not implemented here.
//! [`Severity::EvaluationError`] means an implemented rule applied to an
//! instance and *could not be decided* for it: an operand has a shape the rule
//! cannot read, names a target the file does not contain, or is missing from
//! the schema tables the rule was written against.
//!
//! The last two variants are the important ones. A validator that silently
//! skips what it cannot evaluate reports a clean file and is worse than
//! useless, because a clean report is exactly what a user acts on. Counting
//! the unchecked rules is what makes "no errors" mean something.
//!
//! They differ in what they say about the file. `Unsupported` is a fact about
//! this validator for every file alike, so it leaves conformance alone. An
//! `EvaluationError` is about *this* instance: the rule may well be violated
//! there, so a report carrying one is not conformant.

use std::fmt;

use super::path::Path;

/// How serious a finding is.
///
/// Declaration order is the report's sort order, most actionable first.
/// Non-exhaustive: a new category of finding must not break every match.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Severity {
    /// The file violates the schema.
    Error,
    /// An implemented rule applies to this instance but could not be
    /// decided for it. Makes the report non-conformant, because "could not
    /// check" is not "passed".
    EvaluationError,
    /// Legal, but very likely a mistake.
    Warning,
    /// A rule this validator does not implement. Not a verdict on the file.
    Unsupported,
}

impl fmt::Display for Severity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            Self::Error => "error",
            Self::EvaluationError => "evaluation-error",
            Self::Warning => "warning",
            Self::Unsupported => "unsupported",
        };
        formatter.write_str(text)
    }
}

/// One thing a validator has to say about a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    /// How serious it is.
    pub severity: Severity,
    /// A stable identifier for the check that produced this, e.g.
    /// `structure.reference.dangling` or `where.IfcRoot.WR1`. Callers filter
    /// and suppress on this, so it is part of the contract.
    pub rule: String,
    /// Where the problem is.
    pub path: Path,
    /// What is wrong, in one sentence, without restating the rule id.
    pub message: String,
}

impl Finding {
    /// A schema violation.
    #[must_use]
    pub fn error(rule: impl Into<String>, path: Path, message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Error,
            rule: rule.into(),
            path,
            message: message.into(),
        }
    }

    /// A legal but suspicious condition.
    #[must_use]
    pub fn warning(rule: impl Into<String>, path: Path, message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Warning,
            rule: rule.into(),
            path,
            message: message.into(),
        }
    }

    /// An implemented rule that could not be decided for one instance.
    ///
    /// `rule` is the rule's own id, so a caller filtering on it sees both
    /// verdicts; the severity says which one this is.
    #[must_use]
    pub fn evaluation_error(
        rule: impl Into<String>,
        path: Path,
        message: impl Into<String>,
    ) -> Self {
        Self {
            severity: Severity::EvaluationError,
            rule: rule.into(),
            path,
            message: message.into(),
        }
    }

    /// A rule this validator did not evaluate.
    #[must_use]
    pub fn unsupported(rule: impl Into<String>, path: Path, message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Unsupported,
            rule: rule.into(),
            path,
            message: message.into(),
        }
    }
}

impl fmt::Display for Finding {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}: {} at {}: {}",
            self.severity, self.rule, self.path, self.message
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ifc_model::EntityId;

    /// Severity ordering is what report sorting relies on.
    #[test]
    fn errors_sort_before_warnings_before_unsupported() {
        let mut severities = [
            Severity::Unsupported,
            Severity::Warning,
            Severity::EvaluationError,
            Severity::Error,
        ];
        severities.sort();
        assert_eq!(
            severities,
            [
                Severity::Error,
                Severity::EvaluationError,
                Severity::Warning,
                Severity::Unsupported
            ]
        );
    }

    /// Each severity renders distinctly, so a printed report cannot
    /// conflate "violated" with "could not decide".
    #[test]
    fn every_severity_renders_distinctly() {
        let rendered: Vec<String> = [
            Severity::Error,
            Severity::EvaluationError,
            Severity::Warning,
            Severity::Unsupported,
        ]
        .iter()
        .map(ToString::to_string)
        .collect();
        let mut unique = rendered.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(unique.len(), rendered.len(), "{rendered:?}");
    }

    /// A finding renders its path so a reader can find the entity.
    #[test]
    fn a_finding_names_where_it_applies() {
        let finding = Finding::error(
            "structure.dangling",
            Path::Attribute {
                entity: EntityId(12),
                index: 3,
                name: Some("Representation".into()),
            },
            "points at #99, which does not exist",
        );
        assert_eq!(finding.path.to_string(), "#12.Representation");
        assert_eq!(finding.severity, Severity::Error);
    }
}
