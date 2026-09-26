//! Measure every crate in the workspace from its own sources.
//!
//! Size figures are measured; status is a judgement, declared per crate in
//! `[package.metadata.openbim] status` and never inferred from line counts.

use crate::text::{splitlines, thousands};
use crate::workspace::Workspace;

/// A source file of this many lines or fewer is a doc-comment placeholder that
/// reserves a name. The threshold is published on the page it generates.
const STUB_MAX_LINES: usize = 12;

struct Row {
    name: String,
    loc: usize,
    files: usize,
    stubs: usize,
    tests: usize,
    status: String,
}

pub(super) fn table(workspace: &Workspace) -> Result<String, String> {
    let mut rows = Vec::new();
    for krate in workspace.crates()? {
        let dir = workspace.root.join(&krate.dir);
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
        rows.push(Row {
            status: badge(&krate.status),
            name: krate.name,
            loc,
            files: files.len(),
            stubs,
            tests,
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

/// The status badge the docs site styles (`docs/.vitepress/theme/custom.css`).
pub(crate) fn badge(status: &str) -> String {
    let label = match status {
        "implemented" => "Implemented",
        "partial" => "Partial",
        _ => "Scaffold",
    };
    format!(r#"<span class="status-{status}">{label}</span>"#)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scaffolds_are_counted_from_rows() {
        let page =
            "| `ifc-a` | 1 | 1 | 0 | 0 | <span class=\"status-scaffold\">Scaffold</span> |\n\
                    | `ifc-b` | 1 | 1 | 0 | 0 | x |\n| x |";
        assert_eq!(scaffold_count(page), "1 of 2 crates are scaffolds.");
    }
}
