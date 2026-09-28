//! Running the registered rules under a budget.

use ifc_model::Model;
use ifc_schema::Schema;

use super::budget::Budget;
use super::builtin;
use super::registry::{self, Support};
use crate::report::{Finding, Path, Report};

/// Evaluate every implemented rule, and record every unimplemented one.
///
/// Only rules the schema's release declares are considered, in registry
/// order. The second half is the point: a report from this function
/// distinguishes "conformant" from "conformant as far as we can tell", and
/// says which rules fall in the gap.
pub fn evaluate(model: &Model, schema: &Schema, budget: Budget, report: &mut Report) {
    // A recognised release no registered rule is scoped to (IFC4X1, IFC4X2)
    // gets no WHERE-rule verdict at all. Say so, rather than let the report
    // read as if its rules had passed.
    if let Some(version) = schema.version() {
        if !registry::RULES.iter().any(|entry| entry.applies_to(schema)) {
            report.push(Finding::unsupported(
                "where.release",
                Path::File,
                format!(
                    "no WHERE rule is registered for {}; structural and type checks \
                     ran against its own tables, WHERE rules were not evaluated",
                    version.release_id()
                ),
            ));
            return;
        }
    }
    for entry in registry::implemented() {
        if entry.applies_to(schema) {
            let dispatched = builtin::run(entry, model, schema, report);
            debug_assert!(dispatched, "{} has no native implementation", entry.id);
        }
    }

    for entry in registry::unsupported() {
        if report.findings().len() >= budget.max_findings {
            report.mark_truncated();
            return;
        }
        let Support::Unsupported(reason) = entry.support else {
            continue;
        };
        if !entry.applies_to(schema) {
            continue;
        }
        // Only mention a rule the file could actually trip: an unsupported
        // rule for an entity type the file never uses is noise.
        if let Some(entity) = entry.entity {
            if !model_contains(model, schema, entity) {
                continue;
            }
        }
        report.push(Finding::unsupported(entry.id, Path::File, reason));
    }
}

/// Whether the model holds any instance of `entity` or a subtype of it.
fn model_contains(model: &Model, schema: &Schema, entity: &str) -> bool {
    model
        .type_histogram()
        .iter()
        .any(|(name, _)| schema.is_a(name, entity))
}

#[cfg(all(test, feature = "ifc4"))]
mod intermediate_release_tests {
    use super::*;
    use crate::report::Severity;

    /// IFC4X1 and IFC4X2 validate against their own tables, but no WHERE
    /// rule is scoped to them yet: the report says so instead of reading as
    /// if every rule passed.
    #[test]
    fn an_unscoped_release_reports_that_where_rules_did_not_run() {
        for schema in [ifc_schema::ifc4x1(), ifc_schema::ifc4x2()] {
            let mut report = Report::new();
            evaluate(&Model::new(), schema, Budget::default(), &mut report);
            let findings = report.findings();
            assert_eq!(findings.len(), 1, "{}", schema.name());
            assert_eq!(findings[0].rule, "where.release");
            assert_eq!(findings[0].severity, Severity::Unsupported);
        }
        let mut report = Report::new();
        evaluate(
            &Model::new(),
            ifc_schema::ifc4(),
            Budget::default(),
            &mut report,
        );
        assert!(report.findings().iter().all(|f| f.rule != "where.release"));
    }
}
