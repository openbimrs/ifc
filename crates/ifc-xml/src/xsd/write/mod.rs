//! A [`Model`] written in the buildingSMART XSD configuration.
//!
//! The writer follows the rules the reader reads by, from the same
//! resolved layouts ([`crate::typing::Layouts`]) and the same form per
//! attribute ([`crate::typing::XsdForm`]), plus the configuration choices
//! EXPRESS does not fix ([`super::config`]):
//!
//! - **Every entity at the top level, in model order.** Each element is
//!   named by the entity's type as the schema spells it (`IfcWall`) and
//!   carries `id="i<n>"` from its model id. Reading numbers entities in
//!   document order, so a model numbered `#1`..`#n` in order reads back
//!   with the same ids; any other numbering reads back renumbered in order.
//! - **References, never nesting.** An entity-valued attribute, item or
//!   inverse is `<... ref="i<n>" xsi:nil="true"/>`: the element is nil so
//!   the XSD does not ask for the entity's own content.
//! - **Simple values are XML attributes**, lists of them whitespace
//!   separated and flattened; SELECT values, aggregates the configuration
//!   writes as containers, and binaries are child elements, in the order
//!   the XSD's content model declares them.
//! - **Omitted attributes go through inverses.** An attribute the
//!   configuration leaves off (`IfcRelAggregates.RelatingObject`) is
//!   written as a reference to the relationship inside the inverse element
//!   of the entity it names (`IsDecomposedBy`). A `SET` filled this way
//!   reads back in the document order of those entities.
//! - **The header** is the XSD's: one `name`, `time_stamp` (an
//!   `xs:dateTime`), `author`, `organization`, `preprocessor_version`,
//!   `originating_system`, `authorization`, and the description as
//!   `documentation`. The STEP implementation level has no ifcXML
//!   counterpart and is not written.
//!
//! What the configuration cannot carry exactly is refused with a typed
//! error, never written differently: a value the reader would read back as
//! another (an integer where a real is declared, a string with whitespace in
//! a list attribute, a partial-byte binary), a value the XSD refuses (a
//! string with a line break or over its declared width, an unset mandatory
//! element), and a relationship the configuration has no place for
//! (`IfcRelDefinesByObject.RelatingObject`, a second opening in the one
//! `HasOpenings` the XSD allows).

mod text;
mod value;

use super::config::{self, Element, InverseForm};
use crate::error::XmlError;
use crate::typing::{self, EntityLayout, Layouts, SlotLayout};
use crate::XmlProfile;
use ifc_model::{Entity, EntityId, Model, Value};
use ifc_schema::{AggregateKind, Schema};
use std::collections::HashMap;
use std::fmt::Write as _;
use std::sync::Arc;

const XSI: &str = "http://www.w3.org/2001/XMLSchema-instance";

/// Write `model` in the XSD configuration of `profile`'s release.
pub(crate) fn write(
    schema: &Schema,
    profile: XmlProfile,
    model: &Model,
) -> Result<Vec<u8>, XmlError> {
    if schema.version() != Some(profile.version()) {
        return Err(XmlError::SchemaMismatch {
            profile: profile.schema_token(),
            schema: schema.name().into(),
        });
    }
    let declared = &model.header().schema;
    if declared.len() != 1 || declared[0] != profile.schema_token() {
        return Err(XmlError::Profile {
            expected: profile.schema_token(),
            found: (!declared.is_empty()).then(|| declared.join(", ")),
        });
    }
    let mut writer = Writer::new(schema, model);
    writer.plan()?;
    writer.emit(profile)
}

/// One entity resolved for writing.
struct Planned<'m> {
    id: EntityId,
    entity: &'m Entity,
    layout: Arc<EntityLayout>,
    elements: Arc<[Element]>,
}

pub(super) struct Writer<'m> {
    schema: &'m Schema,
    model: &'m Model,
    layouts: Layouts<'m>,
    elements: HashMap<Arc<str>, Arc<[Element]>>,
    planned: Vec<Planned<'m>>,
    /// Model id to its index in `planned`.
    index: HashMap<EntityId, usize>,
    /// Relationships each `(entity index, element index)` inverse holds.
    placed: HashMap<(usize, usize), Vec<EntityId>>,
    out: String,
}

impl<'m> Writer<'m> {
    fn new(schema: &'m Schema, model: &'m Model) -> Self {
        Self {
            schema,
            model,
            layouts: Layouts::new(schema),
            elements: HashMap::new(),
            planned: Vec::with_capacity(model.len()),
            index: HashMap::with_capacity(model.len()),
            placed: HashMap::new(),
            out: String::with_capacity(model.len() * 160),
        }
    }

