//! Guard where the repository keeps its context.
//!
//! The root `AGENTS.md` is the one file every contributor reads first: short,
//! stable, and the only one of its name. Each crate carries a `README.md`
//! (its crates.io page) with the crate's purpose and the design notes no test,
//! ADR or module doc already holds; the reasoning behind a module lives in its
//! `//!` docs. Open work is not context: it lives in GitHub issues and
//! `TODO(#N)` markers, never in a checked-in plan.
//!
//! These tests keep that shape: a nested `AGENTS.md` or a PLAN.md cannot
//! regrow, a crate cannot ship without a README, a README cannot grow into a
//! manual or a checklist, and a pointer to a README or `AGENTS.md` cannot
//! dangle after a file moves.

use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::path::{Component, Path, PathBuf};

#[path = "support/progressive_markdown.rs"]
mod progressive_markdown;
use progressive_markdown::{context_pointer_tokens, inline_code_tokens};

// This registry deliberately duplicates the initial capability set. A coordinated
// source/module deletion must still change a separate reviewable baseline, and
// every ownership scaffold (`//! Planned owner:`) must be listed here.
const REQUIRED_SCAFFOLD_PATHS: &str = include_str!("required_scaffold_paths.txt");

/// The root `AGENTS.md` is read in full before any change, so it stays short.
const ROOT_AGENTS_MAX_LINES: usize = 120;

/// A crate README is a crates.io page and a place for a few design notes, not
/// a manual: the API belongs in rustdoc, the site in `docs/`.
const README_MAX_LINES: usize = 150;

/// The workspace crate count the scans below must at least reach, so a layout
/// change that makes a filter match nothing fails instead of passing.
const MIN_CRATES: usize = 18;

fn ifc_root() -> PathBuf {
    let workspace_root = cargo_metadata::MetadataCommand::new()
        .no_deps()
        .exec()
        .expect("cargo metadata must describe the runtime workspace")
        .workspace_root
        .into_std_path_buf();
    if workspace_root.join("crates/ifc-model/Cargo.toml").is_file() {
        workspace_root
    } else {
        workspace_root.join("packages/ifc")
    }
}

/// The directory holding every workspace crate except `xtask`. The scaffold
/// registry is relative to it.
fn crates_dir(root: &Path) -> PathBuf {
    root.join("crates")
}

/// Every crate directory (holding a `Cargo.toml`) directly inside `crates/`.
fn crate_dirs(root: &Path) -> Vec<PathBuf> {
    let mut dirs: Vec<_> = std::fs::read_dir(crates_dir(root))
        .expect("read crates/")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.join("Cargo.toml").is_file())
        .collect();
    dirs.sort();
    dirs
}

/// Is this a crate directory belonging to the IFC layer?
///
/// The IFC crates share `crates/` with the bindings, so a directory scan
/// alone would sweep those in. Select by NAME.
fn is_ifc_layer_dir(path: &Path) -> bool {
    let Some(name) = path.file_name().map(|n| n.to_string_lossy().to_string()) else {
        return false;
    };
    (name.starts_with("ifc-") || name == "openbim-ifc") && path.join("Cargo.toml").is_file()
}

/// Collect repository-owned files only. Dependency, generated and local
/// reference trees can contain foreign agent instructions and READMEs.
fn walk(dir: &Path, files: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display())) {
        let path = entry.expect("directory entry").path();
        if path.is_dir() {
            let name = path.file_name().unwrap_or_default();
            if name != "target"
                && name != "references"
                && name != "node_modules"
                && name != "pkg"
                && !name.to_string_lossy().starts_with('.')
            {
                walk(&path, files);
            }
        } else {
            files.push(path);
        }
    }
}

fn named(files: &[PathBuf], name: &str) -> Vec<PathBuf> {
    files
        .iter()
        .filter(|path| path.file_name() == Some(OsStr::new(name)))
        .cloned()
        .collect()
}

