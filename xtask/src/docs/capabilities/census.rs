//! Measure every crate in the workspace from its own sources.
//!
//! Size figures are measured; status is a judgement and is declared, never
//! inferred from line counts.

use super::{IMPLEMENTED, PARTIAL};
use crate::text::{splitlines, thousands};
use crate::workspace::Workspace;

/// A source file of this many lines or fewer is a doc-comment placeholder that
/// reserves a name. The threshold is published on the page it generates.
const STUB_MAX_LINES: usize = 12;

/// Directories with a `Cargo.toml` and `src/` that are tooling, not library
/// crates, and so carry no capability status.
const TOOLING: &[&str] = &["xtask"];

/// Declared status per crate. A crate listed here wins over the committed
/// page, which is how a deliberate status change is made.
const PUBLISHED_STATUS: &[(&str, &str)] = &[
    // Record-model bindings tested from Node (#37); npm publication, a tested
    // browser build and domain views are still open, so not "Implemented".
    ("openbim-ifc-wasm", PARTIAL),
    // Shared host-independent core of the three bindings; complete for the
    // record-model surface they expose.
    ("openbim-ifc-binding-core", IMPLEMENTED),
    // Record-model surface done and tested from C and Python; packaging
    // (CMake, PyPI, non-Linux wheels) is not.
    ("openbim-ifc-capi", PARTIAL),
    ("openbim-ifc-py", PARTIAL),
    ("ifc-author", IMPLEMENTED),
    ("ifc-spatial", IMPLEMENTED),
    ("ifc-classification", IMPLEMENTED),
    ("ifc-approval", IMPLEMENTED),
    ("ifc-control", IMPLEMENTED),
    ("ifc-tabular", IMPLEMENTED),
    ("ifc-constraint", IMPLEMENTED),
    ("ifc-systems", IMPLEMENTED),
    ("ifc-element-type", IMPLEMENTED),
    ("ifc-occurrence", IMPLEMENTED),
    ("ifc-properties", IMPLEMENTED),
    // Clean never means full EXPRESS conformance: aggregate bounds, arbitrary
    // EXPRESS evaluation and INVERSE derivation remain unsupported categories.
    ("ifc-validate", IMPLEMENTED),
    ("ifc-cost", IMPLEMENTED),
    ("ifc-schedule", IMPLEMENTED),
    // IFC2X3 lacks IfcConstructionResourceType and IfcResourceTime, so it is
    // a typed UnsupportedSchema refusal rather than a gap.
    ("ifc-resource", PARTIAL),
    ("ifc-style", IMPLEMENTED),
    ("ifc-structural", IMPLEMENTED),
    // Frame composition, north semantics and IFC4X3 profiles remain open.
    ("ifc-georef", PARTIAL),
    // Transitions, cant assembly, placement and the census remain open.
    ("ifc-alignment", PARTIAL),
];

struct Row {
    name: String,
    loc: usize,
    files: usize,
    stubs: usize,
    tests: usize,
    status: String,
}

pub(super) fn table(workspace: &Workspace, current: &str) -> Result<String, String> {
    let previous = page_statuses(current);
    let mut crates: Vec<std::path::PathBuf> = std::fs::read_dir(&workspace.root)
        .map_err(|error| format!("cannot list the workspace: {error}"))?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.join("Cargo.toml").is_file() && path.join("src").is_dir())
        .collect();
    crates.sort();

    let mut rows = Vec::new();
    for dir in crates {
        let name = dir
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        if TOOLING.contains(&name.as_str()) {
            continue;
        }
        let mut files = Vec::new();
        rust_files(&dir.join("src"), &mut files);
        files.sort();
        let (mut loc, mut stubs) = (0, 0);
        for path in &files {
            let bytes =
                std::fs::read(path).map_err(|error| format!("{}: {error}", path.display()))?;
            let lines = splitlines(&String::from_utf8_lossy(&bytes)).len();
            loc += lines;
            if lines <= STUB_MAX_LINES {
                stubs += 1;
            }
        }
        let tests = std::fs::read_dir(dir.join("tests")).map_or(0, |entries| {
            entries
                .flatten()
                .filter(|entry| entry.path().extension().is_some_and(|e| e == "rs"))
                .count()
        });
        let status = PUBLISHED_STATUS
            .iter()
            .find(|(crate_name, _)| *crate_name == name)
            .map(|(_, status)| (*status).to_owned())
            .or_else(|| {
                previous
                    .iter()
                    .find(|(n, _)| *n == name)
                    .map(|(_, s)| s.clone())
            })
            .ok_or_else(|| {
                format!("{name} has no published status; declare it in PUBLISHED_STATUS")
            })?;
        rows.push(Row {
            name,
            loc,
            files: files.len(),
            stubs,
            tests,
            status,
        });
    }

    // Stable, so equal sizes keep directory order.
    rows.sort_by(|a, b| b.loc.cmp(&a.loc));
    let mut out = vec![
        "| Crate | Source LOC | Files | Stub files | Test files | Status |".to_owned(),
        "| --- | ---: | ---: | ---: | ---: | --- |".to_owned(),
    ];
    for row in rows {
        out.push(format!(
            "| `{}` | {} | {} | {} | {} | {} |",
            row.name,
            thousands(row.loc),
            row.files,
            row.stubs,
            row.tests,
            row.status
        ));
    }
    Ok(out.join("\n"))
}

/// Sentence stating how many crates are scaffolds, counted from the table.
///
/// Prose next to a generated table is exactly where drift reappears: the
/// census was regenerated for months while the sentence beside it kept
/// claiming a stale number.
pub(super) fn scaffold_count(census: &str) -> String {
    let scaffolds = census.matches("status-scaffold").count();
    let total = census
        .lines()
        .filter(|line| crate_row(line).is_some())
        .count();
    format!("{scaffolds} of {total} crates are scaffolds.")
}

/// Statuses already on the committed page: the fallback for a crate that has
/// no declared status.
fn page_statuses(page: &str) -> Vec<(String, String)> {
    page.lines()
        .filter_map(|line| {
            let name = crate_row(line)?;
            let cells: Vec<&str> = line.split('|').collect();
            // "", name, loc, files, stubs, tests, status, ""
            if cells.len() != 8 || !cells[7].is_empty() {
                return None;
            }
            let status = cells[6].strip_prefix(' ')?.strip_suffix(' ')?;
            (!status.is_empty()).then(|| (name.to_owned(), status.to_owned()))
        })
        .collect()
}

/// The crate name of a census row: `` | `name` |``.
fn crate_row(line: &str) -> Option<&str> {
    let rest = line.strip_prefix("| `")?;
    let end = rest.find("` |")?;
    let name = &rest[..end];
    (!name.is_empty()
        && name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-'))
    .then_some(name)
}

fn rust_files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        if kind.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_rows_round_trip() {
        let page = "| `ifc-model` | 1,234 | 5 | 0 | 2 | <span>Implemented</span> |\n| x |";
        assert_eq!(
            page_statuses(page),
            [(
                "ifc-model".to_owned(),
                "<span>Implemented</span>".to_owned()
            )]
        );
        assert_eq!(scaffold_count(page), "0 of 1 crates are scaffolds.");
    }
}
