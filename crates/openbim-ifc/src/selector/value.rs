//! The values a path reads and a facet compares, and the comparison.
//!
//! A comparison against a list holds when it holds for any item, and its
//! negation (`!=`, `!*=`) is the complement of that: `!=` means "no item
//! equals". Text compares with `=` and `*=`; numbers with every operator,
//! the value parsed as a number; `NULL`, `TRUE` and `FALSE` with `=` and
//! `!=` only; a pattern matches text from its start.

use ifc_model::{EntityId, Value};

use super::ast::{Comparison, Literal, Operator};

/// A value read by a [`ValuePath`](super::ValuePath) or compared by a
/// facet.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum SelectedValue {
    /// No value: `$`, an absent property, a relation the entity lacks.
    Null,
    /// `*`: a derived attribute, which holds no value of its own.
    Derived,
    /// A boolean, or a logical's `.T.` / `.F.`.
    Bool(bool),
    /// A logical's `.U.`.
    Unknown,
    /// An integer.
    Integer(i64),
    /// A real.
    Real(f64),
    /// A string.
    Text(String),
    /// A binary literal's digits.
    Binary(String),
    /// An enumeration item, without dots.
    Enum(String),
    /// An entity of the model.
    Entity(EntityId),
    /// Several values: an aggregate, or a key applied to each item of one.
    List(Vec<SelectedValue>),
    /// A property set (or a complex property, or a bounded value's
    /// bounds): each property's name and value, in set order.
    Properties(Vec<(String, SelectedValue)>),
}

impl SelectedValue {
    /// The value of a model attribute; a typed wrapper reads as the value
    /// it wraps.
    #[must_use]
    pub fn from_model(value: &Value) -> Self {
        match value {
            Value::Null => Self::Null,
            Value::Derived => Self::Derived,
            Value::Bool(b) => Self::Bool(*b),
            Value::LogicalUnknown => Self::Unknown,
            Value::Integer(i) => Self::Integer(*i),
            Value::Real(r) => Self::Real(*r),
            Value::Text(text) => Self::Text(text.to_string()),
            Value::Binary(digits) => Self::Binary(digits.to_string()),
            Value::Enum(item) => Self::Enum(item.to_string()),
            Value::Ref(id) => Self::Entity(*id),
            Value::List(items) => Self::List(items.iter().map(Self::from_model).collect()),
            Value::Typed { value, .. } => Self::from_model(value),
        }
    }

    /// The text a pattern or a text comparison reads: a string or an
    /// enumeration item.
    fn text(&self) -> Option<&str> {
        match self {
            Self::Text(text) | Self::Enum(text) => Some(text),
            _ => None,
        }
    }

    fn number(&self) -> Option<f64> {
        match self {
            // Integers beyond 2^53 lose precision here, as any comparison
            // against a decimal literal would.
            #[allow(clippy::cast_precision_loss)]
            Self::Integer(i) => Some(*i as f64),
            Self::Real(r) => Some(*r),
            _ => None,
        }
    }
}

/// Whether `value` satisfies `comparison` against `literal`.
#[must_use]
pub fn compare(value: &SelectedValue, comparison: Comparison, literal: &Literal) -> bool {
    let held = holds(value, comparison.operator, literal);
    held != comparison.negated
}

/// The comparison without its negation; a list holds when an item does.
fn holds(value: &SelectedValue, operator: Operator, literal: &Literal) -> bool {
    if let SelectedValue::List(items) = value {
        return items.iter().any(|item| holds(item, operator, literal));
    }
    match literal {
        Literal::Null => matches!(value, SelectedValue::Null),
        Literal::True => matches!(value, SelectedValue::Bool(true)),
        Literal::False => matches!(value, SelectedValue::Bool(false)),
        Literal::Regex(pattern) => value.text().is_some_and(|text| pattern.is_match(text)),
        Literal::Text(expected) => {
            if let Some(text) = value.text() {
                return match operator {
                    Operator::Equal => text == expected,
                    Operator::Contains => text.contains(expected.as_str()),
                    _ => false,
                };
            }
            let (Some(number), Ok(wanted)) = (value.number(), expected.trim().parse::<f64>())
            else {
                return false;
            };
            match operator {
                // Exact, as the file states it and IfcOpenShell compares.
                #[allow(clippy::float_cmp)]
                Operator::Equal => number == wanted,
                Operator::Contains => false,
                Operator::Greater => number > wanted,
                Operator::GreaterOrEqual => number >= wanted,
                Operator::Less => number < wanted,
                Operator::LessOrEqual => number <= wanted,
            }
        }
    }
}
