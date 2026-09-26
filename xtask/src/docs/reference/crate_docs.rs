//! A crate's `//!` documentation as Markdown the docs site can publish.
//!
//! The overview is the crate docs up to their first heading or code block:
//! the part written as an introduction. Rustdoc-only syntax is flattened so
//! the page never shows a dead intra-doc link, and bare `<` is escaped because
//! VitePress would otherwise parse `Vec<T>` in prose as a Vue component.

use std::path::Path;

use syn::{Expr, ExprLit, Lit, Meta};

/// The crate-level docs of the library rooted at `lib`, up to the first
/// heading or code fence.
pub(super) fn overview(lib: &Path) -> Result<String, String> {
    let source =
        std::fs::read_to_string(lib).map_err(|error| format!("{}: {error}", lib.display()))?;
    let file = syn::parse_file(&source).map_err(|error| format!("{}: {error}", lib.display()))?;
    let mut lines: Vec<String> = Vec::new();
    for attr in &file.attrs {
        let Meta::NameValue(meta) = &attr.meta else {
            continue;
        };
        if !meta.path.is_ident("doc") {
            continue;
        }
        let Expr::Lit(ExprLit {
            lit: Lit::Str(text),
            ..
        }) = &meta.value
        else {
            continue;
        };
        let value = text.value();
        if value.trim().is_empty() {
            // `//!` alone is a paragraph break; `str::lines` would drop it.
            lines.push(String::new());
            continue;
        }
        for line in value.lines() {
            lines.push(line.strip_prefix(' ').unwrap_or(line).to_owned());
        }
    }
    let mut out: Vec<String> = Vec::new();
    for line in lines {
        let trimmed = line.trim_start();
        if trimmed.starts_with('#') || trimmed.starts_with("```") {
            break;
        }
        out.push(escape(&flatten_links(&line)));
    }
    Ok(out.join("\n").trim().to_owned())
}

/// Keep `[text](https://…)` links; reduce rustdoc intra-doc links
/// (`[`Model`]`, `[text](crate::x)`, `[text][ref]`) to their text.
fn flatten_links(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut rest = line;
    while let Some(open) = rest.find('[') {
        out.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        let Some(close) = after.find(']') else {
            out.push_str(&rest[open..]);
            return out;
        };
        let text = &after[..close];
        let tail = &after[close + 1..];
        if let Some(target) = tail.strip_prefix('(') {
            if let Some(end) = target.find(')') {
                let url = &target[..end];
                if url.starts_with("http://") || url.starts_with("https://") {
                    out.push_str(&rest[open..open + 1 + close + 1 + 1 + end + 1]);
                } else {
                    out.push_str(text);
                }
                rest = &target[end + 1..];
                continue;
            }
        }
        if let Some(reference) = tail.strip_prefix('[') {
            if let Some(end) = reference.find(']') {
                out.push_str(text);
                rest = &reference[end + 1..];
                continue;
            }
        }
        out.push_str(text);
        rest = tail;
    }
    out.push_str(rest);
    out
}

/// Escape `<` and `{{` outside inline code spans.
fn escape(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut in_code = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '`' => {
                in_code = !in_code;
                out.push(c);
            }
            '<' if !in_code => out.push_str("&lt;"),
            '{' if !in_code && chars.peek() == Some(&'{') => out.push_str("&#123;"),
            _ => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rustdoc_links_flatten_and_web_links_stay() {
        assert_eq!(
            flatten_links("See [`Model`] and [the view](crate::View) or [x][y]."),
            "See `Model` and the view or x."
        );
        assert_eq!(
            flatten_links("Read [ISO](https://iso.org) now."),
            "Read [ISO](https://iso.org) now."
        );
    }

    #[test]
    fn angle_brackets_escape_outside_code_only() {
        assert_eq!(escape("a Vec<T> and `Vec<T>`"), "a Vec&lt;T> and `Vec<T>`");
    }
}
