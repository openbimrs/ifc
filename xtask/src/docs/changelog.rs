//! Assemble the documentation changelog from per-crate changelogs.
//!
//! Each crate owns its `CHANGELOG.md` and versions independently, so there is
//! no single file to mirror. Every per-crate changelog is read, its entries are
//! grouped by release version, and one aggregated page is written between
//! sentinel comments. Ordering is by version descending, so the newest release
//! across the whole family leads the page whichever crate produced it.

use std::cmp::Ordering;
use std::collections::BTreeMap;

use super::Output;
use crate::text::{splice, splitlines};
use crate::workspace::Workspace;

const TARGET: &str = "docs/project/changelog.md";
const BLOB: &str = "https://github.com/openbimrs/ifc/blob/main";
const BEGIN: &str = "<!-- CHANGELOG:BEGIN -->";
const END: &str = "<!-- CHANGELOG:END -->";

/// One release section of one crate's changelog.
struct Section {
    version: String,
    date: String,
    body: String,
}

pub(super) fn generate(workspace: &Workspace) -> Result<Output, String> {
    let members = workspace.publishable();
    let missing: Vec<&String> = members
        .iter()
        .filter(|name| !workspace.root.join(name).join("CHANGELOG.md").exists())
        .collect();
    if !missing.is_empty() {
        let names: Vec<&str> = missing.iter().map(|name| name.as_str()).collect();
        return Err(format!(
            "crates without a CHANGELOG.md: {}",
            names.join(", ")
        ));
    }
    let body = assemble(workspace, &members)?;
    Output::derive(workspace, TARGET, |current| {
        splice(current, BEGIN, END, &body)
    })
}

/// Group every crate section by version, newest release first.
fn assemble(workspace: &Workspace, members: &[String]) -> Result<String, String> {
    // Insertion-ordered per version so equal versions keep crate order.
    let mut order: Vec<String> = Vec::new();
    let mut by_version: BTreeMap<String, Vec<(String, String, String)>> = BTreeMap::new();
    for name in members {
        let path = workspace.root.join(name).join("CHANGELOG.md");
        let text = std::fs::read_to_string(&path)
            .map_err(|error| format!("cannot read {name}/CHANGELOG.md: {error}"))?;
        for section in parse(&text) {
            // An empty Unreleased section is the normal resting state; listing
            // the crate with nothing under it is just noise.
            if section.body.is_empty() {
                continue;
            }
            if !by_version.contains_key(&section.version) {
                order.push(section.version.clone());
            }
            by_version.entry(section.version).or_default().push((
                name.clone(),
                section.date,
                absolutise(&section.body),
            ));
        }
    }

    order.sort_by(|a, b| compare_versions(b, a));
    let mut blocks: Vec<String> = Vec::new();
    for version in order {
        let mut entries = by_version.remove(&version).unwrap_or_default();
        entries.sort();
        let date = entries
            .iter()
            .map(|(_, date, _)| date.as_str())
            .filter(|date| !date.is_empty())
            .max()
            .map(|date| format!(" - {date}"))
            .unwrap_or_default();
        blocks.push(format!("## [{version}]{date}"));
        blocks.push(String::new());
        for (name, _, body) in entries {
            blocks.push(format!("### {name}"));
            blocks.push(String::new());
            blocks.push(body);
            blocks.push(String::new());
        }
    }
    Ok(blocks.join("\n").trim().to_owned())
}

/// Split a changelog into release sections.
///
/// Link-reference lines at the foot (`[0.2.0]: https://...`) belong to the
/// source file, not to any release body: the assembled page has its own.
fn parse(text: &str) -> Vec<Section> {
    let mut sections = Vec::new();
    let mut current: Option<(String, String)> = None;
    let mut body: Vec<&str> = Vec::new();
    for line in splitlines(text) {
        if let Some((version, date)) = heading(line) {
            if let Some((version, date)) = current.take() {
                sections.push(Section {
                    version,
                    date,
                    body: body.join("\n").trim().to_owned(),
                });
            }
            current = Some((version, date));
            body.clear();
            continue;
        }
        if current.is_some() && !is_link_reference(line) {
            body.push(line);
        }
    }
    if let Some((version, date)) = current {
        sections.push(Section {
            version,
            date,
            body: body.join("\n").trim().to_owned(),
        });
    }
    sections
}

