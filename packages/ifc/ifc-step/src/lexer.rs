//! Byte-level tokenizer over a record body.
//!
//! Operates on `&[u8]` borrowed from the mmap, not `String`. STEP is
//! ASCII-structured with escaped unicode inside string literals (see
//! [`crate::escape`]), so decoding is deferred until a caller actually wants
//! text. That keeps the scan allocation-free on the hot path.
//!
//! Not yet implemented — Stage 1 in `docs/ROADMAP.md`.

/// A lexical token in a STEP record body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Token<'a> {
    /// `#123` — an entity instance reference.
    Ref(u64),
    /// A bare keyword such as an entity or enum name.
    Keyword(&'a [u8]),
    /// A quoted string literal, still escaped.
    Text(&'a [u8]),
    /// A numeric literal, still unparsed.
    Number(&'a [u8]),
    /// `$` — an unset optional attribute.
    Unset,
    /// `*` — a derived attribute.
    Derived,
    /// `(`
    OpenParen,
    /// `)`
    CloseParen,
    /// `,`
    Comma,
}
