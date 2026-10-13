//! ifcZIP (#450, feature `zip`): a STEP or ifcXML model in a ZIP archive.
//!
//! Every read entry point ([`IfcModel::parse`] and its variants,
//! [`IfcModel::open`]) recognizes an archive by its ZIP signature and reads
//! the one model entry it holds, within the facade's default decompression
//! budget: STEP under the call's [`ParseOptions`], ifcXML in the layout its
//! root declares. [`IfcModel::write_ifczip`] writes one.
//!
//! The facade's typed refusals map onto the shared codes: an archive with
//! no model, several, or an entry in another format is `parse`; an
//! encrypted entry or a compression method other than Stored and Deflate
//! is `unsupported`; an entry that inflates past the budget is
//! `budget-exceeded`. Without the feature, an archive is refused with
//! `feature-disabled`.

use crate::{BindingError, IfcModel, ParseOptions};

/// Read the model in the ifcZIP archive `bytes`, STEP under `options`.
#[cfg(feature = "zip")]
pub(crate) fn load(bytes: &[u8], options: ParseOptions) -> Result<IfcModel, BindingError> {
    use ifc::ifczip::{self, Budget, InnerFormat};
    use ifc::Codec;

    let entry = ifczip::extract(bytes, &Budget::DEFAULT).map_err(refusal)?;
    match entry.format {
        InnerFormat::Step => IfcModel::loaded_as(
            "STEP in ifcZIP",
            crate::model::reader(options).read_owned(entry.bytes),
        ),
        _ => ifczip::read_entry(entry)
            .map(|inner| IfcModel {
                inner,
                guid_index: Default::default(),
            })
            .map_err(refusal),
    }
}

#[cfg(not(feature = "zip"))]
pub(crate) fn load(bytes: &[u8], options: ParseOptions) -> Result<IfcModel, BindingError> {
    let _ = (bytes, options);
    Err(BindingError::FeatureDisabled("zip"))
}

/// A facade refusal as the binding error with the same meaning.
#[cfg(feature = "zip")]
fn refusal(error: ifc::ifczip::IfcZipError) -> BindingError {
    use ifc::ifczip::IfcZipError;
    match error {
        IfcZipError::BudgetExceeded { .. } => {
            BindingError::BudgetExceeded(format!("ifcZIP {error}"))
        }
        IfcZipError::Encrypted { .. } | IfcZipError::UnsupportedCompression { .. } => {
            BindingError::Unsupported(format!("ifcZIP {error}"))
        }
        IfcZipError::CodecNotCompiled { feature, .. } => BindingError::FeatureDisabled(feature),
        IfcZipError::InvalidEntryName(_) | IfcZipError::Write(_) => {
            BindingError::Write(format!("ifcZIP: {error}"))
        }
        other => BindingError::Parse(format!("ifcZIP: {other}")),
    }
}

impl IfcModel {
    /// Serialize as an ifcZIP archive: one Deflate entry holding STEP
    /// (`inner` `None` or `"step"`, as `model.ifc`) or ifcXML (`"ifcxml"`,
    /// as `model.ifcXML`, in the layout `xsd_profile` names as for
    /// [`Self::write_ifcxml`]). Any case.
    ///
    /// Refused with `invalid-value` for another `inner`, or an
    /// `xsd_profile` with STEP inside; otherwise as the inner write, and
    /// with `feature-disabled` without the `zip` feature.
    pub fn write_ifczip(
        &self,
        inner: Option<&str>,
        xsd_profile: Option<&str>,
    ) -> Result<Vec<u8>, BindingError> {
        let xml = match inner {
            None => false,
            Some(token) if token.eq_ignore_ascii_case("step") => false,
            Some(token) if token.eq_ignore_ascii_case("ifcxml") => true,
            Some(other) => {
                return Err(BindingError::InvalidValue(format!(
                    "the ifcZIP inner format must be \"step\" or \"ifcxml\", not {other:?}"
                )))
            }
        };
        if !xml && xsd_profile.is_some() {
            return Err(BindingError::InvalidValue(
                "an XSD profile applies to ifcXML inside ifcZIP, not STEP".to_owned(),
            ));
        }
        #[cfg(feature = "zip")]
        {
            use ifc::ifczip::{pack, InnerFormat};
            let (document, format) = if xml {
                (self.write_ifcxml(xsd_profile)?, InnerFormat::IfcXml)
            } else {
                (self.write()?, InnerFormat::Step)
            };
            pack(format.default_entry_name(), &document).map_err(refusal)
        }
        #[cfg(not(feature = "zip"))]
        {
            let _ = xml;
            Err(BindingError::FeatureDisabled("zip"))
        }
    }
}

