//! Executable audit of all IFC4 ADD2 TC1 geometry-resource declarations.

use std::collections::BTreeSet;

use ifc_geometry::resource::functions::FUNCTIONS;

const MANIFEST: &str = include_str!("../references/ifc4-add2-tc1-geometry-declarations.tsv");
const SUPPORT: &str = include_str!("../references/ifc4-add2-tc1-geometry-support.tsv");

fn rows() -> impl Iterator<Item = [&'static str; 5]> {
    MANIFEST.lines().skip(1).map(|line| {
        let mut fields = line.split('\t');
        [
            fields.next().expect("resource"),
            fields.next().expect("kind"),
            fields.next().expect("name"),
            fields.next().expect("abstract"),
            fields.next().expect("type kind"),
        ]
    })
}

#[test]
fn normative_manifest_has_all_163_declarations() {
    let rows: Vec<_> = rows().collect();
    assert_eq!(rows.len(), 163);
    assert_eq!(rows.iter().filter(|row| row[1] == "entity").count(), 112);
    assert_eq!(rows.iter().filter(|row| row[1] == "type").count(), 23);
    assert_eq!(rows.iter().filter(|row| row[4] == "select").count(), 13);
    assert_eq!(rows.iter().filter(|row| row[4] == "enum").count(), 7);
    assert_eq!(rows.iter().filter(|row| row[4] == "defined").count(), 3);
    assert_eq!(rows.iter().filter(|row| row[1] == "function").count(), 28);
    assert_eq!(
        rows.iter()
            .filter(|row| row[1] == "entity" && row[3] == "true")
            .count(),
        23
    );
    assert_eq!(
        rows.iter()
            .filter(|row| row[1] == "entity" && row[3] == "false")
            .count(),
        89
    );

    let unique: BTreeSet<_> = rows.iter().map(|row| (row[0], row[1], row[2])).collect();
    assert_eq!(
        unique.len(),
        rows.len(),
        "duplicate declaration in manifest"
    );
}

#[test]
fn every_declaration_has_bridge_and_neutral_ownership() {
    let expected_details: std::collections::BTreeMap<_, _> = rows()
        .map(|row| ((row[0], row[1], row[2]), (row[3], row[4])))
        .collect();
    let expected: BTreeSet<_> = expected_details.keys().copied().collect();
    let mut actual = BTreeSet::new();
    let mut statuses = std::collections::BTreeMap::<&str, usize>::new();
    let source_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");

    for line in SUPPORT.lines().skip(1) {
        let fields: Vec<_> = line.split('\t').collect();
        assert_eq!(fields.len(), 8, "malformed support row: {line}");
        let key = (fields[0], fields[1], fields[2]);
        assert!(actual.insert(key), "duplicate support row: {key:?}");
        assert_eq!(
            (fields[3], fields[4]),
            *expected_details
                .get(&key)
                .expect("key comes from normative set"),
            "classification drift for {key:?}"
        );
        assert!(
            !fields[5].trim().is_empty(),
            "missing bridge owner: {key:?}"
        );
        let owner_path = fields[5].replace("::", "/");
        assert!(
            source_root.join(format!("{owner_path}.rs")).is_file()
                || source_root.join(&owner_path).join("mod.rs").is_file(),
            "bridge owner module does not exist for {key:?}: {}",
            fields[5]
        );
        assert!(
            !fields[6].trim().is_empty(),
            "missing neutral owner: {key:?}"
        );
        assert!(
            matches!(
                fields[6],
                "axiolid-core"
                    | "axiolid-mesh"
                    | "axiolid-model"
                    | "axiolid-model::SolidOperation"
                    | "axiolid-topology"
                    | "axiolid-curve + axiolid-model::CurveRelation"
                    | "axiolid-surface + axiolid-model::SurfaceRelation"
                    | "ifc-geometry"
                    | "ifc-geometry::constraint + axiolid-model::NodeId"
            ),
            "unknown neutral capability owner `{}` for {key:?}",
            fields[6]
        );
        assert!(
            matches!(
                fields[7],
                "inventory" | "view-or-family" | "modeled-type" | "native-primitive" | "scaffolded"
            ),
            "unknown support status in {key:?}: {}",
            fields[7]
        );
        *statuses.entry(fields[7]).or_default() += 1;
    }

    assert_eq!(actual, expected, "ownership ledger drifted from IFC4");
    assert_eq!(statuses.get("inventory"), Some(&23));
    assert_eq!(statuses.get("view-or-family"), Some(&89));
    assert_eq!(statuses.get("modeled-type"), Some(&23));
    assert_eq!(statuses.get("native-primitive"), Some(&6));
    assert_eq!(statuses.get("scaffolded"), Some(&22));
}

#[test]
fn all_normative_functions_have_exactly_one_owner() {
    let expected: BTreeSet<_> = rows()
        .filter(|row| row[1] == "function")
        .map(|row| row[2].to_ascii_lowercase())
        .collect();
    let actual: BTreeSet<_> = FUNCTIONS
        .iter()
        .map(|support| support.name.to_ascii_lowercase())
        .collect();
    assert_eq!(actual, expected, "function registry drifted from IFC4");
    assert_eq!(FUNCTIONS.len(), 28, "duplicate function owner");
    for support in FUNCTIONS {
        assert!(
            !support.owner.trim().is_empty(),
            "{} has no owner",
            support.name
        );
        if !support.owner.starts_with("axiolid_core::") {
            let owner_path = support.owner.replace("::", "/");
            let source_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
            assert!(
                source_root.join(format!("{owner_path}.rs")).is_file()
                    || source_root.join(&owner_path).join("mod.rs").is_file(),
                "function owner module is missing: {} -> {}",
                support.name,
                support.owner
            );
        }
    }
}
