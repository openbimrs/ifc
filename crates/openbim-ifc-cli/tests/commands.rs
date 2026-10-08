//! Every command, its formats, and its exit codes, through the built binary.

mod support;

use support::{fixture, run, scratch};

const CLEAN: &str = "synthetic-bindings/binding_geometry.ifc";
const DUPLICATE_GUIDS: &str = "ifcopenshell-validate/fail-duplicated-guids-ifc4.ifc";
const PROPERTIES: &str = "synthetic-properties/synthetic_properties.ifc";
/// One warning, no error.
const WARNING: &str = "costing/costing_schedule.ifc";
const IFC2X3: &str = "ifclite-geometry/issue_098_wall_W.ifc";

#[test]
fn usage_errors_exit_2_and_help_exits_0() {
    assert_eq!(run(&["--help"]).code, 0);
    let version = run(&["--version"]);
    assert_eq!(version.code, 0);
    assert!(version.stdout.contains(env!("CARGO_PKG_VERSION")));
    assert_eq!(run::<&str>(&[]).code, 2);
    assert_eq!(run(&["frobnicate"]).code, 2);
    assert_eq!(run(&["validate"]).code, 2, "a file is required");
    assert_eq!(run(&["validate", "--format", "xml", "x.ifc"]).code, 2);
}

#[test]
fn validate_exits_0_on_a_conformant_file() {
    let out = run(&["validate", &fixture(CLEAN)]);
    assert_eq!(out.code, 0, "{}{}", out.stdout, out.stderr);
    assert!(
        out.stdout.contains("IFC4: conformant: 0 errors"),
        "{}",
        out.stdout
    );
}

#[test]
fn validate_exits_1_on_findings_and_points_at_the_line() {
    let file = fixture(DUPLICATE_GUIDS);
    let out = run(&["validate", &file]);
    assert_eq!(out.code, 1);
    assert!(
        out.stdout.contains(&format!(
            "{file}:9: error [global.UniqueGlobalId] #2.GlobalId: "
        )),
        "{}",
        out.stdout
    );
    assert!(out.stdout.contains("not conformant: 1 error"));
}

#[test]
fn warnings_fail_only_when_denied() {
    let file = fixture(WARNING);
    assert_eq!(run(&["validate", &file]).code, 0);
    let denied = run(&["validate", "--deny-warnings", &file]);
    assert_eq!(denied.code, 1);
    assert!(
        denied.stdout.contains("fails on warnings"),
        "{}",
        denied.stdout
    );
}

#[test]
fn a_truncated_report_fails_the_run() {
    let out = run(&["validate", "--max-findings", "0", &fixture(DUPLICATE_GUIDS)]);
    assert_eq!(out.code, 1);
    assert!(out.stdout.contains("truncated"), "{}", out.stdout);
}

#[test]
fn validate_json_reports_every_file_and_an_unreadable_one_exits_2() {
    let clean = fixture(CLEAN);
    let failing = fixture(DUPLICATE_GUIDS);
    let out = run(&[
        "validate",
        "--format",
        "json",
        &clean,
        &failing,
        "missing.ifc",
    ]);
    assert_eq!(out.code, 2, "a file that could not be validated wins");
    let json = out.json();
    assert_eq!(json["passed"], false);
    let files = json["files"].as_array().unwrap();
    assert_eq!(files.len(), 3);
    assert_eq!(files[0]["file"], clean.as_str());
    assert_eq!(files[0]["passed"], true);
    assert_eq!(files[0]["schema"], "IFC4");
    assert_eq!(files[1]["conformant"], false);
    assert_eq!(files[1]["summary"]["errors"], 1);
    let finding = &files[1]["findings"][0];
    assert_eq!(finding["rule"], "global.UniqueGlobalId");
    assert_eq!(finding["severity"], "error");
    assert_eq!(finding["entity"], 2);
    assert_eq!(finding["attribute_name"], "GlobalId");
    assert_eq!(finding["line"], 9);
    assert_eq!(files[2]["error"]["kind"], "io");
}

