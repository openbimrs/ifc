//! Cross-document element references.

/// A reference to one element, optionally inside a named document.
///
/// Cross-document referencing is the shared shape behind BCF viewpoint
/// components and ICDD linkset endpoints. Both name a document and an element
/// within it; only the vocabulary differs.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ElementRef {
    /// The document the element lives in, when the reference crosses one.
    ///
    /// `None` means "the current document" — an intra-document reference.
    pub document: Option<String>,
    /// The element's identifier within that document, verbatim.
    ///
    /// Kept as an opaque string on purpose: an IFC `GlobalId`, an ICDD URI and
    /// a BCF component GUID are not the same syntax, and normalising them here
    /// would lose information that only the owning standard can interpret.
    pub id: String,
}

impl ElementRef {
    /// A reference within the current document.
    #[must_use]
    pub fn local(id: impl Into<String>) -> Self {
        Self {
            document: None,
            id: id.into(),
        }
    }

    /// A reference into another document.
    #[must_use]
    pub fn in_document(document: impl Into<String>, id: impl Into<String>) -> Self {
        Self {
            document: Some(document.into()),
            id: id.into(),
        }
    }

    /// Whether this reference crosses a document boundary.
    #[must_use]
    pub fn is_cross_document(&self) -> bool {
        self.document.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn element_refs_distinguish_scope() {
        let local = ElementRef::local("a");
        assert_eq!(local.document, None);
        assert!(!local.is_cross_document());

        let remote = ElementRef::in_document("d.ifc", "a");
        assert_eq!(remote.document.as_deref(), Some("d.ifc"));
        assert!(remote.is_cross_document());
    }

    #[test]
    fn same_id_in_different_documents_is_not_the_same_element() {
        assert_ne!(
            ElementRef::in_document("a.ifc", "x"),
            ElementRef::in_document("b.ifc", "x")
        );
        assert_ne!(
            ElementRef::local("x"),
            ElementRef::in_document("a.ifc", "x")
        );
    }
}
