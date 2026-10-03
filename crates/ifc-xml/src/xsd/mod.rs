//! The buildingSMART XSD configuration of ifcXML, read into the [`Model`]
//! the STEP form of the same document reads into, and written from it.
//!
//! Reading and writing share the resolved layouts of [`crate::typing`],
//! including the form each attribute takes ([`XsdForm`]); the writer adds
//! the configuration's choices EXPRESS does not fix (`config`) and its
//! own rules (`write`).
//!
//! # The configuration
//!
//! ISO 10303-28 maps EXPRESS to XML; the release XSD (`IFC4.xsd`,
//! `IFC4X3_ADD2.xsd`) fixes one configuration of that mapping. What this
//! reader implements, each rule checked against the XSD by
//! `conformance` (a schema-backed unit test):
//!
//! - **Simple attributes are XML attributes.** An attribute whose declared
//!   type is a simple type -- a measure, label, enumeration, boolean -- is an
//!   XML attribute on the entity element. An aggregate of simple values is a
//!   whitespace-separated list in one XML attribute, flattened when nested,
//!   unless its inner sizes are not fixed or the configuration says
//!   otherwise (`AddressLines`): then it is a container of `-wrapper`
//!   values. The reader reads such an aggregate from either form.
//! - **Entity-typed attributes are child elements that *are* the entity.**
//!   `<OwnerHistory id="i1" ...>` defines the `IfcOwnerHistory` in place;
//!   `xsi:type` names a subtype. `<OwnerHistory ref="i1" xsi:nil="true"/>`
//!   (or `href`) refers to one defined elsewhere.
//! - **SELECT and aggregate attributes are containers.** Their child elements
//!   are the items: entity elements named by their type, references, or
//!   `<IfcLabel-wrapper>`-style typed values. A wrapper in a SELECT is the
//!   STEP typed parameter `IFCLABEL(...)`; a wrapper in an aggregate of a
//!   defined type is the bare value. A nested aggregate is flattened, with
//!   `ifc:arraySize` for the inner sizes the schema does not fix (a global
//!   attribute of the XSD, so namespace-qualified; read unqualified too), or
//!   written as `Seq-...-wrapper` inner lists.
//! - **Inverse attributes are child elements too.** `<IsDecomposedBy>` holds
//!   the `IfcRelAggregates` whose `RelatingObject` is the enclosing entity.
//!   The relationship omits that attribute; reading the inverse supplies it.
//! - **Entities nest anywhere and are numbered in document order.** Every
//!   entity element, at the root or in place, becomes one model entity, ids
//!   `#1`, `#2`, ... in start-tag order. `id` attributes are only reference
//!   targets.
//!
//! - **A derived slot reads as derived.** A redeclared `DERIVE` attribute is
//!   `*` in STEP. The XSD still admits the supertype's XML attribute for it
//!   (an XSD restriction inherits attributes), so a value there is typed
//!   and discarded; as a child element it is refused.
//!
//! - **The configuration leaves some attributes off.** It writes 23 inverse
//!   attributes and leaves 21 explicit attributes off their relationship
//!   (`config`); the reader fills those from the inverse, the writer
//!   writes them into it.
//!
//! Content outside those rules is refused with a typed [`XmlError`], never
//! read as a different model: an attribute the entity does not declare, a
//! value its declared type does not admit, a nested aggregate whose inner
//! sizes neither the schema nor an `arraySize` fixes, `pos`/`path` addressing,
//! external `href`s.

mod config;
#[cfg(test)]
mod conformance;
mod finish;
mod item;
mod open;
mod state;
mod write;

pub(crate) use write::write;

use crate::error::XmlError;
use crate::typing::{self, EntityLayout, InverseLayout, Layouts, Leaf, Lexical, Shape, XsdForm};
use crate::XmlProfile;
use ifc_model::{Entity, EntityId, Model, Value};
use ifc_schema::{AggregateKind, Schema};
use quick_xml::escape::unescape;
use quick_xml::events::{BytesStart, Event};
use quick_xml::name::{QName, ResolveResult};
use quick_xml::reader::NsReader;
use quick_xml::XmlVersion;
use std::collections::HashMap;
use std::sync::Arc;

const XSI: &str = "http://www.w3.org/2001/XMLSchema-instance";

/// Temporary numbers for ids referenced before their element is seen.
const PLACEHOLDER: u64 = 1 << 62;

/// Parse an XSD-configuration document of `profile`'s release.
pub(crate) fn read(schema: &Schema, profile: XmlProfile, bytes: &[u8]) -> Result<Model, XmlError> {
    if schema.version() != Some(profile.version()) {
        return Err(XmlError::SchemaMismatch {
            profile: profile.schema_token(),
            schema: schema.name().into(),
        });
    }
    let mut state = State::new(schema, profile);
    let mut reader = NsReader::from_reader(bytes);
    let mut buf = Vec::new();
    loop {
        let event = reader
            .read_resolved_event_into(&mut buf)
            .map_err(|error| XmlError::Malformed(error.to_string()).at(state.path()))?;
        match event {
            (_, Event::Eof) => break,
            (namespace, Event::Start(element)) => {
                let namespace = owned_namespace(namespace);
                state.open(&reader, namespace, &element)?;
            }
            (namespace, Event::Empty(element)) => {
                let namespace = owned_namespace(namespace);
                state.open(&reader, namespace, &element)?;
                state.close()?;
            }
            (_, Event::End(_)) => state.close()?,
            (_, Event::Text(text)) => {
                let text = text.replace("\r\n", "\n").replace('\r', "\n");
                state.text(&text)?;
            }
            (_, Event::CData(data)) => {
                let text: &str = data.as_ref();
                state.text(text)?;
            }
            (_, Event::GeneralRef(reference)) => {
                let reference = format!("&{};", &*reference);
                let resolved = unescape(&reference)
                    .map_err(|error| XmlError::Malformed(error.to_string()).at(state.path()))?;
                state.text(&resolved)?;
            }
            (_, Event::DocType(_)) => {
                return Err(unsupported("a document type declaration"));
            }
            _ => {}
        }
        buf.clear();
    }
    state.finish()
}

