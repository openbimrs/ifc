//! `remove_product` and `purge` (#478), the shape every host binds, over
//! the IFC2X3, IFC4 and IFC4X3 cascade fixtures. The facade's own suite
//! (`openbim-ifc/tests/cascade_delete.rs`) checks the cascade entity by
//! entity; the hosts' suites only check their conversion.

use openbim_ifc_binding_core::{IfcModel, PurgeOptions, RemoveProductOptions};

// Only `code` is used here.
#[allow(unused_imports)]
mod support;
use support::code;

fn fixture(name: &str) -> IfcModel {
    let path = format!(
        "{}/../../test/fixtures/synthetic-relations/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    IfcModel::open(std::path::Path::new(&path)).expect("fixture reads")
}

/// The fixtures this build bundles a release for.
fn fixtures() -> Vec<IfcModel> {
    let mut out = Vec::new();
    if cfg!(feature = "ifc2x3") {
        out.push(fixture("cascade_ifc2x3.ifc"));
    }
    if cfg!(feature = "ifc4") {
        out.push(fixture("cascade_ifc4.ifc"));
    }
    if cfg!(feature = "ifc4x3") {
        out.push(fixture("cascade_ifc4x3.ifc"));
    }
    assert!(!out.is_empty(), "every build bundles a release");
    out
}

#[cfg(not(feature = "author"))]
#[test]
fn removal_without_the_feature_refuses() {
    for mut model in fixtures() {
        assert_eq!(
            code(model.remove_product(20, RemoveProductOptions::new())),
            "feature-disabled"
        );
        assert_eq!(code(model.purge(PurgeOptions::new())), "feature-disabled");
    }
}

#[cfg(feature = "author")]
mod enabled {
    use super::*;
    use openbim_ifc_binding_core::{Field, ToRecord};

    fn options(rebase: bool, untype: bool, dry_run: bool) -> RemoveProductOptions {
        let mut options = RemoveProductOptions::new();
        options.rebase_placements = rebase;
        options.untype_occurrences = untype;
        options.dry_run = dry_run;
        options
    }

    #[test]
    fn a_dry_run_reports_and_changes_nothing() {
        for mut model in fixtures() {
            let before = model.len();
            let report = model
                .remove_product(100, options(false, false, true))
                .unwrap();
            assert!(!report.applied);
            assert_eq!(model.len(), before);
            let removed: Vec<u64> = report.removed.iter().map(|e| e.id).collect();
            assert_eq!(
                removed,
                [100, 101, 102, 103, 104, 105, 106, 107, 108, 109, 120]
            );
            let map = report.kept.iter().find(|e| e.id == 112).unwrap();
            assert_eq!(
                (map.reason.as_str(), map.users.as_slice()),
                ("shared", &[111][..])
            );
            assert_eq!(map.type_name, "IFCREPRESENTATIONMAP");

            let applied = model
                .remove_product(100, options(false, false, false))
                .unwrap();
            assert!(applied.applied);
            assert_eq!(applied.removed, report.removed);
            assert_eq!(model.len(), before - 11);
            assert!(model.dangling_references().is_empty());
        }
    }

    #[test]
    fn refusals_have_their_codes() {
        for mut model in fixtures() {
            assert_eq!(
                code(model.remove_product(20, options(false, false, false))),
                "still-referenced"
            );
            assert_eq!(
                code(model.remove_product(66, options(false, false, false))),
                "still-referenced"
            );
            assert_eq!(
                code(model.remove_product(61, options(false, false, false))),
                "wrong-entity-type"
            );
            assert_eq!(
                code(model.remove_product(9999, options(false, false, false))),
                "missing-entity"
            );
            let mut unknown = PurgeOptions::new();
            unknown.classes = vec!["IfcNoSuchThing".into()];
            assert_eq!(code(model.purge(unknown)), "unsupported-schema");

            let report = model
                .remove_product(20, options(true, false, false))
                .unwrap();
            assert_eq!(report.rebased, [101, 131]);
            let report = model
                .remove_product(66, options(false, true, false))
                .unwrap();
            assert_eq!(report.removed.len(), 4);
            assert!(model.dangling_references().is_empty());
        }
    }

    #[test]
    fn a_purge_by_class_and_by_default() {
        for mut model in fixtures() {
            let mut psets = PurgeOptions::new();
            psets.classes = vec!["IfcPropertySet".into()];
            psets.dry_run = true;
            let report = model.purge(psets.clone()).unwrap();
            assert!(report.removed.is_empty());
            assert_eq!(report.kept[0].id, 173);
            assert_eq!(report.kept[0].reason, "protected");

            psets.include_roots = true;
            psets.dry_run = false;
            assert_eq!(model.purge(psets).unwrap().removed[0].id, 173);
            let report = model.purge(PurgeOptions::new()).unwrap();
            let removed: Vec<u64> = report.removed.iter().map(|e| e.id).collect();
            assert_eq!(removed, [170, 171, 172, 174]);
        }
    }

    #[test]
    fn the_report_crosses_as_a_record() {
        let mut model = fixtures().remove(0);
        let record = model
            .remove_product(150, options(false, false, true))
            .unwrap()
            .to_record();
        assert_eq!(record.name, "RemovalReport");
        assert_eq!(record.get("applied"), Some(&Field::Bool(false)));
        let Some(Field::List(removed)) = record.get("removed") else {
            panic!("removed is a list");
        };
        let Field::Record(first) = &removed[0] else {
            panic!("of records");
        };
        assert_eq!(first.name, "RemovedEntity");
        assert_eq!(first.get("id"), Some(&Field::Id(150)));
    }
}
