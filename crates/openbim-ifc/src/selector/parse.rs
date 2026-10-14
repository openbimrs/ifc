//! The hand-written recursive-descent parser for selectors and value
//! paths. Every error carries the byte span of the text it refuses.

use std::ops::Range;

use ifc_model::guid::Guid;

use super::ast::{
    is_stop, Comparison, Facet, FacetKind, FacetList, Key, Keyword, Literal, Operator, PathKey,
    Pattern, Selector, ValuePath,
};
use super::error::{ParseError, ParseErrorKind};

type Parsed<T> = Result<T, ParseError>;

/// Parse a whole selector.
pub(super) fn selector(text: &str) -> Parsed<Selector> {
    let mut cursor = Cursor { text, pos: 0 };
    cursor.trivia()?;
    if cursor.at_end() {
        return Err(cursor.error(ParseErrorKind::Empty, 0..text.len()));
    }
    let mut lists = vec![cursor.list()?];
    loop {
        cursor.trivia()?;
        match cursor.peek() {
            None => break,
            Some('+') => {
                cursor.bump();
                cursor.trivia()?;
                // A trailing `+` is allowed, as IfcOpenShell allows it.
                if cursor.at_end() {
                    break;
                }
                lists.push(cursor.list()?);
            }
            Some(found) => {
                return Err(cursor.unexpected(found, "`,`, `+` or the end"));
            }
        }
    }
    Ok(Selector { lists })
}

/// Parse a whole value path.
pub(super) fn value_path(text: &str) -> Parsed<ValuePath> {
    let mut cursor = Cursor { text, pos: 0 };
    cursor.trivia()?;
    if cursor.at_end() {
        return Err(cursor.error(ParseErrorKind::Empty, 0..text.len()));
    }
    let path = cursor.path()?;
    cursor.trivia()?;
    match cursor.peek() {
        None => Ok(path),
        Some(found) => Err(cursor.unexpected(found, "`.` or the end")),
    }
}

struct Cursor<'t> {
    text: &'t str,
    pos: usize,
}

