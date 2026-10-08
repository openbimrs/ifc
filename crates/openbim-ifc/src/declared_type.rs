//! The declared EXPRESS type of an attribute, resolved against a release
//! (#342).
//!
//! An attribute's declaration names a type token (`IfcLabel`,
//! `IfcWallTypeEnum`, `IfcValue`) and the aggregation levels around it.
//! [`declared_type`] follows that token through the release's tables to the
//! shape a value of it takes:
//!
//! - a built-in simple type (`STRING(255)` is a [`SimpleType::String`]);
//! - an entity, written as a reference;
//! - an enumeration with its items;
//! - a defined type with the type it aliases, which may itself be an
//!   aggregate (`IfcCompoundPlaneAngleMeasure = LIST [3:4] OF INTEGER`);
//! - a SELECT with every member resolved, nested SELECTs included;
//! - an aggregate with its element type.
//!
//! A token the tables do not declare is [`DeclaredType::Unresolved`], never
//! a guess. The walk is bounded, so a malformed table with a cyclic alias
//! or SELECT ends in `Unresolved` rather than hanging.
//!
//! [`attribute_type`] does the same for one attribute of an entity type
//! named the way [`attribute_slot`](crate::attribute_slot) names it, with
//! the attribute's own aggregation levels outermost.
//!
//! ```
//! use ifc::{attribute_type, DeclaredType, SimpleType};
//! let schema = ifc::schema::ifc4();
//! let (slot, declared) = attribute_type(schema, "IFCWALL", "Name").unwrap();
//! assert_eq!(slot.index, 2);
//! let DeclaredType::Defined { name, underlying } = declared else { panic!() };
//! assert_eq!((name, *underlying), ("IfcLabel", DeclaredType::Simple(SimpleType::String)));
//! ```
#![cfg(feature = "schema-api")]

use ifc_schema::{AggregateKind, Schema, TypeKind};

use crate::named_attribute::{attribute_slot, AttributeSlot, NamedAttributeError};

/// Alias and SELECT nesting followed before a type is `Unresolved`.
const MAX_DEPTH: usize = 32;

/// An EXPRESS built-in simple type (ISO 10303-11 §8.1), widths dropped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SimpleType {
    /// `INTEGER`.
    Integer,
    /// `REAL`.
    Real,
    /// `NUMBER`: an integer or a real.
    Number,
    /// `STRING`, with or without a width.
    String,
    /// `BOOLEAN`.
    Boolean,
    /// `LOGICAL`: true, false or unknown.
    Logical,
    /// `BINARY`, with or without a width.
    Binary,
}

impl SimpleType {
    /// The EXPRESS keyword, upper-case.
    #[must_use]
    pub const fn keyword(self) -> &'static str {
        match self {
            Self::Integer => "INTEGER",
            Self::Real => "REAL",
            Self::Number => "NUMBER",
            Self::String => "STRING",
            Self::Boolean => "BOOLEAN",
            Self::Logical => "LOGICAL",
            Self::Binary => "BINARY",
        }
    }
}

/// A declared type resolved through a release's tables.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum DeclaredType<'s> {
    /// A built-in simple type.
    Simple(SimpleType),
    /// An entity type: a value is a reference to an instance of it or of
    /// a subtype.
    Entity(&'s str),
    /// An enumeration and its items, in the schema's spelling.
    Enumeration {
        /// The type's name.
        name: &'s str,
        /// The items.
        items: &'s [String],
    },
    /// A defined type and the type it aliases.
    Defined {
        /// The type's name.
        name: &'s str,
        /// What it aliases: a simple type, an aggregate or another defined
        /// type.
        underlying: Box<DeclaredType<'s>>,
    },
    /// A SELECT and its members, each resolved.
    Select {
        /// The type's name.
        name: &'s str,
        /// The members, in declaration order.
        members: Vec<DeclaredType<'s>>,
    },
    /// An aggregate of `element`.
    Aggregate {
        /// `LIST`, `SET`, `BAG` or `ARRAY`.
        kind: AggregateKind,
        /// The element type.
        element: Box<DeclaredType<'s>>,
    },
    /// A token the tables do not resolve, as written.
    Unresolved(&'s str),
}

impl<'s> DeclaredType<'s> {
    /// The declared name of an entity, enumeration, defined type or
    /// SELECT; `None` for a simple type, an aggregate and an unresolved
    /// token.
    #[must_use]
    pub const fn name(&self) -> Option<&'s str> {
        match self {
            Self::Entity(name)
            | Self::Enumeration { name, .. }
            | Self::Defined { name, .. }
            | Self::Select { name, .. } => Some(name),
            _ => None,
        }
    }
}

