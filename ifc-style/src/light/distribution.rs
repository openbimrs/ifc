//! `IfcLightIntensityDistribution` and its `IfcLightDistributionData` rows.

use ifc_model::EntityId;

use crate::error::StyleResult;
use crate::view::Record;

/// Borrowed projection of `IfcLightIntensityDistribution`: a luminaire's
/// photometric distribution, as a curve type plus its sampled data rows.
#[derive(Debug, Clone, Copy)]
pub struct LightIntensityDistribution<'m, 's> {
    record: Record<'m, 's>,
}

impl<'m, 's> LightIntensityDistribution<'m, 's> {
    pub(crate) fn from_record(record: Record<'m, 's>) -> Self {
        Self { record }
    }

    /// The `LightDistributionCurve` attribute: a token from
    /// `IfcLightDistributionCurveEnum` (`TYPE_A`, `TYPE_B`, `TYPE_C`,
    /// `NOTDEFINED`).
    ///
    /// This token is not decoration -- it names which photometric convention
    /// the angles in [`Self::distribution_data`] follow, so the same numbers
    /// mean different directions under Type A, B, and C. A consumer that
    /// ignores it will aim the distribution wrongly. Returned as the raw
    /// schema token for the same reason as `LightEmissionSource`.
    pub fn light_distribution_curve(&self) -> StyleResult<&'m str> {
        self.record.required_enum("LightDistributionCurve")
    }

    /// The `DistributionData` attribute: at least one
    /// `IfcLightDistributionData` reference.
    ///
    /// The schema declares `LIST [1:?]`, so an empty list is a typed error
    /// rather than an empty iterator that silently renders an unlit luminaire.
    pub fn distribution_data(&self) -> StyleResult<Vec<EntityId>> {
        self.record
            .required_refs("DistributionData", "IfcLightDistributionData", 1, None)
    }
}

/// Borrowed projection of `IfcLightDistributionData`: one main-plane angle and
/// the intensities measured across the secondary angles in that plane.
///
/// `SecondaryPlaneAngle` and `LuminousIntensity` are parallel lists: entry `i`
/// of one corresponds to entry `i` of the other. The schema does not state
/// that pairing as a WHERE rule, so [`Self::samples`] enforces it here instead
/// of letting a caller zip two mismatched lists and silently truncate.
#[derive(Debug, Clone, Copy)]
pub struct LightDistributionData<'m, 's> {
    record: Record<'m, 's>,
}

impl<'m, 's> LightDistributionData<'m, 's> {
    pub(crate) fn from_record(record: Record<'m, 's>) -> Self {
        Self { record }
    }

    /// The `MainPlaneAngle` attribute, an `IfcPlaneAngleMeasure`.
    ///
    /// Signed and unbounded: unlike the spot cone angles this is a plain
    /// `IfcPlaneAngleMeasure`, not a positive one, so negative angles are
    /// valid authored data and are not rejected.
    pub fn main_plane_angle(&self) -> StyleResult<f64> {
        self.record.required_number("MainPlaneAngle")
    }

    /// The `SecondaryPlaneAngle` attribute: `LIST [1:?] OF
    /// IfcPlaneAngleMeasure`.
    pub fn secondary_plane_angles(&self) -> StyleResult<Vec<f64>> {
        self.record.required_numbers("SecondaryPlaneAngle", 1)
    }

    /// The `LuminousIntensity` attribute: `LIST [1:?] OF
    /// IfcLuminousIntensityDistributionMeasure` (candela per lumen under the
    /// SI default).
    pub fn luminous_intensities(&self) -> StyleResult<Vec<f64>> {
        self.record.required_numbers("LuminousIntensity", 1)
    }

    /// The `(secondary plane angle, luminous intensity)` pairs.
    ///
    /// Fails with a typed [`crate::StyleError::InvalidValue`] when the two
    /// lists differ in length, because a photometric table with a missing
    /// intensity is corrupt rather than partially usable.
    pub fn samples(&self) -> StyleResult<Vec<(f64, f64)>> {
        let angles = self.secondary_plane_angles()?;
        let intensities = self.luminous_intensities()?;
        if angles.len() != intensities.len() {
            return Err(self.record.mismatched_lists(
                "SecondaryPlaneAngle",
                angles.len(),
                "LuminousIntensity",
                intensities.len(),
            ));
        }
        Ok(angles.into_iter().zip(intensities).collect())
    }
}
