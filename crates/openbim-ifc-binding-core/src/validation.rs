//! Schema validation as plain records (feature `validate`).
//!
//! Runs the facade's validator against the schema the file declares and
//! flattens its report into records every host can carry: strings, ids and
//! counts, in the report's stable sorted order. Without the feature the
//! records still exist, so every host keeps one surface, and
//! [`IfcModel::validate`] refuses with `feature-disabled`.

use crate::{BindingError, IfcModel};

/// One finding: what is wrong, and where.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationFinding {
    /// `error`, `evaluation-error`, `warning` or `unsupported`, as the
    /// validator spells them. `error` and `evaluation-error` make a file
    /// non-conformant; `unsupported` is a rule this validator does not check.
    pub severity: String,
    /// Stable id of the check, e.g. `structure.reference.dangling`.
    pub rule: String,
    /// The entity the finding is about; `None` for the file as a whole.
    pub entity: Option<u64>,
    /// The zero-based attribute slot, when the finding is about one.
    pub attribute_index: Option<usize>,
    /// The attribute's schema name, when the schema declares one.
    pub attribute_name: Option<String>,
    /// The location as text: `<file>`, `#12` or `#12.Name`.
    pub path: String,
    /// What is wrong, in one sentence.
    pub message: String,
}

/// Counts by severity.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ValidationSummary {
    /// Schema violations.
    pub errors: usize,
    /// Implemented rules that could not be decided for an instance.
    pub evaluation_errors: usize,
    /// Legal but suspicious conditions.
    pub warnings: usize,
    /// Rules this validator does not evaluate.
    pub unsupported: usize,
}

/// Everything one validation run found.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ValidationReport {
    /// No errors and no evaluation errors among the recorded findings.
    /// Unsupported rules do not count against it.
    pub conformant: bool,
    /// The run hit its finding budget: the counts are lower bounds and a
    /// conformant verdict is not final.
    pub truncated: bool,
    /// Counts by severity over the recorded findings.
    pub summary: ValidationSummary,
    /// Every recorded finding, sorted by severity, rule, entity and slot.
    pub findings: Vec<ValidationFinding>,
}

impl IfcModel {
    /// Validate against the schema the header declares.
    ///
    /// `max_findings` caps how many findings are recorded (the validator's
    /// default budget, 10,000, when `None`); hitting it sets
    /// [`ValidationReport::truncated`]. Refused with `unsupported-schema`
    /// when the header names no bundled schema, and with `feature-disabled`
    /// in a build without the `validate` feature.
    pub fn validate(&self, max_findings: Option<usize>) -> Result<ValidationReport, BindingError> {
        #[cfg(feature = "validate")]
        {
            run(self, max_findings)
        }
        #[cfg(not(feature = "validate"))]
        {
            let _ = max_findings;
            Err(BindingError::FeatureDisabled("validate"))
        }
    }
}

#[cfg(feature = "validate")]
fn run(model: &IfcModel, max_findings: Option<usize>) -> Result<ValidationReport, BindingError> {
    use ifc::validate::{validate_with, Budget, Path};

    let schema = model.declared_schema()?;
    let budget = max_findings.map_or(Budget::DEFAULT, |max_findings| Budget { max_findings });
    let report = validate_with(&model.inner, schema, budget);
    let summary = report.summary();
    let findings = report
        .sorted()
        .into_iter()
        .map(|finding| {
            let (entity, attribute_index, attribute_name) = match &finding.path {
                Path::File => (None, None, None),
                Path::Entity(id) => (Some(id.0), None, None),
                Path::Attribute {
                    entity,
                    index,
                    name,
                } => (Some(entity.0), Some(*index), name.clone()),
                // A path kind added after this binding: `path` still says
                // where, as text.
                _ => (None, None, None),
            };
            ValidationFinding {
                severity: finding.severity.to_string(),
                rule: finding.rule.clone(),
                entity,
                attribute_index,
                attribute_name,
                path: finding.path.to_string(),
                message: finding.message.clone(),
            }
        })
        .collect();
    Ok(ValidationReport {
        conformant: report.is_conformant(),
        truncated: report.is_truncated(),
        summary: ValidationSummary {
            errors: summary.errors,
            evaluation_errors: summary.evaluation_errors,
            warnings: summary.warnings,
            unsupported: summary.unsupported,
        },
        findings,
    })
}

/// The report as one tagged `list` of findings, each a `list` of seven
/// values: severity (text), rule (text), entity (`ref`, or `null` for the
/// file), attribute index (`integer` or `null`), attribute name (`text` or
/// `null`), path (text), message (text). The C binding's tape form.
pub fn findings_to_tagged(findings: &[ValidationFinding]) -> crate::value::Tagged {
    use crate::value::Tagged;
    let optional = |value: Option<Tagged>| value.unwrap_or(Tagged::Null);
    Tagged::List(
        findings
            .iter()
            .map(|finding| {
                Tagged::List(vec![
                    Tagged::Text(finding.severity.clone()),
                    Tagged::Text(finding.rule.clone()),
                    optional(finding.entity.map(Tagged::Ref)),
                    optional(
                        finding
                            .attribute_index
                            // A slot index is far below i64::MAX.
                            .map(|index| Tagged::Integer(i64::try_from(index).unwrap_or(i64::MAX))),
                    ),
                    optional(finding.attribute_name.clone().map(Tagged::Text)),
                    Tagged::Text(finding.path.clone()),
                    Tagged::Text(finding.message.clone()),
                ])
            })
            .collect(),
    )
}
