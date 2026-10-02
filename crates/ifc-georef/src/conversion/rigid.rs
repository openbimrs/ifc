//! IFC4X3 `IfcRigidOperation`: a translation with no rotation or scale.
//!
//! IFC4X3 ADD2 declares
//!
//! ```text
//! FirstCoordinate  : IfcMeasureValue;
//! SecondCoordinate : IfcMeasureValue;
//! Height           : OPTIONAL IfcLengthMeasure;
//! WHERE SameCoordinateType :
//!   (IFCLENGTHMEASURE IN TYPEOF(First) AND IFCLENGTHMEASURE IN TYPEOF(Second))
//!   OR (IFCPLANEANGLEMEASURE IN TYPEOF(First) AND IFCPLANEANGLEMEASURE IN TYPEOF(Second))
//! ```
//!
//! `IfcMeasureValue` is a SELECT, so each coordinate must carry its type
//! (`IFCLENGTHMEASURE(10.)`); an untyped REAL satisfies neither branch and
//! is a `SameCoordinateType` violation. `TYPEOF` of a defined type includes
//! the types it is defined on, so `IfcPositiveLengthMeasure` and
//! `IfcNonNegativeLengthMeasure` (both `= IfcLengthMeasure`) count as
//! length, and `IfcPositivePlaneAngleMeasure` (`= IfcPlaneAngleMeasure`)
//! as plane angle.
//!
//! # Length coordinates: a translation
//!
//! The two coordinates are the offset along the target `IfcProjectedCRS`'s
//! first and second axes, and `Height` the vertical offset, all in the
//! target's `MapUnit` (the project length unit when it has none), the
//! convention `IfcMapConversion`'s `Eastings`/`Northings` use. "Rigid"
//! means distance-preserving, so the linear part is the identity in
//! metres whatever the two units are. A length rigid operation whose
//! target is not an `IfcProjectedCRS` is refused with `WrongType`: a metre
//! offset on latitude/longitude axes denotes nothing.
//!
//! # Plane-angle coordinates: a geographic offset
//!
//! Angles offset latitude and longitude on an `IfcGeographicCRS`. No
//! metre-to-metre affine transform expresses that, so
//! [`crate::resolve_project_to_map`] refuses the form with
//! [`GeorefError::CoordinateMeasureMismatch`], and
//! [`resolve_geographic_offset_in`] reads it as authored instead. Nothing
//! converts the angles: the schema does not tie `IfcPlaneAngleMeasure`
//! here to the target's `AngleUnit`, so the values and the declared unit
//! are both returned and the caller decides.

use ifc_model::value::Value;
use ifc_model::EntityId;

use crate::context::{operation_source, OperationSource};
use crate::crs::{geographic_crs, projected_crs, GeographicCrs, LengthUnit};
use crate::error::{GeorefError, GeorefResult};
use crate::slot::map_conversion as base;
use crate::slot::rigid_operation as slot;
use crate::view::GeorefView;

use super::operation::{Operation, OperationKind, ProjectToMap};

const LENGTH: &str = "IFCLENGTHMEASURE";
const PLANE_ANGLE: &str = "IFCPLANEANGLEMEASURE";

/// An IFC4X3 `IfcRigidOperation` with plane-angle coordinates, read as
/// authored.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct GeographicOffset {
    /// The `IfcRigidOperation` entity.
    pub operation: EntityId,
    /// The validated `SourceCRS`.
    pub source: OperationSource,
    /// The target `IfcGeographicCRS`, with its declared units.
    pub target_crs: GeographicCrs,
    /// `FirstCoordinate`, the `IfcPlaneAngleMeasure` value as authored.
    pub first_coordinate: f64,
    /// `SecondCoordinate`, the `IfcPlaneAngleMeasure` value as authored.
    pub second_coordinate: f64,
    /// `Height`, the `IfcLengthMeasure` value as authored, when stated.
    pub height: Option<f64>,
}

/// Read an IFC4X3 `IfcRigidOperation` whose coordinates are plane angles.
///
/// # Errors
///
/// [`GeorefError::UnsupportedOperation`] under IFC4, which declares no
/// `IfcRigidOperation`; [`GeorefError::WrongType`] for another entity or a
/// target that is not an `IfcGeographicCRS`;
/// [`GeorefError::RuleViolation`] `SameCoordinateType`;
/// [`GeorefError::CoordinateMeasureMismatch`] for length coordinates,
/// which [`crate::resolve_project_to_map_in`] lowers instead; and every
/// source error of [`crate::resolve_operation_source`].
pub fn resolve_geographic_offset_in(
    view: &GeorefView,
    id: EntityId,
) -> GeorefResult<GeographicOffset> {
    let actual = view.require_known_type(id)?;
    let entity = view.model.get(id).ok_or(GeorefError::MissingEntity {
        referrer: id,
        missing: id,
    })?;
    if !actual.eq_ignore_ascii_case("IFCRIGIDOPERATION") {
        return Err(GeorefError::WrongType {
            entity: id,
            expected: "IFCRIGIDOPERATION",
            actual: actual.to_owned(),
        });
    }
    let op = Operation {
        model: view.model,
        view: Some(view),
        id,
        entity,
    };
    let coordinates = coordinates(&op)?;
    if coordinates.measure != PLANE_ANGLE {
        return Err(GeorefError::CoordinateMeasureMismatch {
            entity: id,
            expected: PLANE_ANGLE,
            actual: coordinates.measure,
        });
    }
    let source = operation_source(view.model, Some(view), id)?;
    let target = op.required_ref(base::TARGET_CRS, "TargetCRS")?;
    let target_crs = geographic_crs(view.model, target)?;
    Ok(GeographicOffset {
        operation: id,
        source,
        target_crs,
        first_coordinate: coordinates.first,
        second_coordinate: coordinates.second,
        height: coordinates.height,
    })
}

