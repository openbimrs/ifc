//! The `IfcLightSource` supertype and its two attribute-free/minimal subtypes.

use ifc_model::EntityId;

use crate::error::StyleResult;
use crate::view::Record;

/// Which concrete `IfcLightSource` subtype an entity declares.
///
/// Resolution walks the schema's inheritance graph, so a subtype introduced by
/// a later schema resolves to its nearest known ancestor rather than being
/// mistaken for an unrelated family. `Spot` is checked before `Positional`
/// because `IfcLightSourceSpot` *is a* `IfcLightSourcePositional`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum LightSourceKind {
    /// `IfcLightSourceAmbient`: non-directional fill light.
    Ambient,
    /// `IfcLightSourceDirectional`: parallel rays, no position.
    Directional,
    /// `IfcLightSourceGoniometric`: photometric light with a distribution curve.
    Goniometric,
    /// `IfcLightSourcePositional`: point light with distance attenuation.
    Positional,
    /// `IfcLightSourceSpot`: positional light narrowed to a cone.
    Spot,
}

/// Borrowed projection of `IfcLightSource`: the colour and intensity every
/// light shares, independent of how it is positioned or aimed.
///
/// `AmbientIntensity` and `Intensity` are both OPTIONAL in all three bundled
/// schemas. An absent value is reported as `None`, never defaulted to `1.0`:
/// choosing a fallback is a renderer policy, and inventing one here would make
/// an unauthored file indistinguishable from one that authored full intensity.
#[derive(Debug, Clone, Copy)]
pub struct LightSource<'m, 's> {
    record: Record<'m, 's>,
}

impl<'m, 's> LightSource<'m, 's> {
    pub(crate) fn from_record(record: Record<'m, 's>) -> Self {
        Self { record }
    }

    /// The concrete subtype this entity declares.
    pub fn kind(&self) -> StyleResult<LightSourceKind> {
        let declared = self.record.type_name();
        for (name, kind) in [
            ("IfcLightSourceSpot", LightSourceKind::Spot),
            ("IfcLightSourceGoniometric", LightSourceKind::Goniometric),
            ("IfcLightSourcePositional", LightSourceKind::Positional),
            ("IfcLightSourceDirectional", LightSourceKind::Directional),
            ("IfcLightSourceAmbient", LightSourceKind::Ambient),
        ] {
            if self.record.is_a(declared, name) {
                return Ok(kind);
            }
        }
        Err(self.record.wrong_type("a concrete IfcLightSource subtype"))
    }

    /// The `Name` attribute, when authored.
    pub fn name(&self) -> StyleResult<Option<&'m str>> {
        self.record.optional_text("Name")
    }

    /// The `LightColour` attribute: a mandatory `IfcColourRgb` reference.
    pub fn light_colour(&self) -> StyleResult<EntityId> {
        self.record.required_ref("LightColour", "IfcColourRgb")
    }

    /// The `AmbientIntensity` attribute, normalized to `[0, 1]` when authored.
    pub fn ambient_intensity(&self) -> StyleResult<Option<f64>> {
        self.record.optional_normalized("AmbientIntensity")
    }

    /// The `Intensity` attribute, normalized to `[0, 1]` when authored.
    pub fn intensity(&self) -> StyleResult<Option<f64>> {
        self.record.optional_normalized("Intensity")
    }
}

/// Borrowed projection of `IfcLightSourceAmbient`.
///
/// The subtype adds no attributes in any bundled schema; it exists so a file
/// can say "this is ambient fill" rather than leaving a reader to infer it.
/// The type is still distinct from [`LightSource`] so that a caller matching
/// on [`LightSourceKind`] gets a value it cannot accidentally aim or position.
#[derive(Debug, Clone, Copy)]
pub struct LightSourceAmbient<'m, 's> {
    record: Record<'m, 's>,
}

impl<'m, 's> LightSourceAmbient<'m, 's> {
    pub(crate) fn from_record(record: Record<'m, 's>) -> Self {
        Self { record }
    }

    /// The inherited `IfcLightSource` colour and intensity attributes.
    pub fn light_source(&self) -> LightSource<'m, 's> {
        LightSource::from_record(self.record)
    }
}

/// Borrowed projection of `IfcLightSourceDirectional`: parallel rays with an
/// orientation but no position.
#[derive(Debug, Clone, Copy)]
pub struct LightSourceDirectional<'m, 's> {
    record: Record<'m, 's>,
}

impl<'m, 's> LightSourceDirectional<'m, 's> {
    pub(crate) fn from_record(record: Record<'m, 's>) -> Self {
        Self { record }
    }

    /// The inherited `IfcLightSource` colour and intensity attributes.
    pub fn light_source(&self) -> LightSource<'m, 's> {
        LightSource::from_record(self.record)
    }

    /// The `Orientation` attribute: a mandatory `IfcDirection` reference.
    ///
    /// Returned as an id, not a vector. Resolving and normalizing a direction
    /// is geometry-resource work; this crate references representation items
    /// without importing geometry types.
    pub fn orientation(&self) -> StyleResult<EntityId> {
        self.record.required_ref("Orientation", "IfcDirection")
    }
}
