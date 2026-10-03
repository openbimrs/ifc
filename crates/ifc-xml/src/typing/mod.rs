//! Values typed from the schema's declarations, never from their text.
//!
//! An XML attribute value is an untyped string. Inferring its kind from the
//! text reads `Name="1"` as an integer and `Name="i7"` as a reference, which
//! is a silently different model. With a schema the reader instead resolves
//! each attribute's declared type to a [`Shape`] -- its aggregation levels
//! and the leaf type they hold -- and types the text by that.
//!
//! Both readers use it: the strict native layout ([`crate::SchemaReading`])
//! and the buildingSMART XSD configuration ([`crate::XmlLayout::Xsd`]). The
//! lexical rules differ only where the two layouts write differently, and
//! [`Lexical`] names which applies.

#[cfg(test)]
mod tests;
mod value;

pub(crate) use value::{conform, hex_binary, list_text, nest, references, scalar};

use crate::error::XmlError;
use ifc_schema::{AggregateKind, Attribute, Bound, Schema, TypeKind};
use std::collections::HashMap;
use std::sync::Arc;

/// Deepest alias chain or nesting followed before refusing the type.
const MAX_DEPTH: usize = 32;

/// One aggregation level of a declared type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Level {
    pub(crate) kind: AggregateKind,
    /// `Some(n)` when the level's size is fixed at `n` (`[n:n]`).
    pub(crate) fixed: Option<usize>,
    /// How many items the level declares: at least, and at most when its
    /// bounds are literals. An `ARRAY [l:u]` holds exactly `u - l + 1`.
    pub(crate) count: (usize, Option<usize>),
}

/// The item count bounds of an aggregation level from its literal bounds.
fn count(kind: AggregateKind, lower: Option<u64>, upper: Option<u64>) -> (usize, Option<usize>) {
    let size = |bound: Option<u64>| bound.and_then(|bound| usize::try_from(bound).ok());
    match (kind, size(lower), size(upper)) {
        (AggregateKind::Array, Some(lower), Some(upper)) if upper >= lower => {
            let items = upper - lower + 1;
            (items, Some(items))
        }
        (AggregateKind::Array, _, _) => (0, None),
        (_, lower, upper) => (lower.unwrap_or(0), upper),
    }
}

/// What a declared type holds once its aggregation levels are removed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Leaf {
    Integer,
    Real,
    Number,
    /// `STRING`, with its width when declared `FIXED`, and its maximum
    /// width when declared `STRING(n)` (fixed or not).
    Text {
        fixed: Option<usize>,
        width: Option<usize>,
    },
    Boolean,
    Logical,
    Binary,
    /// An enumeration type and its declared members.
    Enumeration {
        name: Arc<str>,
        members: Arc<[String]>,
    },
    /// An entity type, by its declared name.
    Entity(Arc<str>),
    /// A SELECT type, by its declared name.
    Select(Arc<str>),
}

impl Leaf {
    /// Whether text can denote a value of this leaf: everything but entity
    /// references and SELECT values, which need an element.
    pub(crate) const fn is_simple(&self) -> bool {
        !matches!(self, Self::Entity(_) | Self::Select(_))
    }

    fn describe(&self) -> String {
        match self {
            Self::Integer => "INTEGER".into(),
            Self::Real => "REAL".into(),
            Self::Number => "NUMBER".into(),
            Self::Text {
                fixed: Some(width), ..
            } => format!("STRING({width}) FIXED"),
            Self::Text { .. } => "STRING".into(),
            Self::Boolean => "BOOLEAN".into(),
            Self::Logical => "LOGICAL".into(),
            Self::Binary => "BINARY".into(),
            Self::Enumeration { name, .. } | Self::Entity(name) | Self::Select(name) => {
                name.to_string()
            }
        }
    }
}

/// A declared type resolved to its aggregation levels and leaf.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Shape {
    /// Aggregation levels, outermost first, including those an aliased
    /// defined type adds (`IfcLineIndex = LIST [2:?] OF ...`).
    pub(crate) levels: Vec<Level>,
    pub(crate) leaf: Leaf,
    /// The innermost type as declared, before alias resolution:
    /// `IfcLengthMeasure` for `LIST [3:3] OF IfcLengthMeasure`.
    pub(crate) named: Arc<str>,
}

