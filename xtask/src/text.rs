//! Small text utilities shared by the generators.

/// Split into lines the way Python's `str.splitlines` does.
///
/// The generators replaced Python scripts whose output is committed; keeping
/// their line semantics (`\r`, `\r\n`, form feed, `\u{2028}`, ...) keeps line
/// counts and parsed sections byte-identical.
pub(crate) fn splitlines(text: &str) -> Vec<&str> {
    let mut lines = Vec::new();
    let mut start = 0;
    let mut chars = text.char_indices().peekable();
    while let Some((index, character)) = chars.next() {
        let is_break = matches!(
            character,
            '\n' | '\r'
                | '\u{0b}'
                | '\u{0c}'
                | '\u{1c}'
                | '\u{1d}'
                | '\u{1e}'
                | '\u{85}'
                | '\u{2028}'
                | '\u{2029}'
        );
        if !is_break {
            continue;
        }
        lines.push(&text[start..index]);
        let mut next = index + character.len_utf8();
        if character == '\r' {
            if let Some(&(lf, '\n')) = chars.peek() {
                chars.next();
                next = lf + 1;
            }
        }
        start = next;
    }
    if start < text.len() {
        lines.push(&text[start..]);
    }
    lines
}

/// Replace the text between `begin` and `end` sentinels with `body`.
///
/// Everything outside the sentinels is prose a human owns and is kept
/// byte-for-byte.
pub(crate) fn splice(text: &str, begin: &str, end: &str, body: &str) -> Result<String, String> {
    let (before, rest) = text
        .split_once(begin)
        .ok_or_else(|| format!("missing {begin} sentinel"))?;
    let (_, after) = rest
        .split_once(end)
        .ok_or_else(|| format!("missing {end} sentinel"))?;
    Ok(format!("{before}{begin}\n\n{body}\n\n{end}{after}"))
}

/// Whether `c` is a regex `\w` character (Unicode letters, digits, `_`).
pub(crate) fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splitlines_matches_python() {
        assert_eq!(splitlines("a\nb\r\nc\rd"), ["a", "b", "c", "d"]);
        assert_eq!(splitlines("a\n"), ["a"]);
        assert_eq!(splitlines("a\n\nb"), ["a", "", "b"]);
        assert_eq!(splitlines("a\u{0c}b"), ["a", "b"]);
        assert!(splitlines("").is_empty());
    }

    #[test]
    fn splice_keeps_prose() {
        let page = "a\n<!-- B -->\nold\n<!-- E -->\nz";
        assert_eq!(
            splice(page, "<!-- B -->", "<!-- E -->", "new").unwrap(),
            "a\n<!-- B -->\n\nnew\n\n<!-- E -->\nz"
        );
        assert!(splice("x", "<!-- B -->", "<!-- E -->", "new").is_err());
    }
}
