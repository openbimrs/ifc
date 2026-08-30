//! Single, bounded, list and enumerated property values.
//!
//! # Slots, verified against the IFC4 EXPRESS schema
//!
//! Every `IfcProperty` starts with `Name` (0) and `Description` (1); the
//! subtype attributes follow. The families disagree about what comes next,
//! and reading slot 2 as "the value" is wrong for four of the six:
//!
//! ```text
//! IfcPropertySingleValue      2 = NominalValue      3 = Unit
//! IfcPropertyEnumeratedValue  2 = EnumerationValues 3 = EnumerationReference
//! IfcPropertyBoundedValue     2 = UpperBoundValue   3 = LowerBoundValue
//!                             4 = Unit              5 = SetPointValue
//! IfcPropertyListValue        2 = ListValues        3 = Unit
//! IfcPropertyTableValue       2 = DefiningValues    3 = DefinedValues
//!                             4 = Expression        5 = DefiningUnit
//!                             6 = DefinedUnit       7 = CurveInterpolation
//! IfcPropertyReferenceValue   2 = UsageName         3 = PropertyReference
//! ```
//!
//! Note `IfcPropertyBoundedValue`: UPPER bound is slot 2, LOWER is slot 3.
//! Reading them in the intuitive order silently inverts every range in the
//! file, and the result still looks like a plausible range.

use std::sync::Arc;

use ifc_model::{EntityId, Model};

use crate::value::MeasureValue;

/// Attribute slots shared by every `IfcProperty`.
const NAME: usize = 0;
const DESCRIPTION: usize = 1;

/// What a property actually states.
///
/// One variant per concrete `IfcProperty` subtype in IFC4. A file stating a
/// subtype this crate does not model yields [`PropertyValue::Unsupported`]
/// carrying the type name, so an unknown property is reported rather than
/// silently dropped from its set.
#[derive(Debug, Clone, PartialEq)]
pub enum PropertyValue {
    /// `IfcPropertySingleValue`: one measure, optionally with a unit.
    Single {
        /// The nominal value. `None` when the file omits it, which is legal.
        value: Option<MeasureValue>,
        /// `IfcNamedUnit`/`IfcDerivedUnit` override for this property.
        unit: Option<EntityId>,
    },
    /// `IfcPropertyEnumeratedValue`: chosen constants plus their enumeration.
    Enumerated {
        /// The selected values.
        values: Vec<MeasureValue>,
        /// The `IfcPropertyEnumeration` they were selected from.
        enumeration: Option<EntityId>,
    },
    /// `IfcPropertyBoundedValue`: a range, optionally with a set point.
    Bounded {
        /// Slot 2. Named explicitly because the schema order is upper-first.
        upper: Option<MeasureValue>,
        /// Slot 3.
        lower: Option<MeasureValue>,
        /// Slot 5.
        set_point: Option<MeasureValue>,
        /// Slot 4.
        unit: Option<EntityId>,
    },
    /// `IfcPropertyListValue`: an ordered list of measures.
    List {
        /// The values, in file order. Order is meaningful in a LIST.
        values: Vec<MeasureValue>,
        /// Unit shared by every entry.
        unit: Option<EntityId>,
    },
    /// `IfcPropertyTableValue`: a defining/defined value mapping.
    Table {
        /// The x column.
        defining: Vec<MeasureValue>,
        /// The y column.
        defined: Vec<MeasureValue>,
        /// How to interpolate between rows, when stated.
        interpolation: Option<Arc<str>>,
    },
    /// `IfcPropertyReferenceValue`: a pointer to another entity.
    Reference {
        /// What the reference is for.
        usage: Option<Arc<str>>,
        /// The referenced entity.
        reference: Option<EntityId>,
    },
    /// `IfcComplexProperty`: nested properties under a usage name.
    Complex {
        /// The grouping name.
        usage: Option<Arc<str>>,
        /// Contained properties, in file order.
        properties: Vec<Property>,
    },
    /// A concrete `IfcProperty` subtype this crate does not model.
    Unsupported {
        /// The declared type, upper-cased.
        type_name: Arc<str>,
    },
}

/// One property: its name, description and value.
#[derive(Debug, Clone, PartialEq)]
pub struct Property {
    /// The `IfcProperty` entity.
    pub id: EntityId,
    /// `Name`. Required by the schema; `None` means the file broke that.
    pub name: Option<Arc<str>>,
    /// `Description`, when stated.
    pub description: Option<Arc<str>>,
    /// The value, by family.
    pub value: PropertyValue,
}

