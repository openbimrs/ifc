//! Code on docs pages comes from tests, never from the page.
//!
//! A test marks the lines a page shows:
//!
//! ```text
//! // docs:snippet index-read        (`#` instead of `//` in Python)
//! use ifc::{Codec, StepCodec};
//! let model = StepCodec.read_bytes(source)?;
//! // docs:end
//! ```
//!
//! and a page holds `<!-- SNIPPET:index-read -->` … `<!-- /SNIPPET -->`,
//! which this module fills with those lines, dedented, in a fenced block.
//! A page can therefore only show code a test compiles and runs.
//!
//! The lint half: a `rust`/`python`/`js`/`ts`/`c` fence outside a snippet
//! region fails, so hand-written code cannot creep back. ADRs are exempt
//! because they are immutable records of their time.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use super::Output;
use crate::text::splitlines;
use crate::workspace::Workspace;

/// Directories whose files may define snippets.
const SOURCES: &[&str] = &[
    "openbim-ifc/tests",
    "ifc-geometry/tests",
    "openbim-ifc-wasm/tests/js",
    "openbim-ifc-py/tests/python",
    "openbim-ifc-capi/tests/c",
];

const LINTED: &[&str] = &[
    "rust",
    "python",
    "py",
    "js",
    "javascript",
    "mjs",
    "ts",
    "typescript",
    "c",
    "cpp",
];

struct Snippet {
    lang: &'static str,
    code: String,
    origin: String,
}

/// Fill every page's snippet regions, composing with the generators that
/// already ran (`capabilities.md` has both generated tables and snippets), and
/// lint every page for hand-written code and hand-typed facts (`drift.rs`).
pub(super) fn apply(workspace: &Workspace, outputs: &mut Vec<Output>) -> Result<(), String> {
    let snippets = collect(workspace)?;
    let mut used: BTreeMap<&str, usize> = snippets.keys().map(|k| (k.as_str(), 0)).collect();
    let mut problems = Vec::new();

    for page in pages(&workspace.root.join("docs")) {
        let rel = page
            .strip_prefix(&workspace.root)
            .unwrap_or(&page)
            .display()
            .to_string();
        let existing = outputs.iter().position(|output| output.path == page);
        let text = match existing {
            Some(index) => outputs[index].updated.clone(),
            None => std::fs::read_to_string(&page).map_err(|error| format!("{rel}: {error}"))?,
        };
        let (filled, names) = fill(&text, &snippets).map_err(|error| format!("{rel}: {error}"))?;
        for name in &names {
            if let Some(count) = used.get_mut(name.as_str()) {
                *count += 1;
            }
        }
        if !rel.starts_with("docs/adr/") {
            problems.extend(unmarked_fences(&filled).into_iter().map(|line| {
                format!(
                    "{rel}:{line}: code fence not sourced from a test; mark the code in a \
                     test with `// docs:snippet <name>` and use `<!-- SNIPPET:<name> -->`"
                )
            }));
            problems.extend(super::drift::problems(&rel, &filled));
        }
        match existing {
            Some(index) => outputs[index].updated = filled,
            None if !names.is_empty() => {
                outputs.push(Output::derive(workspace, &rel, |_| Ok(filled))?)
            }
            None => {}
        }
    }
    for (name, count) in used {
        if count == 0 {
            problems.push(format!(
                "snippet `{name}` ({}) is shown on no page; delete its markers or use it",
                snippets[name].origin
            ));
        }
    }
    if problems.is_empty() {
        Ok(())
    } else {
        Err(problems.join("\n"))
    }
}

/// Every snippet defined under `SOURCES`, by name.
fn collect(workspace: &Workspace) -> Result<BTreeMap<String, Snippet>, String> {
    let mut files = Vec::new();
    for dir in SOURCES {
        walk(&workspace.root.join(dir), &mut files);
    }
    files.sort();
    let mut out: BTreeMap<String, Snippet> = BTreeMap::new();
    for file in files {
        let Some(lang) = language(&file) else {
            continue;
        };
        let rel = file
            .strip_prefix(&workspace.root)
            .unwrap_or(&file)
            .display()
            .to_string();
        let text = std::fs::read_to_string(&file).map_err(|error| format!("{rel}: {error}"))?;
        let comment = if lang == "python" { "#" } else { "//" };
        let start = format!("{comment} docs:snippet ");
        let end = format!("{comment} docs:end");
        let mut open: Option<(String, usize, Vec<&str>)> = None;
        for (index, line) in splitlines(&text).into_iter().enumerate() {
            let trimmed = line.trim();
            if let Some(name) = trimmed.strip_prefix(&start) {
                if let Some((previous, at, _)) = &open {
                    return Err(format!(
                        "{rel}:{}: snippet `{name}` opens inside `{previous}` (line {at})",
                        index + 1
                    ));
                }
                open = Some((name.trim().to_owned(), index + 1, Vec::new()));
            } else if trimmed == end {
                let (name, at, lines) = open
                    .take()
                    .ok_or_else(|| format!("{rel}:{}: `docs:end` without a snippet", index + 1))?;
                let origin = format!("{rel}:{at}");
                if let Some(existing) = out.get(&name) {
                    return Err(format!(
                        "snippet `{name}` is defined twice: {} and {origin}",
                        existing.origin
                    ));
                }
                out.insert(
                    name,
                    Snippet {
                        lang,
                        code: dedent(&lines),
                        origin,
                    },
                );
            } else if let Some((_, _, lines)) = &mut open {
                lines.push(line);
            }
        }
        if let Some((name, at, _)) = open {
            return Err(format!("{rel}:{at}: snippet `{name}` has no `docs:end`"));
        }
    }
    Ok(out)
}