/// Lower a length-coordinate `IfcRigidOperation` to a translation.
pub(super) fn lower(
    op: &Operation<'_, '_>,
    project_unit: LengthUnit,
) -> GeorefResult<ProjectToMap> {
    let coordinates = coordinates(op)?;
    if coordinates.measure != LENGTH {
        return Err(GeorefError::CoordinateMeasureMismatch {
            entity: op.id,
            expected: LENGTH,
            actual: coordinates.measure,
        });
    }
    let source = operation_source(op.model, op.view, op.id)?;
    let target = op.required_ref(base::TARGET_CRS, "TargetCRS")?;
    let target_crs = projected_crs(op.model, target)?;
    let map_unit = target_crs
        .map_unit
        .clone()
        .unwrap_or_else(|| project_unit.clone());
    let metres = map_unit.metres_per_unit;
    let translation = [
        op.finite(
            slot::FIRST_COORDINATE,
            "FirstCoordinate",
            coordinates.first * metres,
        )?,
        op.finite(
            slot::SECOND_COORDINATE,
            "SecondCoordinate",
            coordinates.second * metres,
        )?,
        op.finite(
            slot::HEIGHT,
            "Height",
            coordinates.height.unwrap_or(0.0) * metres,
        )?,
    ];
    Ok(ProjectToMap::new(
        source,
        op.id,
        OperationKind::RigidOperation {
            height: coordinates.height,
        },
        target_crs,
        [
            coordinates.first,
            coordinates.second,
            coordinates.height.unwrap_or(0.0),
        ],
        (project_unit, map_unit),
        1.0,
        (1.0, 0.0),
        [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        translation,
    ))
}

/// The two coordinates, their shared measure, and the optional height.
struct Coordinates {
    measure: &'static str,
    first: f64,
    second: f64,
    height: Option<f64>,
}

fn coordinates(op: &Operation<'_, '_>) -> GeorefResult<Coordinates> {
    let (first_measure, first) = measure(op, slot::FIRST_COORDINATE, "FirstCoordinate")?;
    let (second_measure, second) = measure(op, slot::SECOND_COORDINATE, "SecondCoordinate")?;
    let measure = match (first_measure, second_measure) {
        (Some(a), Some(b)) if a == b => a,
        _ => {
            return Err(GeorefError::RuleViolation {
                entity: op.id,
                rule: "SameCoordinateType",
            })
        }
    };
    let height = op
        .optional_number(slot::HEIGHT, "Height")?
        .map(|value| op.finite(slot::HEIGHT, "Height", value))
        .transpose()?;
    Ok(Coordinates {
        measure,
        first: op.finite(slot::FIRST_COORDINATE, "FirstCoordinate", first)?,
        second: op.finite(slot::SECOND_COORDINATE, "SecondCoordinate", second)?,
        height,
    })
}

/// One `IfcMeasureValue` slot: which `SameCoordinateType` branch its type
/// satisfies (`None` for neither, including an untyped REAL), and its value.
fn measure(
    op: &Operation<'_, '_>,
    index: usize,
    name: &'static str,
) -> GeorefResult<(Option<&'static str>, f64)> {
    let invalid = GeorefError::InvalidAttribute {
        entity: op.id,
        index,
        name,
    };
    match op.entity.attribute(index) {
        None | Some(Value::Null) => Err(GeorefError::MissingAttribute {
            entity: op.id,
            index,
            name,
        }),
        Some(Value::Typed { type_name, value }) => {
            let number = value.as_f64().ok_or(invalid)?;
            let branch = match type_name.to_ascii_uppercase().as_str() {
                "IFCLENGTHMEASURE" | "IFCPOSITIVELENGTHMEASURE" | "IFCNONNEGATIVELENGTHMEASURE" => {
                    Some(LENGTH)
                }
                "IFCPLANEANGLEMEASURE" | "IFCPOSITIVEPLANEANGLEMEASURE" => Some(PLANE_ANGLE),
                _ => None,
            };
            Ok((branch, number))
        }
        Some(value) => value.as_f64().map(|n| (None, n)).ok_or(invalid),
    }
}
