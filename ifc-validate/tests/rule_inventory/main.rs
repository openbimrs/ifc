//! Every rule id this crate can emit is pinned by an adversarial pair.
//!
//! A finding's `rule` is the contract callers filter and suppress on, so an
//! id that no test provokes can silently stop firing, or start firing on
//! legal input, without anything going red. This target closes that gap in
//! two halves:
//!
//! - an **inventory**: the rule ids are read out of the crate's own source,
//!   so a new `Finding::error("...")` cannot ship without a case here;
//! - an **adversarial pair** per id: one fixture that must produce it and a
//!   minimally different one that must not.
//!
//! Rules registered as unsupported are not verdicts; they are covered
//! generically from the registry, so a newly registered gap is exercised
//! without a hand-written case.

mod cases;
mod fixtures;
mod rules;
mod structure;
mod types;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use ifc_model::{Entity, Model};
use ifc_schema::SchemaVersion;
use ifc_validate::where_rule::{self, Support};
use ifc_validate::{validate, Severity};

use cases::Case;

/// Every rule id literal in the crate's non-test source.
///
/// Test modules are cut at their `#[cfg(test)]` marker: they construct
/// findings with made-up ids to exercise sorting and rendering.
fn source_rule_ids() -> BTreeSet<String> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    rust_files(&root, &mut files);
    let mut ids = BTreeSet::new();
    for file in files {
        let text = std::fs::read_to_string(&file)
            .unwrap_or_else(|error| panic!("{}: {error}", file.display()));
        let production = text.split("#[cfg(test)]").next().unwrap_or_default();
        // Comments are prose, and an unpaired quote in one would shift every
        // literal after it; a `'"'` char literal would do the same.
        let code: String = production
            .lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n")
            .replace("'\"'", "''");
        ids.extend(
            string_literals(&code)
                .into_iter()
                .filter(|literal| looks_like_rule_id(literal)),
        );
    }
    ids
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .unwrap_or_else(|error| panic!("{}: {error}", dir.display()))
        .flatten()
        .map(|entry| entry.path())
        .collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

/// The contents of every `"..."` literal, ignoring escapes' meaning.
fn string_literals(source: &str) -> Vec<String> {
    let mut literals = Vec::new();
    let mut chars = source.chars();
    while let Some(c) = chars.next() {
        if c != '"' {
            continue;
        }
        let mut literal = String::new();
        let mut escaped = false;
        for c in chars.by_ref() {
            if escaped {
                escaped = false;
                literal.push(c);
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                break;
            } else {
                literal.push(c);
            }
        }
        literals.push(literal);
    }
    literals
}

/// Whether a literal has the shape of a finding rule id.
///
/// Two families: dotted category ids (`structure.reference.dangling`) and
/// schema rule ids (`IfcMaterialLayer.NormalizedPriority`).
fn looks_like_rule_id(literal: &str) -> bool {
    let word = |part: &str| {
        !part.is_empty() && part.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
    };
    let parts: Vec<&str> = literal.split('.').collect();
    if parts.len() < 2 || !parts.iter().all(|part| word(part)) {
        return false;
    }
    let category = ["header", "structure", "type", "global"].contains(&parts[0]);
    let schema_rule = parts.len() == 2 && parts[0].starts_with("Ifc");
    category || schema_rule
}

/// Ids that carry a verdict on the file, and so need an adversarial pair.
fn verdict_rule_ids() -> BTreeSet<String> {
    let unsupported: BTreeSet<&str> = where_rule::unsupported().map(|entry| entry.id).collect();
    let mut ids = source_rule_ids();
    ids.extend(where_rule::implemented().map(|entry| entry.id.to_string()));
    ids.retain(|id| !unsupported.contains(id.as_str()));
    ids
}

