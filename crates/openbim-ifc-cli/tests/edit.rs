//! `edit`, `export-table` and `import-table` (#472, #474) through the
//! built binary, over the selector model in IFC2X3, IFC4 and IFC4X3: dry
//! runs, writes to a new file or in place, refusals with exit 3 and
//! nothing written, and tables that round-trip as a no-op.

mod support;

use std::path::Path;

use support::{fixture, run, scratch};

const SELECTOR: &str = "synthetic-relations/selector_ifc4.ifc";
const RELEASES: [&str; 3] = [
    "synthetic-relations/selector_ifc2x3.ifc",
    "synthetic-relations/selector_ifc4.ifc",
    "synthetic-relations/selector_ifc4x3.ifc",
];

/// The value `path` reads from every wall, as `select --format csv` lists
/// it, without the header.
fn walls(file: &str, path: &str) -> Vec<String> {
    let out = run(&[
        "select", file, "IfcWall", "--format", "csv", "--value", path,
    ]);
    assert_eq!(out.code, 0, "{file}: {}", out.stderr);
    out.stdout
        .lines()
        .skip(1)
        .map(|line| line.rsplit(',').next().unwrap_or_default().to_owned())
        .collect()
}

fn read(path: &Path) -> Vec<u8> {
    std::fs::read(path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

#[test]
fn edit_plans_with_a_dry_run_and_writes_a_new_file() {
    let file = fixture(SELECTOR);
    let before = read(&support::root().join(&file));
    let dry = run(&[
        "edit",
        &file,
        "IfcWall, type=WT-200",
        "--set",
        "type.Name",
        "WT-250",
        "--set",
        "Pset_WallCommon.FireRating",
        "F120",
        "--dry-run",
    ]);
    assert_eq!(dry.code, 0, "{}", dry.stderr);
    assert_eq!(
        dry.stdout,
        "#17 type.Name (#16 Name): \"WT-200\" -> \"WT-250\"\n\
         #17 Pset_WallCommon.FireRating: \"F90\" -> \"F120\"\n\
         #18 Pset_WallCommon.FireRating: \"F90\" -> \"F120\"\n\
         3 changes planned (dry run; nothing written)\n"
    );

    let dir = scratch("edit-write");
    let output = dir.join("edited.ifc");
    let output = output.to_str().unwrap();
    let written = run(&[
        "edit",
        &file,
        "IfcWall",
        "--set",
        "Description",
        "Load-bearing",
        "--set",
        "Name",
        "$",
        "-o",
        output,
    ]);
    assert_eq!(written.code, 0, "{}", written.stderr);
    assert!(
        written
            .stdout
            .ends_with(&format!("6 changes written to {output}\n")),
        "{}",
        written.stdout
    );
    assert_eq!(walls(output, "Description"), ["Load-bearing"; 3]);
    assert_eq!(walls(output, "Name"), ["", "", ""]);
    assert_eq!(
        read(&support::root().join(&file)),
        before,
        "the input is never changed without --in-place"
    );

    // An existing output needs --force; every edit needs a destination.
    let again = run(&["edit", &file, "IfcWall", "--set", "Name", "W", "-o", output]);
    assert_eq!(again.code, 2);
    assert!(again.stderr.contains("--force"), "{}", again.stderr);
    let nowhere = run(&["edit", &file, "IfcWall", "--set", "Name", "W"]);
    assert_eq!(nowhere.code, 2);
    assert!(nowhere.stderr.contains("--in-place"), "{}", nowhere.stderr);
}

#[test]
fn edit_in_place_keeps_the_input_format() {
    let dir = scratch("edit-in-place");
    for (name, release) in ["ifc2x3", "ifc4", "ifc4x3"].into_iter().zip(RELEASES) {
        let copy = dir.join(format!("{name}.ifc"));
        std::fs::copy(support::root().join(fixture(release)), &copy).unwrap();
        let copy = copy.to_str().unwrap();
        let out = run(&[
            "edit",
            copy,
            "IfcWall, Name=\"Wall C\"",
            "--set",
            "Pset_WallCommon.AcousticRating",
            "IFCLABEL(R50)",
            "--in-place",
        ]);
        assert_eq!(out.code, 0, "{name}: {}", out.stderr);
        assert_eq!(
            walls(copy, "Pset_WallCommon.AcousticRating"),
            ["R45", "R45", "R50"],
            "{name}"
        );
        assert!(std::fs::read(copy).unwrap().starts_with(b"ISO-10303-21;"));
    }
    // An ifcXML input stays ifcXML.
    let xml = dir.join("model.ifcxml");
    let xml = xml.to_str().unwrap();
    assert_eq!(run(&["convert", &fixture(SELECTOR), xml]).code, 0);
    let out = run(&[
        "edit",
        xml,
        "IfcSlab",
        "--set",
        "Name",
        "Floor",
        "--in-place",
    ]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    let info = run(&["info", "--format", "json", xml]).json();
    assert_eq!(info["format"], "ifcXML (native layout)");
    let slab = run(&["select", xml, "IfcSlab", "--format", "csv"]);
    assert!(slab.stdout.contains(",Floor\r\n"), "{}", slab.stdout);
}

#[test]
fn edit_refuses_with_exit_3_and_writes_nothing() {
    let file = fixture(SELECTOR);
    let dir = scratch("edit-refused");
    let output = dir.join("never.ifc");
    let output = output.to_str().unwrap();
    let out = run(&[
        "edit", &file, "IfcWall", "--set", "Name", "Renamed", "--set", "class", "IfcSlab", "-o",
        output,
    ]);
    assert_eq!(out.code, 3, "{}", out.stderr);
    assert_eq!(out.stdout, "3 refusals; nothing written\n");
    assert!(
        out.stderr
            .contains("refused: #17, --set class: not-settable: cannot write \"class\""),
        "{}",
        out.stderr
    );
    assert!(!Path::new(output).exists(), "nothing written on refusal");

    let json = run(&[
        "edit",
        &file,
        "IfcDoor",
        "--set",
        "OverallHeight",
        "tall",
        "--format",
        "json",
        "--dry-run",
    ]);
    assert_eq!(json.code, 3);
    let json = json.json();
    assert_eq!(json["changes"], serde_json::json!([]));
    let refusal = &json["refusals"][0];
    assert_eq!(refusal["entity"], 28);
    assert_eq!(refusal["path"], "OverallHeight");
    assert_eq!(refusal["kind"], "invalid-text");

    // A selector or a path that does not parse: a usage refusal with its
    // span, exit 2.
    let refused = run(&["edit", &file, "IfcWal", "--set", "Name", "x", "--dry-run"]);
    assert_eq!(refused.code, 2);
    assert!(
        refused.stderr.contains("(bytes 0..6)"),
        "{}",
        refused.stderr
    );
    let refused = run(&["edit", &file, "IfcWall", "--set", "type.", "x", "--dry-run"]);
    assert_eq!(refused.code, 2);
    assert!(
        refused.stderr.contains("--set \"type.\"") && refused.stderr.contains("at bytes 5..5"),
        "{}",
        refused.stderr
    );
}

#[test]
fn export_table_writes_csv_tsv_and_xlsx() {
    let file = fixture(SELECTOR);
    let csv = run(&[
        "export-table",
        &file,
        "IfcWall",
        "-c",
        "Name",
        "-c",
        "type.Name",
        "-c",
        "Qto_WallBaseQuantities.Length",
    ]);
    assert_eq!(csv.code, 0, "{}", csv.stderr);
    assert_eq!(
        csv.stdout,
        "GlobalId,Name,type.Name,Qto_WallBaseQuantities.Length\r\n\
         0TG0wWfbZOvmJHW7AuHJ$k,Wall A,WT-200,5\r\n\
         0XLKvWwoErh4p4gJsHE2zX,Wall B,WT-200,\r\n\
         19HFrQ$NB$TW0hRFxhxvxK,Wall C,,\r\n"
    );
    let tsv = run(&[
        "export-table",
        &file,
        "IfcDoor",
        "--key",
        "id",
        "--to",
        "tsv",
        "--null",
        "-",
    ]);
    assert_eq!(tsv.stdout, "id\tclass\tName\n28\tIfcDoor\tDoor 1\n");

    let dir = scratch("export-table");
    let xlsx = dir.join("walls.xlsx");
    let xlsx = xlsx.to_str().unwrap();
    let out = run(&["export-table", &file, "IfcWall", "-o", xlsx]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert!(read(Path::new(xlsx)).starts_with(b"PK"));
    let again = run(&["export-table", &file, "IfcWall", "-o", xlsx]);
    assert_eq!(again.code, 2, "an existing table needs --force");

    // Materials have no GlobalId: keyed by GlobalId they are refused.
    let refused = run(&["export-table", &file, "IfcMaterial"]);
    assert_eq!(refused.code, 2);
    assert!(
        refused.stderr.contains("key the table by id"),
        "{}",
        refused.stderr
    );
}

#[test]
fn an_exported_table_imports_as_a_no_op_in_every_release_and_format() {
    let dir = scratch("table-round-trip");
    for release in RELEASES {
        let file = fixture(release);
        for extension in ["csv", "tsv", "xlsx"] {
            let table = dir.join(format!("walls.{extension}"));
            let table = table.to_str().unwrap();
            let out = run(&[
                "export-table",
                &file,
                "IfcWall + IfcDoor + IfcSlab",
                "--force",
                "-o",
                table,
                "-c",
                "Name",
                "-c",
                "Description",
                "-c",
                "type.Name",
                "-c",
                "Pset_WallCommon.FireRating",
                "-c",
                "Pset_WallCommon.IsExternal",
                "-c",
                "Qto_WallBaseQuantities.Length",
                "-c",
                "material.Name",
                "-c",
                "container.Name",
            ]);
            assert_eq!(out.code, 0, "{release} {extension}: {}", out.stderr);
            let output = dir.join("same.ifc");
            let output = output.to_str().unwrap();
            let import = run(&["import-table", &file, table, "-o", output, "--force"]);
            assert_eq!(import.code, 0, "{release} {extension}: {}", import.stderr);
            assert!(
                import.stdout.starts_with("0 changes written"),
                "{release} {extension}: {}",
                import.stdout
            );
            // The same file `convert` writes: nothing changed.
            let converted = dir.join("converted.ifc");
            let converted = converted.to_str().unwrap();
            assert_eq!(run(&["convert", "--force", &file, converted]).code, 0);
            assert_eq!(
                read(Path::new(output)),
                read(Path::new(converted)),
                "{release} {extension}"
            );
        }
    }
}

#[test]
fn import_table_applies_edits_and_refuses_by_row_and_column() {
    let file = fixture(SELECTOR);
    let dir = scratch("table-import");
    let table = dir.join("walls.csv");
    std::fs::write(
        &table,
        "id,Name,Pset_WallCommon.IsExternal,Qto_WallBaseQuantities.Length\n\
         17,Wall A1,false,5.5\n\
         18,Wall B,FALSE,\n",
    )
    .unwrap();
    let table = table.to_str().unwrap();
    let dry = run(&["import-table", &file, table, "--dry-run"]);
    assert_eq!(dry.code, 0, "{}", dry.stderr);
    assert_eq!(
        dry.stdout,
        "row 2, column B: #17 Name: \"Wall A\" -> \"Wall A1\"\n\
         row 2, column C: #17 Pset_WallCommon.IsExternal: \"TRUE\" -> \"FALSE\"\n\
         row 2, column D: #17 Qto_WallBaseQuantities.Length: \"5\" -> \"5.5\"\n\
         3 changes planned (dry run; nothing written)\n"
    );
    let output = dir.join("edited.ifc");
    let output = output.to_str().unwrap();
    let out = run(&["import-table", &file, table, "-o", output]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert_eq!(walls(output, "Name"), ["Wall A1", "Wall B", "Wall C"]);
    assert_eq!(
        walls(output, "Qto_WallBaseQuantities.Length"),
        ["5.5", "", ""]
    );

    let bad = dir.join("bad.csv");
    std::fs::write(
        &bad,
        "id,Name,Pset_WallCommon.IsExternal,Pset_WallCommon.\n\
         17,Wall A1,maybe,\n\
         99,Ghost,TRUE,\n\
         17,Wall A,TRUE\n",
    )
    .unwrap();
    let refused = run(&[
        "import-table",
        &file,
        bad.to_str().unwrap(),
        "--dry-run",
        "--format",
        "json",
    ]);
    assert_eq!(refused.code, 3, "{}", refused.stderr);
    let json = refused.json();
    let found: Vec<(serde_json::Value, serde_json::Value, String)> = json["refusals"]
        .as_array()
        .unwrap()
        .iter()
        .map(|refusal| {
            (
                refusal["row"].clone(),
                refusal["column"].clone(),
                refusal["kind"].as_str().unwrap().to_owned(),
            )
        })
        .collect();
    use serde_json::json;
    assert_eq!(
        found,
        [
            (json!(1), json!(4), "invalid-header".to_owned()),
            (json!(2), json!(3), "invalid-text".to_owned()),
            (json!(3), json!(1), "unknown-key".to_owned()),
            (json!(4), json!(null), "ragged-row".to_owned()),
        ]
    );
    assert_eq!(json["refusals"][0]["span"], json!([16, 16]));
    assert_eq!(json["changes"], json!([]));

    let unknown = run(&["import-table", &file, "walls.ods", "--dry-run"]);
    assert_eq!(unknown.code, 2);
    assert!(unknown.stderr.contains("--from"), "{}", unknown.stderr);
}