fn has_checkbox(text: &str) -> bool {
    text.lines().any(|line| {
        let line = line.trim_start();
        line.starts_with("- [ ]") || line.starts_with("- [x]") || line.starts_with("- [X]")
    })
}

#[test]
fn the_root_agents_md_is_the_only_one_and_stays_short() {
    let root = ifc_root();
    let mut files = Vec::new();
    walk(&root, &mut files);
    // Guards the walk itself: a scan that sees nothing would pass vacuously.
    assert!(
        named(&files, "Cargo.toml").len() > MIN_CRATES,
        "the repository walk found too few manifests; did the layout move?"
    );

    let plans = named(&files, "PLAN.md");
    assert!(
        plans.is_empty(),
        "open work belongs in GitHub issues and `TODO(#N)` markers, not PLAN.md: {plans:#?}"
    );

    let agents = named(&files, "AGENTS.md");
    assert_eq!(
        agents,
        [root.join("AGENTS.md")],
        "the root AGENTS.md is the only one; put crate context in the crate's \
         README.md and module context in its `//!` docs"
    );

    let text = std::fs::read_to_string(root.join("AGENTS.md")).unwrap();
    assert!(
        !has_checkbox(&text),
        "AGENTS.md holds standing rules, not progress; open work is a GitHub issue"
    );
    let lines = text.lines().count();
    assert!(
        lines <= ROOT_AGENTS_MAX_LINES,
        "AGENTS.md has {lines} lines (cap {ROOT_AGENTS_MAX_LINES}); move crate detail \
         into that crate's README.md or module docs"
    );
}

#[test]
fn every_crate_has_a_small_readme() {
    let root = ifc_root();
    let crates = crate_dirs(&root);
    assert!(
        crates.len() >= MIN_CRATES,
        "expected every workspace crate under crates/, found {}",
        crates.len()
    );
    let mut problems = Vec::new();
    for dir in &crates {
        let readme = dir.join("README.md");
        let Ok(text) = std::fs::read_to_string(&readme) else {
            problems.push(format!("{} has no README.md", dir.display()));
            continue;
        };
        let lines = text.lines().count();
        if lines > README_MAX_LINES {
            problems.push(format!(
                "{} has {lines} lines (cap {README_MAX_LINES}); API detail belongs in rustdoc",
                readme.display()
            ));
        }
        if has_checkbox(&text) {
            problems.push(format!(
                "{} holds a checklist; open work is a GitHub issue",
                readme.display()
            ));
        }
    }
    assert!(
        problems.is_empty(),
        "crate READMEs:\n{}",
        problems.join("\n")
    );
}

