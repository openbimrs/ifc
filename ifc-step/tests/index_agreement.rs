//! The index must agree with the parser, on every fixture in the repository.
//!
//! The scanner finds record boundaries without running the lexer. That is the
//! whole point and also the whole risk: a STEP string can contain `#`, `;`,
//! and parentheses, so a careless scan splits records mid-literal and loses
//! data silently. These tests are what make the fast path trustworthy.

use ifc_model::Codec;
use std::path::{Path, PathBuf};

fn fixtures() -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("test/fixtures");
    let mut out = Vec::new();
    let mut stack = vec![root];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "ifc") {
                out.push(path);
            }
        }
    }
    out.sort();
    out
}

#[test]
fn the_index_finds_exactly_the_entities_the_parser_finds() {
    let files = fixtures();
    assert!(
        files.len() > 20,
        "expected a real corpus, found {}",
        files.len()
    );
    for path in files {
        let bytes = std::fs::read(&path).unwrap();
        let Ok(model) = ifc_step::StepCodec.read_bytes(&bytes) else {
            continue;
        };
        let index = ifc_step::Index::scan(&bytes).expect("a readable file scans");
        let mut eager: Vec<u64> = model.ids().map(|i| i.0).collect();
        let mut lazy: Vec<u64> = index.ids().map(|i| i.0).collect();
        eager.sort_unstable();
        lazy.sort_unstable();
        assert_eq!(lazy, eager, "id mismatch in {}", path.display());
    }
}

#[test]
fn a_lazily_decoded_entity_equals_its_eager_counterpart() {
    for path in fixtures() {
        let bytes = std::fs::read(&path).unwrap();
        let Ok(model) = ifc_step::StepCodec.read_bytes(&bytes) else {
            continue;
        };
        let index = ifc_step::Index::scan(&bytes).expect("a readable file scans");
        for id in model.ids() {
            let eager = model.get(id).unwrap();
            let lazy = index
                .entity(id)
                .expect("a record of a readable file decodes")
                .unwrap_or_else(|| panic!("{} missing #{}", path.display(), id.0));
            assert_eq!(&lazy, eager, "#{} differs in {}", id.0, path.display());
        }
    }
}

const HEAD: &str = "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION(('d'),'2;1');\n\
FILE_NAME('n','t',('a'),('o'),'p','s','z');\nFILE_SCHEMA(('IFC2X3'));\nENDSEC;\nDATA;\n";
const TAIL: &str = "ENDSEC;\nEND-ISO-10303-21;\n";

fn wrap(data: &str) -> Vec<u8> {
    format!("{HEAD}{data}{TAIL}").into_bytes()
}

#[test]
fn subsets_carry_the_files_header_and_are_in_file_order() {
    let bytes = wrap("#3=IFCA(#1);\n#1=IFCB('x');\n#2=IFCC(#3);\n");
    let index = ifc_step::Index::scan(&bytes).unwrap();
    // Out-of-order ids are still found.
    assert_eq!(index.type_of(ifc_model::EntityId(1)), Some("IFCB"));
    let subset = index
        .materialize_closure(&[ifc_model::EntityId(2)])
        .unwrap();
    assert_eq!(subset.header().schema_token(), Some("IFC2X3"));
    let ids: Vec<u64> = subset.ids().map(|id| id.0).collect();
    assert_eq!(ids, [3, 1, 2], "file order, not request order");
}

#[test]
fn defects_are_errors_not_silence() {
    // Framing and representability defects fail the scan.
    for data in [
        "#1=IFCA(1);\njunk\n#2=IFCB(2);\n",
        "#1=(IFCA(1)IFCB(2));\n",
        "#99999999999999999999=IFCA(1);\n",
    ] {
        assert!(ifc_step::Index::scan(&wrap(data)).is_err(), "{data:?}");
    }
    assert!(ifc_step::Index::scan(&[wrap(""), wrap("")].concat()).is_err());
    // A defect inside a record fails when that record is decoded.
    let bytes = wrap("#1=IFCA(1,,2);\n#2=IFCB(99999999999999999999);\n#3=IFCC(#1);\n");
    let index = ifc_step::Index::scan(&bytes).unwrap();
    assert!(index.entity(ifc_model::EntityId(1)).is_err());
    assert!(index.entity(ifc_model::EntityId(2)).is_err());
    assert!(index
        .materialize_closure(&[ifc_model::EntityId(3)])
        .is_err());
    assert!(index.entity(ifc_model::EntityId(4)).unwrap().is_none());
}