fn unsupported(construct: &str) -> XmlError {
    XmlError::Unsupported {
        construct: construct.into(),
    }
}

/// One entity being read: its resolved layout and its slots so far.
///
/// Its location is kept as the enclosing entity's number and the path from
/// there, not as a full path: a path per entity would cost memory in
/// proportion to nesting depth, and is only needed for an error.
struct Pending {
    layout: Arc<EntityLayout>,
    slots: Vec<Option<Value>>,
    /// The enclosing entity, or 0 at the root.
    parent: u64,
    /// The path from the enclosing entity (or the root) to this element.
    relative: Box<str>,
}

/// An inverse attribute's implied value, applied once every entity is read.
struct Backfill {
    relationship: u64,
    parent: u64,
    inverse: InverseLayout,
}

/// A type an element name or `xsi:type` claims for a referenced entity.
struct Claim {
    target: u64,
    claimed: Arc<str>,
    /// The entity whose element holds the reference, or 0 at the root.
    referrer: u64,
}

/// Where a finished value goes.
enum Target {
    /// An explicit attribute slot of an entity.
    Slot { entity: u64, slot: usize },
    /// The next item of the enclosing container.
    Item,
    /// Relationships whose inverted attribute names `parent`.
    Inverse { parent: u64, inverse: InverseLayout },
}

/// An element whose content is a value: a container or a wrapper.
struct ValueFrame {
    target: Target,
    shape: Shape,
    /// Text content (a wrapper of a simple type) rather than child elements.
    text_content: bool,
    /// A SELECT container: exactly one item.
    single: bool,
    /// Wrap the result as this typed parameter.
    wrap: Option<Arc<str>>,
    items: Vec<Value>,
    /// Items that are whole inner lists (`Seq-...-wrapper`), not flat leaves.
    nested_items: Option<bool>,
    text: String,
    array_size: Option<Vec<usize>>,
    attribute: Arc<str>,
}

enum Frame {
    Root,
    Header,
    HeaderField {
        name: String,
        text: String,
    },
    Entity {
        number: u64,
    },
    Value(Box<ValueFrame>),
    /// A reference or nil element: nothing may follow inside it.
    Empty,
}

struct Segment {
    frame: Frame,
    segment: String,
}

/// The attributes of one start tag, sorted by role.
#[derive(Default)]
struct Attributes {
    /// Unprefixed attributes, in document order, values normalized.
    plain: Vec<(String, String)>,
    xsi_type: Option<String>,
    xsi_nil: Option<bool>,
    xsi_schema_location: bool,
}

impl Attributes {
    fn take(&mut self, name: &str) -> Option<String> {
        let index = self.plain.iter().position(|(key, _)| key == name)?;
        Some(self.plain.remove(index).1)
    }
}

struct State<'s> {
    schema: &'s Schema,
    profile: XmlProfile,
    layouts: Layouts<'s>,
    model: Model,
    entities: Vec<Pending>,
    ids: HashMap<String, u64>,
    forward: HashMap<String, u64>,
    forward_names: Vec<String>,
    backfills: Vec<Backfill>,
    claims: Vec<Claim>,
    stack: Vec<Segment>,
    seen_root: bool,
}

/// A resolved element namespace, owned so the reader can be used again.
fn owned_namespace(namespace: ResolveResult<'_>) -> Option<String> {
    match namespace {
        ResolveResult::Bound(namespace) => Some(namespace.as_ref().to_owned()),
        ResolveResult::Unbound => None,
        ResolveResult::Unknown(prefix) => Some(format!("unresolved prefix `{prefix}`")),
    }
}

/// An `href` to an element of this document: `i1` or `#i1`.
fn local_href(href: &str) -> Result<String, XmlError> {
    let href = href.trim();
    let local = href.strip_prefix('#').unwrap_or(href);
    if local.is_empty() || local.contains(['#', '/', ':', '?']) {
        return Err(XmlError::Unsupported {
            construct: format!("the external reference href={href:?}"),
        });
    }
    Ok(local.into())
}

const fn index(number: u64) -> usize {
    (number - 1) as usize
}

fn rewrite(value: &mut Value, resolve: &impl Fn(u64) -> u64) {
    match value {
        Value::Ref(EntityId(number)) => *number = resolve(*number),
        Value::List(items) => items.iter_mut().for_each(|item| rewrite(item, resolve)),
        Value::Typed { value, .. } => rewrite(value, resolve),
        _ => {}
    }
}