/// Replace every `<!-- SNIPPET:name -->` region; return the page and the
/// names it used. Markers sit on their own line; one inside a code fence is an
/// example of the syntax, not a region.
fn fill(text: &str, snippets: &BTreeMap<String, Snippet>) -> Result<(String, Vec<String>), String> {
    const OPEN: &str = "<!-- SNIPPET:";
    const CLOSE: &str = "<!-- /SNIPPET -->";
    let mut out = String::with_capacity(text.len());
    let mut names = Vec::new();
    let mut in_fence = false;
    let mut lines = text.split_inclusive('\n');
    while let Some(line) = lines.next() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") {
            in_fence = !in_fence;
        }
        let Some(head) = trimmed.strip_prefix(OPEN).filter(|_| !in_fence) else {
            out.push_str(line);
            continue;
        };
        let name = head
            .strip_suffix("-->")
            .ok_or("a `<!-- SNIPPET:` marker must be a whole line")?
            .trim()
            .to_owned();
        if name.is_empty()
            || !name
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        {
            return Err(format!(
                "snippet names are lower-case words joined by `-`, found `{name}`"
            ));
        }
        let close = loop {
            match lines.next() {
                Some(body) if body.trim() == CLOSE => break body,
                Some(_) => {}
                None => return Err(format!("snippet region `{name}` has no `{CLOSE}`")),
            }
        };
        let snippet = snippets
            .get(&name)
            .ok_or_else(|| format!("no test defines snippet `{name}`"))?;
        let indent = &line[..line.len() - line.trim_start().len()];
        out.push_str(&format!(
            "{indent}{OPEN}{name} -->\n\n```{}\n{}\n```\n\n{indent}{CLOSE}{}",
            snippet.lang,
            snippet.code,
            if close.ends_with('\n') { "\n" } else { "" }
        ));
        names.push(name);
    }
    Ok((out, names))
}

/// 1-based lines of linted fences outside snippet and generated regions.
///
/// A generated region (`<!-- X:BEGIN -->` … `<!-- X:END -->`) is written by a
/// generator from source, such as the C header's signatures, so its code is
/// not hand-written either.
fn unmarked_fences(text: &str) -> Vec<usize> {
    let mut out = Vec::new();
    let mut in_region = false;
    let mut in_fence = false;
    for (index, line) in splitlines(text).into_iter().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("<!-- SNIPPET:")
            || (trimmed.starts_with("<!-- ") && trimmed.ends_with(":BEGIN -->"))
        {
            in_region = true;
        } else if trimmed.starts_with("<!-- /SNIPPET -->")
            || (trimmed.starts_with("<!-- ") && trimmed.ends_with(":END -->"))
        {
            in_region = false;
        } else if let Some(info) = trimmed.strip_prefix("```") {
            if in_fence {
                in_fence = false;
                continue;
            }
            in_fence = true;
            let lang = info
                .split(|c: char| c == ',' || c == '{' || c.is_whitespace())
                .next()
                .unwrap_or("")
                .to_ascii_lowercase();
            if !in_region && LINTED.contains(&lang.as_str()) {
                out.push(index + 1);
            }
        }
    }
    out
}

fn dedent(lines: &[&str]) -> String {
    let indent = lines
        .iter()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.len() - line.trim_start().len())
        .min()
        .unwrap_or(0);
    lines
        .iter()
        .map(|line| line.get(indent..).unwrap_or("").trim_end())
        .collect::<Vec<_>>()
        .join("\n")
        .trim_matches('\n')
        .to_owned()
}

fn language(path: &Path) -> Option<&'static str> {
    match path.extension()?.to_str()? {
        "rs" => Some("rust"),
        "py" => Some("python"),
        "js" | "mjs" => Some("js"),
        "c" => Some("c"),
        _ => None,
    }
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk(&path, out);
        } else {
            out.push(path);
        }
    }
}

/// Files the site does not publish (`srcExclude` in `docs/.vitepress/config.ts`):
/// agent context and the ADR template, which may quote the markers themselves.
const EXCLUDED: &[&str] = &["AGENTS.md", "PLAN.md", "_template.md"];

/// Every published Markdown page under `docs/`, skipping build output and
/// dependencies.
fn pages(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if path.is_dir() {
            if name != "node_modules" && name != "dist" && name != "cache" {
                out.extend(pages(&path));
            }
        } else if path.extension().is_some_and(|e| e == "md") && !EXCLUDED.contains(&name.as_str())
        {
            out.push(path);
        }
    }
    out.sort();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regions_fill_and_fences_are_linted() {
        let mut snippets = BTreeMap::new();
        snippets.insert(
            "x".to_owned(),
            Snippet {
                lang: "rust",
                code: "let a = 1;".to_owned(),
                origin: "t.rs:1".to_owned(),
            },
        );
        let page =
            "a\n<!-- SNIPPET:x -->\nold\n<!-- /SNIPPET -->\n```rust\nhand\n```\n```text\nok\n```";
        let (filled, names) = fill(page, &snippets).unwrap();
        assert_eq!(names, ["x"]);
        assert!(filled.contains("```rust\nlet a = 1;\n```"));
        assert_eq!(unmarked_fences(&filled), [9]);
        assert!(fill("<!-- SNIPPET:y -->\n<!-- /SNIPPET -->", &snippets).is_err());
        let quoted = "```text\n<!-- SNIPPET:y -->\n<!-- /SNIPPET -->\n```\n";
        assert_eq!(
            fill(quoted, &snippets).unwrap(),
            (quoted.to_owned(), vec![])
        );
    }

    #[test]
    fn dedent_keeps_relative_indentation() {
        assert_eq!(
            dedent(&["    a", "        b", "", "    c"]),
            "a\n    b\n\nc"
        );
    }
}