    /// Resolve every entity, check its values against its declaration, and
    /// place each omitted attribute's relationship in an inverse.
    fn plan(&mut self) -> Result<(), XmlError> {
        for (id, entity) in self.model.iter() {
            let path = || format!("/ifcXML/{}[@id='i{}']", entity.type_name, id.0);
            let layout = self
                .layouts
                .entity(&entity.type_name, false)
                .and_then(|layout| {
                    layout.ok_or_else(|| XmlError::UnknownEntity {
                        name: entity.type_name.to_string(),
                    })
                })
                .map_err(|error| error.at(path()))?;
            if layout.abstract_ {
                return Err(XmlError::AbstractEntity {
                    name: layout.name.to_string(),
                }
                .at(path()));
            }
            if entity.attributes.len() != layout.slots.len() {
                return Err(XmlError::TypeMismatch {
                    declared: format!("{} attributes of {}", layout.slots.len(), layout.name),
                    found: format!("{} values", entity.attributes.len()),
                }
                .at(path()));
            }
            let elements = match self.elements.get(&layout.name) {
                Some(elements) => elements.clone(),
                None => {
                    let elements: Arc<[Element]> = config::elements(self.schema, &layout).into();
                    self.elements.insert(layout.name.clone(), elements.clone());
                    elements
                }
            };
            self.index.insert(id, self.planned.len());
            self.planned.push(Planned {
                id,
                entity,
                layout,
                elements,
            });
        }
        for position in 0..self.planned.len() {
            self.check(position)?;
        }
        Ok(())
    }

    /// One entity's values: each fits its declaration, every reference
    /// resolves to an entity it admits, and omitted attributes are placed.
    fn check(&mut self, position: usize) -> Result<(), XmlError> {
        let planned = &self.planned[position];
        let (id, entity, layout) = (planned.id, planned.entity, planned.layout.clone());
        let element_path = format!("/ifcXML/{}[@id='i{}']", layout.name, id.0);
        let mut references = Vec::new();
        for (slot, (declared, value)) in layout.slots.iter().zip(&entity.attributes).enumerate() {
            let at = |error: XmlError| error.at(format!("{element_path}/{}", declared.name));
            if declared.derived {
                if *value != Value::Derived {
                    return Err(at(unrepresentable(format!(
                        "a value for `{}`, which {} derives",
                        declared.name, layout.name
                    ))));
                }
                continue;
            }
            if *value == Value::Derived {
                return Err(at(unrepresentable(format!(
                    "a derived (*) value for the explicit attribute `{}`",
                    declared.name
                ))));
            }
            typing::conform(self.schema, &declared.shape, value).map_err(at)?;
            value::counts(self.schema, &declared.shape, value).map_err(at)?;
            references.clear();
            typing::references(self.schema, &declared.shape, value, &mut references).map_err(at)?;
            for (target, admitted) in &references {
                let found = self.target(*target).map_err(at)?;
                if !self.schema.accepts_type(admitted, &found.layout.name) {
                    return Err(at(XmlError::TypeMismatch {
                        declared: admitted.to_string(),
                        found: format!("a reference to an entity `{}`", found.layout.name),
                    }));
                }
            }
            if config::omitted(self.schema, &layout.name, &declared.name) {
                self.place(position, slot, value).map_err(at)?;
            }
        }
        Ok(())
    }

