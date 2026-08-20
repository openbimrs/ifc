//! Architecture test: no source file grows into a monolith.
//!
//! The project requirement is modular code, not 5,000-line files. That is easy
//! to agree to and easy to erode one commit at a time, so it is enforced here
//! rather than left to review.
//!
//! # Why a test and not a lint
//!
//! Clippy has no file-length lint, and `rustfmt` does not care. A test is the
//! only mechanism that fails CI on the day a file crosses the line.
//!
//! # Thresholds
//!
//! `WARN_LINES` is generous on purpose: a file legitimately reaching it is
//! usually a module that grew several responsibilities, which is exactly when
//! it should be split. The limit counts every line including docs, because a
//! file nobody can scroll through is unreadable regardless of what fills it.

use std::path::{Path, PathBuf};

/// Hard limit. A source file above this fails the build.
const MAX_LINES: usize = 800;

/// Files intentionally allowed to exceed the limit, with the reason.
/// Empty by design -- add an entry only with a written justification.
const EXEMPT: &[(&str, &str)] = &[];

fn workspace_root() -> PathBuf {
    cargo_metadata::MetadataCommand::new()
        .no_deps()
        .exec()
        .expect("cargo metadata must describe the runtime workspace")
        .workspace_root
        .into_std_path_buf()
}

/// Every `.rs` file under a directory, recursively, skipping `target`.
fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        if path.is_dir() {
            if name != "target" && !name.starts_with('.') {
                rust_files(&path, out);
            }
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

fn is_exempt(path: &Path) -> bool {
    let s = path.to_string_lossy();
    EXEMPT.iter().any(|(frag, _)| s.contains(frag))
}

#[test]
fn no_source_file_is_a_monolith() {
    let root = workspace_root();
    let mut files = Vec::new();
    for group in ["packages", "apps", "bindings"] {
        rust_files(&root.join(group), &mut files);
    }
    assert!(
        files.len() > 50,
        "expected to scan the whole workspace, saw {} files -- is the root path right?",
        files.len()
    );

    let mut offenders: Vec<(PathBuf, usize)> = Vec::new();
    for path in &files {
        if is_exempt(path) {
            continue;
        }
        let lines = std::fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
            .lines()
            .count();
        if lines > MAX_LINES {
            offenders.push((path.clone(), lines));
        }
    }

    assert!(
        offenders.is_empty(),
        "these files exceed {MAX_LINES} lines and should be split into modules:\n{}",
        offenders
            .iter()
            .map(|(p, n)| format!("  {n:>5} lines  {}", p.display()))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/// Return whether a crate root declares at least one Rust module, regardless of
/// that module's visibility.
fn has_module_declaration(source: &str) -> bool {
    syn::parse_file(source)
        .expect("lib.rs must be valid Rust syntax")
        .items
        .iter()
        .any(|item| matches!(item, syn::Item::Mod(_)))
}

#[test]
fn private_modules_activate_the_lib_rs_delegation_budget() {
    assert!(has_module_declaration("mod internal;"));
    assert!(has_module_declaration("pub(crate) mod restricted;"));
    assert!(has_module_declaration("pub mod public;"));
    assert!(!has_module_declaration("pub fn facade() {}"));
}

/// A crate whose `lib.rs` carries real code instead of delegating to modules is
/// the monolith's first stage. `lib.rs` should declare modules, re-export, and
/// document -- not implement.
#[test]
fn lib_rs_delegates_rather_than_implements() {
    let root = workspace_root();
    let mut offenders = Vec::new();

    for group in ["../axiolid", "packages/ifc", "packages/openbim"] {
        let Ok(entries) = std::fs::read_dir(root.join(group)) else {
            continue;
        };
        for entry in entries.flatten() {
            let lib = entry.path().join("src/lib.rs");
            if !lib.exists() {
                continue;
            }
            let text = std::fs::read_to_string(&lib).unwrap();
            let has_modules = has_module_declaration(&text);
            // Count non-doc, non-blank, non-module/re-export lines.
            let code_lines = text
                .lines()
                .map(str::trim)
                .filter(|l| {
                    !l.is_empty()
                        && !l.starts_with("//")
                        && !l.starts_with("pub mod ")
                        && !l.starts_with("pub use ")
                        && !l.starts_with("mod ")
                        && !l.starts_with("use ")
                        && !l.starts_with("#!")
                })
                .count();
            if has_modules && code_lines > 40 {
                offenders.push(format!(
                    "  {} has {code_lines} lines of code beside its module declarations",
                    lib.display()
                ));
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "lib.rs should declare and document modules, not implement behaviour:\n{}",
        offenders.join("\n")
    );
}
