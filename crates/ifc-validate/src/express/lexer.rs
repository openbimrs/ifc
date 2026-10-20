//! EXPRESS tokens (ISO 10303-11 clause 7).
//!
//! Keywords and identifiers are case-insensitive, so every word is kept
//! upper-cased beside its spelling; string literals keep their case, because
//! EXPRESS compares strings exactly. Remarks are skipped: `(* ... *)`, which
//! nests, and `--` to the end of the line.

use std::sync::Arc;

use super::error::{ExpressError, SourcePos};

/// One token.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Tok {
    /// An identifier or keyword: upper-cased, then as written.
    Word(Arc<str>, Arc<str>),
    /// An integer literal.
    Int(i64),
    /// A real literal.
    Real(f64),
    /// A string literal, quotes and doubled quotes resolved.
    Str(Arc<str>),
    /// A binary literal without its `%`.
    Binary(Arc<str>),
    /// A symbol: `:=`, `:=:`, `<*`, `(`, ...
    Sym(&'static str),
    /// The end of the source.
    End,
}

/// A token and where it starts.
#[derive(Debug, Clone)]
pub(crate) struct Spanned {
    pub tok: Tok,
    pub pos: SourcePos,
}

/// Symbols, longest first so a prefix never wins.
const SYMBOLS: &[&str] = &[
    ":<>:", ":=:", ":=", "<>", "<=", ">=", "<*", "||", "**", "(", ")", "[", "]", "{", "}", ",",
    ";", ":", ".", "\\", "+", "-", "*", "/", "=", "<", ">", "?", "|",
];

/// Splits `source` into tokens, ending with [`Tok::End`].
pub(crate) fn tokenize(source: &str) -> Result<Vec<Spanned>, ExpressError> {
    let bytes = source.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    let mut line = 1u32;
    while i < bytes.len() {
        let c = bytes[i];
        if c == b'\n' {
            line += 1;
            i += 1;
            continue;
        }
        if c.is_ascii_whitespace() {
            i += 1;
            continue;
        }
        let pos = SourcePos { line };
        if c == b'(' && bytes.get(i + 1) == Some(&b'*') {
            i = skip_remark(bytes, i, &mut line)?;
            continue;
        }
        if c == b'-' && bytes.get(i + 1) == Some(&b'-') {
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        if c.is_ascii_alphabetic() {
            let start = i;
            while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
            }
            let text = &source[start..i];
            out.push(Spanned {
                tok: Tok::Word(text.to_ascii_uppercase().into(), text.into()),
                pos,
            });
            continue;
        }
        if c.is_ascii_digit() {
            let (tok, next) = number(source, i, pos)?;
            out.push(Spanned { tok, pos });
            i = next;
            continue;
        }
        if c == b'\'' {
            let (text, next) = string(source, i, &mut line)?;
            out.push(Spanned {
                tok: Tok::Str(text.into()),
                pos,
            });
            i = next;
            continue;
        }
        if c == b'%' {
            let start = i + 1;
            i = start;
            while i < bytes.len() && matches!(bytes[i], b'0' | b'1') {
                i += 1;
            }
            out.push(Spanned {
                tok: Tok::Binary(source[start..i].into()),
                pos,
            });
            continue;
        }
        let rest = &source[i..];
        let Some(symbol) = SYMBOLS.iter().find(|symbol| rest.starts_with(**symbol)) else {
            return Err(ExpressError::syntax(
                pos,
                format!(
                    "unexpected character {:?}",
                    rest.chars().next().unwrap_or(' ')
                ),
            ));
        };
        out.push(Spanned {
            tok: Tok::Sym(symbol),
            pos,
        });
        i += symbol.len();
    }
    out.push(Spanned {
        tok: Tok::End,
        pos: SourcePos { line },
    });
    Ok(out)
}

/// Skips a `(* ... *)` remark, which may nest; returns the index after it.
fn skip_remark(bytes: &[u8], mut i: usize, line: &mut u32) -> Result<usize, ExpressError> {
    let start = SourcePos { line: *line };
    let mut depth = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'(' && bytes.get(i + 1) == Some(&b'*') {
            depth += 1;
            i += 2;
        } else if bytes[i] == b'*' && bytes.get(i + 1) == Some(&b')') {
            depth -= 1;
            i += 2;
            if depth == 0 {
                return Ok(i);
            }
        } else {
            if bytes[i] == b'\n' {
                *line += 1;
            }
            i += 1;
        }
    }
    Err(ExpressError::syntax(start, "unterminated remark"))
}

