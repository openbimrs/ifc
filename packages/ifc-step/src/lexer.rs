//! Byte-level tokenizer for STEP physical files.
//!
//! Operates on `&[u8]` rather than `&str`: STEP is ASCII-structured with
//! escaped non-ASCII inside string literals, and a raw high byte in a comment
//! must not abort the parse of an otherwise valid file.
//!
//! # Details that bite
//!
//! - `/* ... */` comments appear **between** tokens, including inside
//!   parameter lists. Every fixture in our corpus uses them.
//! - `''` inside a quoted string is an escaped single quote, not a terminator.
//! - `.T.` and `.ELEMENT.` share a lexical shape; only context distinguishes
//!   a logical from an enum, so both lex as [`Token::Keyword`].
//! - Reals may be written `1.`, `1.0`, `1.E3`, or `-.5`.

/// One lexical unit of a STEP file.
#[derive(Debug, Clone, PartialEq)]
pub enum Token<'a> {
    /// `#42` — entity id or reference.
    Id(u64),
    /// A quoted string, still escaped; use `crate::escape::decode`.
    Text(&'a [u8]),
    /// A binary literal, without its quotes.
    Binary(&'a [u8]),
    /// `.T.`, `.F.`, `.U.`, or an enum constant, without the dots.
    Keyword(&'a [u8]),
    /// A bare identifier such as an entity or type name.
    Name(&'a [u8]),
    /// An integer literal.
    Integer(i64),
    /// A real literal.
    Real(f64),
    /// `$`
    Dollar,
    /// `*`
    Star,
    /// `(`
    OpenParen,
    /// `)`
    CloseParen,
    /// `,`
    Comma,
    /// `=`
    Equals,
    /// `;`
    Semicolon,
}

/// Streaming tokenizer over a byte slice.
pub struct Lexer<'a> {
    input: &'a [u8],
    pos: usize,
}

impl<'a> Lexer<'a> {
    /// Start lexing at the beginning of `input`.
    pub fn new(input: &'a [u8]) -> Self {
        Self { input, pos: 0 }
    }

    /// Current byte offset, for error reporting.
    pub fn offset(&self) -> usize {
        self.pos
    }

    /// Skip whitespace and `/* */` comments.
    fn skip_trivia(&mut self) {
        loop {
            while self.pos < self.input.len() && self.input[self.pos].is_ascii_whitespace() {
                self.pos += 1;
            }
            if self.input[self.pos..].starts_with(b"/*") {
                match find(&self.input[self.pos + 2..], b"*/") {
                    Some(end) => self.pos += 2 + end + 2,
                    // Unterminated comment: consume the rest rather than loop.
                    None => self.pos = self.input.len(),
                }
                continue;
            }
            return;
        }
    }

    /// The next token, or `None` at end of input.
    pub fn next_token(&mut self) -> Option<Token<'a>> {
        self.skip_trivia();
        let b = *self.input.get(self.pos)?;
        match b {
            b'(' => self.single(Token::OpenParen),
            b')' => self.single(Token::CloseParen),
            b',' => self.single(Token::Comma),
            b'=' => self.single(Token::Equals),
            b';' => self.single(Token::Semicolon),
            b'$' => self.single(Token::Dollar),
            b'*' => self.single(Token::Star),
            b'#' => self.lex_id(),
            b'\'' => self.lex_text(),
            b'"' => self.lex_binary(),
            b'.' => self.lex_keyword(),
            b'0'..=b'9' | b'-' | b'+' => self.lex_number(),
            _ => self.lex_name(),
        }
    }

    fn single(&mut self, t: Token<'a>) -> Option<Token<'a>> {
        self.pos += 1;
        Some(t)
    }

    fn lex_id(&mut self) -> Option<Token<'a>> {
        self.pos += 1;
        let start = self.pos;
        while self.pos < self.input.len() && self.input[self.pos].is_ascii_digit() {
            self.pos += 1;
        }
        let digits = std::str::from_utf8(&self.input[start..self.pos]).ok()?;
        Some(Token::Id(digits.parse().ok()?))
    }

    fn lex_text(&mut self) -> Option<Token<'a>> {
        self.pos += 1;
        let start = self.pos;
        loop {
            match self.input.get(self.pos)? {
                b'\'' => {
                    // A doubled quote is an escaped quote, not the end.
                    if self.input.get(self.pos + 1) == Some(&b'\'') {
                        self.pos += 2;
                        continue;
                    }
                    let text = &self.input[start..self.pos];
                    self.pos += 1;
                    return Some(Token::Text(text));
                }
                _ => self.pos += 1,
            }
        }
    }

    fn lex_binary(&mut self) -> Option<Token<'a>> {
        self.pos += 1;
        let start = self.pos;
        while *self.input.get(self.pos)? != b'"' {
            self.pos += 1;
        }
        let bin = &self.input[start..self.pos];
        self.pos += 1;
        Some(Token::Binary(bin))
    }

    fn lex_keyword(&mut self) -> Option<Token<'a>> {
        self.pos += 1;
        let start = self.pos;
        while *self.input.get(self.pos)? != b'.' {
            self.pos += 1;
        }
        let kw = &self.input[start..self.pos];
        self.pos += 1;
        Some(Token::Keyword(kw))
    }

    fn lex_name(&mut self) -> Option<Token<'a>> {
        let start = self.pos;
        while self.pos < self.input.len()
            && (self.input[self.pos].is_ascii_alphanumeric() || self.input[self.pos] == b'_')
        {
            self.pos += 1;
        }
        if self.pos == start {
            // Unknown byte: skip it so one stray character cannot wedge the
            // lexer into an infinite loop.
            self.pos += 1;
            return self.next_token();
        }
        Some(Token::Name(&self.input[start..self.pos]))
    }

    fn lex_number(&mut self) -> Option<Token<'a>> {
        let start = self.pos;
        self.pos += 1;
        let mut is_real = false;
        while self.pos < self.input.len() {
            match self.input[self.pos] {
                b'0'..=b'9' => self.pos += 1,
                b'.' => {
                    is_real = true;
                    self.pos += 1;
                }
                b'e' | b'E' => {
                    is_real = true;
                    self.pos += 1;
                    if matches!(self.input.get(self.pos), Some(b'+') | Some(b'-')) {
                        self.pos += 1;
                    }
                }
                _ => break,
            }
        }
        let text = std::str::from_utf8(&self.input[start..self.pos]).ok()?;
        if is_real {
            // `1.` is valid STEP but not valid Rust float syntax in all cases;
            // trimming a trailing '.' makes it parseable.
            let cleaned = text.trim_end_matches('.');
            Some(Token::Real(cleaned.parse().ok()?))
        } else {
            Some(Token::Integer(text.parse().ok()?))
        }
    }
}

