//! Copying a product, the copy operations in the tape form every host
//! converts to, and appending from another model (#479), over the cascade
//! fixtures. The facade's suites (`openbim-ifc/tests/copy_product.rs`,
//! `append_model.rs`) check the copies entity by entity; this one checks
//! the conversion, the report and the refusal codes.

mod support;

use openbim_ifc_binding_core::IfcModel;
use support::code;

fn fixture(name: &str) -> IfcModel {
    let path = format!(
        "{}/../../test/fixtures/synthetic-relations/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    IfcModel::open(std::path::Path::new(&path)).expect("fixture reads")
}

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
fn copies_without_the_feature_refuse() {
    use openbim_ifc_binding_core::{AppendOptions, CopyProductOptions};
    for mut model in fixtures() {
        assert_eq!(
            code(model.copy_product(20, CopyProductOptions::new())),
            "feature-disabled"
        );
        let source = fixtures().remove(0);
        assert_eq!(
            code(model.append(&source, &[20], &AppendOptions::new())),
            "feature-disabled"
        );
    }
}

#[cfg(feature = "author")]
mod enabled {
    use super::*;
    use openbim_ifc_binding_core::value::Tagged;
    use openbim_ifc_binding_core::{AppendOptions, CopyProductOptions, ToRecord};
    use support::{op, reals, text};

    #[test]
    fn copy_product_reports_and_a_dry_run_changes_nothing() {
        for mut model in fixtures() {
            let mut dry = CopyProductOptions::new();
            dry.dry_run = true;
            let planned = model.copy_product(20, dry).expect("plans");
            assert!(!planned.applied);
            assert!(model
                .attribute_by_name(planned.copy.unwrap(), "Name")
                .is_err());
            let report = model
                .copy_product(20, CopyProductOptions::new())
                .expect("copies");
            assert!(report.applied);
            let wall = report.copy.unwrap();
            assert_eq!(
                model.attribute_by_name(wall, "Name").unwrap(),
                text("Wall A")
            );
            assert!(report.copied.iter().any(|c| c.original == 100), "the door");
            assert!(report.joined.contains(&67), "the type assignment");
            let record = report.to_record();
            assert_eq!(record.name, "CopyReport");
            let names: Vec<&str> = record.fields.iter().map(|(name, _)| *name).collect();
            assert_eq!(
                names,
                [
                    "copy",
                    "copied",
                    "reused",
                    "joined",
                    "skipped",
                    "regenerated",
                    "applied"
                ]
            );
        }
    }

    #[test]
    fn a_seeded_copy_collides_with_its_seed_and_not_with_another() {
        for mut model in fixtures() {
            model
                .copy_product_seeded(20, CopyProductOptions::new(), 548)
                .expect("copies");
            let before = model.write().unwrap();
            let error = model
                .copy_product_seeded(20, CopyProductOptions::new(), 548)
                .unwrap_err();
            assert_eq!(error.code(), "invalid-value");
            assert_eq!(model.write().unwrap(), before);
            model
                .copy_product_seeded(20, CopyProductOptions::new(), 549)
                .expect("another seed copies");
        }
    }

    #[test]
    fn copy_product_options_reach_the_facade() {
        for mut model in fixtures() {
            let mut options = CopyProductOptions::new();
            options.openings = false;
            options.container = false;
            options.offset = Some([0.0, 3.0, 0.0]);
            let report = model.copy_product(20, options).expect("copies");
            assert!(report.skipped.contains(&91) && report.skipped.contains(&19));
            assert!(!report.copied.iter().any(|c| c.original == 81));
            options.placement = Some(18);
            assert_eq!(code(model.copy_product(20, options)), "invalid-value");
            assert_eq!(
                code(model.copy_product(61, CopyProductOptions::new())),
                "wrong-entity-type"
            );
            assert_eq!(
                code(model.copy_product(9999, CopyProductOptions::new())),
                "missing-entity"
            );
        }
    }

    #[test]
    fn copy_operations_read_from_the_tape() {
        for mut model in fixtures() {
            let result = model
                .author(vec![
                    op(
                        "copyProduct",
                        &[
                            ("product", Tagged::Ref(20)),
                            ("offset", reals(&[1.0, 0.0, 0.0])),
                            ("fillings", Tagged::Bool(false)),
                        ],
                    ),
                    op(
                        "copy_entity",
                        &[
                            ("entity", Tagged::Ref(26)),
                            ("share", Tagged::List(vec![text("IfcProfileDef")])),
                        ],
                    ),
                ])
                .expect("copies");
            let solid = result.ids[1].unwrap();
            assert_eq!(
                model.attribute_by_name(solid, "SweptArea").unwrap(),
                Tagged::Ref(27)
            );
            let refused = model.author(vec![op(
                "copy_entity",
                &[
                    ("entity", Tagged::Ref(26)),
                    ("copy", Tagged::List(vec![text("IfcNoSuchThing")])),
                ],
            )]);
            assert_eq!(code(refused), "unsupported-schema");
        }
    }

    #[test]
    fn append_merges_regenerates_and_refuses() {
        if !cfg!(all(feature = "ifc4", feature = "ifc2x3")) {
            return;
        }
        let source = fixture("cascade_ifc4.ifc");
        let mut target = fixture("cascade_ifc4.ifc");
        assert_eq!(
            code(target.append(&source, &[66], &AppendOptions::new())),
            "invalid-value",
            "a kept GlobalId the target holds"
        );
        let mut merge = AppendOptions::new();
        merge.global_ids = "merge".into();
        let report = target.append(&source, &[66], &merge).expect("merges");
        assert!(report.copied.is_empty());
        assert_eq!(report.reused[0].copy, 66);
        let mut regenerate = AppendOptions::new();
        regenerate.global_ids = "regenerate".into();
        regenerate.dry_run = true;
        let planned = target.append(&source, &[20], &regenerate).expect("plans");
        assert!(!planned.applied && planned.skipped.contains(&19));
        regenerate.dry_run = false;
        let report = target.append(&source, &[20], &regenerate).expect("appends");
        assert!(report.applied && !report.regenerated.is_empty());
        let mut odd = AppendOptions::new();
        odd.global_ids = "shuffle".into();
        assert_eq!(code(target.append(&source, &[20], &odd)), "invalid-value");
        let mut old = fixture("cascade_ifc2x3.ifc");
        assert_eq!(
            code(old.append(&source, &[111], &regenerate)),
            "unsupported-schema"
        );
    }
}
