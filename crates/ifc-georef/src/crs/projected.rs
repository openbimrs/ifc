//! Borrowed `IfcProjectedCRS` interpretation.

use ifc_model::value::Value;
use ifc_model::{Entity, EntityId, Model};
use ifc_schema::SchemaVersion;

use crate::crs::unit::{resolve_length_unit, LengthUnit};
use crate::error::{GeorefError, GeorefResult};
use crate::slot::projected_crs as slot;
use crate::slot::well_known_text as wkt_slot;

/// A projected coordinate reference system read from `IfcProjectedCRS`.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct ProjectedCrs {
    /// The `IfcProjectedCRS` entity this was read from.
    pub entity: EntityId,
    /// CRS identifier, conventionally an EPSG code such as `EPSG:25832`.
    ///
    /// Always present under IFC4, which declares it mandatory. IFC4X3 makes
    /// it optional under `NameOrWKT`: an unnamed CRS is read only when
    /// exactly one `IfcWellKnownText` defines it (see `well_known_text`).
    pub name: Option<String>,
    /// Optional human-readable description.
    pub description: Option<String>,
    /// Optional horizontal datum, such as `ETRS89`.
    pub geodetic_datum: Option<String>,
    /// Optional vertical datum, such as `DHHN2016`.
    pub vertical_datum: Option<String>,
    /// Optional projection name, such as `UTM`.
    pub map_projection: Option<String>,
    /// Optional projection zone, such as `32N`.
    pub map_zone: Option<String>,
    /// Explicit target unit. `None` means IFC inherits the project length unit.
    pub map_unit: Option<LengthUnit>,
    /// IFC4X3: the OGC WKT literal of the one `IfcWellKnownText` whose
    /// `CoordinateReferenceSystem` is this CRS, verbatim. Always `None`
    /// under IFC4, which declares no `IfcWellKnownText`.
    pub well_known_text: Option<String>,
}

pub(crate) fn projected_crs(model: &Model, id: EntityId) -> GeorefResult<ProjectedCrs> {
    let entity = model.get(id).ok_or(GeorefError::MissingEntity {
        referrer: id,
        missing: id,
    })?;
    if !entity.is_type("IFCPROJECTEDCRS") {
        return Err(GeorefError::WrongType {
            entity: id,
            expected: "IFCPROJECTEDCRS",
            actual: entity.type_name.to_string(),
        });
    }
    // Slots are pinned against the bundled IFC4 and IFC4X3 tables in
    // `crate::slot`; the two versions declare them differently but place
    // them identically. What differs is whether `Name` is optional.
    let (name, well_known_text) = if declares_ifc4x3(model) {
        name_or_wkt(model, id, entity)?
    } else {
        let name = entity
            .text(slot::NAME)
            .ok_or(GeorefError::MissingAttribute {
                entity: id,
                index: slot::NAME,
                name: "Name",
            })?;
        (Some(name.to_owned()), None)
    };
    let map_unit = match entity.attribute(slot::MAP_UNIT) {
        None | Some(ifc_model::value::Value::Null) => None,
        Some(value) => {
            let unit = value.as_ref_id().ok_or(GeorefError::InvalidAttribute {
                entity: id,
                index: slot::MAP_UNIT,
                name: "MapUnit",
            })?;
            Some(resolve_length_unit(model, unit)?)
        }
    };
    Ok(ProjectedCrs {
        entity: id,
        name,
        description: entity.text(slot::DESCRIPTION).map(str::to_owned),
        geodetic_datum: entity.text(slot::GEODETIC_DATUM).map(str::to_owned),
        vertical_datum: entity.text(slot::VERTICAL_DATUM).map(str::to_owned),
        map_projection: entity.text(slot::MAP_PROJECTION).map(str::to_owned),
        map_zone: entity.text(slot::MAP_ZONE).map(str::to_owned),
        map_unit,
        well_known_text,
    })
}

/// Whether the header binds IFC4X3, the one release where
/// `IfcCoordinateReferenceSystem.Name` is `OPTIONAL`. A missing or
/// ambiguous header keeps the IFC4 rule, which requires the name.
fn declares_ifc4x3(model: &Model) -> bool {
    matches!(
        model.header().schema.as_slice(),
        [token] if SchemaVersion::from_header_token(token) == Some(SchemaVersion::Ifc4x3)
    )
}

/// IFC4X3 `Name : OPTIONAL IfcLabel` with
/// `NameOrWKT : (HIINDEX(WellKnownText) = 1) OR EXISTS(Name)`, where
/// `WellKnownText` is the inverse `SET [0:1] OF IfcWellKnownText FOR
/// CoordinateReferenceSystem`.
///
/// More than one `IfcWellKnownText` for the CRS breaks the inverse's
/// cardinality and is refused whether or not the CRS is named, rather than
/// one definition being picked.
pub(crate) fn name_or_wkt(
    model: &Model,
    id: EntityId,
    entity: &Entity,
) -> GeorefResult<(Option<String>, Option<String>)> {
    let name = match entity.attribute(slot::NAME) {
        None | Some(Value::Null) => None,
        Some(value) => Some(
            value
                .unwrap_typed()
                .as_text()
                .ok_or(GeorefError::InvalidAttribute {
                    entity: id,
                    index: slot::NAME,
                    name: "Name",
                })?
                .to_owned(),
        ),
    };
    let mut definitions = model.of_type("IFCWELLKNOWNTEXT").filter(|(_, wkt)| {
        wkt.attribute(wkt_slot::COORDINATE_REFERENCE_SYSTEM)
            .and_then(Value::as_ref_id)
            == Some(id)
    });
    let well_known_text = match (definitions.next(), definitions.next()) {
        (None, _) => None,
        (Some((wkt_id, wkt)), None) => Some(
            wkt.text(wkt_slot::WELL_KNOWN_TEXT)
                .ok_or(GeorefError::InvalidAttribute {
                    entity: wkt_id,
                    index: wkt_slot::WELL_KNOWN_TEXT,
                    name: "WellKnownText",
                })?
                .to_owned(),
        ),
        (Some(_), Some(_)) => {
            return Err(GeorefError::RuleViolation {
                entity: id,
                rule: "WellKnownText : SET [0:1] OF IfcWellKnownText",
            })
        }
    };
    if name.is_none() && well_known_text.is_none() {
        return Err(GeorefError::RuleViolation {
            entity: id,
            rule: "NameOrWKT",
        });
    }
    Ok((name, well_known_text))
}