/// Resolve the type token `type_text` (`IfcLabel`, `REAL`,
/// `LIST [3:4] OF INTEGER`) against `schema`.
#[must_use]
pub fn declared_type<'s>(schema: &'s Schema, type_text: &'s str) -> DeclaredType<'s> {
    resolve(schema, type_text.trim(), MAX_DEPTH)
}

/// Attribute `name` (any case) of entity type `type_name`: its slot, and
/// its declared type with the attribute's aggregation levels outermost.
///
/// # Errors
///
/// As for [`attribute_slot`].
pub fn attribute_type<'s>(
    schema: &'s Schema,
    type_name: &str,
    name: &str,
) -> Result<(AttributeSlot<'s>, DeclaredType<'s>), NamedAttributeError> {
    let slot = attribute_slot(schema, type_name, name)?;
    let mut declared = declared_type(schema, slot.type_name);
    let levels: Vec<AggregateKind> = schema
        .attribute_at(type_name, slot.index)
        .map(|attribute| {
            let mut kinds: Vec<_> = attribute.aggregation.iter().map(|a| a.kind).collect();
            // A table written before bounds were recorded marks an
            // aggregate without its levels; one level is all it says.
            if kinds.is_empty() && attribute.aggregate {
                kinds.push(AggregateKind::List);
            }
            kinds
        })
        .unwrap_or_default();
    for kind in levels.into_iter().rev() {
        declared = DeclaredType::Aggregate {
            kind,
            element: Box::new(declared),
        };
    }
    Ok((slot, declared))
}

fn resolve<'s>(schema: &'s Schema, text: &'s str, depth: usize) -> DeclaredType<'s> {
    if depth == 0 {
        return DeclaredType::Unresolved(text);
    }
    if let Some(simple) = simple(text) {
        return simple;
    }
    if let Some((kind, element)) = aggregate(text) {
        return DeclaredType::Aggregate {
            kind,
            element: Box::new(resolve(schema, element, depth - 1)),
        };
    }
    // IFC declares entities and types in one namespace; an entity first,
    // as the tables' own checks do.
    if let Some(entity) = schema.entity(text) {
        return DeclaredType::Entity(&entity.name);
    }
    let Some(definition) = schema.type_def(text) else {
        return DeclaredType::Unresolved(text);
    };
    let name = definition.name.as_str();
    match &definition.kind {
        TypeKind::Defined(target) => DeclaredType::Defined {
            name,
            underlying: Box::new(resolve(schema, target.trim(), depth - 1)),
        },
        TypeKind::Enumeration(items) => DeclaredType::Enumeration { name, items },
        TypeKind::Select(members) => DeclaredType::Select {
            name,
            members: members
                .iter()
                .map(|member| resolve(schema, member.trim(), depth - 1))
                .collect(),
        },
        _ => DeclaredType::Unresolved(text),
    }
}

/// The keyword `text` starts with, up to the first non-letter.
fn keyword(text: &str) -> &str {
    let end = text
        .find(|c: char| !c.is_ascii_alphabetic())
        .unwrap_or(text.len());
    &text[..end]
}