#[cfg(all(test, feature = "zip", feature = "ifcxml", feature = "ifc4"))]
mod tests {
    use super::*;

    const FILE: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../test/fixtures/synthetic-bindings/binding_geometry.ifc"
    ));

    fn code(result: Result<IfcModel, BindingError>) -> &'static str {
        result.expect_err("refused").code()
    }

    #[test]
    fn every_read_takes_an_archive_and_step_round_trips() {
        let model = IfcModel::parse(FILE).unwrap();
        let step = model.write().unwrap();
        let zipped = model.write_ifczip(None, None).unwrap();
        assert!(ifc::ifczip::is_zip(&zipped));
        assert_eq!(zipped, model.write_ifczip(Some("STEP"), None).unwrap());
        for back in [
            IfcModel::parse(&zipped).unwrap(),
            IfcModel::parse_owned(zipped.clone()).unwrap(),
            IfcModel::parse_with(&zipped, ParseOptions::lenient()).unwrap(),
        ] {
            assert!(back.write().unwrap() == step, "STEP changed");
        }
    }

    #[test]
    fn ifcxml_inside_reads_in_either_layout() {
        let model = IfcModel::parse(FILE).unwrap();
        for profile in [None, Some("IFC4")] {
            let xml = model.write_ifcxml(profile).unwrap();
            let zipped = model.write_ifczip(Some("ifcxml"), profile).unwrap();
            let entry = ifc::ifczip::extract(&zipped, &ifc::ifczip::Budget::DEFAULT).unwrap();
            assert_eq!(entry.name, "model.ifcXML");
            assert!(entry.bytes == xml, "the archive holds the ifcXML bytes");
            let back = IfcModel::parse(&zipped).unwrap();
            let direct = IfcModel::parse_ifcxml(&xml, profile).unwrap();
            assert!(
                back.write_ifcxml(profile).unwrap() == direct.write_ifcxml(profile).unwrap(),
                "{profile:?}: the archive read differently"
            );
        }
    }

    #[test]
    fn refusals_carry_the_shared_codes() {
        let model = IfcModel::parse(FILE).unwrap();
        assert_eq!(
            model.write_ifczip(Some("json"), None).unwrap_err().code(),
            "invalid-value"
        );
        assert_eq!(
            model.write_ifczip(None, Some("IFC4")).unwrap_err().code(),
            "invalid-value"
        );
        let empty = b"PK\x05\x06\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0";
        assert_eq!(code(IfcModel::parse(empty)), "parse");
        assert_eq!(code(IfcModel::parse(b"PK\x03\x04 damaged")), "parse");

        let mut bomb = b"ISO-10303-21;\n".to_vec();
        bomb.resize(64 << 20, b' ');
        let zipped = ifc::ifczip::pack("model.ifc", &bomb).unwrap();
        assert_eq!(code(IfcModel::parse(&zipped)), "budget-exceeded");

        // An encrypted entry: the general purpose flag's bit 0 set in the
        // local and central headers.
        let mut encrypted = model.write_ifczip(None, None).unwrap();
        for at in 0..encrypted.len() - 4 {
            let offset = match &encrypted[at..at + 4] {
                b"PK\x03\x04" => 6,
                b"PK\x01\x02" => 8,
                _ => continue,
            };
            encrypted[at + offset] |= 1;
        }
        assert_eq!(code(IfcModel::parse(&encrypted)), "unsupported");
    }
}
