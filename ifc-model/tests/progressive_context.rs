//! Guard the progressive context and implementation-plan protocol.
//!
//! `../AGENTS.md` is standing context; `../PLAN.md` is opt-in implementation state.
//! Pairing and shape are checked so a new crate/module cannot silently fall
//! outside the handoff system.

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsStr;
use std::path::{Component, Path, PathBuf};

#[path = "support/progressive_markdown.rs"]
mod progressive_markdown;
use progressive_markdown::{
    context_pointer_tokens, inline_code_tokens, task_checkbox_line_count, task_entries, task_ids,
    task_prerequisites, task_references,
};

// This registry deliberately duplicates the initial capability set. A coordinated
// source/module/PLAN deletion must still change a separate reviewable baseline.
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
    cargo_metadata::MetadataCommand::new()
        .no_deps()
        .exec()
        .expect("cargo metadata must describe the runtime workspace")
        .workspace_root
        .into_std_path_buf()
        .join("packages/ifc")
}

/// Is this a crate directory belonging to the IFC layer?
///
/// The IFC crates share `packages/ifc/` with the group's own AGENTS.md and
/// PLAN.md, so a directory scan alone would sweep those in. Select by NAME.
fn is_ifc_layer_dir(path: &Path) -> bool {
    let Some(name) = path.file_name().map(|n| n.to_string_lossy().to_string()) else {
        return false;
    };
    (name.starts_with("ifc-") || name == "openbim-ifc") && path.join("Cargo.toml").is_file()
}

fn walk(dir: &Path, files: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display())) {
        let path = entry.expect("directory entry").path();
        if path.is_dir() {
            walk(&path, files);
        } else {
            files.push(path);
        }
    }
}

#[test]
fn every_context_boundary_pairs_standing_rules_with_an_opt_in_plan() {
    let root = ifc_root();
    let mut files = Vec::new();
    walk(&root, &mut files);

    let agents: BTreeSet<_> = files
        .iter()
        .filter(|path| path.file_name().is_some_and(|name| name == "AGENTS.md"))
        .map(|path| path.parent().unwrap().to_path_buf())
        .collect();
    let plans: BTreeSet<_> = files
        .iter()
        .filter(|path| path.file_name().is_some_and(|name| name == "PLAN.md"))
        .map(|path| path.parent().unwrap().to_path_buf())
        .collect();

    assert_eq!(
        agents, plans,
        "every AGENTS.md must have an adjacent PLAN.md and vice versa"
    );
    let crate_count = std::fs::read_dir(&root)
        .unwrap()
        .filter_map(Result::ok)
        .filter(|entry| entry.path().join("Cargo.toml").is_file())
        .count();
    assert!(
        agents.len() >= crate_count + 1 + REQUIRED_NESTED_CONTEXTS.len(),
        "expected package root + every crate + required nested boundaries; found {} for {crate_count} crates",
        agents.len()
    );
    for relative in REQUIRED_NESTED_CONTEXTS {
        assert!(
            agents.contains(&root.join(relative)),
            "required progressive boundary is missing: {relative}"
        );
    }

    for dir in agents {
        let agents_text = std::fs::read_to_string(dir.join("AGENTS.md")).unwrap();
        let plan_text = std::fs::read_to_string(dir.join("PLAN.md")).unwrap();
        assert!(
            agents_text.contains("PLAN.md") && agents_text.to_ascii_lowercase().contains("only"),
            "{} must say PLAN.md is opt-in context",
            dir.display()
        );
        assert!(
            !agents_text.contains("- [ ]")
                && !agents_text.contains("- [x]")
                && !agents_text.contains("- [X]"),
            "{} puts progress state in ambient AGENTS.md",
            dir.display()
        );
        assert!(
            plan_text.contains("## Work queue"),
            "{} has no checkable work queue",
            dir.display()
        );
        for stale in [
            "Future paths are listed here rather than created",
            "Create and declare a path with its first real",
        ] {
            assert!(
                !plan_text.contains(stale),
                "{} contains stale scaffold instruction: {stale}",
                dir.join("PLAN.md").display()
            );
        }
        let ids = task_ids(&plan_text);
        let checkbox_lines = task_checkbox_line_count(&plan_text);
        assert_eq!(
            ids.len(),
            checkbox_lines,
            "{} has a malformed task declaration",
            dir.join("PLAN.md").display()
        );
        assert!(!ids.is_empty(), "{} has no task IDs", dir.display());
        let unique: BTreeSet<_> = ids.iter().collect();
        assert_eq!(
            unique.len(),
            ids.len(),
            "{} repeats a task ID",
            dir.display()
        );
        assert!(
            agents_text.lines().count() <= 160,
            "{} is too large for ambient context; move progress/detail into PLAN.md",
            dir.join("AGENTS.md").display()
        );
    }
}

