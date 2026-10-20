//! The whole fixture corpus under every rule of its declared release.

use std::collections::BTreeSet;
use std::path::PathBuf;

use ifc_model::{Codec, Model};
use ifc_schema::SchemaVersion;
use ifc_step::StepCodec;
use ifc_validate::{validate_with_options, Budget, FindingDetail, Path, Report, ValidateOptions};

use super::expected::EXPECTED;
use super::{rules, schema, spec_available};

fn fixtures_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../test/fixtures")
}

fn ifc_files(dir: &std::path::Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .expect("fixture directory reads")
        .flatten()
        .map(|entry| entry.path())
        .collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            ifc_files(&path, out);
        } else if path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("ifc"))
        {
            out.push(path);
        }
    }
}

/// Every fixture with a declared, bundled release, parsed.
pub fn corpus() -> Vec<(String, Model, SchemaVersion)> {
    corpus_at(&fixtures_root())
}

/// Every `.ifc` file under `root` with a declared, bundled release.
pub fn corpus_at(root: &std::path::Path) -> Vec<(String, Model, SchemaVersion)> {
    let root = root.to_path_buf();
    let mut files = Vec::new();
    ifc_files(&root, &mut files);
    let mut out = Vec::new();
    for path in files {
        let name = path
            .strip_prefix(&root)
            .expect("under the root")
            .display()
            .to_string();
        let Ok(model) = StepCodec.read_path(&path) else {
            continue;
        };
        let Some(version) = model
            .header()
            .schema_token()
            .and_then(SchemaVersion::from_header_token)
        else {
            continue;
        };
        out.push((name, model, version));
    }
    out
}

/// The report with every EXPRESS rule of the file's release.
pub fn evaluated(model: &Model, version: SchemaVersion) -> Report {
    validate_with_options(
        model,
        schema(version),
        &ValidateOptions::new()
            .budget(Budget::UNLIMITED)
            .express(rules(version)),
    )
}

/// What IfcOpenShell 0.8.5's `express_rules` mode reports for the same
/// instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Agreement {
    /// The same rule is violated by the same instance.
    Agrees,
    /// IfcOpenShell reports a violation where this evaluator cannot decide
    /// the rule (an evaluation error), e.g. a string compared with zero.
    Undecided,
    /// UNKNOWN here; IfcOpenShell passes UNKNOWN silently.
    Indeterminate,
}

/// Whether a finding is one this target pins: an EXPRESS rule result, or
/// an INVERSE cardinality, which the EXPRESS rules switch on.
fn pinned_kind(finding: &ifc_validate::Finding) -> bool {
    matches!(finding.detail, Some(FindingDetail::Rule(_)))
        || finding.rule == "structure.inverse.cardinality"
}

/// The EXPRESS rule and INVERSE findings of the corpus, as `(fixture,
/// rule, entity, severity)`.
fn rule_findings() -> BTreeSet<(String, String, u64, String)> {
    let mut found = BTreeSet::new();
    for (name, model, version) in corpus() {
        let report = evaluated(&model, version);
        for finding in report.findings() {
            if !pinned_kind(finding) {
                continue;
            }
            let entity = match &finding.path {
                Path::Entity(id) | Path::Attribute { entity: id, .. } => id.0,
                _ => 0,
            };
            found.insert((
                name.clone(),
                finding.rule.clone(),
                entity,
                finding.severity.to_string(),
            ));
        }
    }
    found
}

