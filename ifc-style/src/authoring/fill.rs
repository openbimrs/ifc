//! Authoring fill-area patterns.
//!
//! Split from `external`: hatching and tiling are one concern -- how an
//! area is filled -- and the module was over the monolith limit.

use ifc_model::{EntityId, Model, Transaction, Value};
use ifc_schema::Schema;

use crate::error::StyleResult;

use super::{build_named, invalid_authoring, validate_ref};

/// `IfcHatchLineDistanceSelect`: the spacing between hatch lines.
#[derive(Debug, Clone, Copy)]
pub enum HatchLineDistance {
    /// An `IfcPositiveLengthMeasure`: constant perpendicular spacing.
    Length(f64),
    /// An `IfcVector`: spacing and direction, for a skewed pattern.
    Vector(EntityId),
}

/// Stage an `IfcFillAreaStyleHatching`.
///
/// One family of parallel hatch lines. `HatchLineAngle` is in radians,
/// like every other `IfcPlaneAngleMeasure` in the schema.
///
/// # Errors
///
/// Refuses a non-positive or non-finite line distance, a non-finite
/// angle, and a reference of the wrong type -- including a point that
/// is not 2D, which `PatternStart2D` and `RefHatchLine2D` require.
pub fn create_fill_area_style_hatching(
    tx: &mut Transaction,
    model: &Model,
    schema: &Schema,
    appearance: EntityId,
    distance: HatchLineDistance,
    reference: (Option<EntityId>, Option<EntityId>),
    angle: f64,
) -> StyleResult<EntityId> {
    const ENTITY: &str = "IfcFillAreaStyleHatching";
    let (point_of_reference, pattern_start) = reference;
    validate_ref(tx, model, schema, appearance, "IfcCurveStyle")?;
    if !angle.is_finite() {
        return Err(invalid_authoring(ENTITY, "HatchLineAngle", "not finite"));
    }
    let start = match distance {
        HatchLineDistance::Length(value) => {
            if !value.is_finite() || value <= 0.0 {
                return Err(invalid_authoring(
                    ENTITY,
                    "StartOfNextHatchLine",
                    format!("expected a positive length, got {value}"),
                ));
            }
            Value::Real(value)
        }
        HatchLineDistance::Vector(id) => {
            validate_ref(tx, model, schema, id, "IfcVector")?;
            Value::Ref(id)
        }
    };
    for point in [point_of_reference, pattern_start].into_iter().flatten() {
        validate_ref(tx, model, schema, point, "IfcCartesianPoint")?;
    }
    let mut values = vec![
        ("HatchLineAppearance", Value::Ref(appearance)),
        ("StartOfNextHatchLine", start),
    ];
    if let Some(point) = point_of_reference {
        values.push(("PointOfReferenceHatchLine", Value::Ref(point)));
    }
    if let Some(point) = pattern_start {
        values.push(("PatternStart", Value::Ref(point)));
    }
    values.push(("HatchLineAngle", Value::Real(angle)));
    Ok(tx.create(build_named(schema, ENTITY, values)?))
}

/// Stage an `IfcFillAreaStyleTiles`.
///
/// A repeating tiled fill. `TilingPattern` is `LIST [2:2] OF IfcVector`
/// -- exactly two, because two vectors define the lattice the tile
/// repeats on; one would leave the repetition direction undetermined.
///
/// # Errors
///
/// Refuses a tiling pattern that is not exactly two vectors, an empty
/// tile set (`SET [1:?]`), a non-positive tiling scale, and a
/// reference of the wrong type.
pub fn create_fill_area_style_tiles(
    tx: &mut Transaction,
    model: &Model,
    schema: &Schema,
    tiling_pattern: &[EntityId],
    tiles: &[EntityId],
    tiling_scale: f64,
) -> StyleResult<EntityId> {
    const ENTITY: &str = "IfcFillAreaStyleTiles";
    if tiling_pattern.len() != 2 {
        return Err(invalid_authoring(
            ENTITY,
            "TilingPattern",
            format!("expected exactly two vectors, got {}", tiling_pattern.len()),
        ));
    }
    if tiles.is_empty() {
        return Err(invalid_authoring(ENTITY, "Tiles", "empty"));
    }
    if !tiling_scale.is_finite() || tiling_scale <= 0.0 {
        return Err(invalid_authoring(
            ENTITY,
            "TilingScale",
            format!("expected a positive ratio, got {tiling_scale}"),
        ));
    }
    for vector in tiling_pattern {
        validate_ref(tx, model, schema, *vector, "IfcVector")?;
    }
    for tile in tiles {
        validate_ref(tx, model, schema, *tile, "IfcStyledItem")?;
    }
    let values = vec![
        (
            "TilingPattern",
            Value::List(tiling_pattern.iter().copied().map(Value::Ref).collect()),
        ),
        (
            "Tiles",
            Value::List(tiles.iter().copied().map(Value::Ref).collect()),
        ),
        ("TilingScale", Value::Real(tiling_scale)),
    ];
    Ok(tx.create(build_named(schema, ENTITY, values)?))
}
