//! STEP string escape decoding.
//!
//! ISO-10303-21 encodes non-ASCII text with escape sequences rather than UTF-8:
//! `\S\` for the upper half of a code page, `\X\` for a raw byte, and
//! `\X2\`/`\X4\` for sequences of UTF-16 / UTF-32 code units terminated by
//! `\X0\`. Doubled apostrophes (`''`) are a literal quote.
//!
//! # Why this matters
//!
//! Treating STEP strings as latin-1 is a common shortcut that silently
//! corrupts every non-Western project name, and German umlauts in particular —
//! which is most of the local corpus. Decoding is therefore a first-class
//! module with its own tests, not an afterthought in the lexer.
//!
//! Not yet implemented — Stage 1 in `docs/ROADMAP.md`.

/// Decode a STEP-encoded string literal body into a Rust `String`.
///
/// The input is the raw bytes **between** the surrounding apostrophes.
pub fn decode(_raw: &[u8]) -> String {
    unimplemented!("Stage 1: STEP escape decoding")
}
