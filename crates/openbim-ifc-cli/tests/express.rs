//! `validate --express FILE.exp` (#538), through the built binary.
//!
//! The refusals use synthetic `.exp` text. The runs that evaluate rules read
//! the normative `.exp` files from the checked-out `references/ifc-spec/`
//! (CC BY-ND 4.0, never committed): they skip without it -- unless
//! `IFC_SPEC_REQUIRED` is set, as `scripts/gate.sh` does, which turns the
//! skip into a failure.

mod support;

use std::path::PathBuf;

use support::{fixture, root, run, scratch};

const CLEAN: &str = "synthetic-bindings/binding_geometry.ifc";
const IFC2X3: &str = "ifclite-geometry/issue_098_wall_W.ifc";

/// An IFC4 file whose loop has points of two dimensions.
const POLYLOOP: &str = "ISO-10303-21;
HEADER;
FILE_DESCRIPTION(('ViewDefinition [CoordinationView]'),'2;1');
FILE_NAME('loop.ifc','2026-10-03T00:00:00',('Ann'),('Org'),'pre','sys','');
FILE_SCHEMA(('IFC4'));
ENDSEC;
DATA;
#1=IFCCARTESIANPOINT((0.,0.,0.));
#2=IFCCARTESIANPOINT((1.,0.));
#3=IFCCARTESIANPOINT((1.,1.,0.));
#4=IFCPOLYLOOP((#1,#2,#3));
ENDSEC;
END-ISO-10303-21;
";

/// The normative `.exp` below `references/ifc-spec/`, or `None` (skip).
fn spec(relative: &str) -> Option<PathBuf> {
    let path = root().join("references/ifc-spec").join(relative);
    if path.is_file() {
        return Some(path);
    }
    assert!(
        std::env::var_os("IFC_SPEC_REQUIRED").is_none(),
        "IFC_SPEC_REQUIRED is set but {} was not found",
        path.display()
    );
    eprintln!("skipped: {} not present", path.display());
    None
}

fn write(dir: &std::path::Path, name: &str, text: &str) -> String {
    let path = dir.join(name);
    std::fs::write(&path, text).unwrap();
    path.to_str().unwrap().to_owned()
}

#[test]
fn text_of_another_release_refuses_the_file_with_express_mismatch() {
    let dir = scratch("express-mismatch");
    let exp = write(&dir, "other.exp", "SCHEMA IFC2X3; END_SCHEMA;");
    let file = fixture(CLEAN);
    let out = run(&["validate", "--format", "json", "--express", &exp, &file]);
    assert_eq!(out.code, 2, "{}{}", out.stdout, out.stderr);
    let report = out.json();
    let error = &report["files"][0]["error"];
    assert_eq!(error["kind"], "express-mismatch", "{report}");
    let message = error["message"].as_str().unwrap();
    assert!(
        message.contains("no --express file declares IFC4") && message.contains(&exp),
        "{message}"
    );
    assert!(out.stderr.is_empty() || out.stderr.contains("express-mismatch"));
}

#[test]
fn text_that_does_not_parse_is_invalid_express_and_names_its_file() {
    let dir = scratch("express-invalid");
    let exp = write(&dir, "broken.exp", "SCHEMA IFC4; ENTITY ; END_SCHEMA;");
    let out = run(&[
        "validate",
        "--format",
        "json",
        "--express",
        &exp,
        &fixture(CLEAN),
    ]);
    assert_eq!(out.code, 2, "{}{}", out.stdout, out.stderr);
    let report = out.json();
    let error = &report["files"][0]["error"];
    assert_eq!(error["kind"], "invalid-express", "{report}");
    assert!(error["message"].as_str().unwrap().contains(&exp));
}

#[test]
fn an_unreadable_express_file_ends_the_run_with_io() {
    let out = run(&[
        "validate",
        "--express",
        "no-such-schema.exp",
        &fixture(CLEAN),
    ]);
    assert_eq!(out.code, 2);
    assert!(
        out.stderr.contains("io: no-such-schema.exp"),
        "{}",
        out.stderr
    );
    assert!(
        out.stdout.is_empty(),
        "no file was validated: {}",
        out.stdout
    );
}

#[test]
fn the_summary_names_indeterminate_rules_only_when_express_is_given() {
    let file = fixture(CLEAN);
    let plain = run(&["validate", &file]);
    assert!(!plain.stdout.contains("indeterminate"), "{}", plain.stdout);
    let json = run(&["validate", "--format", "json", &file]).json();
    assert_eq!(json["files"][0]["summary"]["indeterminate"], 0, "{json}");
}

