//! The typed query a selector parses into, and its canonical text.
//!
//! [`Display`](fmt::Display) writes the canonical form of every node:
//! parsing it gives back the same nodes (their byte spans aside) and the
//! same text, so a query can be stored, compared and shown in the form
//! this library reads.

use std::fmt;
use std::ops::Range;

use ifc_model::guid::Guid;

use super::parse;
use super::ParseError;

/// A parsed selector: facet lists joined by `+`, their results united.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Selector {
    /// The facet lists, in source order; never empty.
    pub lists: Vec<FacetList>,
}

impl Selector {
    /// Parse `text` against grammar [`GRAMMAR_VERSION`](super::GRAMMAR_VERSION).
    ///
    /// # Errors
    ///
    /// A [`ParseError`] with the byte span of the offending text.
    pub fn parse(text: &str) -> Result<Self, ParseError> {
        parse::selector(text)
    }
}

impl std::str::FromStr for Selector {
    type Err = ParseError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::parse(text)
    }
}

/// Facets joined by `,`, applied left to right to one working set.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct FacetList {
    /// The facets, in source order; never empty.
    pub facets: Vec<Facet>,
}

/// One facet and where it was written.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Facet {
    /// What the facet selects.
    pub kind: FacetKind,
    /// Its byte range in the parsed text.
    pub span: Range<usize>,
}

/// What a facet selects.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum FacetKind {
    /// One entity by `GlobalId`; added to the working set, or removed with
    /// `!`.
    Instance {
        /// Whether the facet was written `!`.
        negated: bool,
        /// The `GlobalId`.
        global_id: Guid,
    },
    /// Every entity of a class, subtypes included; added to the working
    /// set, or removed with `!`.
    Class {
        /// Whether the facet was written `!`.
        negated: bool,
        /// The class as written (`IfcWall`); resolved against the
        /// release when the selector is evaluated.
        name: String,
    },
    /// An explicit attribute by name (`Name=Wall`), filtering the working
    /// set.
    Attribute {
        /// The attribute name as written.
        name: String,
        /// The comparison.
        comparison: Comparison,
        /// The value compared against.
        value: Literal,
    },
    /// A property or quantity (`Pset_WallCommon.IsExternal=TRUE`),
    /// filtering the working set.
    Property {
        /// The property or quantity set name, or a pattern.
        set: Key,
        /// The property or quantity name, or a pattern.
        property: Key,
        /// The comparison.
        comparison: Comparison,
        /// The value compared against.
        value: Literal,
    },
    /// A relationship keyword (`type=WT-200`), filtering the working set.
    Keyword {
        /// Which relationship.
        keyword: Keyword,
        /// The comparison.
        comparison: Comparison,
        /// The value compared against.
        value: Literal,
    },
    /// A value path (`query:type.Name=WT-200`), filtering the working set.
    Query {
        /// The path, as [`get_value`](super::get_value) reads it.
        path: ValuePath,
        /// The comparison.
        comparison: Comparison,
        /// The value compared against.
        value: Literal,
    },
}

/// The relationship a keyword facet reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Keyword {
    /// `type`: the type object's `Name` or `GlobalId`.
    Type,
    /// `material`: each material's `Name` or `Category`.
    Material,
    /// `classification`: each classification reference's code or `Name`.
    Classification,
    /// `location` (alias `container`): the `Name` or `GlobalId` of each
    /// spatial container or aggregate above the entity.
    Location,
    /// `group`: each group's `Name` or `GlobalId`.
    Group,
}

impl Keyword {
    /// The keyword as the canonical text writes it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Type => "type",
            Self::Material => "material",
            Self::Classification => "classification",
            Self::Location => "location",
            Self::Group => "group",
        }
    }

    /// The keyword `word` names, aliases included.
    #[must_use]
    pub fn from_word(word: &str) -> Option<Self> {
        Some(match word {
            "type" => Self::Type,
            "material" => Self::Material,
            "classification" => Self::Classification,
            "location" | "container" => Self::Location,
            "group" => Self::Group,
            _ => return None,
        })
    }
}

/// A comparison operator, possibly negated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct Comparison {
    /// Whether the operator was written with `!` (`!=`, `!*=`). Only
    /// [`Operator::Equal`] and [`Operator::Contains`] take one.
    pub negated: bool,
    /// The operator.
    pub operator: Operator,
}

impl Comparison {
    /// A comparison of `operator`, negated or not.
    #[must_use]
    pub const fn new(negated: bool, operator: Operator) -> Self {
        Self { negated, operator }
    }
}

/// A comparison operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Operator {
    /// `=`.
    Equal,
    /// `*=`: the text contains the value.
    Contains,
    /// `>`.
    Greater,
    /// `>=`.
    GreaterOrEqual,
    /// `<`.
    Less,
    /// `<=`.
    LessOrEqual,
}

impl Operator {
    /// The operator as written, without negation.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Equal => "=",
            Self::Contains => "*=",
            Self::Greater => ">",
            Self::GreaterOrEqual => ">=",
            Self::Less => "<",
            Self::LessOrEqual => "<=",
        }
    }
}

/// The value side of a comparison.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Literal {
    /// `NULL`: no value.
    Null,
    /// `TRUE`.
    True,
    /// `FALSE`.
    False,
    /// Text, quoted or not; numbers are text until compared.
    Text(String),
    /// `/pattern/`: matched from the start of a text value.
    Regex(Pattern),
}

/// One name in a property facet or a value path: exact text or a pattern.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Key {
    /// The exact name.
    Text(String),
    /// `/pattern/`: names it matches from their start.
    Regex(Pattern),
}