impl Shape {
    /// The shape one aggregation level further in.
    pub(crate) fn inner(&self) -> Self {
        Self {
            levels: self.levels.get(1..).unwrap_or_default().to_vec(),
            leaf: self.leaf.clone(),
            named: self.named.clone(),
        }
    }

    /// The leaf alone, without aggregation: one flat item of this shape.
    pub(crate) fn inner_leaf(&self) -> Self {
        Self {
            levels: Vec::new(),
            leaf: self.leaf.clone(),
            named: self.named.clone(),
        }
    }

    /// The form the buildingSMART XSD configuration gives a value of this
    /// shape when no configuration entry overrides it ([`FORM_OVERRIDES`]).
    fn default_xsd_form(&self) -> XsdForm {
        let Some((_, inner)) = self.levels.split_first() else {
            return match self.leaf {
                Leaf::Entity(_) => XsdForm::Entity,
                Leaf::Select(_) => XsdForm::Select,
                Leaf::Binary => XsdForm::Text,
                _ => XsdForm::Attribute,
            };
        };
        // Simple values aggregate into one whitespace-separated XML
        // attribute, flattened when nested, as long as the inner sizes are
        // fixed so the list can be split again.
        if self.leaf.is_simple() && inner.iter().all(|level| level.fixed.is_some()) {
            XsdForm::Attribute
        } else {
            XsdForm::Container(Items::Flat)
        }
    }

    /// The type a mismatch message names.
    pub(crate) fn describe(&self) -> String {
        let mut text = String::new();
        for level in &self.levels {
            text.push_str(match level.kind {
                AggregateKind::List => "LIST OF ",
                AggregateKind::Set => "SET OF ",
                AggregateKind::Bag => "BAG OF ",
                AggregateKind::Array => "ARRAY OF ",
                _ => "AGGREGATE OF ",
            });
        }
        text.push_str(&self.named);
        if self.leaf.describe() != *self.named {
            text.push_str(" (");
            text.push_str(&self.leaf.describe());
            text.push(')');
        }
        text
    }
}

/// How the XSD configuration writes an explicit attribute's value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum XsdForm {
    /// An XML attribute: a simple value, or a whitespace-separated list of
    /// simple values, flattened when nested.
    Attribute,
    /// A child element whose text is the value: a binary, as `xs:hexBinary`.
    Text,
    /// A child element that is the entity, or refers to it with `ref`.
    Entity,
    /// A child element holding the one item of a SELECT: an entity element
    /// or a `-wrapper` typed value.
    Select,
    /// A child element holding the items of an aggregate.
    Container(Items),
}

/// How a container element holds a nested aggregate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Items {
    /// Every leaf item in order -- entity elements or `-wrapper` values --
    /// with `arraySize` giving the inner sizes the schema does not fix.
    Flat,
    /// One `Seq-T-wrapper` per inner list, holding its items as text.
    Seq,
}

/// Attributes whose XSD form departs from [`Shape::default_xsd_form`], as
/// `(declaring entity, attribute, form)`. The configuration writes most
/// string lists as list attributes, but these three as containers of
/// `-wrapper` values; one nested index list as `Seq-` wrapped inner lists;
/// and, in IFC4X3 ADD2, one nested index list whose inner sizes are not
/// fixed as a flat list attribute, which cannot be split again. The
/// `xsd::conformance` test checks every attribute's form against both XSDs.
const FORM_OVERRIDES: &[(&str, &str, XsdForm)] = &[
    (
        "IfcClassification",
        "ReferenceTokens",
        XsdForm::Container(Items::Flat),
    ),
    (
        "IfcPostalAddress",
        "AddressLines",
        XsdForm::Container(Items::Flat),
    ),
    (
        "IfcTextStyleFontModel",
        "FontFamily",
        XsdForm::Container(Items::Flat),
    ),
    (
        "IfcIndexedPolygonalFaceWithVoids",
        "InnerCoordIndices",
        XsdForm::Container(Items::Seq),
    ),
    (
        "IfcTextureCoordinateIndicesWithVoids",
        "InnerTexCoordIndices",
        XsdForm::Attribute,
    ),
];

