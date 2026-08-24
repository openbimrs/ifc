//! STEP string escape decoding and encoding.
//!
//! ISO 10303-21 predates UTF-8 and encodes non-ASCII text with escape
//! sequences instead:
//!
//! | Sequence | Meaning |
//! | --- | --- |
//! | `''` | a literal apostrophe |
//! | `\\` | a literal backslash |
//! | `\S\c` | code point `c + 128` (ISO 8859-1 upper half) |
//! | `\X\hh` | one raw byte, two hex digits |
//! | `\X2\hhhh...\X0\` | UTF-16BE run |
//! | `\X4\hhhhhhhh...\X0\` | UTF-32BE run |
//!
//! Decoding only latin-1 is a common shortcut that silently mangles every
//! non-Western project name in a model, so all forms are handled here.

/// Decode a STEP string literal body into UTF-8.
///
/// Unrecognized escapes are passed through verbatim rather than dropped: a
/// preserved oddity round-trips, a dropped one is data loss.
pub fn decode(raw: &[u8]) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut i = 0;
    while i < raw.len() {
        match raw[i] {
            b'\'' if raw.get(i + 1) == Some(&b'\'') => {
                out.push('\'');
                i += 2;
            }
            b'\\' => {
                if let Some(consumed) = decode_escape(&raw[i..], &mut out) {
                    i += consumed;
                } else {
                    out.push('\\');
                    i += 1;
                }
            }
            b => {
                // Bytes above 0x7f are latin-1 in practice; `as char` maps
                // them to the matching code point, which is the correct
                // interpretation for an unescaped high byte.
                out.push(b as char);
                i += 1;
            }
        }
    }
    out
}

/// Try to decode one escape sequence starting at `s[0] == b'\\'`.
/// Returns the number of bytes consumed.
fn decode_escape(s: &[u8], out: &mut String) -> Option<usize> {
    match s.get(1)? {
        b'\\' => {
            out.push('\\');
            Some(2)
        }
        b'S' => {
            // \S\c  ->  c + 128
            if s.get(2) == Some(&b'\\') {
                let c = *s.get(3)?;
                out.push((c as u32 + 128) as u8 as char);
                Some(4)
            } else {
                None
            }
        }
        b'X' => match s.get(2)? {
            b'\\' => {
                let hex = s.get(3..5)?;
                let byte = u8::from_str_radix(std::str::from_utf8(hex).ok()?, 16).ok()?;
                out.push(byte as char);
                Some(5)
            }
            b'2' => decode_wide(s, out, 2),
            b'4' => decode_wide(s, out, 4),
            _ => None,
        },
        _ => None,
    }
}

/// Decode a `\X2\....\X0\` or `\X4\....\X0\` run.
fn decode_wide(s: &[u8], out: &mut String, width: usize) -> Option<usize> {
    if s.get(3) != Some(&b'\\') {
        return None;
    }
    let body_start = 4;
    let digits_per_unit = width * 2;
    let mut i = body_start;
    let mut units: Vec<u16> = Vec::new();
    let mut wide: Vec<u32> = Vec::new();
    loop {
        if s[i..].starts_with(b"\\X0\\") {
            i += 4;
            break;
        }
        let hex = s.get(i..i + digits_per_unit)?;
        let text = std::str::from_utf8(hex).ok()?;
        if width == 2 {
            units.push(u16::from_str_radix(text, 16).ok()?);
        } else {
            wide.push(u32::from_str_radix(text, 16).ok()?);
        }
        i += digits_per_unit;
        if i >= s.len() {
            break;
        }
    }
    if width == 2 {
        out.push_str(&String::from_utf16_lossy(&units));
    } else {
        for cp in wide {
            out.push(char::from_u32(cp).unwrap_or('\u{fffd}'));
        }
    }
    Some(i)
}

/// Encode a UTF-8 string as a STEP string literal body.
///
/// ASCII passes through; anything else becomes an `\X2\` UTF-16BE run, which
/// every conforming reader understands.
pub fn encode(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut pending: Vec<u16> = Vec::new();

    // Flush any accumulated non-ASCII run as a single \X2\ block, because one
    // block per run is both smaller and more readable than one per character.
    fn flush(pending: &mut Vec<u16>, out: &mut String) {
        if pending.is_empty() {
            return;
        }
        out.push_str("\\X2\\");
        for unit in pending.iter() {
            out.push_str(&format!("{unit:04X}"));
        }
        out.push_str("\\X0\\");
        pending.clear();
    }

    for ch in text.chars() {
        match ch {
            '\'' => {
                flush(&mut pending, &mut out);
                out.push_str("''");
            }
            '\\' => {
                flush(&mut pending, &mut out);
                out.push_str("\\\\");
            }
            c if c.is_ascii() => {
                flush(&mut pending, &mut out);
                out.push(c);
            }
            c => {
                let mut buf = [0u16; 2];
                pending.extend_from_slice(c.encode_utf16(&mut buf));
            }
        }
    }
    flush(&mut pending, &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_ascii_is_unchanged() {
        assert_eq!(decode(b"Basic Wall"), "Basic Wall");
    }

    #[test]
    fn doubled_quote_becomes_one() {
        assert_eq!(decode(b"it''s"), "it's");
    }

    #[test]
    fn decodes_latin1_upper_half() {
        // \S\D -> 0x44 + 128 = 0xC4 = 'A' with diaeresis
        assert_eq!(decode(b"\\S\\D"), "\u{c4}");
    }

    #[test]
    fn decodes_utf16_runs() {
        // Japanese text via \X2\
        assert_eq!(decode(b"\\X2\\30D330EB\\X0\\"), "\u{30d3}\u{30eb}");
    }

    #[test]
    fn decodes_single_byte_hex() {
        assert_eq!(decode(b"\\X\\41"), "A");
    }

    /// The property that matters for export fidelity.
    #[test]
    fn encode_decode_roundtrip() {
        for original in [
            "Basic Wall",
            "it's",
            "\u{c4}\u{d6}\u{dc}",
            "\u{30d3}\u{30eb}",
        ] {
            let encoded = encode(original);
            assert_eq!(decode(encoded.as_bytes()), original, "failed: {original}");
        }
    }

    #[test]
    fn unknown_escapes_survive_rather_than_vanish() {
        assert_eq!(decode(b"\\Q\\x"), "\\Q\\x");
    }
}