impl<'t> Cursor<'t> {
    fn rest(&self) -> &'t str {
        &self.text[self.pos..]
    }

    fn peek(&self) -> Option<char> {
        self.rest().chars().next()
    }

    fn at_end(&self) -> bool {
        self.pos >= self.text.len()
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += c.len_utf8();
        Some(c)
    }

    fn error(&self, kind: ParseErrorKind, span: Range<usize>) -> ParseError {
        ParseError { kind, span }
    }

    /// The character at the cursor is not what the grammar expects.
    fn unexpected(&self, found: char, expected: &'static str) -> ParseError {
        self.error(
            ParseErrorKind::UnexpectedChar { found, expected },
            self.pos..self.pos + found.len_utf8(),
        )
    }

    /// The input ended, or holds `found`, where `expected` was due.
    fn expected(&self, expected: &'static str) -> ParseError {
        match self.peek() {
            Some(found) => self.unexpected(found, expected),
            None => self.error(
                ParseErrorKind::UnexpectedEnd { expected },
                self.pos..self.pos,
            ),
        }
    }

    /// Skip whitespace and `/* ... */` comments.
    fn trivia(&mut self) -> Parsed<()> {
        loop {
            let rest = self.rest();
            let trimmed = rest.trim_start();
            self.pos += rest.len() - trimmed.len();
            if !self.rest().starts_with("/*") {
                return Ok(());
            }
            let start = self.pos;
            match self.rest()[2..].find("*/") {
                Some(end) => self.pos += 2 + end + 2,
                None => {
                    return Err(
                        self.error(ParseErrorKind::UnterminatedComment, start..self.text.len())
                    )
                }
            }
        }
    }

    /// A run of characters up to the next stop character.
    fn word(&mut self, in_value: bool) -> (&'t str, Range<usize>) {
        let start = self.pos;
        while let Some(c) = self.peek() {
            if is_stop(c, in_value) {
                break;
            }
            self.bump();
        }
        (&self.text[start..self.pos], start..self.pos)
    }

    fn list(&mut self) -> Parsed<FacetList> {
        let mut facets = vec![self.facet()?];
        loop {
            self.trivia()?;
            if self.peek() != Some(',') {
                return Ok(FacetList { facets });
            }
            self.bump();
            facets.push(self.facet()?);
        }
    }

    fn facet(&mut self) -> Parsed<Facet> {
        self.trivia()?;
        let start = self.pos;
        let kind = match self.peek() {
            None => return Err(self.expected("a facet")),
            Some('!') => {
                self.bump();
                self.trivia()?;
                let (word, span) = self.word(false);
                if word.is_empty() {
                    return Err(self.expected("a class or a GlobalId after `!`"));
                }
                standalone(word, span, true)?
            }
            Some('"' | '/') => {
                let set = self.key()?;
                self.property(set)?
            }
            Some(_) if self.rest().starts_with("query:") => {
                self.pos += "query:".len();
                self.trivia()?;
                let path = self.path()?;
                let (comparison, value) = self.comparison_and_value()?;
                FacetKind::Query {
                    path,
                    comparison,
                    value,
                }
            }
            Some(found) => {
                let (word, span) = self.word(false);
                if word.is_empty() {
                    return Err(self.unexpected(found, "a facet"));
                }
                self.trivia()?;
                match self.peek() {
                    Some('.') => self.property(Key::Text(word.to_owned()))?,
                    Some('=' | '!' | '<' | '>' | '*') => self.compared(word, span)?,
                    _ => standalone(word, span, false)?,
                }
            }
        };
        Ok(Facet {
            kind,
            span: start..self.pos,
        })
    }

    /// The rest of a property facet after its set key: `.` key comparison
    /// value.
    fn property(&mut self, set: Key) -> Parsed<FacetKind> {
        self.trivia()?;
        if self.peek() != Some('.') {
            return Err(self.expected("`.` and a property name"));
        }
        self.bump();
        self.trivia()?;
        let property = self.key()?;
        let (comparison, value) = self.comparison_and_value()?;
        Ok(FacetKind::Property {
            set,
            property,
            comparison,
            value,
        })
    }

    /// An attribute or keyword facet: `word` is followed by a comparison.
    fn compared(&mut self, word: &str, span: Range<usize>) -> Parsed<FacetKind> {
        let (comparison, value) = self.comparison_and_value()?;
        if let Some(keyword) = Keyword::from_word(word) {
            return Ok(FacetKind::Keyword {
                keyword,
                comparison,
                value,
            });
        }
        let mut chars = word.chars();
        let first = chars.next().unwrap_or(' ');
        if !first.is_ascii_uppercase() {
            return Err(self.error(ParseErrorKind::UnknownKeyword(word.to_owned()), span));
        }
        if !chars.all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return Err(self.error(ParseErrorKind::InvalidAttributeName(word.to_owned()), span));
        }
        Ok(FacetKind::Attribute {
            name: word.to_owned(),
            comparison,
            value,
        })
    }

    fn comparison_and_value(&mut self) -> Parsed<(Comparison, Literal)> {
        self.trivia()?;
        let start = self.pos;
        let comparison = self.comparison()?;
        self.trivia()?;
        let (value, end) = self.value()?;
        let restricted = matches!(
            value,
            Literal::Null | Literal::True | Literal::False | Literal::Regex(_)
        );
        if restricted && comparison.operator != Operator::Equal {
            return Err(self.error(
                ParseErrorKind::OperatorNotApplicable {
                    operator: comparison.to_string(),
                    value: value.to_string(),
                },
                start..end,
            ));
        }
        Ok((comparison, value))
    }

    fn comparison(&mut self) -> Parsed<Comparison> {
        const EXPECTED: &str = "a comparison (=, !=, *=, !*=, >, >=, <, <=)";
        let negated = self.peek() == Some('!');
        if negated {
            self.bump();
            self.trivia()?;
        }
        let rest = self.rest();
        let (operator, width) = if rest.starts_with("*=") {
            (Operator::Contains, 2)
        } else if rest.starts_with(">=") {
            (Operator::GreaterOrEqual, 2)
        } else if rest.starts_with("<=") {
            (Operator::LessOrEqual, 2)
        } else if rest.starts_with('=') {
            (Operator::Equal, 1)
        } else if rest.starts_with('>') {
            (Operator::Greater, 1)
        } else if rest.starts_with('<') {
            (Operator::Less, 1)
        } else {
            return Err(self.expected(EXPECTED));
        };
        if negated && !matches!(operator, Operator::Equal | Operator::Contains) {
            return Err(self.expected("`=` or `*=` after `!`"));
        }
        self.pos += width;
        Ok(Comparison { negated, operator })
    }

    /// A value and the byte offset where it ends.
    fn value(&mut self) -> Parsed<(Literal, usize)> {
        let value = match self.peek() {
            None => return Err(self.expected("a value")),
            Some('"') => Literal::Text(self.quoted()?),
            Some('/') => Literal::Regex(self.pattern()?),
            Some(found) => {
                let (word, _) = self.word(true);
                match word {
                    "" => return Err(self.unexpected(found, "a value")),
                    "NULL" => Literal::Null,
                    "TRUE" => Literal::True,
                    "FALSE" => Literal::False,
                    text => Literal::Text(text.to_owned()),
                }
            }
        };
        Ok((value, self.pos))
    }

    fn key(&mut self) -> Parsed<Key> {
        match self.peek() {
            Some('"') => Ok(Key::Text(self.quoted()?)),
            Some('/') => Ok(Key::Regex(self.pattern()?)),
            Some(found) => {
                let (word, _) = self.word(false);
                if word.is_empty() {
                    Err(self.unexpected(found, "a name, a \"quoted name\" or a /pattern/"))
                } else {
                    Ok(Key::Text(word.to_owned()))
                }
            }
            None => Err(self.expected("a name")),
        }
    }

    fn path(&mut self) -> Parsed<ValuePath> {
        let mut keys = Vec::new();
        loop {
            let start = self.pos;
            let key = self.key()?;
            keys.push(PathKey {
                key,
                span: start..self.pos,
            });
            self.trivia()?;
            if self.peek() != Some('.') {
                return Ok(ValuePath { keys });
            }
            self.bump();
            self.trivia()?;
        }
    }

    /// A `"..."` string; `\"` and `\\` are escapes, any other backslash is
    /// kept as written.
    fn quoted(&mut self) -> Parsed<String> {
        let start = self.pos;
        self.bump();
        let mut out = String::new();
        loop {
            match self.bump() {
                None => {
                    return Err(
                        self.error(ParseErrorKind::UnterminatedString, start..self.text.len())
                    )
                }
                Some('"') => return Ok(out),
                Some('\\') => match self.peek() {
                    Some(c @ ('"' | '\\')) => {
                        self.bump();
                        out.push(c);
                    }
                    _ => out.push('\\'),
                },
                Some(c) => out.push(c),
            }
        }
    }

    /// A `/.../` pattern; it cannot contain `/`.
    fn pattern(&mut self) -> Parsed<Pattern> {
        let start = self.pos;
        self.bump();
        let Some(length) = self.rest().find('/') else {
            return Err(self.error(ParseErrorKind::UnterminatedPattern, start..self.text.len()));
        };
        let source = &self.rest()[..length];
        self.pos += length + 1;
        if source.is_empty() {
            return Err(self.error(
                ParseErrorKind::InvalidPattern("an empty pattern".to_owned()),
                start..self.pos,
            ));
        }
        Pattern::new(source)
            .map_err(|detail| self.error(ParseErrorKind::InvalidPattern(detail), start..self.pos))
    }
}

/// A word standing alone: a `GlobalId` or a class.
fn standalone(word: &str, span: Range<usize>, negated: bool) -> Parsed<FacetKind> {
    if let Some(global_id) = Guid::parse(word) {
        return Ok(FacetKind::Instance { negated, global_id });
    }
    let is_class = word.len() > 3
        && word
            .get(..3)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("ifc"))
        && word.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
    if is_class {
        return Ok(FacetKind::Class {
            negated,
            name: word.to_owned(),
        });
    }
    Err(ParseError {
        kind: ParseErrorKind::ExpectedClassOrGlobalId(word.to_owned()),
        span,
    })
}
