//! REAL tokens written without a decimal point (`1E-05`).
//!
//! ISO 10303-21 defines `REAL = [ SIGN ] DIGIT { DIGIT } "." { DIGIT } [
//! "E" [ SIGN ] DIGIT { DIGIT } ]`: the point is mandatory, so `1E-05` is
//! not a token of the exchange structure and the strict reader refuses it.
//! Some exporters write it anyway (buildingSMART's own IFC4.x alignment test
//! files, #285). Its meaning is not in doubt -- digits and an exponent can
//! only be a REAL -- so a lenient read inserts the missing point, reads the
//! token as the REAL it spells, and reports one diagnostic per token.
//!
//! The generic `openbim-step` lexer refuses the token before this crate sees
//! a record, so the repair is a byte rewrite ahead of the parse, applied
//! only under the recovery policy. [`find`] recognises a token the way the
//! lexer would reach it: outside quoted strings, binary literals and
//! comments, and not as part of a keyword (`IFC2X3`), an instance name
//! (`#12`) or the fraction or exponent of a real. A token split by an
//! ignored control character is not recognised and is skipped as a
//! malformed record, as before.

use std::borrow::Cow;
use std::ops::Range;

/// One token missing its decimal point.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Missing {
    /// The whole token, sign and exponent included.
    pub(crate) token: Range<usize>,
    /// Where the point belongs: just after the mantissa digits.
    pub(crate) point: usize,
}

/// Every REAL token in `input` that lacks its decimal point, in file order.
pub(crate) fn find(input: &[u8]) -> Vec<Missing> {
    let mut found = Vec::new();
    let mut at = 0;
    while at < input.len() {
        match input[at] {
            quote @ (b'\'' | b'"') => {
                // A doubled apostrophe closes and reopens the string, which
                // this loop handles by toggling twice.
                at += 1;
                while at < input.len() && input[at] != quote {
                    at += 1;
                }
                at += 1;
            }
            b'/' if input.get(at + 1) == Some(&b'*') => {
                at += 2;
                while at < input.len() && !input[at..].starts_with(b"*/") {
                    at += 1;
                }
                at += 2;
            }
            byte if byte.is_ascii_digit() && starts_token(input, at) => {
                let start = if at > 0 && matches!(input[at - 1], b'+' | b'-') {
                    at - 1
                } else {
                    at
                };
                let mut end = at;
                while end < input.len() && input[end].is_ascii_digit() {
                    end += 1;
                }
                if let Some(exponent_end) = exponent(input, end) {
                    found.push(Missing {
                        token: start..exponent_end,
                        point: end,
                    });
                    at = exponent_end;
                } else {
                    at = end;
                }
            }
            _ => at += 1,
        }
    }
    found
}

/// Whether the digit at `at` begins a number token rather than continuing
/// a keyword, an instance name or another number.
fn starts_token(input: &[u8], at: usize) -> bool {
    let Some(&before) = at.checked_sub(1).and_then(|index| input.get(index)) else {
        return true;
    };
    let before = if matches!(before, b'+' | b'-') {
        // A sign belongs to the number only when it is itself not glued to
        // a word (an exponent's sign follows its `E`).
        match at.checked_sub(2).and_then(|index| input.get(index)) {
            Some(&byte) => byte,
            None => return true,
        }
    } else {
        before
    };
    !(before.is_ascii_alphanumeric() || matches!(before, b'_' | b'.' | b'#' | b'!' | b'@'))
}

/// The end of an exponent `E [sign] digit {digit}` starting at `at`.
fn exponent(input: &[u8], at: usize) -> Option<usize> {
    if !matches!(input.get(at), Some(b'e' | b'E')) {
        return None;
    }
    let mut end = at + 1;
    if matches!(input.get(end), Some(b'+' | b'-')) {
        end += 1;
    }
    let digits = end;
    while end < input.len() && input[end].is_ascii_digit() {
        end += 1;
    }
    (end > digits).then_some(end)
}

/// `input` with a point inserted for every entry of `missing`.
pub(crate) fn repair<'a>(input: &'a [u8], missing: &[Missing]) -> Cow<'a, [u8]> {
    if missing.is_empty() {
        return Cow::Borrowed(input);
    }
    let mut out = Vec::with_capacity(input.len() + missing.len());
    let mut from = 0;
    for entry in missing {
        out.extend_from_slice(&input[from..entry.point]);
        out.push(b'.');
        from = entry.point;
    }
    out.extend_from_slice(&input[from..]);
    Cow::Owned(out)
}

/// Maps an offset into the repaired bytes back to the original input.
pub(crate) fn original_offset(missing: &[Missing], repaired: usize) -> usize {
    // The k-th inserted point sits at `point + k` in the repaired bytes.
    let before = missing
        .iter()
        .enumerate()
        .take_while(|(index, entry)| entry.point + index < repaired)
        .count();
    repaired - before
}

/// The token's text, for diagnostics and errors.
pub(crate) fn text(input: &[u8], missing: &Missing) -> String {
    String::from_utf8_lossy(&input[missing.token.clone()]).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokens(input: &str) -> Vec<String> {
        find(input.as_bytes())
            .iter()
            .map(|missing| text(input.as_bytes(), missing))
            .collect()
    }

    #[test]
    fn finds_bare_exponents_and_nothing_else() {
        assert_eq!(
            tokens("#1=IFCX(1E-05,-2E3,(3e+2,4.E1,5.5E2),1.0,7,#8,'9E9',\"0A1E2\",.E1.);"),
            ["1E-05", "-2E3", "3e+2"]
        );
        assert!(tokens("#1=IFC2X3E1(IFC4X3E2,#12E3,!A1E2,/* 1E2 */'a''1E2');").is_empty());
        assert!(tokens("1.E-5 12.5e+3 -0.").is_empty());
    }

    #[test]
    fn repair_inserts_points_and_offsets_map_back() {
        let input = b"(1E2,-3E4)";
        let missing = find(input);
        let repaired = repair(input, &missing);
        assert_eq!(&*repaired, b"(1.E2,-3.E4)");
        assert_eq!(original_offset(&missing, 1), 1);
        assert_eq!(original_offset(&missing, 2), 2);
        assert_eq!(original_offset(&missing, 3), 2);
        assert_eq!(original_offset(&missing, 6), 5);
        assert_eq!(original_offset(&missing, 12), 10);
    }
}
