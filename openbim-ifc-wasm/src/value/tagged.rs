//! Host-independent form of the tagged value encoding.

use ifc::{EntityId, Value};

use super::MAX_NESTING;
use crate::BindingError;

/// The `kind` tag of an encoded value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// `$`.
    Null,
    /// `*`.
    Derived,
    /// `.T.` / `.F.`.
    Bool,
    /// `.U.`.
    Unknown,
    /// An integer literal.
    Integer,
    /// A real literal.
    Real,
    /// A quoted string.
    Text,
    /// A binary literal.
    Binary,
    /// An enumeration constant.
    Enum,
    /// An entity reference.
    Ref,
    /// An aggregate.
    List,
    /// A typed wrapper.
    Typed,
}

impl Kind {
    /// The `kind` string used on the JavaScript side.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Null => "null",
            Self::Derived => "derived",
            Self::Bool => "bool",
            Self::Unknown => "unknown",
            Self::Integer => "integer",
            Self::Real => "real",
            Self::Text => "text",
            Self::Binary => "binary",
            Self::Enum => "enum",
            Self::Ref => "ref",
            Self::List => "list",
            Self::Typed => "typed",
        }
    }

    /// Parse a `kind` string; unknown kinds are an error, never a guess.
    pub fn parse(kind: &str) -> Result<Self, BindingError> {
        Ok(match kind {
            "null" => Self::Null,
            "derived" => Self::Derived,
            "bool" => Self::Bool,
            "unknown" => Self::Unknown,
            "integer" => Self::Integer,
            "real" => Self::Real,
            "text" => Self::Text,
            "binary" => Self::Binary,
            "enum" => Self::Enum,
            "ref" => Self::Ref,
            "list" => Self::List,
            "typed" => Self::Typed,
            other => {
                return Err(BindingError::InvalidValue(format!(
                    "unknown kind {other:?}"
                )))
            }
        })
    }
}

/// One encoded value: a kind plus exactly the payload that kind carries.
///
/// An enum rather than a struct with optional fields, so a `ref` without an
/// id or a `typed` without a type name cannot be represented.
#[derive(Debug, Clone, PartialEq)]
pub enum Tagged {
    /// `$`.
    Null,
    /// `*`.
    Derived,
    /// `.T.` / `.F.`.
    Bool(bool),
    /// `.U.`.
    Unknown,
    /// A 64-bit integer.
    Integer(i64),
    /// A real number.
    Real(f64),
    /// A decoded string.
    Text(String),
    /// Binary literal digits.
    Binary(String),
    /// An enumeration constant, without dots.
    Enum(String),
    /// A reference to entity `#id`.
    Ref(u64),
    /// An aggregate.
    List(Vec<Tagged>),
    /// A typed wrapper such as `IFCLABEL('x')`.
    Typed {
        /// The wrapper's type name.
        type_name: String,
        /// The wrapped value.
        value: Box<Tagged>,
    },
}

impl Tagged {
    /// This value's kind.
    pub const fn kind(&self) -> Kind {
        match self {
            Self::Null => Kind::Null,
            Self::Derived => Kind::Derived,
            Self::Bool(_) => Kind::Bool,
            Self::Unknown => Kind::Unknown,
            Self::Integer(_) => Kind::Integer,
            Self::Real(_) => Kind::Real,
            Self::Text(_) => Kind::Text,
            Self::Binary(_) => Kind::Binary,
            Self::Enum(_) => Kind::Enum,
            Self::Ref(_) => Kind::Ref,
            Self::List(_) => Kind::List,
            Self::Typed { .. } => Kind::Typed,
        }
    }

    /// Encode a model value. Total: every value has exactly one encoding.
    pub fn from_value(value: &Value) -> Self {
        match value {
            Value::Null => Self::Null,
            Value::Derived => Self::Derived,
            Value::Bool(b) => Self::Bool(*b),
            Value::LogicalUnknown => Self::Unknown,
            Value::Integer(i) => Self::Integer(*i),
            Value::Real(r) => Self::Real(*r),
            Value::Text(s) => Self::Text(s.to_string()),
            Value::Binary(s) => Self::Binary(s.to_string()),
            Value::Enum(s) => Self::Enum(s.to_string()),
            Value::Ref(EntityId(id)) => Self::Ref(*id),
            Value::List(items) => Self::List(items.iter().map(Self::from_value).collect()),
            Value::Typed { type_name, value } => Self::Typed {
                type_name: type_name.to_string(),
                value: Box::new(Self::from_value(value)),
            },
        }
    }

    /// Decode into a model value, validating what the model cannot.
    ///
    /// Rejects nesting deeper than [`MAX_NESTING`], an empty enum or type
    /// name, and an enum or type name that is not a STEP identifier -- each
    /// of which the STEP writer would otherwise emit as a malformed file.
    pub fn into_value(self) -> Result<Value, BindingError> {
        self.into_value_at(0)
    }

    fn into_value_at(self, depth: usize) -> Result<Value, BindingError> {
        if depth > MAX_NESTING {
            return Err(BindingError::InvalidValue(format!(
                "nesting deeper than {MAX_NESTING}"
            )));
        }
        Ok(match self {
            Self::Null => Value::Null,
            Self::Derived => Value::Derived,
            Self::Bool(b) => Value::Bool(b),
            Self::Unknown => Value::LogicalUnknown,
            Self::Integer(i) => Value::Integer(i),
            Self::Real(r) => Value::Real(r),
            Self::Text(s) => Value::Text(s.into()),
            Self::Binary(s) => Value::Binary(s.into()),
            Self::Enum(s) => Value::Enum(identifier("enum", s)?.into()),
            Self::Ref(id) => Value::Ref(EntityId(id)),
            Self::List(items) => Value::List(
                items
                    .into_iter()
                    .map(|item| item.into_value_at(depth + 1))
                    .collect::<Result<_, _>>()?,
            ),
            Self::Typed { type_name, value } => Value::Typed {
                type_name: identifier("typed", type_name)?.to_ascii_uppercase().into(),
                value: Box::new(value.into_value_at(depth + 1)?),
            },
        })
    }
}

/// A STEP identifier: a letter, then letters, digits or underscores.
fn identifier(kind: &str, name: String) -> Result<String, BindingError> {
    let mut chars = name.chars();
    let valid = chars.next().is_some_and(|c| c.is_ascii_alphabetic())
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_');
    if valid {
        Ok(name)
    } else {
        Err(BindingError::InvalidValue(format!(
            "{kind} name {name:?} is not a STEP identifier"
        )))
    }
}