    /// The planned entity a reference names.
    fn target(&self, id: EntityId) -> Result<&Planned<'m>, XmlError> {
        self.index
            .get(&id)
            .map(|index| &self.planned[*index])
            .ok_or_else(|| XmlError::UnresolvedReference {
                id: format!("i{}", id.0),
            })
    }

    /// Put the relationship `position` into the inverse of each entity its
    /// omitted attribute `slot` names.
    fn place(&mut self, position: usize, slot: usize, value: &Value) -> Result<(), XmlError> {
        let planned = &self.planned[position];
        let (relationship, layout) = (planned.id, planned.layout.clone());
        let declared: &SlotLayout = &layout.slots[slot];
        let holders: Vec<EntityId> = match value {
            Value::Null => return Ok(()),
            Value::Ref(holder) => vec![*holder],
            Value::List(items) => {
                let ordered = matches!(
                    declared.shape.levels.first().map(|level| level.kind),
                    Some(AggregateKind::List | AggregateKind::Array)
                );
                if ordered && items.len() > 1 {
                    return Err(unrepresentable(format!(
                        "the ordered `{}` of several entities, which an inverse cannot order",
                        declared.name
                    )));
                }
                let mut holders = Vec::with_capacity(items.len());
                for item in items {
                    let Value::Ref(holder) = item else {
                        return Err(unrepresentable(format!(
                            "an item of `{}` that is not an entity reference",
                            declared.name
                        )));
                    };
                    if holders.contains(holder) {
                        return Err(unrepresentable(format!(
                            "`{}` naming the entity #{} twice",
                            declared.name, holder.0
                        )));
                    }
                    holders.push(*holder);
                }
                holders
            }
            _ => {
                return Err(unrepresentable(format!(
                    "the value of `{}`, which the configuration writes only through an inverse",
                    declared.name
                )))
            }
        };
        for holder in holders {
            let index = self.index[&holder];
            let target = &self.planned[index];
            let found = target.elements.iter().position(|element| {
                matches!(element, Element::Inverse { inverse, .. }
                    if *inverse.for_attribute == *declared.name
                        && self.schema.is_a(&layout.name, &inverse.entity))
            });
            let Some(element) = found else {
                return Err(unrepresentable(format!(
                    "`{}.{}` naming an entity `{}`, which has no inverse element for it",
                    layout.name, declared.name, target.layout.name
                )));
            };
            let Element::Inverse { inverse, form } = &target.elements[element] else {
                continue;
            };
            let held = self.placed.entry((index, element)).or_default();
            if *form == InverseForm::Direct && !held.is_empty() {
                return Err(unrepresentable(format!(
                    "a second relationship in `{}.{}`, which the XSD allows once",
                    target.layout.name, inverse.name
                )));
            }
            held.push(relationship);
        }
        Ok(())
    }

    fn emit(mut self, profile: XmlProfile) -> Result<Vec<u8>, XmlError> {
        self.out
            .push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        let _ = writeln!(
            self.out,
            "<ifcXML xmlns=\"{0}\" xmlns:ifc=\"{0}\" xmlns:xsi=\"{XSI}\">",
            profile.namespace()
        );
        text::header(&mut self.out, self.model.header())?;
        for position in 0..self.planned.len() {
            self.entity(position)?;
        }
        self.out.push_str("</ifcXML>\n");
        Ok(self.out.into_bytes())
    }

    /// One top-level entity element.
    fn entity(&mut self, position: usize) -> Result<(), XmlError> {
        let planned = &self.planned[position];
        let (id, entity) = (planned.id, planned.entity);
        let layout = planned.layout.clone();
        let elements = planned.elements.clone();
        let path = format!("/ifcXML/{}[@id='i{}']", layout.name, id.0);
        let _ = write!(self.out, "  <{} id=\"i{}\"", layout.name, id.0);
        for (declared, value) in layout.slots.iter().zip(&entity.attributes) {
            if declared.derived || declared.form != typing::XsdForm::Attribute {
                continue;
            }
            let at = |error: XmlError| error.at(format!("{path}/@{}", declared.name));
            if let Some(text) = value::attribute(declared, value).map_err(at)? {
                self.out.push(' ');
                self.out.push_str(&declared.name);
                self.out.push_str("=\"");
                text::escape(&mut self.out, &text);
                self.out.push('"');
            }
        }
        let start = self.out.len();
        self.out.push_str(">\n");
        let content = self.out.len();
        for (index, element) in elements.iter().enumerate() {
            match element {
                Element::Slot(slot) => {
                    let declared = &layout.slots[*slot];
                    let value = &entity.attributes[*slot];
                    self.slot(declared, value)
                        .map_err(|error| error.at(format!("{path}/{}", declared.name)))?;
                }
                Element::Inverse { inverse, form } => {
                    if let Some(held) = self.placed.remove(&(position, index)) {
                        self.inverse(&inverse.name, &inverse.entity, *form, &held);
                    }
                }
            }
        }
        if self.out.len() == content {
            self.out.truncate(start);
            self.out.push_str("/>\n");
        } else {
            let _ = writeln!(self.out, "  </{}>", layout.name);
        }
        Ok(())
    }

    /// An inverse element holding references to its relationships.
    fn inverse(&mut self, name: &str, declared: &str, form: InverseForm, held: &[EntityId]) {
        match form {
            InverseForm::Direct => {
                for relationship in held {
                    // The holder's inverse resolved; its relationship is planned.
                    let _ = self.reference(name, declared, *relationship, 2);
                }
            }
            InverseForm::Container => {
                let _ = writeln!(self.out, "    <{name}>");
                for relationship in held {
                    let type_name = self.planned[self.index[relationship]].layout.name.clone();
                    let _ = writeln!(
                        self.out,
                        "      <{type_name} ref=\"i{}\" xsi:nil=\"true\"/>",
                        relationship.0
                    );
                }
                let _ = writeln!(self.out, "    </{name}>");
            }
        }
    }
}

fn unrepresentable(construct: String) -> XmlError {
    XmlError::Unrepresentable { construct }
}
