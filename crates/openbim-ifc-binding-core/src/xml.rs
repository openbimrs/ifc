//! The ifcXML codec (feature `ifcxml`).
//!
//! Two layouts, both the facade's:
//!
//! - **native** (no profile): `XmlCodec::default()`, this library's own
//!   lossless layout. Any model round-trips, whatever schema it declares,
//!   with no schema tables needed.
//! - **XSD** (a profile token, `IFC4` or `IFC4X3_ADD2`): `XmlCodec::xsd`,
//!   the buildingSMART configuration the release XSD declares. It reads and
//!   writes schema-strictly and refuses what it cannot carry exactly.
//!
//! Without the feature, both operations refuse with `feature-disabled`.

use crate::{BindingError, IfcModel};

impl IfcModel {
    /// Parse an ifcXML document: the native layout when `xsd_profile` is
    /// `None`, else the XSD configuration of that profile (`IFC4` or
    /// `IFC4X3_ADD2`, case-insensitive).
    ///
    /// Refused with `unsupported-profile` for any other token, with
    /// `unsupported-schema` when the profile's release is not bundled, and
    /// with `parse` when the document does not read.
    pub fn parse_ifcxml(bytes: &[u8], xsd_profile: Option<&str>) -> Result<Self, BindingError> {
        #[cfg(feature = "ifcxml")]
        {
            use ifc::Codec;
            Self::loaded_as("ifcXML", codec(xsd_profile)?.read_bytes(bytes))
        }
        #[cfg(not(feature = "ifcxml"))]
        {
            let _ = (bytes, xsd_profile);
            Err(BindingError::FeatureDisabled("ifcxml"))
        }
    }

    /// Serialize as ifcXML, in the layout [`Self::parse_ifcxml`] reads.
    ///
    /// An XSD-layout write needs the header to declare the profile's schema
    /// token, and refuses with `write` what the configuration cannot carry.
    pub fn write_ifcxml(&self, xsd_profile: Option<&str>) -> Result<Vec<u8>, BindingError> {
        #[cfg(feature = "ifcxml")]
        {
            use ifc::Codec;
            codec(xsd_profile)?
                .write_bytes(&self.inner)
                .map_err(|error| BindingError::Write(format!("ifcXML: {error}")))
        }
        #[cfg(not(feature = "ifcxml"))]
        {
            let _ = xsd_profile;
            Err(BindingError::FeatureDisabled("ifcxml"))
        }
    }
}

/// The codec for a layout choice.
#[cfg(feature = "ifcxml")]
fn codec(xsd_profile: Option<&str>) -> Result<ifc::XmlCodec, BindingError> {
    use std::sync::{Arc, OnceLock};

    use ifc::{Schema, XmlCodec, XmlProfile};

    let Some(token) = xsd_profile else {
        return Ok(XmlCodec::default());
    };
    // The profiles the facade offers, by their schema token. The codec
    // takes its schema shared; one copy per release is made on first use
    // rather than one per call.
    static IFC4: OnceLock<Arc<Schema>> = OnceLock::new();
    static IFC4X3: OnceLock<Arc<Schema>> = OnceLock::new();
    let (profile, cache) = [
        (XmlProfile::Ifc4Add2Tc1, &IFC4),
        (XmlProfile::Ifc4x3Add2, &IFC4X3),
    ]
    .into_iter()
    .find(|(profile, _)| profile.schema_token().eq_ignore_ascii_case(token))
    .ok_or_else(|| BindingError::UnsupportedProfile(token.to_owned()))?;
    let schema = match cache.get() {
        Some(schema) => Arc::clone(schema),
        None => {
            let bundled = ifc::schema::for_version(profile.version()).map_err(|refused| {
                BindingError::UnsupportedSchema(format!("{} ({refused})", profile.schema_token()))
            })?;
            Arc::clone(cache.get_or_init(|| Arc::new(bundled.clone())))
        }
    };
    Ok(XmlCodec::xsd(schema, profile))
}
