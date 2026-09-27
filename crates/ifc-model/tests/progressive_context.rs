//! Guard the progressive context protocol.
//!
//! `../AGENTS.md` is standing context: purpose, boundaries, invariants and
//! gates, nested so an agent reads only the files on the path to its target.
//! Open work is not context: it lives in GitHub issues and `TODO(#N)` markers,
//! never in a checked-in plan. Shape and pointers are checked so a new crate
//! or module cannot silently fall outside the protocol.

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

const REQUIRED_NESTED_CONTEXTS: &[&str] = &[
    "ifc-geometry/src/input",
    "ifc-geometry/src/lower",
    "ifc-geometry/src/resource",
    "ifc-geometry/src/curve",
    "ifc-geometry/src/surface",
    "ifc-geometry/src/solid",
    "ifc-geometry/src/constraint",
    "ifc-geometry/src/select",
    "ifc-geometry/src/rules",
    "ifc-material/src/material",
    "ifc-material/src/layer",
    "ifc-material/src/profile",
    "ifc-material/src/constituent",
    "ifc-material/src/usage",
    "ifc-properties/src/pset",
    "ifc-properties/src/quantity",
    "ifc-properties/src/unit",
    "ifc-properties/src/template",
    "ifc-georef/src/crs",
    "ifc-georef/src/conversion",
    "ifc-georef/src/context",
    "ifc-alignment/src/horizontal",
    "ifc-alignment/src/vertical",
    "ifc-alignment/src/cant",
    "ifc-alignment/src/curve",
    "ifc-alignment/src/placement",
    "ifc-style/src/assignment",
    "ifc-style/src/surface_style",
    "ifc-style/src/texture",
    "ifc-validate/src/structure",
    "ifc-validate/src/type_check",
    "ifc-validate/src/where_rule",
    "ifc-validate/src/report",
    "ifc-model/src/index",
    "ifc-model/src/mutation",
    "ifc-model/src/traverse",
];

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
/// and nested-context registries are relative to it.
fn crates_dir(root: &Path) -> PathBuf {
    root.join("crates")
}

/// Crate directories (holding a `Cargo.toml`) directly inside `dir`.
fn crate_dirs_in(dir: &Path) -> usize {
    std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
        .filter_map(Result::ok)
        .filter(|entry| entry.path().join("Cargo.toml").is_file())
        .count()
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

/// Collect repository-owned context candidates only. Dependency and generated
/// trees can contain foreign agent instructions with unrelated protocols.
fn walk(dir: &Path, files: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display())) {
        let path = entry.expect("directory entry").path();
        if path.is_dir() {
            let name = path.file_name().unwrap_or_default();
            if name != "target"
                && name != "references"
                && name != "node_modules"
                && !name.to_string_lossy().starts_with('.')
            {
                walk(&path, files);
            }
        } else {
            files.push(path);
        }
    }
}

#[test]
fn every_context_boundary_has_standing_rules_and_no_plan() {
    let root = ifc_root();
    let mut files = Vec::new();
    walk(&root, &mut files);

    let plans: Vec<_> = files
        .iter()
        .filter(|path| path.file_name().is_some_and(|name| name == "PLAN.md"))
        .collect();
    assert!(
        plans.is_empty(),
        "open work belongs in GitHub issues and `TODO(#N)` markers, not PLAN.md: {plans:#?}"
    );

    let agents: BTreeSet<_> = files
        .iter()
        .filter(|path| path.file_name().is_some_and(|name| name == "AGENTS.md"))
        .map(|path| path.parent().unwrap().to_path_buf())
        .collect();
    // Crates live in `crates/`; tooling such as `xtask` stays at the root.
    let crates = crates_dir(&root);
    let crate_count = crate_dirs_in(&crates) + crate_dirs_in(&root);
    assert!(
        agents.len() >= crate_count + 1 + REQUIRED_NESTED_CONTEXTS.len(),
        "expected package root + every crate + required nested boundaries; found {} for {crate_count} crates",
        agents.len()
    );
    for relative in REQUIRED_NESTED_CONTEXTS {
        assert!(
            agents.contains(&crates.join(relative)),
            "required progressive boundary is missing: {relative}"
        );
    }

    for dir in agents {
        let agents_text = std::fs::read_to_string(dir.join("AGENTS.md")).unwrap();
        assert!(
            !agents_text.contains("- [ ]")
                && !agents_text.contains("- [x]")
                && !agents_text.contains("- [X]"),
            "{} puts progress state in ambient AGENTS.md; open work is a GitHub issue",
            dir.display()
        );
        assert!(
            agents_text.lines().count() <= 160,
            "{} is too large for ambient context; move detail into module docs",
            dir.join("AGENTS.md").display()
        );
    }
}

