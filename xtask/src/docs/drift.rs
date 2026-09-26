//! Prose patterns that go stale the day they are written.
//!
//! Each rule names a fact that has a generated home, so hand-typing it is
//! always the wrong fix:
//!
//! - a git dependency (`git = "…"`, with or without a `rev`/`tag`): install
//!   from a registry, as `guide/install.md` shows;
//! - `version = "…"` in a TOML example: `cargo add` picks the latest release;
//! - a crate count of ten or more, in words or digits: quote
//!   `facts.crates.total` from `facts.json`;
//! - an absolute `/home/` path: it names one machine and leaks who owns it.
//!
//! Generated regions and wholly generated pages are exempt from all but the
//! last rule, since their text comes from a source file; a home path is wrong
//! wherever it appears.

use crate::text::splitlines;

const NUMBER_WORDS: &[&str] = &[
    "ten",
    "eleven",
    "twelve",
    "thirteen",
    "fourteen",
    "fifteen",
    "sixteen",
    "seventeen",
    "eighteen",
    "nineteen",
    "twenty",
    "thirty",
    "forty",
    "fifty",
    "sixty",
    "seventy",
    "eighty",
    "ninety",
];

/// `page:line: why` for every stale pattern in a page.
pub(super) fn problems(rel: &str, text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let generated = text.contains(super::generated_banner());
    let mut in_region = false;
    let mut fence: Option<String> = None;
    for (index, line) in splitlines(text).into_iter().enumerate() {
        let at = |why: &str| format!("{rel}:{}: {why}", index + 1);
        let trimmed = line.trim_start();
        if line.contains("/home/") {
            out.push(at(
                "absolute `/home/` path; use a path relative to the repository",
            ));
        }
        if trimmed.starts_with("<!-- SNIPPET:")
            || (trimmed.starts_with("<!-- ") && trimmed.ends_with(":BEGIN -->"))
        {
            in_region = true;
            continue;
        }
        if trimmed.starts_with("<!-- /SNIPPET -->")
            || (trimmed.starts_with("<!-- ") && trimmed.ends_with(":END -->"))
        {
            in_region = false;
            continue;
        }
        if let Some(info) = trimmed.strip_prefix("```") {
            fence = match fence {
                Some(_) => None,
                None => Some(info.trim().to_ascii_lowercase()),
            };
            continue;
        }
        if in_region || generated {
            continue;
        }
        if line.contains("git = \"") || line.contains("rev = \"") {
            out.push(at("git dependency; install a released version instead"));
        }
        if fence.as_deref() == Some("toml") && line.contains("version = \"") {
            out.push(at(
                "pinned version in a TOML example; show `cargo add` so it is always current",
            ));
        }
        if fence.is_none() && counts_crates(line) {
            out.push(at(
                "hand-typed crate count; quote `{{ facts.crates.total }}` from facts.json",
            ));
        }
    }
    out
}

/// Whether a line states a count of ten or more crates: `31 crates`,
/// `twenty-one crates`, `Thirty-one workspace crates`.
fn counts_crates(line: &str) -> bool {
    let words: Vec<String> = line
        .split(|c: char| !(c.is_alphanumeric() || c == '-'))
        .filter(|w| !w.is_empty())
        .map(str::to_ascii_lowercase)
        .collect();
    words.iter().enumerate().any(|(i, word)| {
        let crates_follows = words.get(i + 1).is_some_and(|w| w == "crates")
            || words.get(i + 2).is_some_and(|w| w == "crates");
        crates_follows && is_large_number(word)
    })
}

fn is_large_number(word: &str) -> bool {
    if let Ok(n) = word.parse::<u32>() {
        return n >= 10;
    }
    let head = word.split('-').next().unwrap_or(word);
    NUMBER_WORDS.contains(&head)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stale_patterns_are_found_outside_generated_regions() {
        let page = "\
The workspace is Twenty-one crates.
We have 31 publishable crates.
These two crates agree.
```toml
openbim-ifc = { version = \"0.1\" }
```
```bash
cargo add openbim-ifc
```
axiolid-core = { git = \"x\", tag = \"v0.1.8\" }
<!-- X:BEGIN -->
31 crates, version = \"0.1\", rev = \"a\"
see /home/someone/x
<!-- X:END -->";
        let lines: Vec<String> = problems("p.md", page)
            .iter()
            .map(|p| p.split(':').nth(1).unwrap().to_owned())
            .collect();
        assert_eq!(lines, ["1", "2", "5", "10", "13"]);

        let generated = format!(
            "{}\nThe family is 31 crates.",
            crate::docs::generated_banner()
        );
        assert!(problems("p.md", &generated).is_empty());
    }
}