/// Find `needle` in `haystack`, returning its start offset.
fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokens(src: &str) -> Vec<Token<'_>> {
        let mut lexer = Lexer::new(src.as_bytes());
        std::iter::from_fn(|| lexer.next_token()).collect()
    }

    #[test]
    fn lexes_a_typical_record() {
        assert_eq!(
            tokens("#1= IFCCARTESIANPOINT((0.0,1.5,-2.));"),
            vec![
                Token::Id(1),
                Token::Equals,
                Token::Name(b"IFCCARTESIANPOINT"),
                Token::OpenParen,
                Token::OpenParen,
                Token::Real(0.0),
                Token::Comma,
                Token::Real(1.5),
                Token::Comma,
                Token::Real(-2.0),
                Token::CloseParen,
                Token::CloseParen,
                Token::Semicolon,
            ]
        );
    }

    /// Every fixture in the corpus carries `/* ... */` comments inside the
    /// data section; treating them as tokens corrupts attribute positions.
    #[test]
    fn skips_comments_between_tokens() {
        assert_eq!(
            tokens("#1= /* comment */ IFCWALL($);"),
            vec![
                Token::Id(1),
                Token::Equals,
                Token::Name(b"IFCWALL"),
                Token::OpenParen,
                Token::Dollar,
                Token::CloseParen,
                Token::Semicolon,
            ]
        );
    }

    #[test]
    fn doubled_quote_is_an_escaped_quote_not_a_terminator() {
        assert_eq!(tokens("'it''s'"), vec![Token::Text(b"it''s")]);
    }

    #[test]
    fn distinguishes_logical_from_enum_by_shape_only() {
        assert_eq!(
            tokens(".T. .ELEMENT."),
            vec![Token::Keyword(b"T"), Token::Keyword(b"ELEMENT")]
        );
    }

    #[test]
    fn handles_exponent_and_trailing_dot_reals() {
        assert_eq!(tokens("1.E3"), vec![Token::Real(1000.0)]);
        assert_eq!(tokens("2."), vec![Token::Real(2.0)]);
    }

    /// A stray byte must not wedge the lexer.
    #[test]
    fn unknown_bytes_do_not_loop_forever() {
        assert_eq!(tokens("@#5"), vec![Token::Id(5)]);
    }
}