impl Key {
    /// Whether this key names `name`.
    #[must_use]
    pub fn matches(&self, name: &str) -> bool {
        match self {
            Self::Text(text) => text == name,
            Self::Regex(pattern) => pattern.is_match(name),
        }
    }
}

/// A regular expression, kept with its source text.
///
/// The syntax is that of the `regex-lite` crate (Perl-like, no Unicode
/// classes). A pattern matches a text when it matches a prefix of it, as
/// Python's `re.match` does: `/Pset_/` matches every `Pset_` name and
/// `/Pset_.*Common$/` only the common sets.
#[derive(Clone)]
pub struct Pattern {
    source: String,
    anchored: regex_lite::Regex,
}

impl Pattern {
    /// Compile `source`, anchored at the start.
    ///
    /// # Errors
    ///
    /// The compiler's message when `source` is no valid pattern.
    pub fn new(source: &str) -> Result<Self, String> {
        let anchored =
            regex_lite::Regex::new(&format!("^(?:{source})")).map_err(|error| error.to_string())?;
        Ok(Self {
            source: source.to_owned(),
            anchored,
        })
    }

    /// The pattern as written between the slashes.
    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Whether the pattern matches a prefix of `text`.
    #[must_use]
    pub fn is_match(&self, text: &str) -> bool {
        self.anchored.is_match(text)
    }
}

impl PartialEq for Pattern {
    fn eq(&self, other: &Self) -> bool {
        self.source == other.source
    }
}

impl fmt::Debug for Pattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Pattern(/{}/)", self.source)
    }
}

/// A path of keys from an entity to a value: `Name`, `type.Name`,
/// `Pset_WallCommon.FireRating`, `materials.Name`.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct ValuePath {
    /// The keys, in order; never empty.
    pub keys: Vec<PathKey>,
}

impl ValuePath {
    /// Parse `text` as a value path.
    ///
    /// # Errors
    ///
    /// A [`ParseError`] with the byte span of the offending text.
    pub fn parse(text: &str) -> Result<Self, ParseError> {
        parse::value_path(text)
    }
}

impl std::str::FromStr for ValuePath {
    type Err = ParseError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::parse(text)
    }
}

/// One key of a [`ValuePath`] and where it was written.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct PathKey {
    /// The key.
    pub key: Key,
    /// Its byte range in the parsed text.
    pub span: Range<usize>,
}

// --- canonical text -------------------------------------------------------

/// Characters that end an unquoted key or value.
pub(super) fn is_stop(c: char, in_value: bool) -> bool {
    c.is_whitespace()
        || matches!(c, ',' | '+' | '"' | '=' | '<' | '>' | '!' | '*')
        || (!in_value && matches!(c, '.' | '/'))
}

fn write_text(f: &mut fmt::Formatter<'_>, text: &str, in_value: bool) -> fmt::Result {
    // A reserved word as a value, and a key that would read as the start
    // of a query facet, are quoted so that they parse back as text.
    let reserved = if in_value {
        matches!(text, "NULL" | "TRUE" | "FALSE")
    } else {
        text.starts_with("query:")
    };
    let bare = !text.is_empty()
        && !reserved
        && !text.starts_with('/')
        && !text.chars().any(|c| is_stop(c, in_value));
    if bare {
        return f.write_str(text);
    }
    f.write_str("\"")?;
    for c in text.chars() {
        if matches!(c, '"' | '\\') {
            f.write_str("\\")?;
        }
        write!(f, "{c}")?;
    }
    f.write_str("\"")
}

impl fmt::Display for Selector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, list) in self.lists.iter().enumerate() {
            if index > 0 {
                f.write_str(" + ")?;
            }
            write!(f, "{list}")?;
        }
        Ok(())
    }
}

impl fmt::Display for FacetList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, facet) in self.facets.iter().enumerate() {
            if index > 0 {
                f.write_str(", ")?;
            }
            write!(f, "{}", facet.kind)?;
        }
        Ok(())
    }
}

impl fmt::Display for FacetKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let bang = |negated: bool| if negated { "!" } else { "" };
        match self {
            Self::Instance { negated, global_id } => write!(f, "{}{global_id}", bang(*negated)),
            Self::Class { negated, name } => write!(f, "{}{name}", bang(*negated)),
            Self::Attribute {
                name,
                comparison,
                value,
            } => write!(f, "{name}{comparison}{value}"),
            Self::Property {
                set,
                property,
                comparison,
                value,
            } => write!(f, "{set}.{property}{comparison}{value}"),
            Self::Keyword {
                keyword,
                comparison,
                value,
            } => write!(f, "{}{comparison}{value}", keyword.as_str()),
            Self::Query {
                path,
                comparison,
                value,
            } => write!(f, "query:{path}{comparison}{value}"),
        }
    }
}

impl fmt::Display for Comparison {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.negated {
            f.write_str("!")?;
        }
        f.write_str(self.operator.as_str())
    }
}

impl fmt::Display for Literal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Null => f.write_str("NULL"),
            Self::True => f.write_str("TRUE"),
            Self::False => f.write_str("FALSE"),
            Self::Text(text) => write_text(f, text, true),
            Self::Regex(pattern) => write!(f, "/{}/", pattern.source),
        }
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Text(text) => write_text(f, text, false),
            Self::Regex(pattern) => write!(f, "/{}/", pattern.source),
        }
    }
}

impl fmt::Display for ValuePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, key) in self.keys.iter().enumerate() {
            if index > 0 {
                f.write_str(".")?;
            }
            write!(f, "{}", key.key)?;
        }
        Ok(())
    }
}
