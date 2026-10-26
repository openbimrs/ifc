//! Every hand-written rule's adversarial pair, run again through the
//! EXPRESS evaluator.
//!
//! `tests/rule_inventory` pins each rule id with a model that must produce
//! it and a minimally different one that must not. Here the same models
//! are validated with the release's EXPRESS rules supplied, which replace
//! the hand-written rules carrying the same id. Where the two disagree, the
//! difference is listed in [`KNOWN`] with the reason, so a new divergence
//! fails instead of passing unnoticed.

#[path = "../rule_inventory/cases.rs"]
mod cases;
#[path = "../rule_inventory/fixtures.rs"]
mod fixtures;
#[path = "../rule_inventory/forms.rs"]
mod forms;
#[path = "../rule_inventory/property.rs"]
mod property;
#[path = "../rule_inventory/rules.rs"]
mod rules;
#[path = "../rule_inventory/structure.rs"]
mod structure;
#[path = "../rule_inventory/types.rs"]
mod types;

use ifc_model::Model;
use ifc_schema::Schema;
use ifc_validate::{validate_with_options, Report, Severity, ValidateOptions};

use super::spec_available;

/// Cases whose verdict differs once the EXPRESS rules replace the
/// hand-written ones: `(rule id, form, which fixture, why)`.
const KNOWN: &[(&str, &str, &str, &str)] = &[];

fn with_express(model: &Model, schema: &Schema) -> Report {
    let version = schema.version().expect("the inventory uses bundled tables");
    validate_with_options(
        model,
        schema,
        &ValidateOptions::new().express(super::rules(version)),
    )
}

#[test]
fn every_native_case_agrees_with_the_express_rules() {
    if !spec_available() {
        return;
    }
    fixtures::VALIDATOR.with(|validator| validator.set(with_express));
    let mut differences = Vec::new();
    let mut compared = 0;
    for case in cases::all() {
        let failing = (case.fails)();
        if !failing.findings().iter().any(|finding| {
            finding.rule == case.rule
                && matches!(finding.severity, Severity::Error | Severity::Warning)
        }) {
            differences.push((case.rule, case.form, "fails"));
        }
        let passing = (case.passes)();
        if passing
            .findings()
            .iter()
            .any(|finding| finding.rule == case.rule)
        {
            differences.push((case.rule, case.form, "passes"));
        }
        compared += 1;
    }
    fixtures::VALIDATOR.with(|validator| validator.set(ifc_validate::validate));
    assert!(compared > 40, "only {compared} cases ran");
    let known: Vec<(&str, &str, &str)> = KNOWN.iter().map(|(r, f, w, _)| (*r, *f, *w)).collect();
    assert_eq!(
        differences, known,
        "the EXPRESS rules and the hand-written rules disagree"
    );
}
