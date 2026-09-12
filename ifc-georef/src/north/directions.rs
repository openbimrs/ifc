//! True, grid, and project north as distinct 2D directions.
//!
//! IFC distinguishes three "north" references that frequently disagree in
//! real files:
//!
//! - **Project north**: the project's own local Y axis. By construction this
//!   is always `(0, 1)` in the project's own XY plane -- it is the reference
//!   every other north direction is measured against, not something to
//!   resolve from the file.
//! - **True north**: `IfcGeometricRepresentationContext.TrueNorth`, the
//!   geographic (geodetic) northing direction expressed in the project XY
//!   plane. Optional; defaults to `(0, 1)` (project north) when absent, per
//!   the IFC4/IFC4X3 attribute definition.
//! - **Grid north**: the direction of the target map's northing axis,
//!   expressed back in the project XY plane. This is derived from
//!   `IfcMapConversion`'s `XAxisAbscissa`/`XAxisOrdinate` rotation, not read
//!   directly from any single attribute.
//!
//! The schema is explicit that these can disagree, and states the
//! resolution order itself: "If a geographic placement is provided using
//! `IfcMapConversion` then the true north is for information only. In case
//! of inconsistency, the value provided with `IfcMapConversion` shall take
//! precedence." So grid north (when a map conversion exists) is
//! authoritative for georeferencing; true north remains informational
//! metadata. This module keeps both distinguishable rather than collapsing
//! them into one direction, so a caller can still detect and report the
//! disagreement instead of silently picking one.

use ifc_model::value::Value;
use ifc_model::{EntityId, Model};

use crate::conversion::ProjectToMap;
use crate::error::{GeorefError, GeorefResult};

/// A resolved north reference: a unit-length 2D direction in the project's
/// own XY plane, plus which reference it names.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NorthReference {
    /// The project's own Y axis: always `(0, 1)` by definition.
    Project,
    /// Geodetic north, from `IfcGeometricRepresentationContext.TrueNorth`
    /// (or its `(0, 1)` default when the attribute is absent).
    True {
        /// Unit-length direction in the project's XY plane.
        direction: (f64, f64),
    },
    /// The target map's northing axis, derived from an `IfcMapConversion`.
    Grid {
        /// Unit-length direction in the project's XY plane.
        direction: (f64, f64),
    },
}

impl NorthReference {
    /// The direction as a unit vector in the project's XY plane.
    #[must_use]
    pub fn direction(&self) -> (f64, f64) {
        match self {
            Self::Project => project_north_direction(),
            Self::True { direction } | Self::Grid { direction } => *direction,
        }
    }
}

/// The project's own north reference: always `(0, 1)` in its own XY plane.
///
/// This is a definitional constant, not something resolved from a model:
/// "project north" means the project's local Y axis, and every other north
/// reference is stated relative to it.
#[must_use]
pub const fn project_north_direction() -> (f64, f64) {
    (0.0, 1.0)
}

/// Resolve `IfcGeometricRepresentationContext.TrueNorth`.
///
/// `TrueNorth` is attribute slot 5 (`IfcRepresentationContext` contributes 0
/// and 1; `IfcGeometricRepresentationContext` adds `CoordinateSpaceDimension`
/// at 2, `Precision` at 3, `WorldCoordinateSystem` at 4, `TrueNorth` at 5 --
/// identical in IFC4 and IFC4X3). Returns the spec default `(0, 1)` when the
/// attribute is absent, exactly as `IfcGeometricRepresentationContext`
/// documents: "If not present, it defaults to \[0.,1.\]".
pub fn resolve_true_north(model: &Model, context: EntityId) -> GeorefResult<NorthReference> {
    let entity = model.get(context).ok_or(GeorefError::MissingEntity {
        referrer: context,
        missing: context,
    })?;
    if !entity.is_type("IFCGEOMETRICREPRESENTATIONCONTEXT")
        && !entity.is_type("IFCGEOMETRICREPRESENTATIONSUBCONTEXT")
    {
        return Err(GeorefError::WrongType {
            entity: context,
            expected: "IFCGEOMETRICREPRESENTATIONCONTEXT",
            actual: entity.type_name.to_string(),
        });
    }
    let direction = match entity.attribute(5).map(Value::unwrap_typed) {
        None | Some(Value::Null) => project_north_direction(),
        Some(Value::Ref(direction_id)) => {
            direction_ratios_2d(model, *direction_id, context, 5, "TrueNorth")?
        }
        Some(_) => {
            return Err(GeorefError::InvalidAttribute {
                entity: context,
                index: 5,
                name: "TrueNorth",
            })
        }
    };
    Ok(NorthReference::True { direction })
}

