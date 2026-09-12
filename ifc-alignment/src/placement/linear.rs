//! `IfcLinearPlacement` and `IfcPointByDistanceExpression`.
//!
//! IFC4X3 constrains `IfcAxis2PlacementLinear.Location` (WR1) to always be an
//! `IfcPointByDistanceExpression`: linear placement is defined entirely in
//! terms of distance-along a basis curve plus lateral/vertical/longitudinal
//! offsets, never a bare Cartesian point. Resolving it therefore means
//! resolving the distance expression, not a coordinate triple.

use ifc_model::value::Value;
use ifc_model::{EntityId, Model};

use crate::error::{AlignmentError, AlignmentResult};
use crate::horizontal::AlignmentUnits;

/// `DistanceAlong` may be an absolute length or a normalized parameter; the
/// two are never comparable without knowing which was authored.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CurveMeasure {
    /// `IfcLengthMeasure`: absolute distance along the basis curve.
    Length(f64),
    /// `IfcParameterValue`: dimensionless curve parameter.
    Parameter(f64),
}

/// Resolved `IfcPointByDistanceExpression`.
#[derive(Debug, Clone, PartialEq)]
pub struct PointByDistance {
    /// The `IfcPointByDistanceExpression` entity this was read from.
    pub entity: EntityId,
    /// `DistanceAlong`: absolute length or normalized parameter, per
    /// `CurveMeasure`.
    pub distance_along: CurveMeasure,
    /// `OffsetLateral`, when authored.
    pub offset_lateral: Option<f64>,
    /// `OffsetVertical`, when authored.
    pub offset_vertical: Option<f64>,
    /// `OffsetLongitudinal`, when authored.
    pub offset_longitudinal: Option<f64>,
    /// `BasisCurve`: the curve `distance_along` is measured against.
    pub basis_curve: EntityId,
}

/// Resolved `IfcLinearPlacement`.
#[derive(Debug, Clone, PartialEq)]
pub struct LinearPlacement {
    /// The `IfcLinearPlacement` entity this was read from.
    pub entity: EntityId,
    /// `RelativePlacement.Location`: the resolved `IfcPointByDistanceExpression`.
    pub relative_placement: PointByDistance,
    /// `IfcAxis2PlacementLinear.Axis`/`RefDirection`, if the source stated an
    /// explicit orientation rather than deriving one from the basis curve.
    pub has_explicit_axes: bool,
}

/// Read one `IfcPointByDistanceExpression` referenced by `id`.
///
/// Fails if `id` is missing, is not an `IfcPointByDistanceExpression`, or
/// any attribute is missing or holds a value of the wrong kind.
pub fn resolve_point_by_distance(
    model: &Model,
    id: EntityId,
    units: AlignmentUnits,
) -> AlignmentResult<PointByDistance> {
    let entity = model
        .get(id)
        .ok_or(AlignmentError::MissingEntity { entity: id })?;
    if !entity.is_type("IFCPOINTBYDISTANCEEXPRESSION") {
        return Err(AlignmentError::WrongType {
            entity: id,
            expected: "IFCPOINTBYDISTANCEEXPRESSION",
            actual: entity.type_name.to_string(),
        });
    }
    // IFC4X3_ADD2: `IfcPoint` and `IfcGeometricRepresentationItem` and
    // `IfcRepresentationItem` contribute no attributes ahead of this
    // declaration's own slots 0..4: DistanceAlong, OffsetLateral,
    // OffsetVertical, OffsetLongitudinal, BasisCurve.
    let values = &entity.attributes;
    let distance_along = curve_measure(values, id, 0, "DistanceAlong", units)?;
    let offset_lateral = optional_number(values, id, 1, "OffsetLateral")?.map(|v| length(v, units));
    let offset_vertical =
        optional_number(values, id, 2, "OffsetVertical")?.map(|v| length(v, units));
    let offset_longitudinal =
        optional_number(values, id, 3, "OffsetLongitudinal")?.map(|v| length(v, units));
    let basis_curve = reference(values, id, 4, "BasisCurve")?;

    Ok(PointByDistance {
        entity: id,
        distance_along,
        offset_lateral,
        offset_vertical,
        offset_longitudinal,
        basis_curve,
    })
}

