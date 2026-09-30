//! Reading `IfcSite.RefLatitude`, `RefLongitude` and `RefElevation`.

use ifc_model::value::Value;
use ifc_model::{EntityId, Model};
use ifc_schema::SchemaVersion;

use super::angle::{read_angle, Axis, CompoundPlaneAngle};
use crate::error::{GeorefError, GeorefResult};
use crate::slot::site as slot;

/// The WGS84 reference point an `IfcSite` states for its placement origin.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub struct SiteReference {
    /// The `IfcSite` entity this was read from.
    pub entity: EntityId,
    /// The release the model header declares, whose rules were applied.
    pub version: SchemaVersion,
    /// `RefLatitude`, validated against the release's WHERE rules and the
    /// WGS84 range `[-90, 90]`.
    pub ref_latitude: Option<CompoundPlaneAngle>,
    /// `RefLongitude`, validated against the release's WHERE rules and the
    /// WGS84 range `[-180, 180]`.
    pub ref_longitude: Option<CompoundPlaneAngle>,
    /// `RefElevation` in metres: the authored value times the project
    /// length scale. Height above sea level in an unnamed local datum.
    pub ref_elevation: Option<f64>,
}

impl SiteReference {
    /// `RefLatitude` in decimal degrees, north positive.
    #[must_use]
    pub fn latitude_degrees(&self) -> Option<f64> {
        self.ref_latitude.map(|a| a.decimal_degrees())
    }

    /// `RefLongitude` in decimal degrees, east positive.
    #[must_use]
    pub fn longitude_degrees(&self) -> Option<f64> {
        self.ref_longitude.map(|a| a.decimal_degrees())
    }
}

/// Read the reference point of the `IfcSite` `site`.
///
/// `project_metres_per_unit` is the project's `IfcUnitAssignment` length
/// scale, explicit as for [`crate::resolve_project_to_map`]: `RefElevation`
/// is an `IfcLengthMeasure` in the project length unit.
///
/// The release is bound from the header: IFC2X3, IFC4 and IFC4X3 are read;
/// a missing, ambiguous or other schema is refused with the crate's schema
/// errors. Every absent attribute is `None`, never a default.
pub fn site_reference(
    model: &Model,
    site: EntityId,
    project_metres_per_unit: f64,
) -> GeorefResult<SiteReference> {
    let version = site_release(model)?;
    let entity = model.get(site).ok_or(GeorefError::MissingEntity {
        referrer: site,
        missing: site,
    })?;
    if !entity.is_type("IFCSITE") {
        return Err(GeorefError::WrongType {
            entity: site,
            expected: "IFCSITE",
            actual: entity.type_name.to_string(),
        });
    }
    if !project_metres_per_unit.is_finite() || project_metres_per_unit <= 0.0 {
        return Err(GeorefError::InvalidUnit {
            entity: site,
            detail: "project length scale must be finite and positive",
        });
    }
    let ref_latitude = read_angle(
        entity.attribute(slot::REF_LATITUDE),
        site,
        slot::REF_LATITUDE,
        Axis::Latitude,
        version,
    )?;
    let ref_longitude = read_angle(
        entity.attribute(slot::REF_LONGITUDE),
        site,
        slot::REF_LONGITUDE,
        Axis::Longitude,
        version,
    )?;
    let invalid_elevation = GeorefError::InvalidAttribute {
        entity: site,
        index: slot::REF_ELEVATION,
        name: "RefElevation",
    };
    let ref_elevation = match entity
        .attribute(slot::REF_ELEVATION)
        .map(Value::unwrap_typed)
    {
        None | Some(Value::Null) => None,
        Some(value) => {
            let metres = value.as_f64().ok_or(invalid_elevation.clone())? * project_metres_per_unit;
            if !metres.is_finite() {
                return Err(invalid_elevation);
            }
            Some(metres)
        }
    };
    Ok(SiteReference {
        entity: site,
        version,
        ref_latitude,
        ref_longitude,
        ref_elevation,
    })
}

/// The header's release, if it is one whose `IfcSite` layout is verified.
///
/// Wider than [`crate::GeorefView`]: IFC2X3 declares `IfcSite` with the
/// same reference attributes although it has no coordinate operations.
fn site_release(model: &Model) -> GeorefResult<SchemaVersion> {
    let token = match model.header().schema.as_slice() {
        [] => return Err(GeorefError::MissingSchema),
        [token] => token,
        tokens => {
            return Err(GeorefError::AmbiguousSchema {
                tokens: tokens.to_vec(),
            })
        }
    };
    match SchemaVersion::from_header_token(token) {
        Some(version @ (SchemaVersion::Ifc2x3 | SchemaVersion::Ifc4 | SchemaVersion::Ifc4x3)) => {
            Ok(version)
        }
        // IFC4X1/IFC4X2 are bundled but not verified here: refused, never
        // read with another release's rules.
        _ => Err(GeorefError::UnsupportedSchema {
            token: token.clone(),
        }),
    }
}
