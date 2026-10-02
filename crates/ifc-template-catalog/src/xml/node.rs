//! Small internal XML tree used by both PSD and QTO decoders.

use std::collections::BTreeMap;

use quick_xml::escape::unescape;
use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;

use super::XmlImportError;

#[derive(Debug)]
pub(super) struct Node {
    pub name: String,
    pub attributes: BTreeMap<String, String>,
    pub text: String,
    pub children: Vec<Node>,
}

impl Node {
    pub fn child(&self, name: &str) -> Option<&Node> {
        self.children.iter().find(|child| child.name == name)
    }

    pub fn children_named<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a Node> {
        self.children.iter().filter(move |child| child.name == name)
    }

    pub fn text_trimmed(&self) -> Option<&str> {
        let value = self.text.trim();
        (!value.is_empty()).then_some(value)
    }

    pub fn child_text(&self, name: &str) -> Option<&str> {
        self.child(name).and_then(Node::text_trimmed)
    }

    pub fn attribute(&self, name: &str) -> Option<&str> {
        self.attributes.get(name).map(String::as_str)
    }
}

pub(super) fn parse(xml: &str, limits: super::ImportLimits) -> Result<Node, XmlImportError> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(false);
    let mut stack = Vec::new();
    let mut root = None;
    let mut nodes = 0usize;

    loop {
        match reader.read_event() {
            Ok(Event::Start(start)) => {
                check_node(&limits, &mut nodes, stack.len() + 1)?;
                stack.push(new_node(&start)?);
            }
            Ok(Event::Empty(start)) => {
                check_node(&limits, &mut nodes, stack.len() + 1)?;
                let node = new_node(&start)?;
                append(node, &mut stack, &mut root)?;
            }
            // Text arrives verbatim and each entity or character reference
            // as its own event; concatenating both yields the unescaped run.
            Ok(Event::Text(text)) => {
                if let Some(parent) = stack.last_mut() {
                    parent.text.push_str(&text);
                }
            }
            Ok(Event::GeneralRef(reference)) => {
                // Resolve even outside an element so an unknown entity is
                // refused wherever it appears.
                let resolved = unescape(&format!("&{};", &*reference))
                    .map(std::borrow::Cow::into_owned)
                    .map_err(|error| XmlImportError::Xml(error.to_string()))?;
                if let Some(parent) = stack.last_mut() {
                    parent.text.push_str(&resolved);
                }
            }
            Ok(Event::CData(text)) => {
                if let Some(parent) = stack.last_mut() {
                    parent.text.push_str(&text);
                }
            }
            Ok(Event::End(_)) => {
                let node = stack
                    .pop()
                    .ok_or_else(|| XmlImportError::Xml("unexpected closing tag".into()))?;
                append(node, &mut stack, &mut root)?;
            }
            Ok(Event::DocType(_)) => {
                return Err(XmlImportError::Xml(
                    "document type declarations are not supported".into(),
                ));
            }
            Ok(Event::Eof) => break,
            Ok(_) => {}
            Err(error) => return Err(XmlImportError::Xml(error.to_string())),
        }
    }

    if !stack.is_empty() {
        return Err(XmlImportError::Xml("unclosed XML element".into()));
    }
    root.ok_or(XmlImportError::MissingRoot)
}

fn check_node(
    limits: &super::ImportLimits,
    nodes: &mut usize,
    depth: usize,
) -> Result<(), XmlImportError> {
    *nodes += 1;
    if *nodes > limits.max_nodes {
        return Err(XmlImportError::LimitExceeded {
            kind: "nodes",
            limit: limits.max_nodes,
        });
    }
    if depth > limits.max_depth {
        return Err(XmlImportError::LimitExceeded {
            kind: "depth",
            limit: limits.max_depth,
        });
    }
    Ok(())
}

fn new_node(start: &BytesStart<'_>) -> Result<Node, XmlImportError> {
    let name = local_name(start.name().as_ref());
    let mut attributes = BTreeMap::new();
    for attribute in start.attributes() {
        let attribute = attribute.map_err(|error| XmlImportError::Xml(error.to_string()))?;
        let key = local_name(attribute.key.as_ref());
        // Entity and character references only, as before: attribute-value
        // normalisation (`normalized_value`) would turn tabs and line breaks
        // in definitions and descriptions into spaces.
        let value =
            unescape(&attribute.value).map_err(|error| XmlImportError::Xml(error.to_string()))?;
        attributes.insert(key, value.into_owned());
    }
    Ok(Node {
        name,
        attributes,
        text: String::new(),
        children: Vec::new(),
    })
}

fn append(node: Node, stack: &mut [Node], root: &mut Option<Node>) -> Result<(), XmlImportError> {
    if let Some(parent) = stack.last_mut() {
        parent.children.push(node);
    } else if root.replace(node).is_some() {
        return Err(XmlImportError::MultipleRoots);
    }
    Ok(())
}

fn local_name(name: &str) -> String {
    name.rsplit(':').next().unwrap_or(name).to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// References are replaced and nothing else changes: literal tabs and
    /// line breaks survive in attribute values and text, and CDATA is kept
    /// verbatim. Pins the values across quick-xml upgrades.
    #[test]
    fn unescapes_references_and_keeps_literal_whitespace() {
        let xml = "<Root note=\"tab\there\nline\r\nend &amp; &lt;&gt;&quot;&apos; &#x41;&#66;\">\
                   <Text>tab\there\nline\r\nend &amp; &lt;&gt;&quot;&apos; &#x41;&#66;</Text>\
                   <Raw><![CDATA[raw &amp; <kept>]]></Raw></Root>";
        let root = parse(xml, super::super::ImportLimits::default()).unwrap();
        let expected = "tab\there\nline\r\nend & <>\"' AB";
        assert_eq!(root.attribute("note"), Some(expected));
        assert_eq!(
            root.child("Text").map(|node| node.text.as_str()),
            Some(expected)
        );
        assert_eq!(
            root.child("Raw").map(|node| node.text.as_str()),
            Some("raw &amp; <kept>")
        );
    }

    #[test]
    fn refuses_undeclared_entities() {
        for xml in ["<Root a=\"&bogus;\"/>", "<Root>&bogus;</Root>"] {
            let error = parse(xml, super::super::ImportLimits::default())
                .expect_err("an undeclared entity is refused, not passed through");
            assert!(error.to_string().contains("bogus"), "{error}");
        }
    }
}