#[test]
fn the_supplied_rules_fail_a_file_the_plain_run_cannot_judge() {
    let Some(exp) = spec("ifc4-add2-tc1/IFC4.exp") else {
        return;
    };
    let dir = scratch("express-polyloop");
    let file = write(&dir, "loop.ifc", POLYLOOP);
    let exp = exp.to_str().unwrap();

    let plain = run(&["validate", "--include-unsupported", &file]);
    assert!(
        plain
            .stdout
            .contains("unsupported [IfcPolyLoop.AllPointsSameDim]"),
        "{}",
        plain.stdout
    );

    let out = run(&["validate", "--express", exp, &file]);
    assert_eq!(out.code, 1, "{}{}", out.stdout, out.stderr);
    assert!(
        out.stdout.contains("error [IfcPolyLoop.AllPointsSameDim]"),
        "{}",
        out.stdout
    );
    assert!(out.stdout.contains(" indeterminate; "), "{}", out.stdout);

    let json = run(&["validate", "--format", "json", "--express", exp, &file]).json();
    assert!(
        json["files"][0]["summary"]["indeterminate"].is_u64(),
        "{json}"
    );
}

#[test]
fn one_express_file_per_release_serves_a_run_over_both() {
    let (Some(ifc2x3), Some(ifc4)) = (
        spec("ifc2x3-tc1/IFC2X3_TC1.exp"),
        spec("ifc4-add2-tc1/IFC4.exp"),
    ) else {
        return;
    };
    let dir = scratch("express-two-releases");
    let loop_file = write(&dir, "loop.ifc", POLYLOOP);
    let out = run(&[
        "validate",
        "--format",
        "json",
        "--express",
        ifc2x3.to_str().unwrap(),
        "--express",
        ifc4.to_str().unwrap(),
        &fixture(IFC2X3),
        &loop_file,
    ]);
    // The IFC4 file fails its rule; neither file is refused.
    assert_eq!(out.code, 1, "{}{}", out.stdout, out.stderr);
    let report = out.json();
    for (index, schema) in [(0, "IFC2X3"), (1, "IFC4")] {
        let file = &report["files"][index];
        assert!(file.get("error").is_none(), "{file}");
        assert_eq!(file["schema"], schema, "{file}");
        assert!(file["summary"]["indeterminate"].is_u64(), "{file}");
    }
    assert_eq!(report["files"][1]["conformant"], false);
}

#[test]
fn a_broken_express_file_refuses_the_model_in_either_order() {
    let dir = scratch("express-order");
    let good = write(&dir, "good.exp", "SCHEMA IFC4; END_SCHEMA;");
    let bad = write(&dir, "bad2x3.exp", "SCHEMA IFC2X3; ENTITY ; END_SCHEMA;");
    let file = fixture(CLEAN);
    for order in [[&good, &bad], [&bad, &good]] {
        let out = run(&[
            "validate",
            "--format",
            "json",
            "--express",
            order[0],
            "--express",
            order[1],
            &file,
        ]);
        assert_eq!(out.code, 2, "{}{}", out.stdout, out.stderr);
        let report = out.json();
        let error = &report["files"][0]["error"];
        assert_eq!(error["kind"], "invalid-express", "{report}");
        assert!(
            error["message"].as_str().unwrap().contains(&bad),
            "{report}"
        );
    }
}

#[test]
fn two_files_for_one_release_are_refused() {
    let Some(exp) = spec("ifc4-add2-tc1/IFC4.exp") else {
        return;
    };
    let dir = scratch("express-twice");
    let copy = dir.join("IFC4-copy.exp");
    std::fs::copy(&exp, &copy).unwrap();
    let (first, second) = (exp.to_str().unwrap(), copy.to_str().unwrap());
    for order in [[first, second], [second, first]] {
        let out = run(&[
            "validate",
            "--format",
            "json",
            "--express",
            order[0],
            "--express",
            order[1],
            &fixture(CLEAN),
        ]);
        assert_eq!(out.code, 2, "{}{}", out.stdout, out.stderr);
        let report = out.json();
        let error = &report["files"][0]["error"];
        assert_eq!(error["kind"], "express-mismatch", "{report}");
        assert!(
            error["message"]
                .as_str()
                .unwrap()
                .contains("more than one --express file declares IFC4"),
            "{report}"
        );
    }
}
