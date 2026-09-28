//! Public shapes of exact values that are more than one scalar (#150), and
//! of entity-valued attributes (#149).
//!
//! Every scalar inside them is an [`ExactTypedValue`]: its value together
//! with the IFC type the file declared for it, since a list, a bound or a
//! table cell is an `IfcValue` select member and carries its own type.

use std::sync::Arc;

use ifc_model::EntityId;

use super::ExactValue;

/// One present `IfcValue` with its declared type.
#[derive(Debug, Clone, PartialEq)]
pub struct ExactTypedValue {
    /// Declared IFC type as written, for example `IFCLABEL` or
    /// `IFCLENGTHMEASURE`.
    pub value_type: Arc<str>,
    /// The scalar payload; never [`ExactValue::Null`] and never a
    /// composite value.
    pub value: ExactValue,
}

/// A referenced entity, identified exactly and not followed further.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExactEntityRef {
    /// The referenced entity's id.
    pub id: EntityId,
    /// Its IFC type name as written in the file (upper-case).
    pub type_name: Arc<str>,
}

/// An `IfcPropertyEnumeratedValue`: the selected values and the
/// enumeration they are selected from.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct ExactEnumeratedValue {
    /// `EnumerationValues`, in file order. Empty only when the attribute is
    /// `$`, which IFC4 and IFC4X3 allow (no value selected); IFC2X3
    /// requires it, and a present list is never empty (`LIST [1:?]`).
    pub values: Vec<ExactTypedValue>,
    /// `EnumerationReference`, when stated. Every selected value is one of
    /// its values (the release's WHERE rule is checked).
    pub enumeration: Option<ExactEnumeration>,
}

/// An `IfcPropertyEnumeration`: the permitted values of an enumerated
/// property.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct ExactEnumeration {
    /// Entity id of the `IfcPropertyEnumeration`.
    pub id: EntityId,
    /// Its `Name`.
    pub name: Arc<str>,
    /// Its `EnumerationValues`: nonempty, unique, of one declared type.
    pub values: Vec<ExactTypedValue>,
}

/// An `IfcPropertyBoundedValue`: a range, and in IFC4/IFC4X3 a set point.
///
/// The shared `Unit` is [`ExactProperty::unit_id`](super::ExactProperty::unit_id).
/// All stated values have one declared type (the release's WHERE rules are
/// checked).
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct ExactBoundedValue {
    /// `LowerBoundValue`, when stated.
    pub lower: Option<ExactTypedValue>,
    /// `UpperBoundValue`, when stated.
    pub upper: Option<ExactTypedValue>,
    /// `SetPointValue`, when stated. Always `None` in IFC2X3, which does not
    /// declare the attribute.
    pub set_point: Option<ExactTypedValue>,
}

/// An `IfcPropertyTableValue`: rows mapping a defining to a defined value.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct ExactTableValue {
    /// `DefiningValues` zipped with `DefinedValues`, in file order. The two
    /// lists have equal length, each of one declared type, and the defining
    /// values are unique. Empty only when both are `$` (IFC4/IFC4X3).
    pub rows: Vec<ExactTableRow>,
    /// `Expression`, when stated.
    pub expression: Option<Arc<str>>,
    /// `DefiningUnit`, when stated.
    pub defining_unit: Option<EntityId>,
    /// `DefinedUnit`, when stated.
    pub defined_unit: Option<EntityId>,
    /// `CurveInterpolation` (an `IfcCurveInterpolationEnum` member such as
    /// `LINEAR`), when stated. Always `None` in IFC2X3, which does not
    /// declare the attribute.
    pub interpolation: Option<Arc<str>>,
}

/// One row of an [`ExactTableValue`].
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct ExactTableRow {
    /// The defining (x) value.
    pub defining: ExactTypedValue,
    /// The defined (y) value.
    pub defined: ExactTypedValue,
}

/// An `IfcPropertyReferenceValue`: a named pointer to another entity.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ExactReferenceValue {
    /// `UsageName`, when stated.
    pub usage_name: Option<Arc<str>>,
    /// `PropertyReference`: an entity the release's
    /// `IfcObjectReferenceSelect` accepts. Required in IFC2X3, optional in
    /// IFC4 and IFC4X3.
    pub target: Option<ExactEntityRef>,
}

/// An `IfcComplexProperty` or `IfcPhysicalComplexQuantity` (#208).
///
/// A complex groups named members and is no value itself, so an IDS data
/// type or value restriction on it cannot hold, while its presence does.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct ExactComplexValue {
    /// `IfcComplexProperty.UsageName`, or `IfcPhysicalComplexQuantity.Usage`
    /// when stated.
    pub usage: Option<Arc<str>>,
    /// `IfcPhysicalComplexQuantity.Discrimination`; `None` for a complex
    /// property.
    pub discrimination: Option<Arc<str>>,
    /// `IfcPhysicalComplexQuantity.Quality` when stated; `None` for a
    /// complex property.
    pub quality: Option<Arc<str>>,
    /// `HasProperties` or `HasQuantities` in file order, never empty.
    pub members: Vec<ExactComplexMember>,
}

/// One member of an [`ExactComplexValue`], resolved as a set member is.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct ExactComplexMember {
    /// Its `Name`; unique within the complex where the release requires.
    pub name: Arc<str>,
    /// Entity id of the member property or quantity.
    pub id: EntityId,
    /// As [`ExactProperty::value_type`](super::ExactProperty::value_type).
    pub value_type: Option<Arc<str>>,
    /// As [`ExactProperty::unit_id`](super::ExactProperty::unit_id).
    pub unit_id: Option<EntityId>,
    /// The resolved value; [`ExactValue::Complex`] for a nested complex.
    pub value: ExactValue,
}
