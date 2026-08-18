//! The IFC value model — serialization-independent.
//!
//! # Why this lives in `ifc-model`, not in a codec
//!
//! IFC data arrives as STEP/SPF, ifcXML, and prospectively IFC-JSON. If the
//! value type belonged to the STEP codec, every other codec would need its own
//! parallel value type and cross-format conversion would be lossy by
//! construction. The data model owns the values; codecs only translate.
//!
//! # Owned, not borrowed
//!
//! Values own their data rather than borrowing from an mmap. A model must
//! outlive the bytes it was read from — otherwise it cannot be mutated and
//! written back, which is the entire point of round-tripping.

use std::sync::Arc;

/// One attribute slot in an entity.
///
/// This is deliberately a *structural* representation: it records what the file
/// said, not what it means. Interpretation is the job of the domain crates.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// `$` — the attribute is not set.
    Null,
    /// `*` — inherited/derived in a supertype; distinct from `$`.
    Derived,
    /// `.T.` / `.F.`
    Bool(bool),
    /// `.U.` — logical unknown, the third STEP boolean state.
    LogicalUnknown,
    /// An integer literal.
    Integer(i64),
    /// A real literal.
    Real(f64),
    /// A quoted string, already unescaped to UTF-8.
    Text(Arc<str>),
    /// A binary literal (`"0123ABC"`).
    Binary(Arc<str>),
    /// An unquoted enumeration constant such as `.ELEMENT.`
    Enum(Arc<str>),
    /// A reference to another entity by its in-file id (`#42`).
    Ref(EntityId),
    /// An ordered aggregate: list, set, array or bag.
    List(Vec<Value>),
    /// A typed wrapper such as `IFCLENGTHMEASURE(2.5)`.
    Typed {
        /// The declared type name, upper-cased as written.
        type_name: Arc<str>,
        /// The wrapped value.
        value: Box<Value>,
    },
}

/// An entity's identifier as it appeared in the file (`#42` → `42`).
///
/// Preserved verbatim so that a re-exported file keeps its original numbering;
/// stable ids make diffs between two exports readable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EntityId(pub u64);

impl std::fmt::Display for EntityId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{}", self.0)
    }
}

impl Value {
    /// The referenced entity, if this value is a reference.
    pub fn as_ref_id(&self) -> Option<EntityId> {
        match self {
            Value::Ref(id) => Some(*id),
            _ => None,
        }
    }

    /// The text content, if this is a string.
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Value::Text(s) => Some(s),
            _ => None,
        }
    }

    /// The numeric value, accepting either integer or real.
    ///
    /// IFC files are inconsistent about writing `1` versus `1.0` for the same
    /// attribute, so a consumer that insists on one variant will misread real
    /// files.
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Real(r) => Some(*r),
            Value::Integer(i) => Some(*i as f64),
            _ => None,
        }
    }

    /// The items, if this is an aggregate.
    pub fn as_list(&self) -> Option<&[Value]> {
        match self {
            Value::List(items) => Some(items),
            _ => None,
        }
    }

    /// Unwrap a [`Value::Typed`] to the value inside, otherwise `self`.
    ///
    /// Callers almost always want the payload; the type name matters only to
    /// validation and to writers.
    pub fn unwrap_typed(&self) -> &Value {
        match self {
            Value::Typed { value, .. } => value.unwrap_typed(),
            other => other,
        }
    }

    /// Visit every entity reference reachable from this value.
    ///
    /// References nest arbitrarily deep inside aggregates, so link rewriting
    /// and integrity checks need a recursive walk rather than a shallow scan.
    pub fn for_each_ref(&self, f: &mut impl FnMut(EntityId)) {
        match self {
            Value::Ref(id) => f(*id),
            Value::List(items) => items.iter().for_each(|v| v.for_each_ref(f)),
            Value::Typed { value, .. } => value.for_each_ref(f),
            _ => {}
        }
    }
}
