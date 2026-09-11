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
        let index = ifc_step::Index::scan(&bytes);
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
        let index = ifc_step::Index::scan(&bytes);
        for id in model.ids() {
            let eager = model.get(id).unwrap();
            let lazy = index
                .entity(id)
                .unwrap_or_else(|| panic!("{} missing #{}", path.display(), id.0));
            assert_eq!(&lazy, eager, "#{} differs in {}", id.0, path.display());
        }
    }
}
