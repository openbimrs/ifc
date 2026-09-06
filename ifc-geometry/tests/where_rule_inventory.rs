//! Executable inventory of the geometry-resource `WHERE` rules.
//!
//! `RULE-REG` asked for an inventory of every relevant EXPRESS `WHERE` rule
//! and its support state. Prose cannot answer "does the schema constrain
//! this?" -- only the schema can. `openbim-step` now parses rule labels and
//! expressions, so this file checks the committed inventory against what the
//! schema actually declares, and fails when the two drift apart.

use std::collections::{BTreeMap, BTreeSet};

const INVENTORY: &str = include_str!("../data/ifc4-where-rules.tsv");
const DECLARATIONS: &str = include_str!("../data/ifc4-add2-tc1-geometry-declarations.tsv");

/// Rows as `(entity, rule, state, expression)`.
fn rows() -> impl Iterator<Item = (&'static str, &'static str, &'static str, &'static str)> {
    INVENTORY
        .lines()
        .skip(1)
        .filter(|line| !line.is_empty())
        .map(|line| {
            let mut f = line.splitn(4, '\t');
            (
                f.next().expect("entity"),
                f.next().expect("rule"),
                f.next().expect("state"),
                f.next().expect("expression"),
            )
        })
}

/// The geometry-resource entities the inventory is scoped to.
fn geometry_entities() -> BTreeSet<String> {
    DECLARATIONS
        .lines()
        .skip(1)
        .filter_map(|line| {
            let mut f = line.split('\t');
            let _resource = f.next()?;
            let kind = f.next()?;
            let name = f.next()?;
            (kind == "entity").then(|| name.to_ascii_lowercase())
        })
        .collect()
}

/// The inventory must match the schema exactly: no invented rules, none missed.
///
/// This is the check that makes the inventory evidence rather than a claim.
/// A rule added, renamed or removed by a schema repin fails here instead of
/// silently leaving the catalogue stale -- the failure mode that made five
/// separate refusal rationales outlive their cause.
#[test]
fn the_inventory_matches_the_schema_exactly() {
    let scope = geometry_entities();
    let schema = ifc_schema::ifc4();
    let mut declared: BTreeSet<(String, String)> = BTreeSet::new();
    for name in schema.entity_names() {
        let lower = name.to_ascii_lowercase();
        if !scope.contains(&lower) {
            continue;
        }
        let entity = schema.entity(name).expect("named entity resolves");
        for rule in &entity.where_rules {
            declared.insert((lower.clone(), rule.label.clone()));
        }
    }

    let listed: BTreeSet<(String, String)> = rows()
        .map(|(entity, rule, _, _)| (entity.to_owned(), rule.to_owned()))
        .collect();

    let missing: Vec<_> = declared.difference(&listed).collect();
    let invented: Vec<_> = listed.difference(&declared).collect();
    assert!(
        missing.is_empty(),
        "schema declares rules the inventory omits: {missing:?}"
    );
    assert!(
        invented.is_empty(),
        "inventory lists rules the schema does not declare: {invented:?}"
    );
}

/// Every stored expression is the schema's own text, not a paraphrase.
#[test]
fn stored_expressions_match_the_schema_text() {
    let schema = ifc_schema::ifc4();
    let mut by_key: BTreeMap<(String, String), String> = BTreeMap::new();
    for name in schema.entity_names() {
        let entity = schema.entity(name).expect("named entity resolves");
        for rule in &entity.where_rules {
            by_key.insert(
                (name.to_ascii_lowercase(), rule.label.clone()),
                rule.expression.clone(),
            );
        }
    }
    for (entity, rule, _, expression) in rows() {
        let key = (entity.to_owned(), rule.to_owned());
        let actual = by_key.get(&key).expect("inventoried rule exists");
        assert_eq!(actual, expression, "{entity}.{rule} text drifted");
    }
}

/// `implemented` means the rules module names that rule; nothing else counts.
///
/// Without this, a row could claim support that no code provides -- the exact
/// gap between a documented capability and a real one.
#[test]
fn implemented_rows_are_named_by_the_rules_module() {
    const PLACEMENT: &str = include_str!("../src/rules/placement.rs");
    const SOLID: &str = include_str!("../src/rules/solid.rs");
    const GRID: &str = include_str!("../src/rules/grid.rs");
    for (entity, rule, state, _) in rows() {
        if state != "implemented" {
            continue;
        }
        let quoted = format!("\"{rule}\"");
        assert!(
            PLACEMENT.contains(&quoted) || SOLID.contains(&quoted) || GRID.contains(&quoted),
            "{entity}.{rule} claims implementation but no rules module names it"
        );
    }
}

/// Support state is a closed vocabulary.
#[test]
fn every_row_declares_a_known_state() {
    for (entity, rule, state, expression) in rows() {
        assert!(
            matches!(state, "implemented" | "inventoried"),
            "{entity}.{rule} has unknown state {state}"
        );
        assert!(!expression.is_empty(), "{entity}.{rule} has no expression");
    }
}