/// Resolve one `IfcLinearPlacement` referenced by `id` to its distance-along
/// point on a basis curve.
///
/// Fails if `id` is missing or not an `IfcLinearPlacement`, `RelativePlacement`
/// does not resolve to an `IfcAxis2PlacementLinear` whose `Location` is an
/// `IfcPointByDistanceExpression` (WR1), or `Axis`/`RefDirection` are supplied
/// one without the other (WR2).
pub fn resolve_linear_placement(
    model: &Model,
    id: EntityId,
    units: AlignmentUnits,
) -> AlignmentResult<LinearPlacement> {
    let entity = model
        .get(id)
        .ok_or(AlignmentError::MissingEntity { entity: id })?;
    if !entity.is_type("IFCLINEARPLACEMENT") {
        return Err(AlignmentError::WrongType {
            entity: id,
            expected: "IFCLINEARPLACEMENT",
            actual: entity.type_name.to_string(),
        });
    }
    // IFC4X3_ADD2: inherited PlacementRelTo is slot 0; RelativePlacement and
    // CartesianPosition are this declaration's own slots 1..2.
    let values = &entity.attributes;
    let relative_placement_id = reference(values, id, 1, "RelativePlacement")?;
    let placement_entity =
        model
            .get(relative_placement_id)
            .ok_or(AlignmentError::DanglingReference {
                entity: id,
                attribute: "RelativePlacement",
                target: relative_placement_id,
            })?;
    if !placement_entity.is_type("IFCAXIS2PLACEMENTLINEAR") {
        return Err(AlignmentError::WrongType {
            entity: relative_placement_id,
            expected: "IFCAXIS2PLACEMENTLINEAR",
            actual: placement_entity.type_name.to_string(),
        });
    }
    // IFC4X3_ADD2: inherited Location is slot 0; Axis/RefDirection are this
    // declaration's own slots 1..2. WR1 requires Location to be a
    // IfcPointByDistanceExpression -- enforced here, not assumed.
    let placement_values = &placement_entity.attributes;
    let location_id = reference(placement_values, relative_placement_id, 0, "Location")?;
    let relative_placement = resolve_point_by_distance(model, location_id, units)?;
    let has_explicit_axes = matches!(placement_values.get(1), Some(Value::Ref(_)))
        || matches!(placement_values.get(2), Some(Value::Ref(_)));
    // WR2: Axis and RefDirection are both-or-neither.
    let axis_present = matches!(placement_values.get(1), Some(Value::Ref(_)));
    let ref_direction_present = matches!(placement_values.get(2), Some(Value::Ref(_)));
    if axis_present != ref_direction_present {
        return Err(AlignmentError::InvalidSegment {
            entity: relative_placement_id,
            detail: "IfcAxis2PlacementLinear.Axis and RefDirection must both be present or both absent (WR2)",
        });
    }

    Ok(LinearPlacement {
        entity: id,
        relative_placement,
        has_explicit_axes,
    })
}

fn curve_measure(
    values: &[Value],
    id: EntityId,
    slot: usize,
    name: &'static str,
    units: AlignmentUnits,
) -> AlignmentResult<CurveMeasure> {
    // `IfcCurveMeasureSelect = SELECT (IfcLengthMeasure, IfcParameterValue)`.
    // Both wrap a bare real; the codec preserves which was declared as the
    // `Typed` wrapper's type name, and losing that distinction here would
    // make a normalized parameter silently readable as an absolute length.
    match values.get(slot) {
        Some(Value::Typed { type_name, value }) => {
            let raw = value.as_f64().ok_or(AlignmentError::InvalidAttribute {
                entity: id,
                index: slot,
                name,
            })?;
            if type_name.eq_ignore_ascii_case("IFCLENGTHMEASURE") {
                Ok(CurveMeasure::Length(raw * units.length_to_metres))
            } else if type_name.eq_ignore_ascii_case("IFCPARAMETERVALUE") {
                Ok(CurveMeasure::Parameter(raw))
            } else {
                Err(AlignmentError::InvalidAttribute {
                    entity: id,
                    index: slot,
                    name,
                })
            }
        }
        Some(Value::Real(value)) => Ok(CurveMeasure::Length(value * units.length_to_metres)),
        Some(Value::Integer(value)) => {
            Ok(CurveMeasure::Length(*value as f64 * units.length_to_metres))
        }
        _ => Err(AlignmentError::InvalidAttribute {
            entity: id,
            index: slot,
            name,
        }),
    }
}

fn optional_number(
    values: &[Value],
    id: EntityId,
    slot: usize,
    name: &'static str,
) -> AlignmentResult<Option<f64>> {
    match values.get(slot) {
        Some(Value::Null) => Ok(None),
        Some(value) => {
            value
                .unwrap_typed()
                .as_f64()
                .map(Some)
                .ok_or(AlignmentError::InvalidAttribute {
                    entity: id,
                    index: slot,
                    name,
                })
        }
        None => Err(AlignmentError::InvalidAttribute {
            entity: id,
            index: slot,
            name,
        }),
    }
}

fn reference(
    values: &[Value],
    id: EntityId,
    slot: usize,
    name: &'static str,
) -> AlignmentResult<EntityId> {
    values
        .get(slot)
        .and_then(|value| value.as_ref_id())
        .ok_or(AlignmentError::InvalidAttribute {
            entity: id,
            index: slot,
            name,
        })
}

fn length(value: f64, units: AlignmentUnits) -> f64 {
    value * units.length_to_metres
}
