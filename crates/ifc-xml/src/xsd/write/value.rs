//! Attribute values in the form the configuration gives their declaration.

use super::text::{self, escape};
use super::{unrepresentable, Writer};
use crate::error::XmlError;
use crate::typing::{self, Items, Leaf, Shape, SlotLayout, XsdForm};
use ifc_model::{EntityId, Value};
use ifc_schema::Schema;
use std::fmt::Write as _;

/// The text of an attribute written as an XML attribute, or `None` when
/// it is unset. The XSD declares every such attribute optional.
pub(super) fn attribute(declared: &SlotLayout, value: &Value) -> Result<Option<String>, XmlError> {
    match value {
        Value::Null => Ok(None),
        _ if declared.shape.levels.is_empty() => {
            text::scalar(&declared.shape.leaf, value).map(Some)
        }
        _ => tokens(&declared.shape, value).map(Some),
    }
}

/// A nested aggregate of simple values as one whitespace-separated list,
/// flattened. Reading splits it again by the fixed inner sizes, so every
/// inner list must have its declared size.
fn tokens(shape: &Shape, value: &Value) -> Result<String, XmlError> {
    if shape
        .levels
        .iter()
        .skip(1)
        .any(|level| level.fixed.is_none())
    {
        return Err(unrepresentable(format!(
            "a {} as one list: its inner sizes are not fixed, so it cannot be split again",
            shape.describe()
        )));
    }
    let mut leaves = Vec::new();
    flatten(shape, value, 0, &mut leaves, &mut Vec::new())?;
    let mut out = String::new();
    for leaf in leaves {
        let mut token = text::scalar(&shape.leaf, leaf)?;
        if matches!(shape.leaf, Leaf::Text { .. }) {
            token = text::token(token)?;
        }
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(&token);
    }
    Ok(out)
}

/// Collect the leaves of a nested aggregate in order, checking each inner
/// list against its fixed size and recording the sizes found per level:
/// `None` once two lists of one level differ.
fn flatten<'v>(
    shape: &Shape,
    value: &'v Value,
    depth: usize,
    leaves: &mut Vec<&'v Value>,
    sizes: &mut Vec<Option<usize>>,
) -> Result<(), XmlError> {
    if depth == shape.levels.len() {
        if matches!(value, Value::Null | Value::Derived) {
            return Err(unrepresentable("an unset item of an aggregate".into()));
        }
        leaves.push(value);
        return Ok(());
    }
    let Value::List(items) = value else {
        return Err(XmlError::TypeMismatch {
            declared: shape.describe(),
            found: "a value that is not a list".into(),
        });
    };
    if let Some(fixed) = shape.levels[depth].fixed {
        if depth > 0 && items.len() != fixed {
            return Err(XmlError::TypeMismatch {
                declared: shape.describe(),
                found: format!("an inner list of {} items, not {fixed}", items.len()),
            });
        }
    }
    if sizes.len() <= depth {
        sizes.push(Some(items.len()));
    } else if sizes[depth] != Some(items.len()) {
        sizes[depth] = None;
    }
    for item in items {
        flatten(shape, item, depth + 1, leaves, sizes)?;
    }
    Ok(())
}

/// Check every aggregate in `value` holds as many items as its level
/// declares. The XSD turns those bounds into `minOccurs`/`maxOccurs` on a
/// container's items or `minLength`/`maxLength` on a list attribute, so a
/// count outside them is a document the XSD refuses.
pub(super) fn counts(schema: &Schema, shape: &Shape, value: &Value) -> Result<(), XmlError> {
    match value {
        Value::List(items) => {
            let Some(level) = shape.levels.first() else {
                return Ok(());
            };
            let (least, most) = level.count;
            if items.len() < least || most.is_some_and(|most| items.len() > most) {
                let most = most.map_or_else(|| "?".to_string(), |most| most.to_string());
                return Err(unrepresentable(format!(
                    "{} items in a {} declared with [{least}:{most}] items",
                    items.len(),
                    shape.describe()
                )));
            }
            let inner = shape.inner();
            for item in items {
                counts(schema, &inner, item)?;
            }
            Ok(())
        }
        Value::Typed { type_name, value } => match schema.type_def(type_name) {
            Some(definition) => {
                let inner = typing::type_shape(schema, &definition.name)?;
                counts(schema, &inner, value)
            }
            None => Ok(()),
        },
        _ => Ok(()),
    }
}

