//! The tests that justify the architecture.
//!
//! Two claims are made elsewhere in prose; this file makes them falsifiable:
//!
//! 1. Every committed fixture parses.
//! 2. Parse to write to re-parse preserves the model **structurally** —
//!    including entities whose meaning no crate in this build understands.

use ifc_model::{Codec, Model};
use ifc_step::StepCodec;
use std::path::{Path, PathBuf};

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../test/fixtures")
}

/// Every `.ifc` file committed to the corpus.
fn fixtures() -> Vec<PathBuf> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().is_some_and(|e| e == "ifc") {
                out.push(path);
            }
        }
    }
    let mut out = Vec::new();
    walk(&fixture_dir(), &mut out);
    out.sort();
    out
}

/// Compare two models by content rather than by bytes.
///
/// Byte equality is the wrong assertion: real lexemes normalize on write
/// (`1.0` becomes `1.`) and comments are dropped. What must hold is that the
/// entity graph is identical.
fn assert_structurally_equal(a: &Model, b: &Model, context: &str) {
    assert_eq!(a.len(), b.len(), "{context}: entity count differs");
    assert_eq!(
        a.header().schema_token(),
        b.header().schema_token(),
        "{context}: schema token differs"
    );
    for (id, entity) in a.iter() {
        let other = b
            .get(id)
            .unwrap_or_else(|| panic!("{context}: entity {id} missing after roundtrip"));
        assert_eq!(
            entity.type_name, other.type_name,
            "{context}: type of {id} differs"
        );
        assert_eq!(
            entity.attributes.len(),
            other.attributes.len(),
            "{context}: attribute count of {id} ({}) differs",
            entity.type_name
        );
        assert_eq!(
            entity.attributes, other.attributes,
            "{context}: attributes of {id} ({}) differ",
            entity.type_name
        );
    }
}

#[test]
fn every_committed_fixture_parses() {
    let files = fixtures();
    assert!(
        files.len() >= 19,
        "expected the committed corpus, found {}",
        files.len()
    );

    let codec = StepCodec;
    for path in &files {
        let model = codec
            .read_path(path)
            .unwrap_or_else(|e| panic!("failed to parse {}: {e}", path.display()));
        assert!(
            !model.is_empty(),
            "{} parsed to zero entities",
            path.display()
        );
    }
}

#[test]
fn every_fixture_survives_a_roundtrip() {
    let codec = StepCodec;
    for path in fixtures() {
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let original = codec.read_path(&path).unwrap();
        let bytes = codec.write_bytes(&original).unwrap();
        let reparsed = codec
            .read_bytes(&bytes)
            .unwrap_or_else(|e| panic!("{name}: output did not re-parse: {e}"));
        assert_structurally_equal(&original, &reparsed, &name);
    }
}

/// The claim that makes the feature-gated design safe.
///
/// This test build contains **no domain crate at all** — no `ifc-cost`, no
/// `ifc-schedule`, nothing that knows what a cost item is. A file full of cost
/// entities must still survive a round-trip untouched. If the model held typed
/// domain structs instead of structural entities, this could not pass.
#[test]
fn cost_data_survives_without_any_domain_crate_compiled() {
    let source = br#"ISO-10303-21;
HEADER;
FILE_DESCRIPTION((''),'2;1');
FILE_NAME('cost.ifc','2026-01-01T00:00:00',(''),(''),'','','');
FILE_SCHEMA(('IFC4'));
ENDSEC;
DATA;
#1= IFCCOSTSCHEDULE('3vB2Y0dTv1LhX9ZzQqFbcd',$,'Budget',$,$,$,$,$,.BUDGET.,$,$);
#2= IFCCOSTITEM('1aB2Y0dTv1LhX9ZzQqFbce',$,'Excavation',$,$,(#3),$,.NOTDEFINED.);
#3= IFCCOSTVALUE('Estimate',$,IFCMONETARYMEASURE(12345.67),$,$,$,$,$);
#4= IFCUNKNOWNFUTUREENTITY('who knows',(1,2,3),.T.,#1);
ENDSEC;
END-ISO-10303-21;
"#;

    let codec = StepCodec;
    let model = codec.read_bytes(source).unwrap();
    assert_eq!(model.len(), 4);

    // The cost entities are present and intact, despite nothing here knowing
    // what "cost" means.
    let cost_item = model.of_type("IFCCOSTITEM").next().unwrap().1;
    assert_eq!(cost_item.text(2), Some("Excavation"));

    let cost_value = model.of_type("IFCCOSTVALUE").next().unwrap().1;
    assert_eq!(
        cost_value.attribute(2).unwrap().unwrap_typed().as_f64(),
        Some(12345.67)
    );

    // An entity type from no schema we know still round-trips.
    let unknown = model.of_type("IFCUNKNOWNFUTUREENTITY").next().unwrap().1;
    assert_eq!(unknown.attributes.len(), 4);

    let bytes = codec.write_bytes(&model).unwrap();
    let reparsed = codec.read_bytes(&bytes).unwrap();
    assert_structurally_equal(&model, &reparsed, "cost roundtrip");

    let text = String::from_utf8(bytes).unwrap();
    assert!(text.contains("IFCMONETARYMEASURE(12345.67)"));
    assert!(text.contains("IFCUNKNOWNFUTUREENTITY"));
}

#[test]
fn unicode_names_survive_a_roundtrip() {
    let source = "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\nFILE_NAME('u.ifc','',(''),(''),'','','');\nFILE_SCHEMA(('IFC4'));\nENDSEC;\nDATA;\n#1= IFCWALL('2O2Fr$t4X7Zf8NOew3FLOH',$,'\\X2\\30D330EB\\X0\\',$,$,$,$,$);\nENDSEC;\nEND-ISO-10303-21;\n";

    let codec = StepCodec;
    let model = codec.read_bytes(source.as_bytes()).unwrap();
    let wall = model.of_type("IFCWALL").next().unwrap().1;
    assert_eq!(wall.text(2), Some("\u{30d3}\u{30eb}"));

    let bytes = codec.write_bytes(&model).unwrap();
    let reparsed = codec.read_bytes(&bytes).unwrap();
    let wall2 = reparsed.of_type("IFCWALL").next().unwrap().1;
    assert_eq!(wall2.text(2), Some("\u{30d3}\u{30eb}"));
}