#[test]
fn unsupported_rules_are_listed_only_on_request() {
    let file = fixture(CLEAN);
    let plain = run(&["validate", "--format", "json", &file]).json();
    let listed = run(&[
        "validate",
        "--format",
        "json",
        "--include-unsupported",
        &file,
    ])
    .json();
    let unsupported = plain["files"][0]["summary"]["unsupported"]
        .as_u64()
        .unwrap();
    assert!(unsupported > 0);
    assert_eq!(plain["files"][0]["findings"].as_array().unwrap().len(), 0);
    assert_eq!(
        listed["files"][0]["findings"].as_array().unwrap().len() as u64,
        unsupported
    );
}

#[test]
fn an_unbundled_or_missing_schema_is_refused() {
    let dir = scratch("schema-refusal");
    let source = std::fs::read_to_string(support::root().join(fixture(CLEAN))).unwrap();
    for (name, schema) in [
        ("future.ifc", "FILE_SCHEMA(('IFC9'));"),
        ("none.ifc", "FILE_SCHEMA(());"),
    ] {
        let path = dir.join(name);
        std::fs::write(&path, source.replace("FILE_SCHEMA(('IFC4'));", schema)).unwrap();
        let out = run(&["validate", path.to_str().unwrap()]);
        assert_eq!(out.code, 2, "{name}: {}{}", out.stdout, out.stderr);
        assert!(out.stderr.contains("unsupported-schema"), "{}", out.stderr);
        let tree = run(&["tree", path.to_str().unwrap()]);
        assert_eq!(tree.code, 2);
        // `info` needs no tables and says so instead of failing.
        let info = run(&["info", path.to_str().unwrap()]);
        assert_eq!(info.code, 0, "{}", info.stderr);
    }
}

#[test]
fn a_file_that_is_neither_step_nor_ifcxml_is_refused() {
    let dir = scratch("not-ifc");
    let path = dir.join("notes.txt");
    std::fs::write(&path, "hello").unwrap();
    let out = run(&["info", path.to_str().unwrap()]);
    assert_eq!(out.code, 2);
    assert!(out.stderr.contains("parse:"), "{}", out.stderr);
}

/// Entity count and per-type counts: what a codec round trip must keep.
fn census(path: &str) -> serde_json::Value {
    let out = run(&["info", "--format", "json", "--all-types", path]);
    assert_eq!(out.code, 0, "{path}: {}", out.stderr);
    let json = out.json();
    serde_json::json!({ "entities": json["entities"], "types": json["types"] })
}

#[test]
fn convert_round_trips_through_both_ifcxml_layouts() {
    let dir = scratch("convert");
    for name in [CLEAN, PROPERTIES] {
        let source = fixture(name);
        let expected = census(&source);
        for layout in ["native", "xsd"] {
            let xml = dir.join(format!("{layout}.ifcxml"));
            let back = dir.join(format!("{layout}.ifc"));
            let out = run(&[
                "convert",
                "--force",
                "--layout",
                layout,
                &source,
                xml.to_str().unwrap(),
            ]);
            assert_eq!(out.code, 0, "{name} -> {layout}: {}", out.stderr);
            let info = run(&["info", "--format", "json", xml.to_str().unwrap()]).json();
            let format = if layout == "xsd" {
                "ifcXML (XSD layout)"
            } else {
                "ifcXML (native layout)"
            };
            assert_eq!(info["format"], format, "the layout is detected on read");
            let out = run(&[
                "convert",
                "--force",
                xml.to_str().unwrap(),
                back.to_str().unwrap(),
            ]);
            assert_eq!(out.code, 0, "{name} <- {layout}: {}", out.stderr);
            assert_eq!(
                census(back.to_str().unwrap()),
                expected,
                "{name} via {layout}"
            );
        }
    }
}

