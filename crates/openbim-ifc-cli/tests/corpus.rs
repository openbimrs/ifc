//! Every command over the whole fixture corpus, and the SARIF it writes
//! checked against the shape the SARIF 2.1.0 schema requires.
//!
//! The schema itself (OASIS, `sarif-schema-2.1.0.json`) is not vendored;
//! [`assert_sarif`] checks what it requires of every property this tool
//! writes: the log's `version` and `runs`, the driver's `name`, a result's
//! `message.text`, a `level` from the schema's enum, a `ruleIndex` that
//! points at the rule whose `id` is the `ruleId`, an artifact `index` that
//! points at the artifact with the same `uri`, a `startLine` of at least 1,
//! and an invocation's required `executionSuccessful`.

mod support;

use serde_json::Value;
use support::{corpus, fixture, run};

/// Properties the SARIF 2.1.0 schema declares for each object this tool
/// writes; anything else is a typo the schema would not reject (it allows
/// additional properties only in property bags) but a consumer would miss.
const LOG: &[&str] = &["$schema", "version", "runs"];
const RUN: &[&str] = &["tool", "invocations", "artifacts", "results"];
const DRIVER: &[&str] = &[
    "name",
    "version",
    "semanticVersion",
    "informationUri",
    "rules",
];
const RESULT: &[&str] = &["ruleId", "ruleIndex", "level", "message", "locations"];
const LEVELS: &[&str] = &["none", "note", "warning", "error"];

fn keys_within(value: &Value, allowed: &[&str], what: &str) {
    let object = value
        .as_object()
        .unwrap_or_else(|| panic!("{what} is not an object"));
    for key in object.keys() {
        assert!(
            allowed.contains(&key.as_str()),
            "{what} has unexpected property {key}"
        );
    }
}

fn text(value: &Value, what: &str) -> String {
    value
        .as_str()
        .filter(|text| !text.is_empty())
        .unwrap_or_else(|| panic!("{what} must be a non-empty string, is {value}"))
        .to_owned()
}

/// Panic unless `log` has the shape SARIF 2.1.0 requires; returns the
/// number of results.
fn assert_sarif(log: &Value) -> usize {
    keys_within(log, LOG, "log");
    assert_eq!(log["version"], "2.1.0");
    assert_eq!(
        log["$schema"],
        "https://json.schemastore.org/sarif-2.1.0.json"
    );
    let runs = log["runs"].as_array().expect("runs is an array");
    assert_eq!(runs.len(), 1);
    let run = &runs[0];
    keys_within(run, RUN, "run");
    let driver = &run["tool"]["driver"];
    keys_within(driver, DRIVER, "driver");
    assert_eq!(text(&driver["name"], "driver.name"), "openbim-ifc");
    let rules: Vec<String> = driver["rules"]
        .as_array()
        .expect("rules is an array")
        .iter()
        .map(|rule| {
            keys_within(rule, &["id", "shortDescription"], "rule");
            text(
                &rule["shortDescription"]["text"],
                "rule.shortDescription.text",
            );
            text(&rule["id"], "rule.id")
        })
        .collect();
    let mut unique = rules.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(unique.len(), rules.len(), "rule ids are unique");

    let artifacts: Vec<String> = run["artifacts"]
        .as_array()
        .expect("artifacts is an array")
        .iter()
        .map(|artifact| text(&artifact["location"]["uri"], "artifact uri"))
        .collect();
    let check_artifact = |location: &Value| {
        let artifact = &location["physicalLocation"]["artifactLocation"];
        let index = artifact["index"].as_u64().expect("artifact index") as usize;
        assert_eq!(text(&artifact["uri"], "artifact uri"), artifacts[index]);
        if let Some(region) = location["physicalLocation"].get("region") {
            assert!(region["startLine"].as_u64().expect("startLine") >= 1);
        }
    };

    let invocations = run["invocations"].as_array().expect("invocations");
    assert_eq!(invocations.len(), 1);
    let successful = invocations[0]["executionSuccessful"]
        .as_bool()
        .expect("executionSuccessful is required");
    let notifications = invocations[0]["toolExecutionNotifications"]
        .as_array()
        .expect("notifications");
    assert_eq!(successful, notifications.is_empty());
    for notification in notifications {
        assert_eq!(notification["level"], "error");
        text(&notification["message"]["text"], "notification message");
        check_artifact(&notification["locations"][0]);
    }

    let results = run["results"].as_array().expect("results is an array");
    for result in results {
        keys_within(result, RESULT, "result");
        let rule = text(&result["ruleId"], "ruleId");
        let index = result["ruleIndex"].as_u64().expect("ruleIndex") as usize;
        assert_eq!(rules[index], rule, "ruleIndex points at ruleId");
        let level = text(&result["level"], "level");
        assert!(LEVELS.contains(&level.as_str()), "level {level}");
        text(&result["message"]["text"], "message.text");
        let locations = result["locations"].as_array().expect("locations");
        assert_eq!(locations.len(), 1);
        check_artifact(&locations[0]);
        if let Some(logical) = locations[0].get("logicalLocations") {
            let logical = &logical[0];
            assert!(text(&logical["fullyQualifiedName"], "name").starts_with('#'));
            let kind = text(&logical["kind"], "kind");
            assert!(kind == "object" || kind == "member", "kind {kind}");
        }
    }
    results.len()
}

