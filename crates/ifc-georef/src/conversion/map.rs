//! IFC4/IFC4X3 `IfcMapConversion` and IFC4X3 `IfcMapConversionScaled`
//! lowered to a metre-to-metre neutral transform. The entry points that
//! dispatch every coordinate-operation subtype live in `operation.rs`.
//!
//! # `IfcMapConversionScaled`
//!
//! IFC4X3 ADD2 declares `FactorX`, `FactorY`, `FactorZ : IfcReal` after the
//! eight inherited `IfcMapConversion` attributes. The factors scale the
//! source axes before the rotation, together with `Scale`:
//!
//! ```text
//! E = Eastings  + Scale * (a * FactorX * x - b * FactorY * y)
//! N = Northings + Scale * (b * FactorX * x + a * FactorY * y)
//! H = OrthogonalHeight + Scale * FactorZ * z
//! ```
//!
//! The `.exp` carries no formula; this is the one IfcOpenShell's
//! `ifcopenshell.util.geolocation.xyz2enh` implements for the subtype
//! (`references/competitor/ifcopenshell`), which reduces to the
//! `IfcMapConversion` formula when every factor is `1`. A general
//! `Transform3` holds the resulting non-uniform linear part exactly, so
//! the subtype resolves rather than being refused. A factor that is zero,
//! negative or non-finite is refused like a non-positive `Scale`.

use axiolid_core::{Mat3, Transform3, Vec3};

use crate::context::operation_source;
use crate::crs::{projected_crs, LengthUnit};
use crate::error::{GeorefError, GeorefResult};
use crate::slot::map_conversion as slot;
use crate::slot::map_conversion_scaled as scaled_slot;

use super::operation::{Operation, OperationKind, ProjectToMap};

/// Lower an `IfcMapConversion`, or with `scaled` its IFC4X3
/// `IfcMapConversionScaled` subtype.
pub(super) fn lower(
    op: &Operation<'_, '_>,
    project_unit: LengthUnit,
    scaled: bool,
) -> GeorefResult<ProjectToMap> {
    let id = op.id;
    // Slots are pinned against the bundled IFC4 and IFC4X3 tables in
    // `crate::slot`.
    let source = operation_source(op.model, op.view, id)?;
    let source_crs = source.entity();
    let target_ref = op.required_ref(slot::TARGET_CRS, "TargetCRS")?;
    let target_crs = projected_crs(op.model, target_ref)?;
    let map_unit = target_crs
        .map_unit
        .clone()
        .unwrap_or_else(|| project_unit.clone());

    let eastings = op.required_number(slot::EASTINGS, "Eastings")?;
    let northings = op.required_number(slot::NORTHINGS, "Northings")?;
    let height = op.required_number(slot::ORTHOGONAL_HEIGHT, "OrthogonalHeight")?;
    let a = op
        .optional_number(slot::X_AXIS_ABSCISSA, "XAxisAbscissa")?
        .unwrap_or(1.0);
    let b = op
        .optional_number(slot::X_AXIS_ORDINATE, "XAxisOrdinate")?
        .unwrap_or(0.0);
    let norm = a.hypot(b);
    if !norm.is_finite() || norm <= f64::EPSILON {
        return Err(GeorefError::DegenerateAxis { entity: id });
    }
    let (a, b) = (a / norm, b / norm);
    let declared_scale = op.optional_number(slot::SCALE, "Scale")?.unwrap_or(1.0);
    if !declared_scale.is_finite() || declared_scale <= 0.0 {
        return Err(GeorefError::InvalidScale {
            entity: id,
            value: declared_scale,
        });
    }
    op.finite(slot::EASTINGS, "Eastings", eastings)?;
    op.finite(slot::NORTHINGS, "Northings", northings)?;
    op.finite(slot::ORTHOGONAL_HEIGHT, "OrthogonalHeight", height)?;

    let (kind, factors) = if scaled {
        let factors = (
            factor(op, scaled_slot::FACTOR_X, "FactorX")?,
            factor(op, scaled_slot::FACTOR_Y, "FactorY")?,
            factor(op, scaled_slot::FACTOR_Z, "FactorZ")?,
        );
        (OperationKind::MapConversionScaled { factors }, factors)
    } else {
        (OperationKind::MapConversion, (1.0, 1.0, 1.0))
    };

    // IFC formula: E/N/H are target-map units; Scale maps source project
    // units to target units. The neutral operation takes and returns metres.
    let scale = declared_scale * map_unit.metres_per_unit / project_unit.metres_per_unit;
    let (fx, fy, fz) = factors;
    for axis_scale in [scale, scale * fx, scale * fy, scale * fz] {
        if !axis_scale.is_finite() || axis_scale <= 0.0 {
            return Err(GeorefError::InvalidScale {
                entity: id,
                value: axis_scale,
            });
        }
    }
    let x = Vec3::new(a, b, 0.0) * (scale * fx);
    let y = Vec3::new(-b, a, 0.0) * (scale * fy);
    let z = Vec3::new(0.0, 0.0, scale * fz);
    let translation = Vec3::new(eastings, northings, height) * map_unit.metres_per_unit;
    op.finite(slot::EASTINGS, "Eastings", translation.x)?;
    op.finite(slot::NORTHINGS, "Northings", translation.y)?;
    op.finite(slot::ORTHOGONAL_HEIGHT, "OrthogonalHeight", translation.z)?;
    let transform = Transform3::from_mat3_translation(Mat3::from_cols(x, y, z), translation);

    Ok(ProjectToMap {
        source_crs,
        source,
        operation: id,
        kind,
        target_crs,
        transform,
        project_unit,
        map_unit,
        declared_scale,
        x_axis_direction: (a, b),
    })
}

/// A mandatory `IfcMapConversionScaled` factor: finite and positive.
fn factor(op: &Operation<'_, '_>, index: usize, name: &'static str) -> GeorefResult<f64> {
    let value = op.required_number(index, name)?;
    if value.is_finite() && value > 0.0 {
        Ok(value)
    } else {
        Err(GeorefError::InvalidAttribute {
            entity: op.id,
            index,
            name,
        })
    }
}
