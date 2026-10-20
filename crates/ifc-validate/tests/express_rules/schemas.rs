//! Every bundled release's EXPRESS text loads, pairs with its tables, and
//! is fully evaluable.

use ifc_validate::express::{ExpressError, ExpressRules};

use super::{rules, schema, spec_available, RELEASES};

/// Each release parses, resolves every name and agrees with its bundled
/// tables on every entity's slots.
#[test]
fn every_release_loads_against_its_own_tables() {
    if !spec_available() {
        return;
    }
    for (version, _) in RELEASES {
        let rules = rules(version);
        assert!(
            rules
                .schema_name()
                .eq_ignore_ascii_case(schema(version).name()),
            "{version:?}"
        );
        assert!(rules.rule_ids().count() > 200, "{version:?}");
    }
}

/// Every rule, type rule, global rule, derived attribute and function of
/// every release is evaluable: nothing the IFC schemas use is refused.
#[test]
fn every_declaration_of_every_release_is_evaluable() {
    if !spec_available() {
        return;
    }
    for (version, _) in RELEASES {
        let coverage = rules(version).coverage();
        assert!(
            coverage.not_evaluable.is_empty(),
            "{version:?}: {:?}",
            coverage.not_evaluable
        );
        assert_eq!(coverage.entity_rules.total, coverage.entity_rules.evaluable);
        assert!(
            coverage.entity_rules.total > 200,
            "{version:?}: {coverage:?}"
        );
        assert_eq!(coverage.global_rules.total, 2, "{version:?}");
        assert!(coverage.functions.total >= 38, "{version:?}: {coverage:?}");
    }
}

/// Rule text is never paired with another release's tables.
#[test]
fn the_text_of_one_release_is_refused_for_another() {
    if !spec_available() {
        return;
    }
    let ifc4 = std::fs::read(
        super::spec_root()
            .expect("present")
            .join("ifc4-add2-tc1/IFC4.exp"),
    )
    .expect("reads");
    let refused = ExpressRules::parse_bytes(&ifc4, ifc_schema::ifc4x3())
        .expect_err("IFC4 text for IFC4X3 tables");
    assert!(
        matches!(refused, ExpressError::SchemaMismatch { .. }),
        "{refused}"
    );
}

/// The committed coverage table, regenerated from the normative schemas.
const COVERAGE: &str = "data/express-coverage.tsv";

/// `data/express-coverage.tsv`, which the docs' coverage page is generated
/// from, states what the evaluator covers of every release: counts and
/// names only, no rule text. `UPDATE_EXPRESS_COVERAGE=1` rewrites it.
#[test]
fn the_committed_coverage_table_is_current() {
    if !spec_available() {
        return;
    }
    let mut table = String::from(
        "release\tentity_rules\tentity_rules_evaluable\ttype_rules\ttype_rules_evaluable\t\
         global_rules\tglobal_rules_evaluable\tfunctions\tfunctions_evaluable\tderived\t\
         derived_evaluable\tnot_evaluable\n",
    );
    for (version, _) in RELEASES {
        let c = rules(version).coverage();
        let missing: Vec<&str> = c
            .not_evaluable
            .iter()
            .map(|(name, _)| name.as_str())
            .collect();
        table.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            version.release_id(),
            c.entity_rules.total,
            c.entity_rules.evaluable,
            c.type_rules.total,
            c.type_rules.evaluable,
            c.global_rules.total,
            c.global_rules.evaluable,
            c.functions.total,
            c.functions.evaluable,
            c.derived.total,
            c.derived.evaluable,
            if missing.is_empty() {
                "-".to_owned()
            } else {
                missing.join(",")
            },
        ));
    }
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(COVERAGE);
    if std::env::var_os("UPDATE_EXPRESS_COVERAGE").is_some() {
        std::fs::write(&path, &table).expect("writes the table");
    }
    let committed = std::fs::read_to_string(&path).unwrap_or_default();
    assert_eq!(
        committed, table,
        "{COVERAGE} is stale; rerun with UPDATE_EXPRESS_COVERAGE=1, then `cargo run -p xtask -- docs`"
    );
}
