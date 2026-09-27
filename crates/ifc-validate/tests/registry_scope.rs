//! Every registered rule is scoped exactly as the normative EXPRESS declares
//! it.
//!
//! A rule entry names its declaring entity, its label and the releases it
//! runs under. Each of those was once wrong in a way no behavioural test
//! noticed: `NoSelfReference` was registered on the subtype
//! `IfcRelAssignsToGroupByFactor`, which declares nothing;
//! `NoRelatedTypeObject` ran under IFC2X3, which does not declare it; and
//! `IfcPolyLoop.WR21` was reported under IFC4, which calls the same
//! predicate `AllPointsSameDim`. This target reads the checked-out
//! `references/ifc-spec/*.exp` and requires, for every entry and every
//! bundled release, that the release declares the rule exactly when the
//! entry says it does.
//!
//! The schemas are CC BY-ND 4.0 and not committed, so the test skips without
//! them -- unless `IFC_SPEC_REQUIRED` is set, as `scripts/gate.sh` does.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use ifc_schema::SchemaVersion;
use ifc_validate::where_rule::RULES;

/// The normative file for each bundled release.
const RELEASES: [(SchemaVersion, &str); 3] = [
    (SchemaVersion::Ifc2x3, "ifc2x3-tc1/IFC2X3_TC1.exp"),
    (SchemaVersion::Ifc4, "ifc4-add2-tc1/IFC4.exp"),
    (SchemaVersion::Ifc4x3, "ifc4x3-add2/IFC4X3_ADD2.exp"),
];

fn spec_root() -> Option<PathBuf> {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    [
        "../../references/ifc-spec",
        "../../../../references/ifc-spec",
    ]
    .into_iter()
    .map(|rel| crate_dir.join(rel))
    .find(|path| path.is_dir())
}

/// What one release declares: WHERE labels and UNIQUE labels per entity
/// (upper-cased name), and global RULE names.
#[derive(Default)]
struct Declared {
    wheres: BTreeMap<String, BTreeSet<String>>,
    uniques: BTreeMap<String, BTreeSet<String>>,
    globals: BTreeSet<String>,
}

/// Reads the rule labels an EXPRESS schema declares.
///
/// A label is the identifier before a `:` that opens a line inside a WHERE
/// or UNIQUE clause; `:=:` and `:<>:` continuation lines are not labels.
fn declared(source: &str) -> Declared {
    #[derive(Clone, Copy, PartialEq)]
    enum Clause {
        Other,
        Where,
        Unique,
    }
    let mut out = Declared::default();
    let mut entity: Option<String> = None;
    let mut clause = Clause::Other;
    for line in source.lines() {
        let trimmed = line.trim();
        let upper = trimmed.to_ascii_uppercase();
        if let Some(rest) = upper.strip_prefix("ENTITY ") {
            let name = rest.trim_end_matches(';').split_whitespace().next();
            entity = name.map(str::to_owned);
            clause = Clause::Other;
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("RULE ") {
            if let Some(name) = rest.split_whitespace().next() {
                out.globals.insert(name.to_owned());
            }
            continue;
        }
        if upper.starts_with("END_ENTITY") {
            entity = None;
            continue;
        }
        let Some(current) = &entity else {
            continue;
        };
        match upper.as_str() {
            "WHERE" => {
                clause = Clause::Where;
                continue;
            }
            "UNIQUE" => {
                clause = Clause::Unique;
                continue;
            }
            "DERIVE" | "INVERSE" => {
                clause = Clause::Other;
                continue;
            }
            _ => {}
        }
        let Some((label, rest)) = trimmed.split_once(':') else {
            continue;
        };
        let label = label.trim();
        let is_label = !label.is_empty()
            && label.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
            && !rest.starts_with(['=', '<']);
        if !is_label {
            continue;
        }
        let target = match clause {
            Clause::Where => &mut out.wheres,
            Clause::Unique => &mut out.uniques,
            Clause::Other => continue,
        };
        target
            .entry(current.clone())
            .or_default()
            .insert(label.to_owned());
    }
    out
}

/// Whether `release` declares the rule `id` on `entity` (or globally).
fn declares(release: &Declared, id: &str, entity: Option<&str>) -> bool {
    match entity {
        Some(entity) => {
            let label = id
                .strip_prefix(entity)
                .and_then(|rest| rest.strip_prefix('.'))
                .unwrap_or_else(|| panic!("{id} is not `{entity}.<label>`"));
            release
                .wheres
                .get(&entity.to_ascii_uppercase())
                .is_some_and(|labels| labels.contains(label))
        }
        // `IfcRoot.UR1` is a UNIQUE clause, registered file-wide.
        None if id == "global.UniqueGlobalId" => release
            .uniques
            .get("IFCROOT")
            .is_some_and(|labels| labels.contains("UR1")),
        None => release
            .globals
            .contains(id.strip_prefix("global.").unwrap_or(id)),
    }
}

#[test]
fn every_registered_rule_runs_exactly_under_the_releases_that_declare_it() {
    let Some(root) = spec_root() else {
        assert!(
            std::env::var_os("IFC_SPEC_REQUIRED").is_none(),
            "IFC_SPEC_REQUIRED is set but references/ifc-spec was not found"
        );
        eprintln!("skipped: references/ifc-spec not present");
        return;
    };
    let mut mismatches = Vec::new();
    let mut checked = 0usize;
    for (version, file) in RELEASES {
        let bytes = std::fs::read(root.join(file)).unwrap_or_else(|e| panic!("{file}: {e}"));
        let source: String = bytes.iter().map(|&byte| byte as char).collect();
        let release = declared(&source);
        // The parser is itself a guard: one that found nothing would make
        // every "not declared" claim pass vacuously.
        assert!(
            release.wheres.len() > 200 && release.globals.len() >= 2,
            "{file}: parsed only {} entities with WHERE rules",
            release.wheres.len()
        );
        for entry in RULES {
            let claims = entry.releases.contains(&version);
            let actual = declares(&release, entry.id, entry.entity);
            if claims != actual {
                mismatches.push(format!(
                    "{} under {version:?}: registry says {claims}, {file} says {actual}",
                    entry.id
                ));
            }
            checked += 1;
        }
    }
    assert!(
        checked >= 60,
        "only {checked} (rule, release) pairs checked"
    );
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
}

/// The label parser reads WHERE and UNIQUE labels and nothing else.
#[test]
fn the_label_parser_reads_only_labels() {
    let source = "\
ENTITY IfcX
 SUBTYPE OF (IfcY);
\tA : IfcLabel;
 UNIQUE
\tUR1 : A;
 WHERE
\tWR1 : SIZEOF(QUERY(Temp <* B |
     Temp :=: SELF)) = 0;
\tNamed : A :<>: B;
END_ENTITY;
RULE IfcOne FOR (IfcX);
END_RULE;
";
    let parsed = declared(source);
    let wheres: Vec<&str> = parsed.wheres["IFCX"].iter().map(String::as_str).collect();
    assert_eq!(wheres, ["Named", "WR1"]);
    assert!(parsed.uniques["IFCX"].contains("UR1"));
    assert!(parsed.globals.contains("IfcOne"));
}