/// The corpus produces exactly the EXPRESS findings pinned in
/// `expected.rs`, every one a defect IfcOpenShell's rule evaluation finds
/// too or one it cannot see (UNKNOWN, or a rule it fails to evaluate).
///
/// # Why the fixtures are not fixed
///
/// Fifty-five rule violations live in fixtures that predate this
/// evaluator. None is a defect a fixture exists to exhibit, and each is
/// left in place for a stated reason:
///
/// - upstream copies (`ifclite-geometry/`, `ifcopenshell-validate/`) are
///   not ours to edit; their provenance pins their bytes;
/// - generated fixtures (`synthetic-surfaces/`, `synthetic-coverage/`,
///   `synthetic-properties/`, `synthetic-systems/`) are regenerated from
///   their scripts, never edited, and regeneration draws new GlobalIds;
/// - the hand-authored ones (`synthetic-relations/`, `costing/`,
///   `hostile/`, two IFC4X3 surfaces) carry owner-history, true-north and
///   representation-type defects unrelated to what they test, and are
///   copied verbatim into the binding suites (`OpenBim.Ifc.Tests`), so an
///   edit reaches every language's tests.
///
/// The table keeps them visible: a fixture edit that fixes or adds a
/// violation fails here until the table says so.
#[test]
fn the_corpus_reports_exactly_its_pinned_findings() {
    if !spec_available() {
        return;
    }
    let found = rule_findings();
    let pinned: BTreeSet<(String, String, u64, String)> = EXPECTED
        .iter()
        .map(|(fixture, rule, entity, severity, _)| {
            (
                (*fixture).to_owned(),
                (*rule).to_owned(),
                *entity,
                (*severity).to_owned(),
            )
        })
        .collect();
    let unexpected: Vec<_> = found.difference(&pinned).collect();
    let missing: Vec<_> = pinned.difference(&found).collect();
    assert!(
        unexpected.is_empty() && missing.is_empty(),
        "unexpected: {unexpected:#?}\nmissing: {missing:#?}"
    );
    // The table is a guard only while it has rows: an evaluator that found
    // nothing would otherwise pass against an emptied table.
    assert!(pinned.len() >= 50, "{}", pinned.len());
    for (_, _, _, severity, agreement) in EXPECTED {
        let consistent = match agreement {
            Agreement::Agrees => *severity == "error",
            Agreement::Undecided => *severity == "evaluation-error",
            Agreement::Indeterminate => *severity == "indeterminate",
        };
        assert!(consistent, "{severity} cannot be {agreement:?}");
    }
}

/// Where the EXPRESS rules replace a hand-written rule, both reach the same
/// verdicts on the corpus.
#[test]
fn the_hand_written_rules_agree_with_the_express_rules_on_the_corpus() {
    if !spec_available() {
        return;
    }
    let mut differences = Vec::new();
    let mut compared = 0;
    for (name, model, version) in corpus() {
        let native = ifc_validate::validate_with(&model, schema(version), Budget::UNLIMITED);
        let express = evaluated(&model, version);
        for entry in ifc_validate::where_rule::implemented() {
            if !rules(version).covers(entry.id) || !entry.applies_to(schema(version)) {
                continue;
            }
            let verdicts = |report: &Report| {
                let mut found: Vec<String> = report
                    .findings()
                    .iter()
                    .filter(|f| f.rule == entry.id || f.rule == format!("global.{}", entry.id))
                    .map(|f| format!("{} {}", f.severity, f.path))
                    .collect();
                found.sort();
                found
            };
            compared += 1;
            let (hand, evaluated) = (verdicts(&native), verdicts(&express));
            if hand != evaluated {
                differences.push(format!("{name} {}: {hand:?} vs {evaluated:?}", entry.id));
            }
        }
    }
    assert!(compared > 100, "only {compared} comparisons");
    assert!(differences.is_empty(), "{differences:#?}");
}

/// Prints every EXPRESS rule finding for the `.ifc` files under
/// `$IFC_EXPRESS_CORPUS`, one tab-separated line each (file, rule, entity,
/// severity), for comparison with another validator.
#[test]
#[ignore = "reads an external corpus named by IFC_EXPRESS_CORPUS"]
fn external_corpus() {
    let Some(root) = std::env::var_os("IFC_EXPRESS_CORPUS") else {
        return;
    };
    for (name, model, version) in corpus_at(std::path::Path::new(&root)) {
        let report = evaluated(&model, version);
        for finding in report.sorted() {
            if !pinned_kind(finding) {
                continue;
            }
            let entity = match &finding.path {
                Path::Entity(id) | Path::Attribute { entity: id, .. } => id.0.to_string(),
                _ => String::new(),
            };
            println!(
                "TSV\t{name}\t{}\t{entity}\t{}",
                finding.rule, finding.severity
            );
        }
        println!("FILE\t{name}\t{}", report.summary());
    }
}