#[test]
fn every_ifc_crate_has_local_context_and_completion_log() {
    let root = ifc_root();
    let mut crates = 0;
    for entry in std::fs::read_dir(&root).expect("read packages/") {
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
        let plan = std::fs::read_to_string(path.join("PLAN.md"))
            .unwrap_or_else(|e| panic!("{} lacks PLAN.md: {e}", path.display()));
        for heading in [
            "Status:",
            "## Planned file map",
            "## Work queue",
            "## Completion log",
        ] {
            assert!(
                plan.contains(heading),
                "{}/PLAN.md lacks {heading}",
                path.display()
            );
        }
    }
    assert!(crates >= 18, "expected all IFC crates, found {crates}");
}

fn normalized_relative(path: &Path) -> bool {
    !path.as_os_str().is_empty()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

#[test]
fn compiled_scaffold_maps_match_real_owned_source_files() {
    let root = ifc_root();
    let mut planned_paths = BTreeSet::new();
    for entry in std::fs::read_dir(&root).expect("read packages/") {
        let crate_dir = entry.expect("directory entry").path();
        if !is_ifc_layer_dir(&crate_dir) {
            continue;
        }
        let plan = std::fs::read_to_string(crate_dir.join("PLAN.md")).unwrap();
        if !plan.contains("compiled private scaffold modules") {
            continue;
        }
        for token in plan.split('`').skip(1).step_by(2) {
            if token.starts_with("src/") && token.ends_with(".rs") {
                let relative = Path::new(token);
                assert!(
                    normalized_relative(relative),
                    "{}/PLAN.md contains non-normal scaffold path {token}",
                    crate_dir.display()
                );
                let package_relative = PathBuf::from(crate_dir.file_name().unwrap()).join(relative);
                assert!(
                    planned_paths.insert(package_relative),
                    "{}/PLAN.md repeats compiled scaffold path {token}",
                    crate_dir.display()
                );
                assert!(
                    crate_dir.join(relative).is_file(),
                    "{}/PLAN.md claims compiled path {token}, but it does not exist",
                    crate_dir.display()
                );
            }
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
            let relative = source.strip_prefix(&crate_dir).unwrap().to_string_lossy();
            assert!(
                plan.contains(&format!("`{relative}`")),
                "{} is an ownership scaffold missing from {}/PLAN.md",
                source.display(),
                crate_dir.display()
            );
        }
    }
    assert!(
        planned_paths.len() >= 150,
        "expected the compiled capability scaffold, found {} planned paths",
        planned_paths.len()
    );
}

#[test]
fn required_scaffold_capability_seams_are_preserved() {
    let root = ifc_root();
    let required: Vec<_> = REQUIRED_SCAFFOLD_PATHS
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect();
    assert!(
        required.len() >= 187,
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

        let crate_dir = root.join(parts[0]);
        assert!(
            crate_dir.join("Cargo.toml").is_file(),
            "required scaffold path has no IFC crate owner: {token}"
        );
        let crate_relative = relative.strip_prefix(Path::new(parts[0])).unwrap();
        let plan = std::fs::read_to_string(crate_dir.join("PLAN.md")).unwrap();
        assert!(
            plan.contains(&format!("`{}`", crate_relative.display())),
            "required scaffold capability seam is missing from its crate PLAN.md: {token}"
        );
    }
}

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

    for file in files.iter().filter(|path| {
        path.extension()
            .is_some_and(|ext| ext == OsStr::new("md") || ext == OsStr::new("rs"))
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
fn source_docs_point_to_local_plans_not_the_global_roadmap() {
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
        "source docs bypass progressive PLAN.md files: {offenders:#?}"
    );
}

fn plan_paths(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    walk(root, &mut files);
    files
        .into_iter()
        .filter(|path| path.file_name().is_some_and(|name| name == "PLAN.md"))
        .collect()
}

fn prerequisite_cycles(graph: &BTreeMap<String, BTreeSet<String>>) -> BTreeSet<String> {
    fn visit(
        node: &str,
        graph: &BTreeMap<String, BTreeSet<String>>,
        state: &mut BTreeMap<String, u8>,
        stack: &mut Vec<String>,
        cycles: &mut BTreeSet<String>,
    ) {
        match state.get(node).copied() {
            Some(1) => {
                let start = stack.iter().position(|item| item == node).unwrap();
                let mut cycle = stack[start..].to_vec();
                cycle.push(node.to_owned());
                cycles.insert(cycle.join(" -> "));
                return;
            }
            Some(2) => return,
            _ => {}
        }
        state.insert(node.to_owned(), 1);
        stack.push(node.to_owned());
        if let Some(requirements) = graph.get(node) {
            for requirement in requirements {
                visit(requirement, graph, state, stack, cycles);
            }
        }
        stack.pop();
        state.insert(node.to_owned(), 2);
    }

    let mut state = BTreeMap::new();
    let mut cycles = BTreeSet::new();
    for task in graph.keys() {
        visit(task, graph, &mut state, &mut Vec::new(), &mut cycles);
    }
    cycles
}

#[test]
fn prerequisite_graph_detects_cycles() {
    let graph = BTreeMap::from([
        ("TASK-A".to_owned(), BTreeSet::from(["TASK-B".to_owned()])),
        ("TASK-B".to_owned(), BTreeSet::from(["TASK-A".to_owned()])),
    ]);
    assert_eq!(
        prerequisite_cycles(&graph),
        BTreeSet::from(["TASK-A -> TASK-B -> TASK-A".to_owned()])
    );
}

#[test]
fn every_plan_reference_resolves_to_one_task_owner() {
    let root = ifc_root();
    let plans = plan_paths(&root);
    let mut owners = BTreeMap::<String, (PathBuf, bool)>::new();
    let mut duplicates = Vec::new();

    for path in &plans {
        let text = std::fs::read_to_string(path).unwrap();
        for task in task_entries(&text) {
            if let Some((previous, _)) =
                owners.insert(task.id.clone(), (path.clone(), task.complete))
            {
                duplicates.push(format!(
                    "{} is declared by both {} and {}",
                    task.id,
                    previous.display(),
                    path.display()
                ));
            }
        }
    }
    assert!(
        duplicates.is_empty(),
        "task IDs need one state owner:\n{}",
        duplicates.join("\n")
    );

    let known: BTreeSet<_> = owners.keys().cloned().collect();
    let mut graph = BTreeMap::<String, BTreeSet<String>>::new();
    let mut unresolved = Vec::new();
    let mut premature = Vec::new();
    for path in &plans {
        let text = std::fs::read_to_string(path).unwrap();
        for reference in task_references(&text).difference(&known) {
            unresolved.push(format!("{}: {reference}", path.display()));
        }
        for (task, requirements) in task_prerequisites(&text) {
            graph
                .entry(task.clone())
                .or_default()
                .extend(requirements.iter().cloned());
            let complete = owners.get(&task).expect("task owner").1;
            for requirement in requirements {
                let prerequisite_complete = owners
                    .get(&requirement)
                    .unwrap_or_else(|| panic!("unresolved prerequisite {requirement}"))
                    .1;
                if complete && !prerequisite_complete {
                    premature.push(format!(
                        "{task} is complete while prerequisite {requirement} is pending"
                    ));
                }
            }
        }
    }
    assert!(
        unresolved.is_empty(),
        "PLAN references without task owners:\n{}",
        unresolved.join("\n")
    );
    assert!(
        premature.is_empty(),
        "completed tasks with pending prerequisites:\n{}",
        premature.join("\n")
    );
    let cycles = prerequisite_cycles(&graph);
    assert!(
        cycles.is_empty(),
        "cyclic PLAN prerequisites:\n{}",
        cycles.into_iter().collect::<Vec<_>>().join("\n")
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