#[test]
fn publishable_crates_ship_their_readme() {
    let metadata = cargo_metadata::MetadataCommand::new()
        .no_deps()
        .exec()
        .expect("cargo metadata");
    let crates = crates_dir(&ifc_root());
    let mut publishable = 0;
    let mut missing = Vec::new();
    for package in &metadata.packages {
        let dir = package.manifest_path.parent().expect("manifest dir");
        if dir.parent().map(|p| p.as_std_path()) != Some(crates.as_path()) {
            continue;
        }
        // `publish = false` is reported as an empty registry list.
        if package.publish.as_ref().is_some_and(Vec::is_empty) {
            continue;
        }
        publishable += 1;
        // Read the manifest, not `package.readme`: cargo fills that in from
        // a README.md it finds on disk, so it cannot tell a declared page
        // from an accidental one.
        let manifest = std::fs::read_to_string(&package.manifest_path).expect("manifest");
        let package_table = manifest
            .split_once("[package]")
            .map(|(_, rest)| rest.split("\n[").next().unwrap_or(rest))
            .unwrap_or_default();
        if !package_table
            .lines()
            .any(|line| line.trim() == r#"readme = "README.md""#)
        {
            missing.push(package.name.to_string());
        }
    }
    assert!(
        publishable >= MIN_CRATES,
        "expected the publishable crates under crates/, found {publishable}"
    );
    assert!(
        missing.is_empty(),
        "publishable crates must declare `readme = \"README.md\"` as their crates.io page: {missing:?}"
    );
}

fn normalized_relative(path: &Path) -> bool {
    !path.as_os_str().is_empty()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

fn required_scaffold_paths() -> Vec<&'static str> {
    REQUIRED_SCAFFOLD_PATHS
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect()
}

#[test]
fn ownership_scaffolds_are_registered() {
    let root = crates_dir(&ifc_root());
    let registered: BTreeSet<_> = required_scaffold_paths().into_iter().collect();
    let mut scaffolds = 0;
    let mut unregistered = Vec::new();
    for entry in std::fs::read_dir(&root).expect("read crates/") {
        let crate_dir = entry.expect("directory entry").path();
        if !is_ifc_layer_dir(&crate_dir) {
            continue;
        }
        let mut source_files = Vec::new();
        walk(&crate_dir.join("src"), &mut source_files);
        for source in source_files
            .into_iter()
            .filter(|path| path.extension().is_some_and(|ext| ext == "rs"))
        {
            let text = std::fs::read_to_string(&source).unwrap();
            if !text.contains("//! Planned owner:") {
                continue;
            }
            scaffolds += 1;
            let relative = source.strip_prefix(&root).unwrap();
            assert!(normalized_relative(relative));
            let relative = relative.to_string_lossy().replace('\\', "/");
            if !registered.contains(relative.as_str()) {
                unregistered.push(relative);
            }
        }
    }
    assert!(
        unregistered.is_empty(),
        "ownership scaffolds missing from tests/required_scaffold_paths.txt:\n{}",
        unregistered.join("\n")
    );
    // Guards the scan itself: a layout change that finds nothing must not pass.
    assert!(
        scaffolds >= 1,
        "found no `//! Planned owner:` scaffold; did the source layout move?"
    );
}

#[test]
fn required_scaffold_capability_seams_are_preserved() {
    let root = crates_dir(&ifc_root());
    let required = required_scaffold_paths();
    // Ratchet, not a constant: it may only be lowered when a seam is removed
    // deliberately and the reason is recorded. Lowered 187 -> 185 on
    // 2026-09-05 when `input::profile` and `input::topology` were deleted;
    // both were doc-only placeholders whose slots are owned by
    // `lower::profile` and `resource::topology`/`solid::brep` respectively, so
    // keeping them would have mandated a second reader of the same slots.
    assert!(
        required.len() >= 185,
        "the explicit capability baseline must not shrink silently"
    );

    let mut sorted = required.clone();
    sorted.sort_unstable();
    assert_eq!(required, sorted, "required scaffold paths must be sorted");
    let unique: BTreeSet<_> = required.iter().copied().collect();
    assert_eq!(
        unique.len(),
        required.len(),
        "required scaffold paths must be unique"
    );

    for token in required {
        let relative = Path::new(token);
        assert!(
            normalized_relative(relative),
            "required scaffold path is not normalized: {token}"
        );
        let parts: Vec<_> = relative.iter().collect();
        assert!(
            parts.len() >= 3 && parts[1] == OsStr::new("src"),
            "required scaffold path must be <crate>/src/<file>.rs: {token}"
        );
        assert_eq!(
            relative.extension(),
            Some(OsStr::new("rs")),
            "required scaffold path is not Rust source: {token}"
        );
        assert!(
            root.join(relative).is_file(),
            "required scaffold capability seam is missing: {token}"
        );

        assert!(
            root.join(parts[0]).join("Cargo.toml").is_file(),
            "required scaffold path has no IFC crate owner: {token}"
        );
    }
}

/// A pointer to a context document: a README, the root AGENTS.md, or a
/// retired name (a nested AGENTS.md, a PLAN.md), which is then reported as
/// missing. A glob such as `crates/*/README.md` names a pattern, not a file.
fn is_context_pointer(token: &str) -> bool {
    !token.contains("://")
        && !token.starts_with("mailto:")
        && !token.chars().any(|c| c.is_whitespace() || c == '*')
        && Path::new(token).file_name().is_some_and(|name| {
            name == OsStr::new("AGENTS.md")
                || name == OsStr::new("README.md")
                || name == OsStr::new("PLAN.md")
        })
}

/// Resolve a pointer as written: relative to the file that names it, or, for
/// a repository path such as `test/fixtures/README.md`, to the root.
fn resolve_pointer(file: &Path, root: &Path, token: &str) -> Option<PathBuf> {
    [file.parent().unwrap().join(token), root.join(token)]
        .into_iter()
        .find(|candidate| candidate.is_file())
}

#[test]
fn context_document_pointers_resolve() {
    let root = ifc_root();
    let canonical_root = root.canonicalize().expect("canonical IFC package root");
    let mut files = Vec::new();
    walk(&root, &mut files);
    let mut broken = Vec::new();
    let mut resolved = 0;

    // ADRs are immutable records of their time, so a pointer in one may name a
    // file that has since been retired (ADR 0016 retired PLAN.md).
    let adrs = root.join("docs/adr");
    for file in files.iter().filter(|path| {
        path.extension()
            .is_some_and(|ext| ext == OsStr::new("md") || ext == OsStr::new("rs"))
            && !path.starts_with(&adrs)
    }) {
        let text = std::fs::read_to_string(file).unwrap();
        for token in context_pointer_tokens(&text)
            .into_iter()
            .filter(|token| is_context_pointer(token))
        {
            if Path::new(&token).is_absolute() {
                broken.push(format!("{} -> absolute {token}", file.display()));
                continue;
            }
            let Some(target) = resolve_pointer(file, &root, &token) else {
                broken.push(format!("{} -> missing {token}", file.display()));
                continue;
            };
            match target.canonicalize() {
                Ok(path) if path.starts_with(&canonical_root) => resolved += 1,
                Ok(_) => broken.push(format!("{} -> outside package {token}", file.display())),
                Err(_) => broken.push(format!("{} -> unreadable {token}", file.display())),
            }
        }
    }

    assert!(
        broken.is_empty(),
        "README/AGENTS.md pointers that do not resolve:\n{}",
        broken.join("\n")
    );
    // Guards the extraction: the root AGENTS.md and the contributing docs
    // name READMEs, so finding none means the token scan broke.
    assert!(
        resolved >= 3,
        "found only {resolved} context pointers; did the extraction break?"
    );
}

#[test]
fn readme_source_paths_exist() {
    let root = ifc_root();
    let mut missing = Vec::new();
    let mut checked = 0;
    for dir in crate_dirs(&root) {
        let Ok(text) = std::fs::read_to_string(dir.join("README.md")) else {
            continue;
        };
        for token in inline_code_tokens(&text)
            .into_iter()
            .filter(|token| token.ends_with(".rs") && !token.contains(char::is_whitespace))
        {
            checked += 1;
            if !dir.join(&token).is_file() && !root.join(&token).is_file() {
                missing.push(format!("{}/README.md -> {token}", dir.display()));
            }
        }
    }
    assert!(
        missing.is_empty(),
        "crate READMEs name Rust files that do not exist:\n{}",
        missing.join("\n")
    );
    assert!(
        checked >= 1,
        "no README names a Rust file; did the token scan break?"
    );
}

#[test]
fn source_docs_do_not_point_at_the_retired_global_roadmap() {
    let root = ifc_root();
    let mut files = Vec::new();
    walk(&root, &mut files);
    let offenders: Vec<_> = files
        .into_iter()
        .filter(|path| path.extension().is_some_and(|ext| ext == "rs"))
        .filter(|path| {
            std::fs::read_to_string(path)
                .map(|text| text.contains(concat!("docs/", "ROADMAP.md")))
                .unwrap_or(false)
        })
        .collect();
    assert!(
        offenders.is_empty(),
        "source docs point at the retired global roadmap; cite the issue instead: {offenders:#?}"
    );
}
