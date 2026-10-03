//! The XSD reader's configuration rules, checked against the release XSDs.
//!
//! The reader derives each attribute's XML form from the EXPRESS schema
//! (simple values are XML attributes, entities and SELECTs child elements,
//! and so on). This walks every entity type the normative XSD declares and
//! checks the two agree: every XML attribute and child element the XSD
//! declares is one the reader recognizes, in the form the reader expects,
//! and every explicit attribute the schema declares has its XSD
//! counterpart. The XSDs are CC BY-ND 4.0 and not committed:
//! `scripts/fetch-ifc-schemas.sh` fetches them, and with
//! `IFC_SPEC_REQUIRED` set a missing file fails rather than skips.

use super::config::{self, InverseForm};
use crate::typing::{Items, Layouts, Leaf, XsdForm};
use ifc_schema::Schema;
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use std::collections::HashMap;
use std::path::PathBuf;

/// One element of the parsed XSD: its local name, attributes and children.
#[derive(Debug, Default)]
struct Node {
    name: String,
    attributes: Vec<(String, String)>,
    children: Vec<Node>,
}

impl Node {
    fn attribute(&self, name: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }

    fn child(&self, name: &str) -> Option<&Node> {
        self.children.iter().find(|child| child.name == name)
    }

    fn descendant(&self, name: &str) -> Option<&Node> {
        self.children.iter().find_map(|child| {
            (child.name == name)
                .then_some(child)
                .or_else(|| child.descendant(name))
        })
    }
}

fn parse(bytes: &[u8]) -> Node {
    let mut reader = Reader::from_reader(bytes);
    let mut stack = vec![Node::default()];
    let mut buf = Vec::new();
    let local = |name: &str| name.rsplit(':').next().unwrap_or(name).to_string();
    loop {
        match reader.read_event_into(&mut buf).expect("well-formed XSD") {
            Event::Eof => break,
            Event::Start(element) => {
                stack.push(node(&element, local));
            }
            Event::Empty(element) => {
                let leaf = node(&element, local);
                stack.last_mut().expect("parent").children.push(leaf);
            }
            Event::End(_) => {
                let done = stack.pop().expect("open element");
                stack.last_mut().expect("parent").children.push(done);
            }
            _ => {}
        }
        buf.clear();
    }
    stack.pop().expect("document")
}

fn node(element: &quick_xml::events::BytesStart<'_>, local: impl Fn(&str) -> String) -> Node {
    Node {
        name: local(element.name().as_ref()),
        attributes: element
            .attributes()
            .map(|attribute| {
                let attribute = attribute.expect("attribute");
                (
                    attribute.key.as_ref().to_string(),
                    attribute
                        .normalized_value(quick_xml::XmlVersion::Implicit1_0)
                        .expect("value")
                        .into_owned(),
                )
            })
            .collect(),
        children: Vec::new(),
    }
}

/// The element names of an entity complex type's content model, resolved
/// through its base types: an extension appends to its base's content, a
/// restriction restates it.
fn content(complex_types: &HashMap<&str, &Node>, name: &str) -> Vec<String> {
    let Some(complex) = complex_types.get(name) else {
        return Vec::new();
    };
    let Some(derivation) = complex.child("complexContent").and_then(|content| {
        content
            .child("extension")
            .or_else(|| content.child("restriction"))
    }) else {
        return Vec::new();
    };
    let mut out = if derivation.name == "extension" {
        let base = derivation.attribute("base").unwrap_or_default();
        content(complex_types, base.trim_start_matches("ifc:"))
    } else {
        Vec::new()
    };
    if let Some(sequence) = derivation.child("sequence") {
        out.extend(
            sequence
                .children
                .iter()
                .filter(|child| child.name == "element")
                .filter_map(|child| child.attribute("name"))
                .map(str::to_string),
        );
    }
    out
}

/// The fetched XSD, or `None` (a skip) when absent and not required.
fn xsd(release: &str, file: &str) -> Option<Vec<u8>> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for base in [
        "../../references/ifc-spec",
        "../../../../references/ifc-spec",
    ] {
        let path = root.join(base).join(release).join(file);
        if let Ok(bytes) = std::fs::read(&path) {
            return Some(bytes);
        }
    }
    assert!(
        std::env::var_os("IFC_SPEC_REQUIRED").is_none(),
        "IFC_SPEC_REQUIRED is set but references/ifc-spec/{release}/{file} was not found; \
         run scripts/fetch-ifc-schemas.sh"
    );
    eprintln!("skipped: references/ifc-spec/{release}/{file} not present");
    None
}

