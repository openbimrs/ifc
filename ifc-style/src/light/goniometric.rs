//! `IfcLightSourceGoniometric`: the photometric light source.

use ifc_model::EntityId;

use crate::error::StyleResult;
use crate::light::source::LightSource;
use crate::view::Record;

/// Members of the `IfcLightDistributionDataSourceSelect` type.
///
/// Kept beside the accessor that validates against it so the select's
/// membership is stated once, in the same place it is enforced.
const DISTRIBUTION_DATA_SOURCE: &[&str] =
    &["IfcExternalReference", "IfcLightIntensityDistribution"];

/// Borrowed projection of `IfcLightSourceGoniometric`: a real luminaire
/// described by photometric data rather than a renderer's approximation.
///
/// This is the only light source carrying absolute photometric quantities
/// (luminous flux, colour temperature, lamp technology). Every other subtype
/// is a normalized shading construct. The values are reported exactly as
/// authored; this crate performs no photometric or colour-space conversion.
#[derive(Debug, Clone, Copy)]
pub struct LightSourceGoniometric<'m, 's> {
    record: Record<'m, 's>,
}

impl<'m, 's> LightSourceGoniometric<'m, 's> {
    pub(crate) fn from_record(record: Record<'m, 's>) -> Self {
        Self { record }
    }

    /// The inherited `IfcLightSource` colour and intensity attributes.
    pub fn light_source(&self) -> LightSource<'m, 's> {
        LightSource::from_record(self.record)
    }

    /// The `Position` attribute: a mandatory `IfcAxis2Placement3D` reference.
    ///
    /// A full placement, not a bare point: a luminaire's intensity
    /// distribution is defined in its own local axes, so the orientation is
    /// what makes `IfcLightIntensityDistribution` angles meaningful.
    pub fn position(&self) -> StyleResult<EntityId> {
        self.record.required_ref("Position", "IfcAxis2Placement3D")
    }

    /// The `ColourAppearance` attribute, when authored: an `IfcColourRgb`
    /// reference giving the perceived colour.
    ///
    /// Distinct from the inherited `LightColour`, which is the shading colour.
    /// A lamp can have one chromaticity and be rendered with another.
    pub fn colour_appearance(&self) -> StyleResult<Option<EntityId>> {
        self.record.optional_ref("ColourAppearance", "IfcColourRgb")
    }

    /// The `ColourTemperature` attribute, an
    /// `IfcThermodynamicTemperatureMeasure`.
    ///
    /// Mandatory. Reported in the file's own temperature unit; IFC's SI
    /// default is kelvin, but the value is not converted here.
    pub fn colour_temperature(&self) -> StyleResult<f64> {
        self.record.required_number("ColourTemperature")
    }

    /// The `LuminousFlux` attribute, an `IfcLuminousFluxMeasure` (lumen under
    /// the SI default). Mandatory.
    pub fn luminous_flux(&self) -> StyleResult<f64> {
        self.record.required_number("LuminousFlux")
    }

    /// The `LightEmissionSource` attribute: the lamp technology token from
    /// `IfcLightEmissionSourceEnum`.
    ///
    /// Returned as the raw schema token rather than a Rust enum. The
    /// enumeration is schema-versioned, and this crate's census marks such
    /// tokens `SchemaValue` -- promoting one to a closed Rust type would make
    /// a future schema's new member unrepresentable.
    pub fn light_emission_source(&self) -> StyleResult<&'m str> {
        self.record.required_enum("LightEmissionSource")
    }

    /// The `LightDistributionDataSource` attribute: a mandatory reference to
    /// either an `IfcLightIntensityDistribution` (inline photometric data) or
    /// an `IfcExternalReference` (e.g. an EULUMDAT or IES file).
    ///
    /// The select is validated, so a reference to an unrelated entity is a
    /// typed [`crate::StyleError::ReferenceType`] rather than a surprise at
    /// the point of use.
    pub fn light_distribution_data_source(&self) -> StyleResult<EntityId> {
        self.record.required_ref_select(
            "LightDistributionDataSource",
            "IfcLightDistributionDataSourceSelect",
            DISTRIBUTION_DATA_SOURCE,
        )
    }
}