/// Which layout's lexical rules type a text value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Lexical {
    /// This crate's own layout: `i<n>` references, a `NUMBER` keeps the
    /// integer or real kind its literal spells, binary text kept as written.
    Native,
    /// XML Schema datatypes as the buildingSMART XSD maps them: `NUMBER` is
    /// `xs:double`, `BOOLEAN` is `xs:boolean`, binary is `xs:hexBinary`.
    Xsd,
}

/// One explicit attribute slot of an entity, resolved.
#[derive(Debug, Clone)]
pub(crate) struct SlotLayout {
    pub(crate) name: Arc<str>,
    pub(crate) shape: Shape,
    /// Declared `OPTIONAL`.
    pub(crate) optional: bool,
    /// The form the XSD configuration writes the value in.
    pub(crate) form: XsdForm,
    /// Redeclared `DERIVE` in the entity or one of its supertypes: the slot
    /// holds `*` and carries no value.
    pub(crate) derived: bool,
}

/// One `INVERSE` attribute visible on an entity.
#[derive(Debug, Clone)]
pub(crate) struct InverseLayout {
    pub(crate) name: Arc<str>,
    /// The entity whose explicit attribute points back.
    pub(crate) entity: Arc<str>,
    /// That explicit attribute's name.
    pub(crate) for_attribute: Arc<str>,
}

/// An entity type resolved for reading: its slots in Part 21 order.
#[derive(Debug, Clone)]
pub(crate) struct EntityLayout {
    /// The declared name, as the schema spells it.
    pub(crate) name: Arc<str>,
    /// The upper-case name the STEP reader stores.
    pub(crate) upper: Arc<str>,
    pub(crate) abstract_: bool,
    pub(crate) slots: Vec<SlotLayout>,
    pub(crate) inverses: Vec<InverseLayout>,
}

impl EntityLayout {
    /// The slot an explicit attribute name occupies. Exact match: XML names
    /// are case-sensitive and the schema's spelling is the XSD's.
    pub(crate) fn slot(&self, name: &str) -> Option<usize> {
        self.slots.iter().position(|slot| &*slot.name == name)
    }

    /// The inverse attribute of that name, most specific declaration first.
    pub(crate) fn inverse(&self, name: &str) -> Option<&InverseLayout> {
        self.inverses.iter().find(|inverse| &*inverse.name == name)
    }
}

/// Resolved entity layouts, built once per type and document.
pub(crate) struct Layouts<'s> {
    schema: &'s Schema,
    cache: HashMap<String, Arc<EntityLayout>>,
}

impl<'s> Layouts<'s> {
    pub(crate) fn new(schema: &'s Schema) -> Self {
        Self {
            schema,
            cache: HashMap::new(),
        }
    }

    pub(crate) const fn schema(&self) -> &'s Schema {
        self.schema
    }

    /// The layout of an entity type, or `None` when the schema does not
    /// declare it. `exact` requires the schema's own spelling.
    pub(crate) fn entity(
        &mut self,
        name: &str,
        exact: bool,
    ) -> Result<Option<Arc<EntityLayout>>, XmlError> {
        let Some(definition) = self.schema.entity(name) else {
            return Ok(None);
        };
        if exact && definition.name != name {
            return Ok(None);
        }
        let key = definition.name.to_ascii_uppercase();
        if let Some(layout) = self.cache.get(&key) {
            return Ok(Some(layout.clone()));
        }
        let layout = Arc::new(build_layout(self.schema, &definition.name)?);
        self.cache.insert(key, layout.clone());
        Ok(Some(layout))
    }
}