impl Writer<'_> {
    /// An explicit attribute written as a child element.
    pub(super) fn slot(&mut self, declared: &SlotLayout, value: &Value) -> Result<(), XmlError> {
        let name = &*declared.name;
        if *value == Value::Null {
            if declared.optional {
                return Ok(());
            }
            return Err(unrepresentable(format!(
                "an unset `{name}`: the XSD requires the element of a mandatory attribute"
            )));
        }
        match declared.form {
            XsdForm::Entity => {
                let (Value::Ref(id), Leaf::Entity(declared_type)) = (value, &declared.shape.leaf)
                else {
                    return Err(mismatch(&declared.shape, value));
                };
                self.reference(name, declared_type, *id, 2)?;
            }
            XsdForm::Text => {
                let content = text::scalar(&declared.shape.leaf, value)?;
                let _ = writeln!(self.out, "    <{name}>{content}</{name}>");
            }
            XsdForm::Select => {
                let _ = writeln!(self.out, "    <{name}>");
                self.item(&declared.shape, value, 3)?;
                let _ = writeln!(self.out, "    </{name}>");
            }
            XsdForm::Container(items) => self.container(name, &declared.shape, value, items)?,
            XsdForm::Attribute => {}
        }
        Ok(())
    }

    /// An aggregate's container element and its items.
    fn container(
        &mut self,
        name: &str,
        shape: &Shape,
        value: &Value,
        items: Items,
    ) -> Result<(), XmlError> {
        let Value::List(outer) = value else {
            return Err(mismatch(shape, value));
        };
        if items == Items::Seq {
            // One `Seq-T-wrapper` per inner list, its items as text.
            let inner = shape.inner();
            let _ = writeln!(self.out, "    <{name}>");
            for list in outer {
                let content = tokens(&inner, list)?;
                let _ = writeln!(
                    self.out,
                    "      <Seq-{0}-wrapper>{content}</Seq-{0}-wrapper>",
                    shape.named
                );
            }
            let _ = writeln!(self.out, "    </{name}>");
            return Ok(());
        }
        let mut leaves = Vec::new();
        let mut sizes = Vec::new();
        flatten(shape, value, 0, &mut leaves, &mut sizes)?;
        let _ = write!(self.out, "    <{name}");
        let unfixed = shape
            .levels
            .iter()
            .skip(1)
            .any(|level| level.fixed.is_none());
        if unfixed {
            // The inner sizes the schema does not fix: only a rectangular
            // nesting has them, and an empty one has none to give.
            if outer.is_empty() || sizes.len() != shape.levels.len() || sizes.contains(&None) {
                return Err(unrepresentable(format!(
                    "the {} of uneven or empty inner lists: arraySize gives one size per level",
                    shape.describe()
                )));
            }
            let sizes: Vec<String> = sizes.iter().flatten().map(usize::to_string).collect();
            // A global attribute of the XSD, so qualified.
            let _ = write!(self.out, " ifc:arraySize=\"{}\"", sizes.join(" "));
        }
        if leaves.is_empty() {
            self.out.push_str("/>\n");
            return Ok(());
        }
        self.out.push_str(">\n");
        let leaf = shape.inner_leaf();
        for item in leaves {
            self.item(&leaf, item, 3)?;
        }
        let _ = writeln!(self.out, "    </{name}>");
        Ok(())
    }

    /// One item of a container or SELECT: an entity reference element named
    /// by the entity's type, or a `-wrapper` value.
    fn item(&mut self, shape: &Shape, value: &Value, depth: usize) -> Result<(), XmlError> {
        let pad = "  ".repeat(depth);
        match (&shape.leaf, value) {
            (Leaf::Entity(_) | Leaf::Select(_), Value::Ref(id)) => {
                let type_name = self.type_name(*id)?;
                let _ = writeln!(
                    self.out,
                    "{pad}<{type_name} ref=\"i{}\" xsi:nil=\"true\"/>",
                    id.0
                );
            }
            (Leaf::Select(_), Value::Typed { type_name, value }) => {
                let definition = self
                    .schema
                    .type_def(type_name)
                    .ok_or_else(|| mismatch(shape, value))?;
                let inner = typing::type_shape(self.schema, &definition.name)?;
                self.wrapper(&definition.name, &inner, value, depth)?;
            }
            (leaf, _) if leaf.is_simple() => {
                // An aggregate of a defined type: the wrapper is the bare item.
                let defined = self
                    .schema
                    .type_def(&shape.named)
                    .is_some_and(|definition| *definition.name == *shape.named);
                if !defined {
                    return Err(unrepresentable(format!(
                        "an item of `{}`, which has no -wrapper element",
                        shape.named
                    )));
                }
                let named = shape.named.clone();
                self.wrapper(&named, shape, value, depth)?;
            }
            _ => return Err(mismatch(shape, value)),
        }
        Ok(())
    }

    /// A `<T-wrapper>` holding a value of the defined type `T`.
    fn wrapper(
        &mut self,
        type_name: &str,
        shape: &Shape,
        value: &Value,
        depth: usize,
    ) -> Result<(), XmlError> {
        let pad = "  ".repeat(depth);
        let content = if shape.levels.is_empty() && shape.leaf.is_simple() {
            text::scalar(&shape.leaf, value)?
        } else if shape.leaf.is_simple() {
            tokens(shape, value)?
        } else if matches!(shape.leaf, Leaf::Entity(_)) && shape.levels.len() == 1 {
            // An aggregate defined type of entities, such as
            // `IfcPropertySetDefinitionSet`: its items are references.
            let Value::List(items) = value else {
                return Err(mismatch(shape, value));
            };
            let _ = writeln!(self.out, "{pad}<{type_name}-wrapper>");
            let leaf = shape.inner_leaf();
            for item in items {
                self.item(&leaf, item, depth + 1)?;
            }
            let _ = writeln!(self.out, "{pad}</{type_name}-wrapper>");
            return Ok(());
        } else {
            return Err(unrepresentable(format!(
                "a value of `{type_name}` ({}) in a -wrapper",
                shape.describe()
            )));
        };
        let _ = write!(self.out, "{pad}<{type_name}-wrapper>");
        escape(&mut self.out, &content);
        let _ = writeln!(self.out, "</{type_name}-wrapper>");
        Ok(())
    }

    /// An element named for its place, referring to entity `id`: with
    /// `xsi:type` when the entity's type is not the declared one, which the
    /// XSD requires when the declared type is abstract.
    pub(super) fn reference(
        &mut self,
        name: &str,
        declared: &str,
        id: EntityId,
        depth: usize,
    ) -> Result<(), XmlError> {
        let pad = "  ".repeat(depth);
        let type_name = self.type_name(id)?;
        let _ = write!(self.out, "{pad}<{name}");
        if !type_name.eq_ignore_ascii_case(declared) {
            let _ = write!(self.out, " xsi:type=\"{type_name}\"");
        }
        let _ = writeln!(self.out, " ref=\"i{}\" xsi:nil=\"true\"/>", id.0);
        Ok(())
    }

    /// The element name of a referenced entity: its type as the schema
    /// spells it.
    fn type_name(&self, id: EntityId) -> Result<std::sync::Arc<str>, XmlError> {
        self.target(id).map(|target| target.layout.name.clone())
    }
}

fn mismatch(shape: &Shape, value: &Value) -> XmlError {
    XmlError::TypeMismatch {
        declared: shape.describe(),
        found: format!("{value:?}"),
    }
}
