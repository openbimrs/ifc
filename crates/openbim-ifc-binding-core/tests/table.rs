//! Tables of values by path every host binds (#474), over the selector
//! fixtures this build bundles: export, an unchanged import that writes
//! nothing, a dry run, an applied edit, and refusals as data with nothing
//! written. The facade's suite covers the semantics; this one the
//! binding's answers and codes.

use openbim_ifc_binding_core::record::ToRecord;
use openbim_ifc_binding_core::{BindingError, IfcModel, TableImport};

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
        out.push(fixture("selector_ifc2x3.ifc"));
    }
    if cfg!(feature = "ifc4") {
        out.push(fixture("selector_ifc4.ifc"));
    }
    if cfg!(feature = "ifc4x3") {
        out.push(fixture("selector_ifc4x3.ifc"));
    }
    out
}

fn columns(paths: &[&str]) -> Vec<String> {
    paths.iter().map(|path| (*path).to_owned()).collect()
}

fn written(model: &IfcModel) -> Vec<u8> {
    model.write().expect("writes")
}

#[test]
fn an_exported_table_imports_as_a_no_op() {
    let mut formats = vec!["csv", "tsv"];
    if cfg!(feature = "sheet-xlsx") {
        formats.push("xlsx");
    }
    for mut model in fixtures() {
        let before = written(&model);
        for format in &formats {
            let table = model
                .export_table(
                    "IfcWall",
                    &columns(&["Name", "type.Name"]),
                    format,
                    None,
                    "",
                )
                .unwrap();
            let import = model.import_table(&table, format, None, "", false).unwrap();
            assert_eq!(
                import,
                TableImport {
                    applied: false,
                    changes: Vec::new(),
                    refusals: Vec::new()
                },
                "{format}"
            );
        }
        assert_eq!(written(&model), before);
    }
}

#[test]
fn an_edited_table_is_planned_then_applied() {
    for mut model in fixtures() {
        let table = model
            .export_table(
                "IfcWall",
                &columns(&["id", "type.Name"]),
                "csv",
                Some("id"),
                "",
            )
            .unwrap();
        let text = String::from_utf8(table).unwrap();
        assert!(text.starts_with("id,type.Name\r\n17,WT-200\r\n"), "{text}");
        let edited = text.replace("17,WT-200", "17,WT-250");
        let before = written(&model);
        let dry = model
            .import_table(edited.as_bytes(), "CSV", None, "", true)
            .unwrap();
        assert!(!dry.applied);
        assert_eq!(dry.changes.len(), 1);
        let change = &dry.changes[0];
        assert_eq!(
            (
                change.row,
                change.column,
                change.entity,
                change.target_entity
            ),
            (2, 2, 17, 16)
        );
        assert_eq!(
            (change.before.as_str(), change.after.as_str()),
            ("WT-200", "WT-250")
        );
        assert_eq!(written(&model), before, "a dry run writes nothing");

        let applied = model
            .import_table(edited.as_bytes(), "csv", Some("id"), "", false)
            .unwrap();
        assert!(applied.applied);
        assert_eq!(applied.to_record().name, "TableImport");
        let renamed = model.select("IfcWallType, Name=WT-250").unwrap();
        assert_eq!(renamed, [16]);
    }
}

#[test]
fn refusals_are_data_and_write_nothing() {
    for mut model in fixtures() {
        let before = written(&model);
        // An empty GlobalId writes NULL to an attribute that is not optional.
        let table = "id,Name,GlobalId,Name.\n17,Wall A,,\n99,X,,\n";
        let import = model
            .import_table(table.as_bytes(), "csv", Some("id"), "", false)
            .unwrap();
        assert!(!import.applied);
        let found: Vec<(Option<usize>, Option<usize>, &str)> = import
            .refusals
            .iter()
            .map(|refusal| (refusal.row, refusal.column, refusal.code.as_str()))
            .collect();
        assert_eq!(
            found,
            [
                (Some(1), Some(4), "invalid-header"),
                (Some(2), Some(3), "type-mismatch"),
                (Some(3), Some(1), "unknown-key"),
            ]
        );
        assert_eq!(import.refusals[0].span, Some((5, 5)));
        assert_eq!(written(&model), before);

        let malformed = model
            .import_table(b"id,Name\n17,Wa\"ll\n", "csv", None, "", false)
            .unwrap();
        assert_eq!(malformed.refusals[0].code, "malformed-table");
    }
}

#[test]
fn bad_arguments_raise() {
    for mut model in fixtures() {
        let code = |error: BindingError| error.code();
        assert_eq!(
            model
                .export_table("IfcWall", &[], "ods", None, "")
                .map_err(code),
            Err("invalid-value")
        );
        assert_eq!(
            model
                .export_table("IfcWall", &[], "csv", Some("guid2"), "")
                .map_err(code),
            Err("invalid-value")
        );
        assert_eq!(
            model
                .export_table("IfcWal", &[], "csv", None, "")
                .map_err(code),
            Err("invalid-selector")
        );
        assert_eq!(
            model
                .export_table("IfcMaterial", &[], "csv", None, "")
                .map_err(code),
            Err("wrong-entity-type")
        );
        let xlsx = model.export_table("IfcWall", &[], "xlsx", None, "");
        if cfg!(feature = "sheet-xlsx") {
            assert!(xlsx.unwrap().starts_with(b"PK"));
            assert_eq!(
                model
                    .import_table(b"not a workbook", "xlsx", None, "", false)
                    .map_err(code),
                Err("parse")
            );
        } else {
            assert_eq!(xlsx.map_err(code), Err("feature-disabled"));
        }
    }
}