/// Check every entity type of one XSD; returns how many were checked.
fn check(schema: &Schema, document: &Node) -> usize {
    let top = document.child("schema").expect("XSD root");
    let mut layouts = Layouts::new(schema);
    let mut failures = Vec::new();
    let simple_types: HashMap<&str, &Node> = top
        .children
        .iter()
        .filter(|child| child.name == "simpleType")
        .filter_map(|child| Some((child.attribute("name")?, child)))
        .collect();
    // A named attribute type is a list when it, or the `List-` simple type
    // a complex type of that name extends (IFC4X3 `RefLatitude`), is one.
    let complex_types: HashMap<&str, &Node> = top
        .children
        .iter()
        .filter(|child| child.name == "complexType")
        .filter_map(|child| Some((child.attribute("name")?, child)))
        .collect();
    let names_list = |named: &str| {
        let named = named.trim_start_matches("ifc:");
        let base = complex_types
            .get(named)
            .and_then(|complex| complex.descendant("extension"))
            .and_then(|extension| extension.attribute("base"))
            .map(|base| base.trim_start_matches("ifc:"));
        [Some(named), base]
            .into_iter()
            .flatten()
            .filter_map(|name| simple_types.get(name))
            .any(|simple| simple.descendant("list").is_some())
    };
    let mut checked = 0;
    for complex in top
        .children
        .iter()
        .filter(|child| child.name == "complexType")
    {
        let Some(name) = complex.attribute("name") else {
            continue;
        };
        let Some(definition) = schema.entity(name).filter(|entity| entity.name == name) else {
            continue;
        };
        let layout = layouts
            .entity(name, true)
            .expect("layout")
            .expect("declared entity");
        checked += 1;
        let Some(extension) = complex.descendant("extension") else {
            // A pure restriction: a subtype that only redeclares inherited
            // attributes DERIVE (`IfcMirroredProfileDef`) adds nothing.
            if complex.descendant("restriction").is_none() {
                failures.push(format!("{name}: neither extension nor restriction"));
            }
            continue;
        };
        let mut seen = Vec::new();
        for attribute in extension
            .children
            .iter()
            .filter(|child| child.name == "attribute")
        {
            let Some(attribute_name) = attribute.attribute("name") else {
                continue;
            };
            seen.push(attribute_name.to_string());
            let Some(slot) = layout.slot(attribute_name) else {
                failures.push(format!("{name}.{attribute_name}: XML attribute, no slot"));
                continue;
            };
            let shape = &layout.slots[slot].shape;
            let is_list = attribute.descendant("list").is_some()
                || attribute.attribute("type").is_some_and(names_list);
            let form = layout.slots[slot].form;
            if form != XsdForm::Attribute || is_list == shape.levels.is_empty() {
                failures.push(format!(
                    "{name}.{attribute_name}: XSD list={is_list}, reader shape {}",
                    shape.describe()
                ));
            }
        }
        let elements = extension
            .child("sequence")
            .map(|sequence| {
                sequence
                    .children
                    .iter()
                    .filter(|child| child.name == "element")
            })
            .into_iter()
            .flatten();
        for element in elements {
            let Some(element_name) = element.attribute("name") else {
                continue;
            };
            seen.push(element_name.to_string());
            let complex_type = element.child("complexType");
            let typed = element.attribute("type").is_some();
            let group = complex_type.is_some_and(|complex| complex.child("group").is_some());
            let sequence = complex_type.and_then(|complex| complex.child("sequence"));
            if let Some(slot) = layout.slot(element_name) {
                let shape = &layout.slots[slot].shape;
                let form = layout.slots[slot].form;
                let accepted = if typed {
                    matches!(form, XsdForm::Entity | XsdForm::Text)
                } else if group {
                    // A SELECT, or an aggregate of one: either way the items
                    // are the SELECT's entity elements and wrappers.
                    matches!(shape.leaf, Leaf::Select(_))
                        && (form == XsdForm::Select) == shape.levels.is_empty()
                } else if let Some(sequence) = sequence {
                    // A container: entity items, wrappers of the leaf type,
                    // or `Seq-` wrapped inner lists of a nested aggregate.
                    let item = sequence.child("element");
                    let wrapper = item
                        .and_then(|item| item.attribute("ref").or_else(|| item.attribute("name")))
                        .map(|item| item.trim_start_matches("ifc:"))
                        .unwrap_or_default();
                    let expected = if form == XsdForm::Container(Items::Seq) {
                        format!("Seq-{}-wrapper", shape.named)
                    } else {
                        format!("{}-wrapper", shape.named)
                    };
                    matches!(form, XsdForm::Container(_))
                        && (!shape.leaf.is_simple() || wrapper == expected)
                } else {
                    false
                };
                if !accepted {
                    failures.push(format!(
                        "{name}.{element_name}: element form the reader refuses for {}",
                        shape.describe()
                    ));
                }
                // A reference is an empty element with `xsi:nil`, so an
                // entity element must be nillable; and the writer refuses an
                // unset value wherever the XSD requires the element.
                if form == XsdForm::Entity && element.attribute("nillable") != Some("true") {
                    failures.push(format!(
                        "{name}.{element_name}: entity element not nillable"
                    ));
                }
                if element.attribute("minOccurs") != Some("0") && layout.slots[slot].optional {
                    failures.push(format!("{name}.{element_name}: required, but OPTIONAL"));
                }
            } else if let Some(inverse) = layout.inverse(element_name) {
                let direct = typed || complex_type.is_none() && element.attribute("ref").is_some();
                let configured = config::elements(schema, &layout).into_iter().find_map(
                    |element| match element {
                        config::Element::Inverse {
                            inverse: found,
                            form,
                        } if found.name == inverse.name => Some(form),
                        _ => None,
                    },
                );
                let expected = if direct {
                    InverseForm::Direct
                } else {
                    InverseForm::Container
                };
                if configured != Some(expected) {
                    failures.push(format!(
                        "{name}.{element_name}: inverse form {expected:?}, configured {configured:?}"
                    ));
                }
            } else {
                failures.push(format!(
                    "{name}.{element_name}: child element, no attribute"
                ));
            }
        }
        // Every explicit attribute this entity declares has an XSD form,
        // exactly unless the configuration leaves it off: the writer then
        // writes the relationship inside an inverse of the entity it names,
        // and the reader fills the attribute from it.
        for attribute in &definition.attributes {
            let derived = layout
                .slot(&attribute.name)
                .is_some_and(|slot| layout.slots[slot].derived);
            let omitted = config::omitted(schema, name, &attribute.name);
            if !derived && seen.contains(&attribute.name) == omitted {
                failures.push(format!(
                    "{name}.{}: in the XSD {}, configured omitted {omitted}",
                    attribute.name,
                    seen.contains(&attribute.name)
                ));
            }
        }
        // The child elements in content-model order, inherited ones first,
        // as the writer emits them.
        let declared = content(&complex_types, name);
        let configured: Vec<String> = config::elements(schema, &layout)
            .iter()
            .map(|element| match element {
                config::Element::Slot(slot) => layout.slots[*slot].name.to_string(),
                config::Element::Inverse { inverse, .. } => inverse.name.to_string(),
            })
            .collect();
        if declared != configured {
            failures.push(format!(
                "{name}: XSD content {declared:?}, writer order {configured:?}"
            ));
        }
    }
    // Every enumeration's XSD values are its EXPRESS members in lower case,
    // as the writer spells them.
    for simple in top
        .children
        .iter()
        .filter(|child| child.name == "simpleType")
    {
        let Some(name) = simple.attribute("name") else {
            continue;
        };
        let Some(ifc_schema::TypeKind::Enumeration(members)) = schema
            .type_def(name)
            .filter(|definition| definition.name == name)
            .map(|definition| &definition.kind)
        else {
            continue;
        };
        let values: Vec<String> = simple
            .descendant("restriction")
            .map(|restriction| {
                restriction
                    .children
                    .iter()
                    .filter_map(|child| child.attribute("value"))
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default();
        let expected: Vec<String> = members
            .iter()
            .map(|member| member.to_ascii_lowercase())
            .collect();
        if values != expected {
            failures.push(format!(
                "{name}: XSD values {values:?}, members {expected:?}"
            ));
        }
    }
    // Every typed wrapper names a defined type, which is what the reader
    // resolves it by.
    for element in top.children.iter().filter(|child| child.name == "element") {
        let Some(wrapper) = element
            .attribute("name")
            .and_then(|name| name.strip_suffix("-wrapper"))
        else {
            continue;
        };
        if schema
            .type_def(wrapper)
            .is_none_or(|definition| definition.name != wrapper)
        {
            failures.push(format!("{wrapper}-wrapper names no defined type"));
        }
    }
    assert!(
        failures.is_empty(),
        "{} disagreements between the XSD and the reader:\n{}",
        failures.len(),
        failures.join("\n")
    );
    checked
}

#[test]
fn ifc4_add2_tc1_xsd_matches_the_reader_configuration() {
    let Some(bytes) = xsd("ifc4-add2-tc1", "IFC4.xsd") else {
        return;
    };
    let checked = check(ifc_schema::ifc4(), &parse(&bytes));
    // Every entity IFC4 ADD2 TC1 declares: 776.
    assert!(checked >= 776, "checked only {checked} entity types");
}

#[test]
fn ifc4x3_add2_xsd_matches_the_reader_configuration() {
    let Some(bytes) = xsd("ifc4x3-add2", "IFC4X3_ADD2.xsd") else {
        return;
    };
    let checked = check(ifc_schema::ifc4x3(), &parse(&bytes));
    assert!(checked >= 876, "checked only {checked} entity types");
}
