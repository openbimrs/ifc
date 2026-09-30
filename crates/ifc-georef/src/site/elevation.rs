//! Comparing `IfcSite.RefElevation` with the map height of the site origin.
//!
//! The rule and its spec basis are documented on the parent module.

use axiolid_core::Point3;
use ifc_model::EntityId;
use ifc_schema::SchemaVersion;

use super::reference::SiteReference;
use crate::conversion::ProjectToMap;
use crate::error::{GeorefError, GeorefResult};

/// Suggested tolerance, in metres, for [`relate_site_elevation`]: 1 cm.
///
/// Above the rounding a millimetre, foot or inch unit round trip
/// introduces, yet fine enough that a wrong or mismatched height datum is
/// not absorbed as noise. A caller with its own survey requirements passes
/// its own tolerance.
pub const SITE_ELEVATION_TOLERANCE_M: f64 = 0.01;

/// Both heights of the site origin, in metres, and how far apart they are.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub struct SiteElevationComparison {
    /// The `IfcSite` compared.
    pub site: EntityId,
    /// `IfcSite.RefElevation`, metres.
    pub ref_elevation: f64,
    /// `IfcMapConversion.OrthogonalHeight`, metres: the map height of the
    /// world coordinate system's origin.
    pub orthogonal_height: f64,
    /// Map height of the site origin, metres: the site origin carried
    /// through the map conversion. Equals `orthogonal_height` for a site
    /// origin at world height `0`.
    pub site_origin_map_height: f64,
    /// `ref_elevation - site_origin_map_height`, metres.
    pub difference: f64,
    /// The tolerance the difference was judged against, metres.
    pub tolerance: f64,
    /// The target `IfcProjectedCRS`.
    pub target_crs: EntityId,
    /// The target CRS's `VerticalDatum`, verbatim; `None` when unstated.
    /// `RefElevation` names no datum, so the comparison assumes it is this
    /// one; judging that is left to the caller.
    pub vertical_datum: Option<String>,
}

/// How `IfcSite.RefElevation` relates to the map conversion's height.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub enum SiteElevationCheck {
    /// The site states no `RefElevation`; nothing to compare.
    RefElevationAbsent,
    /// No map conversion was supplied (always the case under IFC2X3, which
    /// declares none); only `RefElevation` locates the site vertically.
    NoMapConversion,
    /// The two heights agree within the tolerance.
    Consistent(SiteElevationComparison),
    /// The two heights differ by more than the tolerance. A finding, not an
    /// error: both values are kept and neither is preferred.
    Disagreement(SiteElevationComparison),
}

/// Relate `site.ref_elevation` to `map`'s `OrthogonalHeight` and the target
/// CRS's `VerticalDatum`.
///
/// `site_origin_z` is the height of the `IfcSite` placement origin in the
/// project's world coordinate system, in metres: `0.0` for a site placed at
/// the world origin. `tolerance` is in metres, finite and non-negative;
/// [`SITE_ELEVATION_TOLERANCE_M`] is the suggested value.
///
/// Refuses a non-finite `site_origin_z` or an invalid `tolerance` with
/// [`GeorefError::InvalidParameter`], and a map conversion paired with an
/// IFC2X3 site with [`GeorefError::UnsupportedSchema`].
pub fn relate_site_elevation(
    site: &SiteReference,
    map: Option<&ProjectToMap>,
    site_origin_z: f64,
    tolerance: f64,
) -> GeorefResult<SiteElevationCheck> {
    if !tolerance.is_finite() || tolerance < 0.0 {
        return Err(GeorefError::InvalidParameter {
            name: "tolerance",
            value: tolerance,
        });
    }
    if !site_origin_z.is_finite() {
        return Err(GeorefError::InvalidParameter {
            name: "site_origin_z",
            value: site_origin_z,
        });
    }
    if site.version == SchemaVersion::Ifc2x3 && map.is_some() {
        return Err(GeorefError::UnsupportedSchema {
            token: "IFC2X3".to_owned(),
        });
    }
    let Some(ref_elevation) = site.ref_elevation else {
        return Ok(SiteElevationCheck::RefElevationAbsent);
    };
    let Some(map) = map else {
        return Ok(SiteElevationCheck::NoMapConversion);
    };
    let orthogonal_height = map.transform.transform_point3(Point3::new(0.0, 0.0, 0.0)).z;
    let site_origin_map_height = map
        .transform
        .transform_point3(Point3::new(0.0, 0.0, site_origin_z))
        .z;
    let difference = ref_elevation - site_origin_map_height;
    let comparison = SiteElevationComparison {
        site: site.entity,
        ref_elevation,
        orthogonal_height,
        site_origin_map_height,
        difference,
        tolerance,
        target_crs: map.target_crs.entity,
        vertical_datum: map.target_crs.vertical_datum.clone(),
    };
    Ok(if difference.abs() <= tolerance {
        SiteElevationCheck::Consistent(comparison)
    } else {
        SiteElevationCheck::Disagreement(comparison)
    })
}
