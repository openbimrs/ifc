//! Executable inventory of the geometry-resource `WHERE` rules.
//!
//! `RULE-REG` asked for an inventory of every relevant EXPRESS `WHERE` rule
//! and its support state. Prose cannot answer "does the schema constrain
//! this?" -- only the schema can. `openbim-step` now parses rule labels and
//! expressions, so this file checks the committed inventory against what the
//! schema actually declares, and fails when the two drift apart.

mod where_rule_inventory {
    pub mod batches;
    pub mod cases;
}

use std::collections::{BTreeMap, BTreeSet};

use ifc_geometry::rules::validate_model;
use ifc_model::Model;

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
    const CURVE: &str = include_str!("../src/rules/curve.rs");
    const SCALAR: &str = include_str!("../src/rules/scalar.rs");
    const CARD: &str = include_str!("../src/rules/cardinality.rs");
    const TYPING: &str = include_str!("../src/rules/typing.rs");
    const SURFACE: &str = include_str!("../src/rules/surface.rs");
    for (entity, rule, state, _) in rows() {
        if state != "implemented" {
            continue;
        }
        // Both the rule label AND the entity type must appear in the same
        // module. Matching the bare label let one IfcBooleanResult.SameDim
        // implementation mark SameDim implemented for four other entities.
        let quoted = format!("\"{rule}\"");
        let typed = format!("\"{}\"", entity.to_ascii_uppercase());
        assert!(
            [PLACEMENT, SOLID, GRID, CURVE, SCALAR, CARD, TYPING, SURFACE]
                .iter()
                .any(|src| src.contains(&quoted) && src.contains(&typed)),
            "{entity}.{rule} claims implementation but no module names both it and its entity"
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

/// Every `implemented` row must actually fire on violating data and stay
/// silent on conforming data.
///
/// The other inventory tests prove the catalog matches the schema and that
/// a rules module names each implemented rule. Neither proves the check
/// works: a rule that never fires would satisfy both. This runs each one
/// against a conforming and a violating model and asserts both directions.
#[test]
fn implemented_rules_fire_on_violations_and_stay_silent_otherwise() {
    // Every implemented rule must have a case: otherwise this test only
    // proves whatever the author remembered to write.
    let covered: BTreeSet<(&str, &str)> =
        all_cases().into_iter().map(|(e, l, _, _)| (e, l)).collect();
    for (entity, rule, state, _) in rows() {
        if state == "implemented" {
            assert!(
                covered.contains(&(entity, rule)),
                "{entity}.{rule} is implemented but has no pass/fail case"
            );
        }
    }
    for (entity, label, good, bad) in all_cases() {
        let clean = validate_model(&good);
        assert!(
            !clean.iter().any(|v| v.rule == label),
            "{entity}.{label} fired on conforming data: {clean:?}"
        );
        let violations = validate_model(&bad);
        assert!(
            violations.iter().any(|v| v.rule == label),
            "{entity}.{label} did not fire on violating data: {violations:?}"
        );
    }
}

/// Every pass/fail pair, from both case modules.
fn all_cases() -> Vec<(&'static str, &'static str, Model, Model)> {
    let mut v = where_rule_inventory::cases::cases();
    v.extend(where_rule_inventory::batches::cases());
    v
}