#[test]
fn convert_writes_standard_output_and_refuses_what_it_cannot_do() {
    let source = fixture(CLEAN);
    let out = run(&["convert", &source, "-", "--to", "step"]);
    assert_eq!(out.code, 0);
    assert!(out.stdout.starts_with("ISO-10303-21;"));
    assert!(out.stdout.contains("IFCWALL"));

    let dir = scratch("convert-refusals");
    let target = dir.join("out.ifcxml");
    let target = target.to_str().unwrap();
    for (args, needle) in [
        (vec!["convert", source.as_str(), "-"], "needs --to"),
        (
            vec!["convert", source.as_str(), "out.json"],
            "cannot tell the output format",
        ),
        (
            vec!["convert", source.as_str(), "x.ifc", "--layout", "xsd"],
            "ifcXML output only",
        ),
    ] {
        let out = run(&args);
        assert_eq!(out.code, 2, "{args:?}");
        assert!(out.stderr.contains(needle), "{args:?}: {}", out.stderr);
    }
    let ifc2x3 = fixture(IFC2X3);
    let out = run(&["convert", &ifc2x3, target, "--layout", "xsd"]);
    assert_eq!(out.code, 2);
    assert!(
        out.stderr.contains("unsupported: the XSD layout exists"),
        "{}",
        out.stderr
    );
    assert!(
        !std::path::Path::new(target).exists(),
        "nothing written on refusal"
    );

    assert_eq!(run(&["convert", &source, target]).code, 0);
    let again = run(&["convert", &source, target]);
    assert_eq!(again.code, 2, "an existing output needs --force");
    assert!(again.stderr.contains("--force"));
}

#[test]
fn an_ifcxml_document_of_unknown_layout_needs_naming() {
    let dir = scratch("layout");
    let path = dir.join("bare.ifcxml");
    std::fs::write(
        &path,
        "<?xml version=\"1.0\"?>\n<ifcXML xmlns=\"urn:x\"></ifcXML>\n",
    )
    .unwrap();
    let out = run(&["info", path.to_str().unwrap()]);
    assert_eq!(out.code, 2);
    assert!(out.stderr.contains("--input-layout"), "{}", out.stderr);
}

#[test]
fn info_reports_header_schema_and_counts() {
    let file = fixture(CLEAN);
    let json = run(&["info", "--format", "json", "--all-types", &file]).json();
    assert_eq!(json["format"], "STEP");
    assert_eq!(json["schema"]["token"], "IFC4");
    assert_eq!(json["schema"]["release"], "IFC4_ADD2_TC1");
    assert_eq!(json["schema"]["bundled"], true);
    assert_eq!(json["header"]["name"], "binding_geometry.ifc");
    assert_eq!(json["entities"], 54);
    let walls = json["types"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["type"] == "IFCWALL")
        .expect("IFCWALL counted");
    assert_eq!(walls["count"], 1);

    let human = run(&["info", &file]);
    assert_eq!(human.code, 0);
    assert!(human.stdout.contains("IFC4 (IFC4_ADD2_TC1, bundled)"));
    assert!(human.stdout.contains("Entities              54"));
}

#[test]
fn psets_by_id_and_by_global_id_agree_in_every_format() {
    let file = fixture(PROPERTIES);
    let table = run(&["psets", &file, "#30"]);
    assert_eq!(table.code, 0, "{}", table.stderr);
    assert!(table
        .stdout
        .starts_with("#30 IFCWALL 'Wall A' 31t64Uzj98DhdKplMxBpF5"));
    assert!(table
        .stdout
        .contains("Pset_WallCommon         occurrence  IsExternal    false"));
    assert!(table
        .stdout
        .contains("Pset_WallCommon         type #29    FireRating    F30"));
    assert!(
        table.stdout.contains("Layers.Inner"),
        "complex members are rows"
    );
    assert_eq!(
        run(&["psets", &file, "31t64Uzj98DhdKplMxBpF5"]).stdout,
        table.stdout
    );
    assert_eq!(run(&["psets", &file, "30"]).stdout, table.stdout);

    let json = run(&["psets", &file, "30", "--format", "json"]).json();
    assert_eq!(json["entity"], 30);
    assert_eq!(json["global_id"], "31t64Uzj98DhdKplMxBpF5");
    let sets = json["sets"].as_array().unwrap();
    let occurrence = &sets[0];
    assert_eq!(occurrence["name"], "Pset_WallCommon");
    assert_eq!(occurrence["source"], "occurrence");
    assert_eq!(occurrence["properties"][0]["name"], "IsExternal");
    assert_eq!(occurrence["properties"][0]["value"], false);
    let inherited = sets.iter().find(|set| set["source"] == "type").unwrap();
    assert_eq!(inherited["source_id"], 29);
    let quantities = sets
        .iter()
        .find(|set| set["name"] == "Qto_WallBaseQuantities")
        .unwrap();
    let layers = quantities["properties"]
        .as_array()
        .unwrap()
        .iter()
        .find(|property| property["name"] == "Layers")
        .unwrap();
    assert_eq!(layers["kind"], "complex");
    assert_eq!(layers["value"]["members"].as_array().unwrap().len(), 2);
    assert!(
        quantities["properties"]
            .as_array()
            .unwrap()
            .iter()
            .all(|property| property["name"] != "Layers.Inner"),
        "members stay inside the complex in JSON"
    );

    let csv = run(&["psets", &file, "2CazjTQP11iu6o3c47EEd8", "--format", "csv"]);
    assert_eq!(csv.code, 0);
    let lines: Vec<&str> = csv
        .stdout
        .split("\r\n")
        .filter(|line| !line.is_empty())
        .collect();
    assert_eq!(
        lines[0],
        "set,set_id,source,source_id,property,property_id,kind,value,value_type,unit"
    );
    assert!(
        lines.contains(&"Pset_Families,51,occurrence,,Layers,44,list,\"[0.012, 0.15, 0.012]\",,")
    );
    assert!(lines.contains(&"Pset_WallCommon,36,type,29,FireRating,35,value,F30,IFCLABEL,"));
}

