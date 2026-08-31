//! Borrowed `IfcProjectedCRS` interpretation.

use ifc_model::{EntityId, Model};

use crate::crs::unit::{resolve_length_unit, LengthUnit};
use crate::error::{GeorefError, GeorefResult};

#[derive(Debug, Clone, PartialEq)]
pub struct ProjectedCrs {
    pub entity: EntityId,
    pub name: String,
    pub description: Option<String>,
    pub geodetic_datum: Option<String>,
    pub vertical_datum: Option<String>,
    pub map_projection: Option<String>,
    pub map_zone: Option<String>,
    /// Explicit target unit. `None` means IFC inherits the project length unit.
    pub map_unit: Option<LengthUnit>,
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
    // IFC4 ADD2 TC1 IfcCoordinateReferenceSystem contributes slots 0..3;
    // IfcProjectedCRS adds MapProjection, MapZone, and MapUnit at 4..6.
    let name = entity
        .text(0)
        .ok_or(GeorefError::MissingAttribute {
            entity: id,
            index: 0,
            name: "Name",
        })?
        .to_owned();
    let map_unit = match entity.attribute(6) {
        None | Some(ifc_model::value::Value::Null) => None,
        Some(value) => {
            let unit = value.as_ref_id().ok_or(GeorefError::InvalidAttribute {
                entity: id,
                index: 6,
                name: "MapUnit",
            })?;
            Some(resolve_length_unit(model, unit)?)
        }
    };
    Ok(ProjectedCrs {
        entity: id,
        name,
        description: entity.text(1).map(str::to_owned),
        geodetic_datum: entity.text(2).map(str::to_owned),
        vertical_datum: entity.text(3).map(str::to_owned),
        map_projection: entity.text(4).map(str::to_owned),
        map_zone: entity.text(5).map(str::to_owned),
        map_unit,
    })
}
