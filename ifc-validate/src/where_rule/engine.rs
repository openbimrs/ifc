//! Running the registered rules under a budget.

use ifc_model::Model;
use ifc_schema::Schema;

use super::budget::Budget;
use super::builtin;
use super::registry::{self, Support};
use crate::report::{Finding, Path, Report};

/// Evaluate every implemented rule, and record every unimplemented one.
///
/// The second half is the point: a report from this function distinguishes
/// "conformant" from "conformant as far as we can tell", and says which rules
/// fall in the gap.
pub fn evaluate(model: &Model, schema: &Schema, budget: Budget, report: &mut Report) {
    builtin::single_project_instance(model, report);
    builtin::unique_global_id(model, schema, report);
    builtin::no_related_type_object(model, schema, report);

    for entry in registry::unsupported() {
        if report.findings().len() >= budget.max_findings {
            report.mark_truncated();
            return;
        }
        let Support::Unsupported(reason) = entry.support else {
            continue;
        };
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