fn build_layout(schema: &Schema, name: &str) -> Result<EntityLayout, XmlError> {
    let definition = schema
        .entity(name)
        .ok_or_else(|| XmlError::UnknownEntity { name: name.into() })?;
    let chain: Vec<&str> = std::iter::once(definition.name.as_str())
        .chain(schema.supertypes(name))
        .collect();
    let derived = |slot: &str| {
        chain
            .iter()
            .filter_map(|entity| schema.entity(entity))
            .any(|entity| entity.is_derived(slot))
    };
    let mut slots = Vec::new();
    for attribute in schema.attributes(name) {
        let shape = attribute_shape(schema, attribute)?;
        let form = FORM_OVERRIDES
            .iter()
            .find(|(declarer, overridden, _)| {
                *overridden == attribute.name && schema.is_a(name, declarer)
            })
            .map_or_else(|| shape.default_xsd_form(), |(_, _, form)| *form);
        slots.push(SlotLayout {
            name: attribute.name.as_str().into(),
            shape,
            optional: attribute.optional,
            form,
            derived: derived(&attribute.name),
        });
    }
    // Nearest declaration first, so a subtype's redeclaration shadows.
    let mut inverses: Vec<InverseLayout> = Vec::new();
    for entity in chain.iter().filter_map(|entity| schema.entity(entity)) {
        for inverse in &entity.inverses {
            if inverses.iter().any(|seen| *seen.name == *inverse.name) {
                continue;
            }
            inverses.push(InverseLayout {
                name: inverse.name.as_str().into(),
                entity: inverse.entity.as_str().into(),
                for_attribute: inverse.for_attribute.as_str().into(),
            });
        }
    }
    Ok(EntityLayout {
        name: definition.name.as_str().into(),
        upper: definition.name.to_ascii_uppercase().into(),
        abstract_: definition.abstract_,
        slots,
        inverses,
    })
}

/// The shape of an explicit attribute's declared type.
pub(crate) fn attribute_shape(schema: &Schema, attribute: &Attribute) -> Result<Shape, XmlError> {
    if attribute.aggregate && attribute.aggregation.is_empty() {
        // A table written before aggregate bounds were recorded cannot say
        // how deep the aggregate is.
        return Err(unsupported(format!(
            "attribute `{}` declares an aggregate without recorded levels",
            attribute.name
        )));
    }
    let mut levels: Vec<Level> = attribute
        .aggregation
        .iter()
        .map(|aggregation| Level {
            kind: aggregation.kind,
            fixed: fixed_size(&aggregation.lower, &aggregation.upper),
            count: count(
                aggregation.kind,
                aggregation.lower.as_integer(),
                aggregation.upper.as_integer(),
            ),
        })
        .collect();
    let leaf = resolve(schema, &attribute.type_name, &mut levels, 0)?;
    Ok(Shape {
        levels,
        leaf,
        named: attribute.type_name.as_str().into(),
    })
}

/// The shape of a named type, as a typed wrapper names it.
pub(crate) fn type_shape(schema: &Schema, name: &str) -> Result<Shape, XmlError> {
    let mut levels = Vec::new();
    let leaf = resolve(schema, name, &mut levels, 0)?;
    Ok(Shape {
        levels,
        leaf,
        named: name.into(),
    })
}

fn fixed_size(lower: &Bound, upper: &Bound) -> Option<usize> {
    match (lower.as_integer(), upper.as_integer()) {
        (Some(lower), Some(upper)) if lower == upper => usize::try_from(lower).ok(),
        _ => None,
    }
}

/// Resolve a type expression to its leaf, appending aggregation levels.
fn resolve(
    schema: &Schema,
    expression: &str,
    levels: &mut Vec<Level>,
    depth: usize,
) -> Result<Leaf, XmlError> {
    if depth > MAX_DEPTH {
        return Err(unsupported(format!(
            "type `{expression}` nests or aliases too deeply"
        )));
    }
    let expression = expression.trim();
    if let Some(rest) = aggregate_prefix(expression, levels)? {
        return resolve(schema, rest, levels, depth + 1);
    }
    if let Some(leaf) = primitive(expression) {
        return Ok(leaf);
    }
    if let Some(entity) = schema.entity(expression) {
        return Ok(Leaf::Entity(entity.name.as_str().into()));
    }
    let Some(definition) = schema.type_def(expression) else {
        return Err(unsupported(format!(
            "type `{expression}` is not declared by the schema"
        )));
    };
    match &definition.kind {
        TypeKind::Enumeration(members) => Ok(Leaf::Enumeration {
            name: definition.name.as_str().into(),
            members: members.clone().into(),
        }),
        TypeKind::Select(_) => Ok(Leaf::Select(definition.name.as_str().into())),
        TypeKind::Defined(target) => resolve(schema, target, levels, depth + 1),
        _ => Err(unsupported(format!(
            "type `{expression}` has a declaration form this reader does not know"
        ))),
    }
}

