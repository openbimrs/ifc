//! Lines, columns and short quotes of the source for diagnostics.
//!
//! Positions follow `openbim_step::Source::location`: the line counts `\n`
//! bytes, the column counts the Unicode characters of the line before the
//! offset, decoding invalid UTF-8 as U+FFFD. [`positions`] answers many
//! offsets in one forward pass instead of one scan from the start each.

use std::ops::Range;

use ifc_model::SourcePosition;

/// Most characters of the source a diagnostic quotes.
pub(crate) const EXCERPT_CHARS: usize = 120;

/// The positions of `offsets` in `input`, in the order of `offsets`.
///
/// An offset past the end is clamped to the end. Within a line the column
/// carries forward from the previous offset, so many offsets on one long line
/// cost one pass over it, not one pass each.
pub(crate) fn positions(input: &[u8], offsets: &[usize]) -> Vec<SourcePosition> {
    let mut order: Vec<usize> = (0..offsets.len()).collect();
    order.sort_by_key(|&index| offsets[index]);
    let mut found = vec![SourcePosition::new(1, 1); offsets.len()];
    let (mut at, mut line, mut line_start) = (0, 1, 0);
    // The column at `checkpoint`, 1-based, on the current line.
    let (mut checkpoint, mut checkpoint_column) = (0, 1);
    for index in order {
        let offset = offsets[index].min(input.len());
        for (step, byte) in input[at..offset].iter().enumerate() {
            if *byte == b'\n' {
                line += 1;
                line_start = at + step + 1;
                (checkpoint, checkpoint_column) = (line_start, 1);
            }
        }
        at = offset;
        // Decoding is lossy, so the count of two halves equals the count of
        // the whole only when the split lies before an ASCII byte (or at
        // the end); otherwise count from the line start again.
        if !input.get(checkpoint).is_none_or(u8::is_ascii) {
            (checkpoint, checkpoint_column) = (line_start, 1);
        }
        let column = checkpoint_column
            + String::from_utf8_lossy(&input[checkpoint..offset])
                .chars()
                .count();
        (checkpoint, checkpoint_column) = (offset, column);
        found[index] = SourcePosition::new(line, column);
    }
    found
}

/// A quote of the first [`EXCERPT_CHARS`] characters of `range`, line breaks
/// and tabs as spaces, with `…` when the range goes on.
pub(crate) fn excerpt_of(input: &[u8], range: Range<usize>) -> String {
    let end = range.end.min(input.len());
    let start = range.start.min(end);
    // Four bytes cover any character; one byte more shows the range goes on.
    let take = (end - start).min(EXCERPT_CHARS * 4 + 1);
    let text = String::from_utf8_lossy(&input[start..start + take]);
    let mut chars = text.chars();
    let mut quote: String = chars.by_ref().take(EXCERPT_CHARS).map(plain).collect();
    if chars.next().is_some() || take < end - start {
        quote.push('…');
    }
    quote
}

/// Line breaks and tabs as a space, so a quote stays on one line.
pub(crate) fn plain(c: char) -> char {
    if matches!(c, '\r' | '\n' | '\t') {
        ' '
    } else {
        c
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openbim_step::{Source, Span};

    fn reference(input: &[u8], offset: usize) -> SourcePosition {
        let at = Source::new("", input).location(Span::new(offset, offset));
        SourcePosition::new(at.line, at.column)
    }

    fn agrees(input: &[u8]) {
        // Unsorted, repeated and past-the-end offsets on purpose.
        let mut offsets: Vec<usize> = (0..=input.len() + 2).rev().collect();
        offsets.push(0);
        let got = positions(input, &offsets);
        for (offset, position) in offsets.iter().zip(got) {
            assert_eq!(position, reference(input, *offset), "offset {offset}");
        }
    }

    #[test]
    fn matches_the_substrate_on_lf() {
        agrees(b"ab\ncd\n\nef");
    }

    #[test]
    fn matches_the_substrate_on_crlf() {
        agrees(b"ab\r\ncd\r\n#1=X();\r\n");
    }

    #[test]
    fn matches_the_substrate_on_multibyte_utf8() {
        agrees("a\u{e4}\u{20ac}b\n\u{1f600}x\n".as_bytes());
    }

    #[test]
    fn matches_the_substrate_on_invalid_utf8() {
        agrees(b"a\xffb\n\xc3\xe4\n\xe2\x82");
    }

    #[test]
    fn many_offsets_on_one_line_agree_with_the_substrate() {
        // Mixed widths, invalid bytes and a few line breaks, so checkpoints
        // land before ASCII and before continuation bytes.
        let mut input = Vec::new();
        for n in 0..40 {
            input.extend_from_slice(format!("#{n}=X('a").as_bytes());
            input.extend_from_slice(["\u{e4}", "\u{20ac}", "\u{1f600}"][n % 3].as_bytes());
            input.extend_from_slice(if n % 7 == 0 { b"\xff" } else { b"b" });
            input.extend_from_slice(if n % 13 == 12 { b"');\n" } else { b"');" });
        }
        agrees(&input);
    }

    #[test]
    fn twenty_thousand_records_on_one_line_are_positioned() {
        let input: Vec<u8> = (0..20_000)
            .flat_map(|n| format!("#{n}=X();").into_bytes())
            .collect();
        let starts: Vec<usize> = (0..60_000).map(|n| n * input.len() / 60_000).collect();
        let got = positions(&input, &starts);
        assert!(got.iter().all(|p| p.line() == 1));
        assert!(got.windows(2).all(|w| w[0].column() <= w[1].column()));
        assert_eq!(got[0], SourcePosition::new(1, 1));
    }

    #[test]
    fn offset_at_the_end_is_the_last_position() {
        assert_eq!(positions(b"ab\ncd", &[5]), [SourcePosition::new(2, 3)]);
        assert_eq!(positions(b"", &[0]), [SourcePosition::new(1, 1)]);
    }

    #[test]
    fn excerpt_cuts_at_the_limit_and_flattens_breaks() {
        assert_eq!(excerpt_of(b"a\r\n\tb", 0..5), "a   b");
        let long = "x".repeat(300);
        let quote = excerpt_of(long.as_bytes(), 0..300);
        assert_eq!(quote.chars().count(), EXCERPT_CHARS + 1);
        assert!(quote.ends_with('…'));
        let exact = "y".repeat(EXCERPT_CHARS);
        assert_eq!(excerpt_of(exact.as_bytes(), 0..EXCERPT_CHARS), exact);
        assert_eq!(excerpt_of(b"abc", 2..99), "c");
    }
}
