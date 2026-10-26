//! Validation with caller-supplied EXPRESS rules (#538).
//!
//! The refusals need no schema text. The happy path reads the IFC4 `.exp`
//! from the checked-out `references/ifc-spec/` (CC BY-ND 4.0, never
//! committed), so it skips without it -- unless `IFC_SPEC_REQUIRED` is set,
//! as `scripts/gate.sh` does, which turns the skip into a failure.
#![cfg(feature = "validate-express")]

use std::path::PathBuf;

use openbim_ifc_binding_core::{BindingError, IfcModel, ValidationReport};

/// An IFC4 file with two loops whose points have two dimensions.
const FILE: &str = "ISO-10303-21;
HEADER;
FILE_DESCRIPTION(('ViewDefinition [CoordinationView]'),'2;1');
FILE_NAME('t.ifc','2026-10-03T00:00:00',('Ann'),('Org'),'pre','sys','auth');
FILE_SCHEMA(('IFC4'));
ENDSEC;
DATA;
#1=IFCCARTESIANPOINT((0.,0.,0.));
#2=IFCCARTESIANPOINT((1.,0.));
#3=IFCCARTESIANPOINT((1.,1.,0.));
#4=IFCPOLYLOOP((#1,#2,#3));
#5=IFCPOLYLOOP((#3,#2,#1));
ENDSEC;
END-ISO-10303-21;
";

fn model() -> IfcModel {
    IfcModel::parse(FILE.as_bytes()).expect("the file parses")
}

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

/// Whether the schemas are present; panics when they are required but
/// missing.
fn spec_available() -> bool {
    let present = spec_root().is_some();
    if !present {
        assert!(
            std::env::var_os("IFC_SPEC_REQUIRED").is_none(),
            "IFC_SPEC_REQUIRED is set but references/ifc-spec was not found"
        );
        eprintln!("skipped: references/ifc-spec not present");
    }
    present
}

#[test]
fn text_for_another_release_is_an_express_mismatch() {
    let error = model()
        .validate_express("SCHEMA IFC2X3; END_SCHEMA;", None)
        .expect_err("an IFC2X3 source on an IFC4 model");
    assert!(
        matches!(error, BindingError::ExpressMismatch(_)),
        "{error:?}"
    );
    assert_eq!(error.code(), "express-mismatch");
}

#[test]
fn text_disagreeing_with_the_tables_is_an_express_mismatch() {
    // The right schema name, but none of IFC4's entities: the tables and
    // the source disagree.
    let error = model()
        .validate_express("SCHEMA IFC4; END_SCHEMA;", None)
        .expect_err("a source that omits the release's entities");
    assert_eq!(error.code(), "express-mismatch", "{error:?}");
}

#[test]
fn text_that_does_not_parse_is_invalid_express() {
    let error = model()
        .validate_express("SCHEMA IFC4; ENTITY ; END_SCHEMA;", None)
        .expect_err("an entity with no name");
    assert!(
        matches!(error, BindingError::InvalidExpress(_)),
        "{error:?}"
    );
    assert_eq!(error.code(), "invalid-express");
}

#[test]
fn the_supplied_rules_decide_what_the_plain_run_leaves_unsupported() {
    if !spec_available() {
        return;
    }
    let path = spec_root()
        .expect("checked by spec_available")
        .join("ifc4-add2-tc1/IFC4.exp");
    let text = std::fs::read_to_string(&path).expect("IFC4.exp reads");
    let model = model();
    let rule = "IfcPolyLoop.AllPointsSameDim";
    let severity = |report: &ValidationReport| {
        report
            .findings
            .iter()
            .find(|finding| finding.rule == rule)
            .map(|finding| finding.severity.clone())
    };

    let plain = model.validate(None).expect("the plain run");
    assert_eq!(severity(&plain).as_deref(), Some("unsupported"));
    assert_eq!(plain.summary.indeterminate, 0);

    let report = model
        .validate_express(&text, None)
        .expect("the supplied rules evaluate");
    assert_eq!(severity(&report).as_deref(), Some("error"));
    assert!(!report.conformant);
    assert!(report.summary.errors >= 1);

    // Two violating loops against a budget of one: the report stops there.
    let capped = model
        .validate_express(&text, Some(1))
        .expect("the supplied rules evaluate");
    assert_eq!(capped.findings.len(), 1);
    assert!(capped.truncated);
}