/// Resolve grid north from an already-computed `IfcMapConversion` operation.
///
/// The map conversion rotation takes a project XY vector `(x, y)` to a map
/// XY vector `(ax - by, bx + ay)` where `(a, b)` is the normalized
/// `(XAxisAbscissa, XAxisOrdinate)` pair
/// ([`ProjectToMap::x_axis_direction`]). Grid north is the map's northing
/// axis, `(0, 1)` in map coordinates, pulled back through the *inverse* of
/// that rotation into the project frame: since the rotation matrix is
/// orthonormal, its inverse is its transpose, giving `(b, a)`.
///
/// A worked check: `a = 1, b = 0` (no rotation) gives grid north `(0, 1)`,
/// i.e. grid north coincides with project north when the project's local X
/// axis is already aligned with the map's easting axis. `a = 0, b = 1`
/// (project X rotated 90 degrees onto map northing) gives grid north
/// `(1, 0)`, i.e. the project's own X axis now points toward map north --
/// consistent with the project frame having been rotated a quarter turn
/// relative to the map.
#[must_use]
pub fn grid_north_direction(operation: &ProjectToMap) -> NorthReference {
    let (a, b) = operation.x_axis_direction;
    NorthReference::Grid { direction: (b, a) }
}

/// Read an `IfcDirection`'s two-component `DirectionRatios` and normalize.
fn direction_ratios_2d(
    model: &Model,
    direction_id: EntityId,
    referrer: EntityId,
    index: usize,
    name: &'static str,
) -> GeorefResult<(f64, f64)> {
    let direction = model.get(direction_id).ok_or(GeorefError::MissingEntity {
        referrer,
        missing: direction_id,
    })?;
    if !direction.is_type("IFCDIRECTION") {
        return Err(GeorefError::WrongType {
            entity: direction_id,
            expected: "IFCDIRECTION",
            actual: direction.type_name.to_string(),
        });
    }
    let ratios = direction
        .attribute(0)
        .map(Value::unwrap_typed)
        .and_then(Value::as_list)
        .ok_or(GeorefError::InvalidAttribute {
            entity: referrer,
            index,
            name,
        })?;
    let (x, y) = match ratios {
        [x, y] => (
            x.unwrap_typed()
                .as_f64()
                .ok_or(GeorefError::InvalidAttribute {
                    entity: direction_id,
                    index: 0,
                    name: "DirectionRatios",
                })?,
            y.unwrap_typed()
                .as_f64()
                .ok_or(GeorefError::InvalidAttribute {
                    entity: direction_id,
                    index: 0,
                    name: "DirectionRatios",
                })?,
        ),
        // North2D requires exactly two ratios; a 3D direction here is a
        // schema violation this crate refuses rather than silently
        // projecting to 2D.
        _ => {
            return Err(GeorefError::InvalidAttribute {
                entity: referrer,
                index,
                name,
            })
        }
    };
    let norm = x.hypot(y);
    if !norm.is_finite() || norm <= f64::EPSILON {
        return Err(GeorefError::NonFiniteDirection {
            entity: direction_id,
        });
    }
    Ok((x / norm, y / norm))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use ifc_model::Entity;

    use super::*;
    use crate::conversion::resolve_project_to_map;

    fn id(value: u64) -> EntityId {
        EntityId(value)
    }

    fn r(value: u64) -> Value {
        Value::Ref(id(value))
    }

    fn real(value: f64) -> Value {
        Value::Real(value)
    }

    #[test]
    fn true_north_defaults_to_project_north_when_absent() {
        let mut model = Model::new();
        model.insert(
            id(1),
            Entity::new("IFCGEOMETRICREPRESENTATIONCONTEXT", vec![]),
        );

        let north = resolve_true_north(&model, id(1)).expect("default applies");
        assert_eq!(north.direction(), (0.0, 1.0));
        assert!(matches!(north, NorthReference::True { .. }));
    }

    #[test]
    fn true_north_reads_an_explicit_declared_direction() {
        let mut model = Model::new();
        model.insert(
            id(2),
            Entity::new(
                "IFCDIRECTION",
                vec![Value::List(vec![real(1.0), real(1.0)])],
            ),
        );
        model.insert(
            id(1),
            Entity::new(
                "IFCGEOMETRICREPRESENTATIONCONTEXT",
                vec![
                    Value::Null,
                    Value::Null,
                    Value::Integer(3),
                    Value::Null,
                    Value::Null,
                    r(2),
                ],
            ),
        );

        let north = resolve_true_north(&model, id(1)).expect("explicit direction resolves");
        let root_two = 2.0_f64.sqrt();
        let (x, y) = north.direction();
        assert!((x - 1.0 / root_two).abs() < 1e-12);
        assert!((y - 1.0 / root_two).abs() < 1e-12);
    }

    #[test]
    fn true_north_rejects_a_zero_length_direction() {
        let mut model = Model::new();
        model.insert(
            id(2),
            Entity::new(
                "IFCDIRECTION",
                vec![Value::List(vec![real(0.0), real(0.0)])],
            ),
        );
        model.insert(
            id(1),
            Entity::new(
                "IFCGEOMETRICREPRESENTATIONCONTEXT",
                vec![
                    Value::Null,
                    Value::Null,
                    Value::Integer(3),
                    Value::Null,
                    Value::Null,
                    r(2),
                ],
            ),
        );

        assert!(matches!(
            resolve_true_north(&model, id(1)),
            Err(GeorefError::NonFiniteDirection { entity }) if entity == id(2)
        ));
    }

    #[test]
    fn grid_north_matches_project_north_when_the_map_conversion_has_no_rotation() {
        let mut model = Model::new();
        model.insert(
            id(1),
            Entity::new("IFCGEOMETRICREPRESENTATIONCONTEXT", vec![]),
        );
        model.insert(
            id(2),
            Entity::new(
                "IFCPROJECTEDCRS",
                vec![Value::Text(Arc::from("EPSG:25832"))],
            ),
        );
        model.insert(
            id(4),
            Entity::new(
                "IFCMAPCONVERSION",
                vec![
                    r(1),
                    r(2),
                    real(0.0),
                    real(0.0),
                    real(0.0),
                    real(1.0),
                    real(0.0),
                    real(1.0),
                ],
            ),
        );

        let operation = resolve_project_to_map(&model, id(4), 1.0).expect("resolves");
        let grid_north = grid_north_direction(&operation);
        assert_eq!(grid_north.direction(), (0.0, 1.0));
    }

    #[test]
    fn grid_north_rotates_a_quarter_turn_when_the_map_conversion_does() {
        let mut model = Model::new();
        model.insert(
            id(1),
            Entity::new("IFCGEOMETRICREPRESENTATIONCONTEXT", vec![]),
        );
        model.insert(
            id(2),
            Entity::new(
                "IFCPROJECTEDCRS",
                vec![Value::Text(Arc::from("EPSG:25832"))],
            ),
        );
        model.insert(
            id(4),
            Entity::new(
                "IFCMAPCONVERSION",
                vec![
                    r(1),
                    r(2),
                    real(0.0),
                    real(0.0),
                    real(0.0),
                    real(0.0),
                    real(1.0),
                    real(1.0),
                ],
            ),
        );

        let operation = resolve_project_to_map(&model, id(4), 1.0).expect("resolves");
        let grid_north = grid_north_direction(&operation);
        let (x, y) = grid_north.direction();
        assert!((x - 1.0).abs() < 1e-12);
        assert!(y.abs() < 1e-12);
    }

    #[test]
    fn project_north_is_always_the_project_y_axis() {
        assert_eq!(
            NorthReference::Project.direction(),
            project_north_direction()
        );
        assert_eq!(project_north_direction(), (0.0, 1.0));
    }
}