#[test]
fn every_ifc_crate_has_local_context() {
    let root = crates_dir(&ifc_root());
    let mut crates = 0;
    for entry in std::fs::read_dir(&root).expect("read crates/") {
        let path = entry.expect("directory entry").path();
        if !is_ifc_layer_dir(&path) {
            continue;
        }
        crates += 1;
        assert!(
            path.join("AGENTS.md").is_file(),
            "{} lacks AGENTS.md",
            path.display()
        );
    }
    assert!(crates >= 18, "expected all IFC crates, found {crates}");
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

/// A pointer to a context document. PLAN.md is retired but still
/// recognised, so a leftover pointer to one is reported as missing.
fn is_context_pointer(token: &str) -> bool {
    !token.contains("://")
        && !token.starts_with("mailto:")
        && !token.chars().any(char::is_whitespace)
        && Path::new(token)
            .file_name()
            .is_some_and(|name| name == OsStr::new("AGENTS.md") || name == OsStr::new("PLAN.md"))
}

#[test]
fn context_document_pointers_resolve_and_chain_to_their_parent() {
    let root = ifc_root();
    let canonical_root = root.canonicalize().expect("canonical IFC package root");
    let mut files = Vec::new();
    walk(&root, &mut files);
    let mut broken = Vec::new();

    // ADRs are immutable records of their time, so a pointer in one may name a
    // file that has since been retired (ADR 0016 retired PLAN.md).
    let adrs = root.join("docs/adr");
    for file in files.iter().filter(|path| {
        path.extension()
            .is_some_and(|ext| ext == OsStr::new("md") || ext == OsStr::new("rs"))
            && !path.starts_with(&adrs)
    }) {
        let text = std::fs::read_to_string(file).unwrap();
        let targets: Vec<_> = context_pointer_tokens(&text)
            .into_iter()
            .filter(|token| is_context_pointer(token))
            .map(|token| (file.parent().unwrap().join(&token), token))
            .collect();
        for (target, token) in &targets {
            if Path::new(token).is_absolute() {
                broken.push(format!("{} -> absolute {token}", file.display()));
                continue;
            }
            if !target.is_file() {
                broken.push(format!("{} -> missing {token}", file.display()));
                continue;
            }
            match target.canonicalize() {
                Ok(resolved) if resolved.starts_with(&canonical_root) => {}
                Ok(_) => broken.push(format!("{} -> outside package {token}", file.display())),
                Err(_) => broken.push(format!("{} -> unreadable {token}", file.display())),
            }
        }

        if file.file_name() != Some(OsStr::new("AGENTS.md")) || file == &root.join("AGENTS.md") {
            continue;
        }
        let expected_parent = file
            .parent()
            .unwrap()
            .ancestors()
            .skip(1)
            .map(|ancestor| ancestor.join("AGENTS.md"))
            .find(|candidate| candidate.is_file())
            .expect("non-root AGENTS.md must have parent context");
        let points_to_parent = targets
            .iter()
            .any(|(target, _)| target.canonicalize().ok() == expected_parent.canonicalize().ok());
        if !points_to_parent {
            broken.push(format!(
                "{} does not point to parent {}",
                file.display(),
                expected_parent.display()
            ));
        }
    }

    assert!(
        broken.is_empty(),
        "broken progressive-context pointers:\n{}",
        broken.join("\n")
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

#[test]
fn context_source_pointers_resolve_to_existing_scaffold_owners() {
    let root = ifc_root();
    let mut files = Vec::new();
    walk(&root, &mut files);
    let mut missing = Vec::new();
    let stale_phrases = [
        "Create and declare source",
        "Add and declare a Rust file",
        "Create a planned Rust file",
    ];

    for context in files
        .into_iter()
        .filter(|path| path.file_name().is_some_and(|name| name == "AGENTS.md"))
    {
        let text = std::fs::read_to_string(&context).unwrap();
        for phrase in stale_phrases {
            assert!(
                !text.contains(phrase),
                "{} tells agents to recreate compiled scaffold files",
                context.display()
            );
        }
        let Some(src_index) = context
            .components()
            .position(|component| component.as_os_str() == "src")
        else {
            continue;
        };
        let crate_dir: PathBuf = context.components().take(src_index).collect();
        for token in inline_code_tokens(&text)
            .into_iter()
            .filter(|token| token.ends_with(".rs"))
        {
            let target = if token.starts_with("src/") {
                crate_dir.join(&token)
            } else {
                context.parent().unwrap().join(&token)
            };
            if !target.is_file() {
                missing.push(format!("{} -> {token}", context.display()));
            }
        }
    }
    assert!(
        missing.is_empty(),
        "nested contexts point at missing Rust owners:\n{}",
        missing.join("\n")
    );
}
