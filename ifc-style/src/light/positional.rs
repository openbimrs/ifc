//! `IfcLightSourcePositional` and its `IfcLightSourceSpot` subtype.

use ifc_model::EntityId;

use crate::error::StyleResult;
use crate::light::source::LightSource;
use crate::view::Record;

/// Borrowed projection of `IfcLightSourcePositional`: a point light with a
/// quadratic distance-attenuation law.
///
/// The three attenuation coefficients are plain `IfcReal` in every bundled
/// schema -- unbounded and unsigned -- so they are returned verbatim. IFC
/// inherits the law from ISO/IEC 14772-1 (VRML): attenuation at distance `d`
/// is `1 / max(c + l*d + q*d*d, 1)`. This crate does not evaluate it; clamping
/// and unit policy belong to whatever renders the light.
#[derive(Debug, Clone, Copy)]
pub struct LightSourcePositional<'m, 's> {
    record: Record<'m, 's>,
}

impl<'m, 's> LightSourcePositional<'m, 's> {
    pub(crate) fn from_record(record: Record<'m, 's>) -> Self {
        Self { record }
    }

    /// The inherited `IfcLightSource` colour and intensity attributes.
    pub fn light_source(&self) -> LightSource<'m, 's> {
        LightSource::from_record(self.record)
    }

    /// The `Position` attribute: a mandatory `IfcCartesianPoint` reference.
    ///
    /// Note the asymmetry with `IfcLightSourceGoniometric`, which uses a full
    /// `IfcAxis2Placement3D`: a positional light is rotationally symmetric, so
    /// the schema gives it a bare point and no axes.
    pub fn position(&self) -> StyleResult<EntityId> {
        self.record.required_ref("Position", "IfcCartesianPoint")
    }

    /// The `Radius` attribute, an `IfcPositiveLengthMeasure`.
    ///
    /// Reported in the file's own length unit. This crate does not resolve
    /// `IfcUnitAssignment`; scaling is the caller's job, as it is for every
    /// other length this crate touches.
    pub fn radius(&self) -> StyleResult<f64> {
        self.record.positive("Radius", "IfcPositiveLengthMeasure")
    }

    /// The `ConstantAttenuation` coefficient.
    pub fn constant_attenuation(&self) -> StyleResult<f64> {
        self.record.required_number("ConstantAttenuation")
    }

    /// The `DistanceAttenuation` (linear) coefficient.
    pub fn distance_attenuation(&self) -> StyleResult<f64> {
        self.record.required_number("DistanceAttenuation")
    }

    /// The `QuadricAttenuation` (quadratic) coefficient.
    pub fn quadric_attenuation(&self) -> StyleResult<f64> {
        self.record.required_number("QuadricAttenuation")
    }
}

/// Borrowed projection of `IfcLightSourceSpot`: a positional light narrowed to
/// a cone and aimed along an orientation.
#[derive(Debug, Clone, Copy)]
pub struct LightSourceSpot<'m, 's> {
    record: Record<'m, 's>,
}

impl<'m, 's> LightSourceSpot<'m, 's> {
    pub(crate) fn from_record(record: Record<'m, 's>) -> Self {
        Self { record }
    }

    /// The inherited `IfcLightSourcePositional` position and attenuation.
    pub fn positional(&self) -> LightSourcePositional<'m, 's> {
        LightSourcePositional::from_record(self.record)
    }

    /// The inherited `IfcLightSource` colour and intensity attributes.
    pub fn light_source(&self) -> LightSource<'m, 's> {
        LightSource::from_record(self.record)
    }

    /// The `Orientation` attribute: a mandatory `IfcDirection` reference
    /// giving the cone's axis.
    pub fn orientation(&self) -> StyleResult<EntityId> {
        self.record.required_ref("Orientation", "IfcDirection")
    }

    /// The `ConcentrationExponent` attribute, when authored.
    ///
    /// OPTIONAL in all three bundled schemas. It is *not* defaulted to `1.0`
    /// here; an absent exponent means the file expressed no falloff
    /// preference, which is different from authoring linear falloff.
    pub fn concentration_exponent(&self) -> StyleResult<Option<f64>> {
        self.record.optional_number("ConcentrationExponent")
    }

    /// The `SpreadAngle` attribute, an `IfcPositivePlaneAngleMeasure`.
    ///
    /// The half-angle of the full cone, in the file's plane-angle unit --
    /// commonly radians, but IFC permits degrees via `IfcUnitAssignment`, so
    /// a caller must resolve units before comparing this to `beam_width_angle`
    /// from a different file.
    pub fn spread_angle(&self) -> StyleResult<f64> {
        self.record
            .positive("SpreadAngle", "IfcPositivePlaneAngleMeasure")
    }

    /// The `BeamWidthAngle` attribute, an `IfcPositivePlaneAngleMeasure`:
    /// the half-angle of the inner, full-intensity cone.
    pub fn beam_width_angle(&self) -> StyleResult<f64> {
        self.record
            .positive("BeamWidthAngle", "IfcPositivePlaneAngleMeasure")
    }
}
