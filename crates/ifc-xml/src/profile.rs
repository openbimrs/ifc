//! Supported ifcXML release profiles.

/// A release-specific ifcXML namespace/profile contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum XmlProfile {
    /// IFC4 ADD2 TC1, as declared by the bundled official XSD.
    Ifc4Add2Tc1,
}

impl XmlProfile {
    /// The exact XML namespace declared by the release XSD.
    #[must_use]
    pub const fn namespace(self) -> &'static str {
        match self {
            Self::Ifc4Add2Tc1 => {
                "https://standards.buildingsmart.org/IFC/RELEASE/IFC4/ADD2_TC1/XML"
            }
        }
    }

    /// The canonical IFC schema token carried by this codec's root metadata.
    #[must_use]
    pub const fn schema_token(self) -> &'static str {
        match self {
            Self::Ifc4Add2Tc1 => "IFC4",
        }
    }
}