#[test]
fn validate_sarif_has_the_schema_shape_over_the_corpus() {
    let files = corpus();
    let mut args = vec![
        "validate".to_owned(),
        "--format".into(),
        "sarif".into(),
        "--include-unsupported".into(),
    ];
    args.extend(files.iter().cloned());
    args.push("missing.ifc".into());
    let out = run(&args);
    assert_eq!(out.code, 2, "missing.ifc could not be validated");
    let log = out.json();
    let results = assert_sarif(&log);
    assert!(results > files.len(), "the corpus has findings: {results}");
    let artifacts = log["runs"][0]["artifacts"].as_array().unwrap();
    assert_eq!(artifacts.len(), files.len() + 1);
    let levels: Vec<&str> = log["runs"][0]["results"]
        .as_array()
        .unwrap()
        .iter()
        .map(|result| result["level"].as_str().unwrap())
        .collect();
    assert!(levels.contains(&"error"));
    assert!(levels.contains(&"warning"));
    assert!(levels.contains(&"note"), "unsupported rules are notes");
}

#[test]
fn a_clean_sarif_log_has_no_results_and_succeeded() {
    let out = run(&[
        "validate",
        "--format",
        "sarif",
        &fixture("synthetic-bindings/binding_geometry.ifc"),
    ]);
    assert_eq!(out.code, 0);
    let log = out.json();
    assert_eq!(assert_sarif(&log), 0);
    assert_eq!(
        log["runs"][0]["invocations"][0]["executionSuccessful"],
        true
    );
}

#[test]
fn lint_sarif_has_the_schema_shape() {
    let mut args = vec!["lint".to_owned(), "--format".into(), "sarif".into()];
    args.extend(corpus());
    let out = run(&args);
    assert_eq!(out.code, 1, "the corpus has unreachable products");
    assert!(assert_sarif(&out.json()) > 0);
}

/// Every command answers for every fixture: `info` and `tree` succeed,
/// `validate` and `lint` give a verdict (0 or 1), and every JSON output
/// parses. A fixture that cannot be answered would exit 2.
#[test]
fn every_command_answers_for_every_fixture() {
    for file in corpus() {
        for (args, codes) in [
            (vec!["info", "--format", "json"], &[0][..]),
            (vec!["tree", "--format", "json"], &[0][..]),
            (vec!["validate", "--format", "json"], &[0, 1][..]),
            (vec!["lint", "--format", "json"], &[0, 1][..]),
        ] {
            let mut args: Vec<&str> = args.clone();
            args.push(&file);
            let out = run(&args);
            assert!(
                codes.contains(&out.code),
                "{args:?} exited {}: {}",
                out.code,
                out.stderr
            );
            out.json();
        }
    }
}
