//! Supported ifcXML release profiles.

/// A release-specific ifcXML namespace/profile contract.
///
/// A profile fixes the namespace and schema token, not the layout: native
/// output under a profile is not valid against the release XSD, whose
/// configuration this crate reads ([`crate::XmlLayout::Xsd`]) but does not
/// write.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum XmlProfile {
    /// IFC4 ADD2 TC1, whose namespace the official XSD (`IFC4.xsd`) declares.
    Ifc4Add2Tc1,
    /// IFC4.3 ADD2, whose namespace its official XSD (`IFC4X3_ADD2.xsd`)
    /// declares.
    Ifc4x3Add2,
}

impl XmlProfile {
    /// The exact XML namespace declared by the release XSD.
    #[must_use]
    pub const fn namespace(self) -> &'static str {
        match self {
            Self::Ifc4Add2Tc1 => {
                "https://standards.buildingsmart.org/IFC/RELEASE/IFC4/ADD2_TC1/XML"
            }
            Self::Ifc4x3Add2 => "https://standards.buildingsmart.org/IFC/RELEASE/IFC4/3/ADD2",
        }
    }

    /// Every namespace an XSD-configuration document of this release may
    /// declare, [`Self::namespace`] first.
    ///
    /// IFC4 ADD2 TC1 also accepts `http://www.buildingsmart-tech.org/ifcXML/IFC4/Add2`:
    /// the release's own published examples (Annex E of the ADD2 TC1
    /// documentation) declare it, as the XSD did before its namespace moved
    /// to the `standards.buildingsmart.org` URL. Only the XSD layout reader
    /// accepts it; this crate's own layout is checked against
    /// [`Self::namespace`] alone.
    #[must_use]
    pub const fn namespaces(self) -> &'static [&'static str] {
        match self {
            Self::Ifc4Add2Tc1 => &[
                "https://standards.buildingsmart.org/IFC/RELEASE/IFC4/ADD2_TC1/XML",
                "http://www.buildingsmart-tech.org/ifcXML/IFC4/Add2",
            ],
            Self::Ifc4x3Add2 => &["https://standards.buildingsmart.org/IFC/RELEASE/IFC4/3/ADD2"],
        }
    }

    /// The canonical IFC schema token carried by this codec's root metadata.
    #[must_use]
    pub const fn schema_token(self) -> &'static str {
        match self {
            Self::Ifc4Add2Tc1 => "IFC4",
            Self::Ifc4x3Add2 => "IFC4X3_ADD2",
        }
    }

    /// The schema release this profile's documents are written against.
    #[cfg(feature = "schema")]
    #[must_use]
    pub const fn version(self) -> ifc_schema::SchemaVersion {
        match self {
            Self::Ifc4Add2Tc1 => ifc_schema::SchemaVersion::Ifc4,
            Self::Ifc4x3Add2 => ifc_schema::SchemaVersion::Ifc4x3,
        }
    }
}
