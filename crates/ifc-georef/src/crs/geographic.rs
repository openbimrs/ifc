//! Borrowed IFC4X3 `IfcGeographicCRS` interpretation.
//!
//! IFC4X3 ADD2 declares
//! `IfcGeographicCRS SUBTYPE OF (IfcCoordinateReferenceSystem)` with
//! `PrimeMeridian : OPTIONAL IfcIdentifier`, `AngleUnit : OPTIONAL
//! IfcNamedUnit` (`WHERE AngleUnitIsPlaneAngle`) and `HeightUnit : OPTIONAL
//! IfcNamedUnit` (`WHERE HeightUnitIsLength`). IFC4 does not declare the
//! entity; the schema-pinned readers refuse it there before this runs.
//!
//! The inherited `Name` is `OPTIONAL` under `NameOrWKT`, exactly as for
//! `IfcProjectedCRS` in IFC4X3, so the same rule is applied. An absent
//! unit stays `None`: nothing here assumes degrees or metres.

use ifc_model::value::Value;
use ifc_model::{EntityId, Model};

use crate::crs::projected::name_or_wkt;
use crate::crs::unit::{resolve_angle_unit, resolve_length_unit, AngleUnit, LengthUnit};
use crate::error::{GeorefError, GeorefResult};
use crate::slot::geographic_crs as slot;
use crate::slot::projected_crs as base;

/// A geographic coordinate reference system read from IFC4X3
/// `IfcGeographicCRS`.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct GeographicCrs {
    /// The `IfcGeographicCRS` entity this was read from.
    pub entity: EntityId,
    /// CRS identifier, conventionally an EPSG code such as `EPSG:4326`.
    /// `None` only when exactly one `IfcWellKnownText` defines the CRS.
    pub name: Option<String>,
    /// Optional human-readable description.
    pub description: Option<String>,
    /// Optional geodetic datum, such as `WGS84`.
    pub geodetic_datum: Option<String>,
    /// Optional prime meridian identifier, such as `Greenwich`.
    pub prime_meridian: Option<String>,
    /// The declared unit of latitude and longitude. `None` when the file
    /// does not state one; no unit is assumed.
    pub angle_unit: Option<AngleUnit>,
    /// The declared unit of ellipsoidal height. `None` when the file does
    /// not state one; no unit is assumed.
    pub height_unit: Option<LengthUnit>,
    /// The OGC WKT literal of the one `IfcWellKnownText` defining this CRS.
    pub well_known_text: Option<String>,
}

pub(crate) fn geographic_crs(model: &Model, id: EntityId) -> GeorefResult<GeographicCrs> {
    let entity = model.get(id).ok_or(GeorefError::MissingEntity {
        referrer: id,
        missing: id,
    })?;
    if !entity.is_type("IFCGEOGRAPHICCRS") {
        return Err(GeorefError::WrongType {
            entity: id,
            expected: "IFCGEOGRAPHICCRS",
            actual: entity.type_name.to_string(),
        });
    }
    let (name, well_known_text) = name_or_wkt(model, id, entity)?;
    let unit_ref = |index: usize, name: &'static str| match entity.attribute(index) {
        None | Some(Value::Null) => Ok(None),
        Some(value) => value
            .as_ref_id()
            .map(Some)
            .ok_or(GeorefError::InvalidAttribute {
                entity: id,
                index,
                name,
            }),
    };
    let angle_unit = unit_ref(slot::ANGLE_UNIT, "AngleUnit")?
        .map(|unit| resolve_angle_unit(model, unit))
        .transpose()?;
    let height_unit = unit_ref(slot::HEIGHT_UNIT, "HeightUnit")?
        .map(|unit| resolve_length_unit(model, unit))
        .transpose()?;
    Ok(GeographicCrs {
        entity: id,
        name,
        description: entity.text(base::DESCRIPTION).map(str::to_owned),
        geodetic_datum: entity.text(base::GEODETIC_DATUM).map(str::to_owned),
        prime_meridian: entity.text(slot::PRIME_MERIDIAN).map(str::to_owned),
        angle_unit,
        height_unit,
        well_known_text,
    })
}