/// Read one property by id.
///
/// Recursion depth is bounded: `IfcComplexProperty` may nest, and a file that
/// makes a complex property reach itself would otherwise recurse forever. The
/// schema forbids self-reference, but a reader must not trust that.
pub fn property(model: &Model, id: EntityId) -> Option<Property> {
    read_property(model, id, 0)
}

/// Maximum `IfcComplexProperty` nesting followed before giving up.
///
/// Deep enough for any real authoring tool; shallow enough that a cyclic file
/// terminates. Exceeding it yields `Unsupported` rather than a panic.
const MAX_COMPLEX_DEPTH: usize = 16;

fn read_property(model: &Model, id: EntityId, depth: usize) -> Option<Property> {
    let entity = model.get(id)?;
    let ty = entity.type_name.to_ascii_uppercase();
    let name = entity.attributes.get(NAME).and_then(text);
    let description = entity.attributes.get(DESCRIPTION).and_then(text);

    let value = match ty.as_str() {
        "IFCPROPERTYSINGLEVALUE" => PropertyValue::Single {
            value: entity.attributes.get(2).and_then(MeasureValue::read),
            unit: entity.attributes.get(3).and_then(entity_ref),
        },
        "IFCPROPERTYENUMERATEDVALUE" => PropertyValue::Enumerated {
            values: entity
                .attributes
                .get(2)
                .and_then(MeasureValue::read_list)
                .unwrap_or_default(),
            enumeration: entity.attributes.get(3).and_then(entity_ref),
        },
        // Upper is slot 2, lower is slot 3. See the module note.
        "IFCPROPERTYBOUNDEDVALUE" => PropertyValue::Bounded {
            upper: entity.attributes.get(2).and_then(MeasureValue::read),
            lower: entity.attributes.get(3).and_then(MeasureValue::read),
            unit: entity.attributes.get(4).and_then(entity_ref),
            set_point: entity.attributes.get(5).and_then(MeasureValue::read),
        },
        "IFCPROPERTYLISTVALUE" => PropertyValue::List {
            values: entity
                .attributes
                .get(2)
                .and_then(MeasureValue::read_list)
                .unwrap_or_default(),
            unit: entity.attributes.get(3).and_then(entity_ref),
        },
        "IFCPROPERTYTABLEVALUE" => PropertyValue::Table {
            defining: entity
                .attributes
                .get(2)
                .and_then(MeasureValue::read_list)
                .unwrap_or_default(),
            defined: entity
                .attributes
                .get(3)
                .and_then(MeasureValue::read_list)
                .unwrap_or_default(),
            interpolation: entity.attributes.get(7).and_then(enum_text),
        },
        "IFCPROPERTYREFERENCEVALUE" => PropertyValue::Reference {
            usage: entity.attributes.get(2).and_then(text),
            reference: entity.attributes.get(3).and_then(entity_ref),
        },
        "IFCCOMPLEXPROPERTY" => {
            let properties = if depth >= MAX_COMPLEX_DEPTH {
                Vec::new()
            } else {
                entity
                    .attributes
                    .get(3)
                    .and_then(refs)
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|child| *child != id)
                    .filter_map(|child| read_property(model, child, depth + 1))
                    .collect()
            };
            PropertyValue::Complex {
                usage: entity.attributes.get(2).and_then(text),
                properties,
            }
        }
        _ => PropertyValue::Unsupported {
            type_name: ty.as_str().into(),
        },
    };

    Some(Property {
        id,
        name,
        description,
        value,
    })
}

/// Read an enumeration constant such as `.LINEAR.`
///
/// Distinct from `text`: an IFC enum is written unquoted, so a reader
/// expecting `Value::Text` finds nothing and silently reports "unstated".
fn enum_text(value: &ifc_model::Value) -> Option<Arc<str>> {
    match value.unwrap_typed() {
        ifc_model::Value::Enum(t) | ifc_model::Value::Text(t) => Some(t.clone()),
        _ => None,
    }
}

fn text(value: &ifc_model::Value) -> Option<Arc<str>> {
    match value.unwrap_typed() {
        ifc_model::Value::Text(t) => Some(t.clone()),
        _ => None,
    }
}

fn entity_ref(value: &ifc_model::Value) -> Option<EntityId> {
    match value.unwrap_typed() {
        ifc_model::Value::Ref(id) => Some(*id),
        _ => None,
    }
}

fn refs(value: &ifc_model::Value) -> Option<Vec<EntityId>> {
    match value {
        ifc_model::Value::List(items) => Some(items.iter().filter_map(entity_ref).collect()),
        _ => None,
    }
}