/// The scanner itself is a guard, so it is checked against known ids and a
/// floor: a scanner that silently matched nothing would make the inventory
/// test pass vacuously.
#[test]
fn the_source_scan_finds_the_rule_ids_it_must() {
    let ids = source_rule_ids();
    for known in [
        "header.schema.missing",
        "structure.reference.dangling",
        "structure.unique.duplicate_global_id",
        "type.enumeration.member",
        "global.UniqueGlobalId",
        "IfcRelSpaceBoundary.CorrectPhysOrVirt",
    ] {
        assert!(ids.contains(known), "scanner missed {known}: {ids:?}");
    }
    assert!(ids.len() >= 35, "only {} ids found: {ids:?}", ids.len());
    assert!(looks_like_rule_id("structure.required.missing"));
    assert!(!looks_like_rule_id("2;1"));
    assert!(!looks_like_rule_id("IFCWALL"));
}

/// No rule id ships without a case, and no case pins an id nothing emits.
#[test]
fn every_emitted_rule_id_has_an_adversarial_pair() {
    let emitted = verdict_rule_ids();
    let covered: BTreeSet<String> = cases::all()
        .iter()
        .map(|case| case.rule.to_string())
        .collect();
    let untested: Vec<_> = emitted.difference(&covered).collect();
    assert!(
        untested.is_empty(),
        "rule ids without a failing+passing case in tests/rule_inventory: {untested:?}"
    );
    let stale: Vec<_> = covered.difference(&emitted).collect();
    assert!(
        stale.is_empty(),
        "cases pin rule ids the crate no longer emits: {stale:?}"
    );
}

/// Each case's failing fixture produces its rule id as a verdict, and its
/// passing fixture does not produce it at all.
#[test]
fn every_case_fires_on_its_failing_fixture_only() {
    let mut failures = Vec::new();
    for Case {
        rule,
        form,
        fails,
        passes,
    } in cases::all()
    {
        let failing = fails();
        let fired: Vec<_> = failing
            .findings()
            .iter()
            .filter(|finding| finding.rule == *rule)
            .collect();
        if !fired
            .iter()
            .any(|finding| matches!(finding.severity, Severity::Error | Severity::Warning))
        {
            failures.push(format!(
                "{rule} ({form}): failing fixture did not produce it; got {:#?}",
                failing.findings()
            ));
        }
        let passing = passes();
        let spurious: Vec<_> = passing
            .findings()
            .iter()
            .filter(|finding| finding.rule == *rule)
            .collect();
        if !spurious.is_empty() {
            failures.push(format!(
                "{rule} ({form}): passing fixture produced it: {spurious:#?}"
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// Every unsupported rule is admitted when the file could trip it, under
/// each release that declares it, and is quiet when the file holds nothing
/// it constrains or its release does not declare it.
///
/// Driven from the registry, so a newly registered gap is covered without a
/// hand-written case.
#[test]
fn every_unsupported_rule_is_reported_exactly_where_it_applies() {
    let mut checked = 0usize;
    for entry in where_rule::unsupported() {
        let Support::Unsupported(reason) = entry.support else {
            unreachable!("unsupported() yields only unsupported entries");
        };
        for version in [
            SchemaVersion::Ifc2x3,
            SchemaVersion::Ifc4,
            SchemaVersion::Ifc4x3,
        ] {
            let schema = ifc_schema::for_version(version).expect("bundled tables");
            let declared = entry.releases.contains(&version);
            let reported = |model: &Model| {
                validate(model, schema)
                    .findings()
                    .iter()
                    .filter(|finding| finding.rule == entry.id)
                    .map(|finding| (finding.severity, finding.message.clone()))
                    .collect::<Vec<_>>()
            };
            let admitted = [(Severity::Unsupported, reason.to_string())];
            let empty = Model::new();
            let mut model = Model::new();
            if let Some(entity) = entry.entity {
                model.push(Entity::new(entity.to_ascii_uppercase(), Vec::new()));
                assert!(
                    reported(&empty).is_empty(),
                    "{} is noise in a file without {entity}",
                    entry.id
                );
            }
            if declared {
                assert_eq!(
                    reported(&model),
                    admitted,
                    "{} must be admitted under {version:?}",
                    entry.id
                );
            } else {
                assert!(
                    reported(&model).is_empty(),
                    "{} is not declared by {version:?} and must not be reported",
                    entry.id
                );
            }
        }
        checked += 1;
    }
    assert!(checked >= 5, "only {checked} unsupported rules exercised");
}