/// `## [0.2.1] - 2026-09-23` or `## [Unreleased]` → (version, date).
pub(crate) fn heading(line: &str) -> Option<(String, String)> {
    let rest = line.strip_prefix("## [")?;
    let close = rest.find(']')?;
    let version = &rest[..close];
    if version.is_empty() {
        return None;
    }
    let tail = rest[close + 1..].trim_end();
    if tail.is_empty() {
        return Some((version.to_owned(), String::new()));
    }
    let date = tail.trim_start().strip_prefix('-')?.trim_start();
    if date.is_empty() || date.contains(char::is_whitespace) {
        return None;
    }
    Some((version.to_owned(), date.to_owned()))
}

/// `[label]: target` at the start of a line.
fn is_link_reference(line: &str) -> bool {
    let Some(rest) = line.strip_prefix('[') else {
        return false;
    };
    let Some(close) = rest.find(']') else {
        return false;
    };
    if close == 0 {
        return false;
    }
    let Some(after) = rest[close + 1..].strip_prefix(':') else {
        return false;
    };
    let target = after.trim_start();
    after.len() > target.len() && !target.is_empty()
}

/// Rewrite repo-relative `[text](target)` links to absolute GitHub URLs.
///
/// A per-crate changelog is read in the repository, where `../CHANGELOG.md`
/// resolves, and on the docs site, where it does not and VitePress fails the
/// build on the dead link.
pub(crate) fn absolutise(body: &str) -> String {
    let mut out = String::with_capacity(body.len());
    let mut rest = body;
    while let Some(open) = rest.find('[') {
        match link_at(&rest[open..]) {
            Some((text, target, length)) => {
                out.push_str(&rest[..open]);
                if target.contains("://") || target.starts_with('#') {
                    out.push_str(&rest[open..open + length]);
                } else {
                    let clean = target.trim_start_matches(['.', '/']);
                    out.push_str(&format!("[{text}]({BLOB}/{clean})"));
                }
                rest = &rest[open + length..];
            }
            None => {
                out.push_str(&rest[..=open]);
                rest = &rest[open + 1..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// Match `[text](target)` at the start of `s`: non-empty text without `]`,
/// non-empty target without `)`. Returns (text, target, matched length).
fn link_at(s: &str) -> Option<(&str, &str, usize)> {
    let inner = s.strip_prefix('[')?;
    let close = inner.find(']')?;
    if close == 0 {
        return None;
    }
    let after = inner[close + 1..].strip_prefix('(')?;
    let end = after.find(')')?;
    if end == 0 {
        return None;
    }
    let length = 1 + close + 2 + end + 1;
    Some((&inner[..close], &after[..end], length))
}

/// Unreleased first, then versions numerically, pieces split on `.`, `-`, `+`.
pub(crate) fn compare_versions(a: &str, b: &str) -> Ordering {
    let key = |version: &str| -> (u8, Vec<u64>) {
        if version.eq_ignore_ascii_case("unreleased") {
            return (1, Vec::new());
        }
        let parts = version
            .split(['.', '-', '+'])
            .map(|piece| {
                if !piece.is_empty() && piece.bytes().all(|b| b.is_ascii_digit()) {
                    piece.parse().unwrap_or(0)
                } else {
                    0
                }
            })
            .collect();
        (0, parts)
    };
    key(a).cmp(&key(b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn headings_parse_with_and_without_dates() {
        assert_eq!(
            heading("## [0.2.1] - 2026-09-23"),
            Some(("0.2.1".into(), "2026-09-23".into()))
        );
        assert_eq!(
            heading("## [Unreleased]"),
            Some(("Unreleased".into(), String::new()))
        );
        assert_eq!(heading("## [0.1.0] - a b"), None);
        assert_eq!(heading("### [0.1.0]"), None);
    }

    #[test]
    fn relative_links_become_absolute_and_others_stay() {
        assert_eq!(
            absolutise("see [root](../CHANGELOG.md), [x](https://a.b) and [y](#z)"),
            format!("see [root]({BLOB}/CHANGELOG.md), [x](https://a.b) and [y](#z)")
        );
        assert_eq!(absolutise("[a] (b) [] (c)"), "[a] (b) [] (c)");
    }

    #[test]
    fn unreleased_sorts_after_every_version() {
        assert_eq!(compare_versions("Unreleased", "9.9.9"), Ordering::Greater);
        assert_eq!(compare_versions("0.10.0", "0.9.0"), Ordering::Greater);
        assert_eq!(compare_versions("0.2.0", "0.2.0"), Ordering::Equal);
    }

    #[test]
    fn link_references_are_recognised() {
        assert!(is_link_reference("[0.2.0]: https://x"));
        assert!(!is_link_reference("[0.2.0]:https://x"));
        assert!(!is_link_reference("- [0.2.0]: x"));
    }
}