/// `LIST [1:?] OF UNIQUE X` -> push one level and return `X`.
fn aggregate_prefix<'e>(
    expression: &'e str,
    levels: &mut Vec<Level>,
) -> Result<Option<&'e str>, XmlError> {
    let upper = expression.to_ascii_uppercase();
    let kind = [
        ("LIST", AggregateKind::List),
        ("SET", AggregateKind::Set),
        ("BAG", AggregateKind::Bag),
        ("ARRAY", AggregateKind::Array),
    ]
    .into_iter()
    .find(|(keyword, _)| {
        upper.starts_with(keyword)
            && upper[keyword.len()..]
                .chars()
                .next()
                .is_some_and(|next| next == ' ' || next == '[')
    });
    let Some((keyword, kind)) = kind else {
        return Ok(None);
    };
    let malformed = || unsupported(format!("aggregate type `{expression}` is malformed"));
    let rest = expression[keyword.len()..].trim_start();
    let (fixed, items, rest) = if let Some(bounds) = rest.strip_prefix('[') {
        let close = bounds.find(']').ok_or_else(malformed)?;
        let (lower, upper) = bounds[..close].split_once(':').ok_or_else(malformed)?;
        let (lower, upper) = (
            lower.trim().parse::<u64>().ok(),
            upper.trim().parse::<u64>().ok(),
        );
        let fixed = match (lower, upper) {
            (Some(lower), Some(upper)) if lower == upper => usize::try_from(lower).ok(),
            _ => None,
        };
        (
            fixed,
            count(kind, lower, upper),
            bounds[close + 1..].trim_start(),
        )
    } else {
        (None, (0, None), rest)
    };
    let rest = rest
        .strip_prefix("OF")
        .or_else(|| rest.strip_prefix("of"))
        .ok_or_else(malformed)?
        .trim_start();
    let mut rest = rest;
    for qualifier in ["UNIQUE", "OPTIONAL"] {
        if rest.to_ascii_uppercase().starts_with(qualifier)
            && rest[qualifier.len()..].starts_with(' ')
        {
            rest = rest[qualifier.len()..].trim_start();
        }
    }
    levels.push(Level {
        kind,
        fixed,
        count: items,
    });
    Ok(Some(rest))
}

fn primitive(expression: &str) -> Option<Leaf> {
    let upper = expression.to_ascii_uppercase();
    let word = upper
        .split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
        .next()
        .unwrap_or_default();
    let leaf = match word {
        "INTEGER" => Leaf::Integer,
        "REAL" => Leaf::Real,
        "NUMBER" => Leaf::Number,
        "BOOLEAN" => Leaf::Boolean,
        "LOGICAL" => Leaf::Logical,
        "BINARY" => Leaf::Binary,
        "STRING" => {
            let width: Option<usize> = (|| {
                let open = upper.find('(')?;
                let close = upper.find(')')?;
                upper.get(open + 1..close)?.trim().parse().ok()
            })();
            let fixed = width.filter(|_| upper.contains("FIXED"));
            Leaf::Text { fixed, width }
        }
        _ => return None,
    };
    Some(leaf)
}

fn unsupported(construct: String) -> XmlError {
    XmlError::Unsupported { construct }
}

fn invalid(leaf: &Leaf, text: &str) -> XmlError {
    XmlError::InvalidScalar {
        kind: leaf.describe(),
        value: text.into(),
    }
}
