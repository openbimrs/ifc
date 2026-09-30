//! The status of every crate in the workspace, with the issues behind it.
//!
//! Status is a judgement, declared per crate in `[package.metadata.openbim]
//! status` and never inferred from size; `gaps` names the open issues that
//! keep a crate from a higher status, and a `partial` crate must name one.
//!
//! The table is committed and checked, so everything in it must be stable
//! under ordinary work. It used to publish line, file and test-file counts,
//! sorted by size: those changed with almost every pull request, so any two
//! open pull requests conflicted on this table. A later count of short
//! "stub" files mixed placeholders with small real modules and said nothing
//! about what a crate lacks. What remains changes exactly when the page
//! should: a crate's declared status or its gap list moves. Rows are sorted by
//! name so they never reorder.

use crate::workspace::Workspace;

const ISSUES: &str = "https://github.com/openbimrs/ifc/issues";

pub(super) fn table(workspace: &Workspace) -> Result<String, String> {
    let mut crates = workspace.crates()?;
    crates.sort_by(|a, b| a.name.cmp(&b.name));
    let mut out = vec![
        "| Crate | Status | Open gaps |".to_owned(),
        "| --- | --- | --- |".to_owned(),
    ];
    for krate in crates {
        let gaps: Vec<String> = krate
            .gaps
            .iter()
            .map(|issue| format!("[#{issue}]({ISSUES}/{issue})"))
            .collect();
        out.push(format!(
            "| `{}` | {} | {} |",
            krate.name,
            badge(&krate.status),
            gaps.join(", ")
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
        let page = "| `ifc-a` | <span class=\"status-scaffold\">Scaffold</span> | |\n\
                    | `ifc-b` | x | |\n| x |";
        assert_eq!(scaffold_count(page), "1 of 2 crates are scaffolds.");
    }
}
