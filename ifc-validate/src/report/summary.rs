//! Deterministic counts and the assembled report.

use std::fmt;

use super::finding::{Finding, Severity};
use super::path::path_key;

/// Counts by severity, for a one-line verdict.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Summary {
    /// How many schema violations.
    pub errors: usize,
    /// How many suspicious-but-legal findings.
    pub warnings: usize,
    /// How many rules went unevaluated.
    pub unsupported: usize,
}

impl Summary {
    /// Whether the file violated anything this validator checked.
    ///
    /// Deliberately ignores [`Severity::Unsupported`]: an unchecked rule is a
    /// statement about the validator, not about the file. Callers that want
    /// "clean *and* fully checked" must also test [`Summary::unsupported`],
    /// and the distinction is the point.
    #[must_use]
    pub const fn is_conformant(&self) -> bool {
        self.errors == 0
    }
}

impl fmt::Display for Summary {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} errors, {} warnings, {} unsupported",
            self.errors, self.warnings, self.unsupported
        )
    }
}

/// Everything a validation run produced.
#[derive(Debug, Clone)]
pub struct Report {
    findings: Vec<Finding>,
    truncated: bool,
    max_findings: usize,
}

impl Default for Report {
    fn default() -> Self {
        Self {
            findings: Vec::new(),
            truncated: false,
            max_findings: usize::MAX,
        }
    }
}

impl PartialEq for Report {
    fn eq(&self, other: &Self) -> bool {
        self.findings == other.findings && self.truncated == other.truncated
    }
}

impl Eq for Report {}

impl Report {
    /// An empty report.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// An empty report that stores at most `max_findings` entries.
    pub(crate) fn with_max_findings(max_findings: usize) -> Self {
        Self {
            max_findings,
            ..Self::default()
        }
    }

    /// Records a finding, or marks the report truncated when its cap is full.
    pub fn push(&mut self, finding: Finding) {
        if self.findings.len() >= self.max_findings {
            self.truncated = true;
            return;
        }
        self.findings.push(finding);
    }

    /// Records that the run stopped early because it hit its finding budget.
    ///
    /// A report that silently stops at N findings claims the file has N
    /// problems. This flag is what keeps that claim honest.
    pub fn mark_truncated(&mut self) {
        self.truncated = true;
    }

    /// Whether the run stopped before checking everything.
    #[must_use]
    pub const fn is_truncated(&self) -> bool {
        self.truncated
    }

    /// Every finding, in the order produced.
    ///
    /// Callers wanting a stable order across runs should use
    /// [`Report::sorted`]: production order follows traversal, which is
    /// deterministic for a given model but not meaningful.
    #[must_use]
    pub fn findings(&self) -> &[Finding] {
        &self.findings
    }

    /// How many findings of each severity.
    #[must_use]
    pub fn summary(&self) -> Summary {
        let mut summary = Summary::default();
        for finding in &self.findings {
            match finding.severity {
                Severity::Error => summary.errors += 1,
                Severity::Warning => summary.warnings += 1,
                Severity::Unsupported => summary.unsupported += 1,
            }
        }
        summary
    }

    /// Findings in a stable, explicitly defined order.
    ///
    /// Sorted by severity, then rule id, then entity id, then slot. Two runs
    /// over the same file always produce byte-identical output, which is what
    /// makes a report diffable in CI.
    #[must_use]
    pub fn sorted(&self) -> Vec<&Finding> {
        let mut sorted: Vec<&Finding> = self.findings.iter().collect();
        sorted.sort_by(|left, right| {
            left.severity
                .cmp(&right.severity)
                .then_with(|| left.rule.cmp(&right.rule))
                .then_with(|| path_key(&left.path).cmp(&path_key(&right.path)))
                .then_with(|| left.message.cmp(&right.message))
        });
        sorted
    }

    /// Merges another report into this one, preserving the truncation flag.
    pub fn extend(&mut self, other: Self) {
        let other_truncated = other.truncated;
        for finding in other.findings {
            self.push(finding);
        }
        self.truncated |= other_truncated;
    }

    /// Whether anything this validator checked was violated.
    #[must_use]
    pub fn is_conformant(&self) -> bool {
        self.summary().is_conformant()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::Path;
    use ifc_model::EntityId;

    #[test]
    fn unsupported_rules_do_not_make_a_file_non_conformant() {
        let mut report = Report::new();
        report.push(Finding::unsupported(
            "where.IfcRoot.WR1",
            Path::File,
            "not implemented",
        ));
        assert!(
            report.is_conformant(),
            "an unchecked rule is a fact about the validator"
        );
        assert_eq!(report.summary().unsupported, 1);
    }

    #[test]
    fn sorting_is_total_and_stable() {
        let mut report = Report::new();
        report.push(Finding::warning("b.rule", Path::Entity(EntityId(9)), "w"));
        report.push(Finding::error("a.rule", Path::Entity(EntityId(2)), "e"));
        report.push(Finding::error("a.rule", Path::Entity(EntityId(1)), "e"));
        let order: Vec<String> = report
            .sorted()
            .iter()
            .map(|finding| finding.path.to_string())
            .collect();
        assert_eq!(order, ["#1", "#2", "#9"], "errors first, then by entity");
    }

    /// A capped report cannot be overfilled through a merge.
    #[test]
    fn extend_honors_the_storage_cap() {
        let mut target = Report::with_max_findings(1);
        let mut source = Report::new();
        source.push(Finding::error("a", Path::File, "first"));
        source.push(Finding::error("b", Path::File, "second"));

        target.extend(source);

        assert_eq!(target.findings().len(), 1);
        assert!(target.is_truncated());
    }

    /// A truncated report says so, rather than implying the file is clean.
    #[test]
    fn truncation_survives_a_merge() {
        let mut left = Report::new();
        let mut right = Report::new();
        right.mark_truncated();
        left.extend(right);
        assert!(left.is_truncated());
    }
}