/// An integer or real literal starting at `start`.
fn number(source: &str, start: usize, pos: SourcePos) -> Result<(Tok, usize), ExpressError> {
    let bytes = source.as_bytes();
    let mut i = start;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    let mut real = false;
    // `1.` and `1.5` are reals; `1.E-3` too. A `.` followed by a letter
    // other than `E` would be a qualifier, which no literal can carry.
    if i < bytes.len() && bytes[i] == b'.' {
        let next = bytes.get(i + 1).copied();
        let exponent = matches!(next, Some(b'E' | b'e'));
        if !next.is_some_and(|n| n.is_ascii_alphabetic()) || exponent {
            real = true;
            i += 1;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
        }
    }
    if real && i < bytes.len() && matches!(bytes[i], b'E' | b'e') {
        let mut j = i + 1;
        if j < bytes.len() && matches!(bytes[j], b'+' | b'-') {
            j += 1;
        }
        if j < bytes.len() && bytes[j].is_ascii_digit() {
            while j < bytes.len() && bytes[j].is_ascii_digit() {
                j += 1;
            }
            i = j;
        }
    }
    let text = &source[start..i];
    let tok = if real {
        let normalized = text.replace(".E", ".0E").replace(".e", ".0e");
        let normalized = if normalized.ends_with('.') {
            format!("{normalized}0")
        } else {
            normalized
        };
        Tok::Real(
            normalized
                .parse()
                .map_err(|_| ExpressError::syntax(pos, format!("bad real literal {text}")))?,
        )
    } else {
        Tok::Int(
            text.parse()
                .map_err(|_| ExpressError::syntax(pos, format!("bad integer literal {text}")))?,
        )
    };
    Ok((tok, i))
}

/// A `'...'` literal starting at `start`, `''` standing for one quote.
fn string(source: &str, start: usize, line: &mut u32) -> Result<(String, usize), ExpressError> {
    let bytes = source.as_bytes();
    let begin = SourcePos { line: *line };
    let mut i = start + 1;
    let mut text = String::new();
    let mut run = i;
    while i < bytes.len() {
        if bytes[i] == b'\'' {
            text.push_str(&source[run..i]);
            if bytes.get(i + 1) == Some(&b'\'') {
                text.push('\'');
                i += 2;
                run = i;
                continue;
            }
            return Ok((text, i + 1));
        }
        if bytes[i] == b'\n' {
            *line += 1;
        }
        i += 1;
    }
    Err(ExpressError::syntax(begin, "unterminated string literal"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn toks(source: &str) -> Vec<Tok> {
        tokenize(source)
            .expect("tokenizes")
            .into_iter()
            .map(|s| s.tok)
            .collect()
    }

    #[test]
    fn words_are_upper_cased_and_remarks_skipped() {
        let found = toks("Self (* a (* nested *) remark *) -- tail\n.Dim");
        assert_eq!(
            found,
            vec![
                Tok::Word("SELF".into(), "Self".into()),
                Tok::Sym("."),
                Tok::Word("DIM".into(), "Dim".into()),
                Tok::End
            ]
        );
    }

    #[test]
    fn numbers_strings_and_symbols() {
        let found = toks("[1.,0.] 1.E-5 2 'it''s' :<>: :=: <* %01");
        assert_eq!(
            found,
            vec![
                Tok::Sym("["),
                Tok::Real(1.0),
                Tok::Sym(","),
                Tok::Real(0.0),
                Tok::Sym("]"),
                Tok::Real(1e-5),
                Tok::Int(2),
                Tok::Str("it's".into()),
                Tok::Sym(":<>:"),
                Tok::Sym(":=:"),
                Tok::Sym("<*"),
                Tok::Binary("01".into()),
                Tok::End
            ]
        );
    }

    #[test]
    fn an_unterminated_string_is_refused() {
        assert!(tokenize("'open").is_err());
    }
}