#[test]
fn psets_of_a_type_object_are_its_own() {
    let json = run(&["psets", &fixture(PROPERTIES), "29", "--format", "json"]).json();
    assert_eq!(json["type"], "IFCWALLTYPE");
    assert_eq!(json["sets"][0]["source"], "type");
    assert_eq!(json["sets"][0]["source_id"], 29);
}

#[test]
fn psets_refuses_with_the_kind_of_the_problem() {
    let file = fixture(PROPERTIES);
    for (entity, kind) in [
        ("34", "wrong-entity-type"),
        ("#999", "missing-entity"),
        ("0000000000000000000000", "missing-entity"),
        ("wall", "usage"),
    ] {
        let out = run(&["psets", &file, entity]);
        assert_eq!(out.code, 2, "{entity}");
        assert!(
            out.stderr.contains(&format!("openbim-ifc: {kind}: ")),
            "{entity}: {}",
            out.stderr
        );
    }
}

#[test]
fn tree_shows_the_spatial_structure() {
    let file = fixture(CLEAN);
    let human = run(&["tree", &file]);
    assert_eq!(human.code, 0);
    assert_eq!(
        human.stdout,
        "IFCPROJECT #9 '#328 binding geometry'\n\
         └─ IFCSITE #13 'Site'\n   \
         └─ IFCBUILDINGSTOREY #17 'Level 1'  (4 elements)\n"
    );
    let listed = run(&["tree", "--elements", &file]);
    assert!(
        listed.stdout.contains("      ├─ · IFCWALL #36 'Wall'\n"),
        "{}",
        listed.stdout
    );

    let json = run(&["tree", "--format", "json", &file]).json();
    assert_eq!(json["release"], "IFC4_ADD2_TC1");
    let project = &json["roots"][0];
    assert_eq!(project["kind"], "project");
    let storey = &project["children"][0]["children"][0];
    assert_eq!(storey["kind"], "storey");
    assert_eq!(storey["global_id"], "1Hc8kQe0j4zP7mUq2XbW5d");
    assert_eq!(storey["elements"].as_array().unwrap().len(), 4);
    assert_eq!(storey["elements"][0]["type"], "IFCWALL");
}

#[test]
fn lint_exits_1_on_a_product_no_viewer_draws() {
    let file = fixture(CLEAN);
    let out = run(&["lint", &file]);
    assert_eq!(out.code, 1);
    assert!(
        out.stdout.contains(&format!(
            "{file}:54: warning [unreachable.no-model-context] IFCBUILDINGELEMENTPROXY #53 'Axis only': "
        )),
        "{}",
        out.stdout
    );
    let json = run(&["lint", "--format", "json", &file]).json();
    assert_eq!(json["passed"], false);
    assert_eq!(json["files"][0]["findings"][0]["entity"], 53);

    let clean = run(&["lint", &fixture(PROPERTIES)]);
    assert_eq!(clean.code, 0, "{}", clean.stdout);
}
