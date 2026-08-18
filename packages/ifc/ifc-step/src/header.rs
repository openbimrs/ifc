//! File magic and the `HEADER;` section.
//!
//! Implemented: [`is_step_file`]. Still to come: `FILE_DESCRIPTION`,
//! `FILE_NAME`, and `FILE_SCHEMA` extraction — the last of which selects the
//! schema table in `ifc-schema`.

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
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../test/fixtures")
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
