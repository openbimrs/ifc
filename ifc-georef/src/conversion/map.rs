//! IFC4 `IfcMapConversion` lowered to a metre-to-metre neutral transform.

use axiolid_core::{Mat3, Transform3, Vec3};
use ifc_model::value::Value;
use ifc_model::{Entity, EntityId, Model};

use crate::crs::{projected_crs, LengthUnit, ProjectedCrs};
use crate::error::{GeorefError, GeorefResult};

#[derive(Debug, Clone, PartialEq)]
pub struct ProjectToMap {
    pub source_crs: EntityId,
    pub target_crs: ProjectedCrs,
    /// Affine operation from neutral project metres to neutral map metres.
    pub transform: Transform3,
    pub project_unit: LengthUnit,
    pub map_unit: LengthUnit,
    /// IFC's declared scale before source/target unit normalization.
    pub declared_scale: f64,
}

/// Resolve an `IfcMapConversion` and normalize both frames to metres.
///
/// `project_metres_per_unit` is the project's `IfcUnitAssignment` length scale.
/// It is explicit here because project units are owned by the caller's model
/// loading boundary, while `MapUnit` is owned by the target CRS.
pub fn resolve_project_to_map(
    model: &Model,
    id: EntityId,
    project_metres_per_unit: f64,
) -> GeorefResult<ProjectToMap> {
    if !project_metres_per_unit.is_finite() || project_metres_per_unit <= 0.0 {
        return Err(GeorefError::InvalidUnit {
            entity: id,
            detail: "project length scale must be finite and positive",
        });
    }
    let entity = model.get(id).ok_or(GeorefError::MissingEntity {
        referrer: id,
        missing: id,
    })?;
    let actual_type = entity.type_name.to_ascii_uppercase();
    if actual_type == "IFCMAPCONVERSIONSCALED" {
        return Err(GeorefError::UnsupportedOperation {
            entity: id,
            actual: actual_type,
        });
    }
    if actual_type != "IFCMAPCONVERSION" {
        return Err(GeorefError::WrongType {
            entity: id,
            expected: "IFCMAPCONVERSION",
            actual: entity.type_name.to_string(),
        });
    }
    // IFC4 ADD2 TC1 IfcMapConversion declaration order: SourceCRS,
    // TargetCRS, Eastings, Northings, OrthogonalHeight, XAxis*, Scale.
    let source_crs = required_ref(entity, id, 0, "SourceCRS")?;
    let target_ref = required_ref(entity, id, 1, "TargetCRS")?;
    let target_crs = projected_crs(model, target_ref)?;
    let project_unit = LengthUnit {
        name: "PROJECT_LENGTH_UNIT".into(),
        metres_per_unit: project_metres_per_unit,
    };
    let map_unit = target_crs
        .map_unit
        .clone()
        .unwrap_or_else(|| project_unit.clone());

    let eastings = required_number(entity, id, 2, "Eastings")?;
    let northings = required_number(entity, id, 3, "Northings")?;
    let height = required_number(entity, id, 4, "OrthogonalHeight")?;
    let a = optional_number(entity, id, 5, "XAxisAbscissa")?.unwrap_or(1.0);
    let b = optional_number(entity, id, 6, "XAxisOrdinate")?.unwrap_or(0.0);
    let norm = a.hypot(b);
    if !norm.is_finite() || norm <= f64::EPSILON {
        return Err(GeorefError::DegenerateAxis { entity: id });
    }
    let (a, b) = (a / norm, b / norm);
    let declared_scale = optional_number(entity, id, 7, "Scale")?.unwrap_or(1.0);
    if !declared_scale.is_finite() || declared_scale <= 0.0 {
        return Err(GeorefError::InvalidScale {
            entity: id,
            value: declared_scale,
        });
    }
    for (index, name, value) in [
        (2, "Eastings", eastings),
        (3, "Northings", northings),
        (4, "OrthogonalHeight", height),
    ] {
        if !value.is_finite() {
            return Err(GeorefError::InvalidAttribute {
                entity: id,
                index,
                name,
            });
        }
    }

    // IFC formula: E/N/H are target-map units; Scale maps source project
    // units to target units. The neutral operation takes and returns metres.
    let scale = declared_scale * map_unit.metres_per_unit / project_metres_per_unit;
    if !scale.is_finite() || scale <= 0.0 {
        return Err(GeorefError::InvalidScale {
            entity: id,
            value: scale,
        });
    }
    let x = Vec3::new(a * scale, b * scale, 0.0);
    let y = Vec3::new(-b * scale, a * scale, 0.0);
    let z = Vec3::new(0.0, 0.0, scale);
    let translation = Vec3::new(eastings, northings, height) * map_unit.metres_per_unit;
    for (index, name, value) in [
        (2, "Eastings", translation.x),
        (3, "Northings", translation.y),
        (4, "OrthogonalHeight", translation.z),
    ] {
        if !value.is_finite() {
            return Err(GeorefError::InvalidAttribute {
                entity: id,
                index,
                name,
            });
        }
    }
    let transform = Transform3::from_mat3_translation(Mat3::from_cols(x, y, z), translation);

    Ok(ProjectToMap {
        source_crs,
        target_crs,
        transform,
        project_unit,
        map_unit,
        declared_scale,
    })
}

fn required_ref(
    entity: &Entity,
    id: EntityId,
    index: usize,
    name: &'static str,
) -> GeorefResult<EntityId> {
    entity
        .reference(index)
        .ok_or(GeorefError::MissingAttribute {
            entity: id,
            index,
            name,
        })
}

fn required_number(
    entity: &Entity,
    id: EntityId,
    index: usize,
    name: &'static str,
) -> GeorefResult<f64> {
    entity
        .attribute(index)
        .and_then(|v| v.unwrap_typed().as_f64())
        .ok_or(GeorefError::MissingAttribute {
            entity: id,
            index,
            name,
        })
}

fn optional_number(
    entity: &Entity,
    id: EntityId,
    index: usize,
    name: &'static str,
) -> GeorefResult<Option<f64>> {
    match entity.attribute(index).map(Value::unwrap_typed) {
        None | Some(Value::Null) => Ok(None),
        Some(value) => value
            .as_f64()
            .map(Some)
            .ok_or(GeorefError::InvalidAttribute {
                entity: id,
                index,
                name,
            }),
    }
}
