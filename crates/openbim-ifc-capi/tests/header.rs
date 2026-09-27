//! The committed C header must be exactly what cbindgen generates now.
//!
//! A hand-maintained second copy of the ABI would drift from the Rust
//! exports; generating it and diffing in the gate makes drift a test
//! failure. `UPDATE_HEADER=1` rewrites the committed file instead.

use std::path::PathBuf;

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn generated() -> String {
    let dir = crate_dir();
    let config = cbindgen::Config::from_file(dir.join("cbindgen.toml")).expect("cbindgen.toml");
    let mut out = Vec::new();
    cbindgen::Builder::new()
        .with_crate(&dir)
        .with_config(config)
        .generate()
        .expect("cbindgen generates the header")
        .write(&mut out);
    // Normalize line endings so the check is platform-independent.
    String::from_utf8(out)
        .expect("UTF-8 header")
        .replace("\r\n", "\n")
}

#[test]
fn the_committed_header_matches_the_exports() {
    let path = crate_dir().join("include/openbim_ifc.h");
    let fresh = generated();
    if std::env::var_os("UPDATE_HEADER").is_some() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, &fresh).unwrap();
        return;
    }
    let committed = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{}: {e}; run with UPDATE_HEADER=1", path.display()));
    assert!(
        committed == fresh,
        "include/openbim_ifc.h is stale; regenerate with UPDATE_HEADER=1 cargo test -p openbim-ifc-capi --test header"
    );
}

#[test]
fn every_export_is_versioned() {
    let header = generated();
    let exports: Vec<&str> = header
        .lines()
        .filter_map(|line| line.split_once(" openbim_ifc_").map(|(_, rest)| rest))
        .collect();
    assert!(exports.len() >= 18, "found {} exports", exports.len());
    for export in exports {
        assert!(
            export.starts_with("v0_1_"),
            "unversioned export: openbim_ifc_{export}"
        );
    }
}
