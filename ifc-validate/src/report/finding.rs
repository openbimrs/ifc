//! What a validator says when something is wrong.
//!
//! # Severity is about conformance, not about how annoyed you should be
//!
//! [`Severity::Error`] means the file violates the schema: a required
//! attribute is absent, a reference points at nothing, a GUID is duplicated.
//! [`Severity::Warning`] means the file is legal but suspicious.
//! [`Severity::Unsupported`] means *this validator did not check* -- the rule
//! exists in the schema and is not implemented here.
//!
//! That third variant is the important one. A validator that silently skips
//! what it cannot evaluate reports a clean file and is worse than useless,
//! because a clean report is exactly what a user acts on. Counting the
//! unchecked rules is what makes "no errors" mean something.

use std::fmt;

use super::path::Path;

/// How serious a finding is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Severity {
    /// The file violates the schema.
    Error,
    /// Legal, but very likely a mistake.
    Warning,
    /// A rule this validator does not implement. Not a verdict on the file.
    Unsupported,
}

impl fmt::Display for Severity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            Self::Error => "error",
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
        let mut severities = [Severity::Unsupported, Severity::Warning, Severity::Error];
        severities.sort();
        assert_eq!(
            severities,
            [Severity::Error, Severity::Warning, Severity::Unsupported]
        );
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
