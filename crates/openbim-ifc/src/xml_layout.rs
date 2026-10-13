//! The ifcXML layout a document declares, read from its root start tag.
//!
//! The native layout's root element carries a `schema` attribute; an
//! XSD-configuration document declares its release's namespace. A reader
//! that is handed a document without being told its layout (an ifcZIP
//! entry, the command line's `--input-layout auto`) asks this module rather
//! than guessing from the first bytes it fails on.

#![cfg(feature = "ifcxml")]

use ifc_xml::XmlProfile;

/// The layout an ifcXML document's root element declares.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum DeclaredXmlLayout {
    /// This library's native layout: the root carries a `schema` attribute,
    /// the header token of the release it was written against.
    Native {
        /// The `schema` attribute's value, e.g. `IFC4`.
        schema: String,
        /// Whether attributes are named by position (`a0`, `a1`, ...), as
        /// the codec writes them without schema tables, rather than after
        /// the schema. A strict schema-aware read refuses positional names;
        /// a lenient one resolves both.
        positional: bool,
    },
    /// The buildingSMART XSD configuration of the release whose namespace
    /// the root declares.
    Xsd(XmlProfile),
    /// Neither: no `schema` attribute and no known release namespace.
    Undeclared,
}

/// The layout `bytes` declares in its root start tag.
///
/// A `schema` attribute wins over a namespace, because the native layout
/// written under a release profile declares that release's namespace too.
#[must_use]
pub fn declared_xml_layout(bytes: &[u8]) -> DeclaredXmlLayout {
    let root = root_tag(bytes);
    if let Some(schema) = attribute(&root, "schema") {
        return DeclaredXmlLayout::Native {
            schema,
            positional: uses_positional_names(bytes),
        };
    }
    [XmlProfile::Ifc4Add2Tc1, XmlProfile::Ifc4x3Add2]
        .into_iter()
        .find(|profile| {
            profile
                .namespaces()
                .iter()
                .any(|namespace| root.contains(&format!("\"{namespace}\"")))
        })
        .map_or(DeclaredXmlLayout::Undeclared, DeclaredXmlLayout::Xsd)
}

/// Does the document name an attribute `a<digits>`, as an XML attribute
/// (` a0="`) or a child element (`<a0>`)? The native writer names every
/// attribute that way when it has no schema names for the entity's type,
/// and never otherwise: schema names that look positional are not used.
fn uses_positional_names(bytes: &[u8]) -> bool {
    bytes.windows(2).enumerate().any(|(at, pair)| {
        let opens = pair[0] == b'<' || pair[0].is_ascii_whitespace();
        if !opens || pair[1] != b'a' {
            return false;
        }
        let rest = &bytes[at + 2..];
        let digits = rest.iter().take_while(|byte| byte.is_ascii_digit()).count();
        digits > 0
            && rest
                .get(digits)
                .is_some_and(|end| matches!(end, b'=' | b'>' | b'/') || end.is_ascii_whitespace())
    })
}

/// The text of the document's root start tag, from `<` to `>`.
fn root_tag(bytes: &[u8]) -> String {
    let head = String::from_utf8_lossy(&bytes[..bytes.len().min(64 * 1024)]);
    let mut rest = head.as_ref();
    while let Some(start) = rest.find('<') {
        let after = &rest[start + 1..];
        // Declarations, processing instructions and comments come first.
        if after.starts_with('?') || after.starts_with('!') {
            let end = if after.starts_with("!--") {
                after.find("-->").map(|end| end + 3)
            } else {
                after.find('>').map(|end| end + 1)
            };
            match end {
                Some(end) => rest = &after[end..],
                None => return String::new(),
            }
            continue;
        }
        return after
            .find('>')
            .map_or(after, |end| &after[..end])
            .to_owned();
    }
    String::new()
}

/// The value of attribute `name` in a start tag.
fn attribute(tag: &str, name: &str) -> Option<String> {
    let mut rest = tag;
    while let Some(at) = rest.find(name) {
        let before = rest[..at].chars().next_back();
        let after = rest[at + name.len()..].trim_start();
        if before.is_some_and(char::is_whitespace) {
            if let Some(value) = after.strip_prefix('=') {
                let value = value.trim_start();
                let quote = value.chars().next()?;
                if quote == '"' || quote == '\'' {
                    let body = &value[1..];
                    return body.find(quote).map(|end| body[..end].to_owned());
                }
            }
        }
        rest = &rest[at + name.len()..];
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_root_tag_skips_the_prolog() {
        let tag =
            root_tag(b"<?xml version=\"1.0\"?>\n<!-- c > d -->\n<ifcXML schema=\"IFC4\" a='b'>");
        assert_eq!(tag, "ifcXML schema=\"IFC4\" a='b'");
        assert_eq!(attribute(&tag, "schema").as_deref(), Some("IFC4"));
        assert_eq!(attribute(&tag, "a").as_deref(), Some("b"));
        assert_eq!(attribute("x xschema=\"1\"", "schema"), None);
    }

    #[test]
    fn the_layout_follows_the_schema_attribute_then_the_namespace() {
        assert_eq!(
            declared_xml_layout(b"<ifcXML schema=\"IFC2X3\">"),
            DeclaredXmlLayout::Native {
                schema: "IFC2X3".to_owned(),
                positional: false
            }
        );
        assert_eq!(
            declared_xml_layout(b"<ifcXML schema=\"IFC4\">\n  <IFCWALL id=\"i1\" a0=\"x\"/>"),
            DeclaredXmlLayout::Native {
                schema: "IFC4".to_owned(),
                positional: true
            }
        );
        assert!(uses_positional_names(b"<E>\n<a12>\n"));
        assert!(!uses_positional_names(b"<E Name=\"a1\" area=\"2\"/>"));
        let xsd = format!(
            "<ifc:ifcXML xmlns:ifc=\"{}\">",
            XmlProfile::Ifc4x3Add2.namespace()
        );
        assert_eq!(
            declared_xml_layout(xsd.as_bytes()),
            DeclaredXmlLayout::Xsd(XmlProfile::Ifc4x3Add2)
        );
        assert_eq!(
            declared_xml_layout(b"<ifcXML>"),
            DeclaredXmlLayout::Undeclared
        );
    }
}
