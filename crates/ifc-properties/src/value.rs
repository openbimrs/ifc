//! `IfcValue` measure types and their interpretation.
//!
//! # The measure name is data, not decoration
//!
//! IFC writes property values as typed wrappers: `IFCLENGTHMEASURE(2.5)`,
//! `IFCCOUNTMEASURE(2.)`, `IFCTHERMALTRANSMITTANCEMEASURE(0.24)`. All three
//! are a STEP real. Discarding the wrapper and keeping `2.5` loses the only
//! statement the file makes about what the number MEANS -- and 2.5 metres,
//! 2.5 items and 2.5 W/m2K are not interchangeable.
//!
//! So a measure keeps its declared type name and exposes the scalar
//! separately. `Model::text_at`/`f64_at` already unwrap typed values, which is
//! right for plain attributes and wrong here.
//!
//! # Untyped values are legal
//!
//! `IfcValue` is a SELECT, and exporters do write bare literals where a
//! measure was expected. That is not an error to reject: it is a value whose
//! measure is unstated, and `measure: None` says exactly that.

use std::sync::Arc;

use ifc_model::Value;

/// The scalar payload of a property value.
///
/// Mirrors the STEP literal forms `IfcValue` can bottom out in. `Real` and
/// `Integer` stay distinct because `IfcCountMeasure` is an integer count and
/// silently widening it to `f64` invites `2.9999999` counts.
#[derive(Debug, Clone, PartialEq)]
pub enum Scalar {
    /// `.T.` / `.F.`
    Bool(bool),
    /// `.U.` -- the third STEP boolean state, distinct from absent.
    LogicalUnknown,
    /// An integer literal.
    Integer(i64),
    /// A real literal.
    Real(f64),
    /// A quoted string.
    Text(Arc<str>),
    /// An unquoted enumeration constant.
    Enum(Arc<str>),
    /// A binary literal.
    Binary(Arc<str>),
}

impl Scalar {
    /// The numeric value, when the scalar is a number.
    ///
    /// Integers widen to `f64` here because a caller asking for a number has
    /// already accepted floating point. The distinction is preserved in the
    /// enum for callers that care.
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Real(v) => Some(*v),
            Self::Integer(v) => Some(*v as f64),
            _ => None,
        }
    }

    /// The text, when the scalar is text or an enumeration constant.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::Text(v) | Self::Enum(v) => Some(v),
            _ => None,
        }
    }
}

/// A property value together with the measure type the file declared.
#[derive(Debug, Clone, PartialEq)]
pub struct MeasureValue {
    /// The declared measure, upper-cased as written, e.g. `IFCLENGTHMEASURE`.
    ///
    /// `None` when the file wrote a bare literal. Legal, and materially
    /// different from a stated measure: nothing can be converted or compared
    /// dimensionally without it.
    pub measure: Option<Arc<str>>,
    /// The underlying scalar.
    pub scalar: Scalar,
}

impl MeasureValue {
    /// Read a value, keeping its measure wrapper.
    ///
    /// Returns `None` for `$` (absent) and for aggregates: `IfcValue` is a
    /// SELECT of single measures, so a list where a measure belongs is
    /// malformed rather than a value this type can represent.
    pub fn read(value: &Value) -> Option<Self> {
        Self::read_inner(value, None)
    }

    fn read_inner(value: &Value, measure: Option<Arc<str>>) -> Option<Self> {
        let scalar = match value {
            // Nested wrappers do occur; the OUTERMOST name is the declared
            // measure, so an inner one must not overwrite it.
            Value::Typed { type_name, value } => {
                let name = measure.unwrap_or_else(|| type_name.clone());
                return Self::read_inner(value, Some(name));
            }
            Value::Bool(v) => Scalar::Bool(*v),
            Value::LogicalUnknown => Scalar::LogicalUnknown,
            Value::Integer(v) => Scalar::Integer(*v),
            Value::Real(v) => Scalar::Real(*v),
            Value::Text(v) => Scalar::Text(v.clone()),
            Value::Enum(v) => Scalar::Enum(v.clone()),
            Value::Binary(v) => Scalar::Binary(v.clone()),
            // `$`, `*`, references and aggregates are not single measures.
            Value::Null | Value::Derived | Value::Ref(_) | Value::List(_) => return None,
        };
        Some(Self { measure, scalar })
    }

    /// Read a list of values, e.g. `IfcPropertyListValue.ListValues`.
    ///
    /// A non-list yields `None`; an element that is not a measure is skipped
    /// rather than dropping the whole list, because one malformed entry
    /// should not hide the rest.
    pub fn read_list(value: &Value) -> Option<Vec<Self>> {
        match value {
            Value::List(items) => Some(items.iter().filter_map(Self::read).collect()),
            _ => None,
        }
    }

    /// The numeric value, when there is one.
    pub fn as_f64(&self) -> Option<f64> {
        self.scalar.as_f64()
    }

    /// The declared measure, when the file stated one.
    pub fn measure(&self) -> Option<&str> {
        self.measure.as_deref()
    }
}