/// A built-in simple type, with an optional width: `STRING(22) FIXED`.
fn simple(text: &str) -> Option<DeclaredType<'static>> {
    let head = keyword(text);
    let rest = text[head.len()..].trim_start();
    if !(rest.is_empty() || rest.starts_with('(')) {
        return None;
    }
    let simple = match head.to_ascii_uppercase().as_str() {
        "INTEGER" => SimpleType::Integer,
        "REAL" => SimpleType::Real,
        "NUMBER" => SimpleType::Number,
        "STRING" => SimpleType::String,
        "BOOLEAN" => SimpleType::Boolean,
        "LOGICAL" => SimpleType::Logical,
        "BINARY" => SimpleType::Binary,
        _ => return None,
    };
    Some(DeclaredType::Simple(simple))
}

/// An aggregate written out: `LIST [3:4] OF UNIQUE INTEGER`.
fn aggregate(text: &str) -> Option<(AggregateKind, &str)> {
    let kind = match keyword(text).to_ascii_uppercase().as_str() {
        "LIST" => AggregateKind::List,
        "SET" => AggregateKind::Set,
        "BAG" => AggregateKind::Bag,
        "ARRAY" => AggregateKind::Array,
        _ => return None,
    };
    let at = text.to_ascii_uppercase().find(" OF ")?;
    let mut element = text[at + 4..].trim();
    for prefix in ["UNIQUE ", "OPTIONAL "] {
        if element.len() >= prefix.len() && element[..prefix.len()].eq_ignore_ascii_case(prefix) {
            element = element[prefix.len()..].trim_start();
        }
    }
    Some((kind, element))
}

#[cfg(all(test, feature = "ifc4", feature = "ifc2x3", feature = "ifc4x3"))]
mod tests {
    use super::*;

    #[test]
    fn a_label_is_a_defined_string() {
        for schema in [
            ifc_schema::ifc2x3(),
            ifc_schema::ifc4(),
            ifc_schema::ifc4x3(),
        ] {
            let (_, declared) = attribute_type(schema, "IFCWALL", "name").unwrap();
            assert_eq!(
                declared,
                DeclaredType::Defined {
                    name: "IfcLabel",
                    underlying: Box::new(DeclaredType::Simple(SimpleType::String)),
                },
                "{}",
                schema.name()
            );
        }
    }

    #[test]
    fn an_enumeration_lists_its_items() {
        let (_, declared) =
            attribute_type(ifc_schema::ifc4(), "IFCWALL", "PredefinedType").unwrap();
        let DeclaredType::Enumeration { name, items } = declared else {
            panic!("{declared:?}");
        };
        assert_eq!(name, "IfcWallTypeEnum");
        assert!(items
            .iter()
            .any(|item| item.eq_ignore_ascii_case("STANDARD")));
    }

    #[test]
    fn aggregates_wrap_outermost_first() {
        let (_, declared) =
            attribute_type(ifc_schema::ifc4(), "IFCCARTESIANPOINTLIST3D", "CoordList").unwrap();
        let DeclaredType::Aggregate { element, .. } = declared else {
            panic!("{declared:?}");
        };
        let DeclaredType::Aggregate { element, .. } = *element else {
            panic!("two levels");
        };
        assert_eq!(element.name(), Some("IfcLengthMeasure"));
    }

    #[test]
    fn a_defined_aggregate_resolves_its_element() {
        let declared = declared_type(ifc_schema::ifc4(), "IfcCompoundPlaneAngleMeasure");
        let DeclaredType::Defined { underlying, .. } = declared else {
            panic!("{declared:?}");
        };
        assert_eq!(
            *underlying,
            DeclaredType::Aggregate {
                kind: AggregateKind::List,
                element: Box::new(DeclaredType::Simple(SimpleType::Integer)),
            }
        );
    }

    #[test]
    fn a_select_resolves_its_members_and_unknown_tokens_stay_unresolved() {
        let declared = declared_type(ifc_schema::ifc4(), "IfcValue");
        let DeclaredType::Select { members, .. } = &declared else {
            panic!("{declared:?}");
        };
        assert!(members.iter().any(|member| matches!(
            member,
            DeclaredType::Select {
                name: "IfcMeasureValue",
                ..
            }
        )));
        assert_eq!(
            declared_type(ifc_schema::ifc4(), "IfcNoSuchType"),
            DeclaredType::Unresolved("IfcNoSuchType")
        );
    }
}
