//! `ifc-parser` — STEP physical file (IFC-SPF) reader.
//!
//! # Design (carried over from a validated sibling implementation)
//!
//! The parse plan is: mmap the file, split it into partitions **aligned to
//! record starts**, scan partitions in parallel with rayon, then resolve
//! `#id` references into dense indices.
//!
//! # The pitfall that must not be re-learned
//!
//! Partition boundaries must resync to a record start (`#<digits>=`). Counting
//! parenthesis depth from an arbitrary byte offset collapses the entire file
//! into a single partition, because the depth never returns to zero when you
//! begin mid-record. This is a real, previously-hit bug; the regression test
//! for it is mandatory when the partitioner lands.
//!
//! # Status
//!
//! Scaffold. Header detection is implemented and tested against real fixtures;
//! the parallel body scan is Stage 1 in `docs/ROADMAP.md`.

use thiserror::Error;

/// Why a parse failed.
#[derive(Debug, Error, PartialEq)]
pub enum ParseError {
    #[error("not a STEP physical file: missing ISO-10303-21 magic")]
    NotStepFile,
    #[error("malformed header: {0}")]
    MalformedHeader(String),
}

/// Does this byte slice start with the STEP magic?
///
/// Leading whitespace and a UTF-8 BOM are tolerated — both occur in files
/// produced by real authoring tools.
pub fn is_step_file(bytes: &[u8]) -> bool {
    let bytes = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
    let start = bytes
        .iter()
        .position(|b| !b.is_ascii_whitespace())
        .unwrap_or(bytes.len());
    bytes[start..].starts_with(b"ISO-10303-21")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// Fixtures live at the workspace root, outside this crate.
    fn fixture_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../test/fixtures")
    }

    #[test]
    fn accepts_leading_whitespace_and_bom() {
        assert!(is_step_file(b"ISO-10303-21;"));
        assert!(is_step_file(b"\r\n  ISO-10303-21;"));
        assert!(is_step_file(b"\xEF\xBB\xBFISO-10303-21;"));
        assert!(!is_step_file(b"<?xml version=\"1.0\"?>"));
    }

    /// Exercises the real fixture corpus rather than a synthetic string.
    #[test]
    fn every_committed_fixture_is_recognized_as_a_step_file() {
        let mut checked = 0;
        for sub in ["ifclite-geometry", "ifcopenshell-validate"] {
            let dir = fixture_root().join(sub);
            let entries = std::fs::read_dir(&dir)
                .unwrap_or_else(|e| panic!("fixture dir {} unreadable: {e}", dir.display()));
            for entry in entries {
                let path = entry.unwrap().path();
                if path.extension().is_some_and(|e| e == "ifc") {
                    let bytes = std::fs::read(&path).unwrap();
                    assert!(
                        is_step_file(&bytes),
                        "fixture not recognized as STEP: {}",
                        path.display()
                    );
                    checked += 1;
                }
            }
        }
        assert_eq!(checked, 19, "expected the committed fixture count");
    }
}
