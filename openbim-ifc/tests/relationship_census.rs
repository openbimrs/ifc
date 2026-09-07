//! The `IfcRel*` census in `docs/capabilities.md` must match the source.
//!
//! The "Objectified relationship traversal" row is hand-written prose
//! outside the generated sentinel blocks, so nothing gated it. It named
//! three families and claimed the rest were uninterpreted while sixteen
//! crates read twenty-six. This test makes the number falsifiable.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// Concrete `IfcRel*` families named by any crate source file.
fn families_read(workspace: &Path) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    let mut stack: Vec<PathBuf> = vec![workspace.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if path.is_dir() {
                // Crate roots, and everything beneath a src/ once inside
                // one: sources live in nested modules (src/resource/,
                // src/connectivity/). Restricting to dirs literally named
                // src undercounted by six families.
                let inside_src = dir.components().any(|c| c.as_os_str() == "src");
                if inside_src
                    || name == "src"
                    || name.starts_with("ifc-")
                    || name.starts_with("openbim-")
                {
                    stack.push(path);
                }
            } else if name.ends_with(".rs") && dir.components().any(|c| c.as_os_str() == "src") {
                let Ok(body) = std::fs::read_to_string(&path) else {
                    continue;
                };
                collect(&body, &mut found);
            }
        }
    }
    found
}

/// The six abstract supertypes are not families a file can instantiate.
const ABSTRACT: [&str; 6] = [
    "ifcrelationship",
    "ifcrelassigns",
    "ifcrelassociates",
    "ifcrelconnects",
    "ifcreldecomposes",
    "ifcreldefines",
];

fn collect(body: &str, found: &mut BTreeSet<String>) {
    // Case-insensitive: crates name these both as EXPRESS-cased type names
    // in docs and as UPPERCASE STEP type names in code. Matching only the
    // former undercounted by five families.
    //
    // Comment lines are skipped. A module doc that *describes* a
    // relationship -- often to contrast its slot layout with one the crate
    // does read -- is not a reader, and counting it overstated the census
    // by two families (IfcRelServicesBuildings, IfcRelSpaceBoundary).
    let lower: String = body
        .lines()
        .filter(|line| {
            let t = line.trim_start();
            !(t.starts_with("//") || t.starts_with("*") || t.starts_with("#!["))
        })
        .collect::<Vec<_>>()
        .join("\n")
        .to_ascii_lowercase();
    let mut rest = lower.as_str();
    while let Some(at) = rest.find("ifcrel") {
        let tail = &rest[at..];
        let end = tail
            .char_indices()
            .find(|(_, c)| !c.is_ascii_alphanumeric())
            .map_or(tail.len(), |(i, _)| i);
        let name = &tail[..end];
        if name.len() > "ifcrel".len() && !ABSTRACT.contains(&name) {
            found.insert(name.to_string());
        }
        rest = &tail[end.max(1)..];
    }
}

#[test]
fn capabilities_states_the_real_relationship_count() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf();
    let doc = std::fs::read_to_string(workspace.join("docs/capabilities.md"))
        .expect("capabilities.md is readable");
    let row = doc
        .lines()
        .find(|l| l.starts_with("| Objectified relationship traversal"))
        .expect("the relationship row exists");

    // The row opens "| Objectified relationship traversal | <span...> | N of
    // the schema's 40 concrete ...", so the first bare integer after the
    // status cell is the claim.
    // The row states "N of the schema.s 40 concrete IfcRel* families",
    // so the first bare integer in the rationale cell is the claim.
    let claimed: usize = row
        .split_whitespace()
        .find_map(|t| t.parse::<usize>().ok())
        .expect("row states a count");
    let actual = families_read(&workspace).len();
    assert_eq!(
        claimed, actual,
        "capabilities.md claims {claimed} IfcRel* families but the source reads {actual}"
    );
}
