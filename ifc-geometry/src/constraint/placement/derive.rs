//! Deriving a placement frame from the basis curve.
//!
//! The cached `CartesianPosition` path resolves a placement without any
//! computation. When an authoring tool omits it, the frame has to be
//! derived by evaluating the basis curve at the authored distance, which is
//! computation and therefore an opt-in capability under ADR 0004.
//!
//! This module holds the IFC-side half: it turns an `IfcLinearPlacement`
//! into a curve, a measure and an offset, then asks an injected
//! [`CurveEvaluator`] for the frame. It never picks an evaluator, and
//! `ifc-geometry` links no implementation.

use axiolid_contracts::GeomError;
use axiolid_core::Frame3;
use axiolid_curve::Curve3;
use axiolid_curve_evaluate_contract::{CurveEvaluator, CurveMeasure as KernelMeasure};
use ifc_alignment::{AlignmentUnits, CurveMeasure, PointByDistance};
use ifc_model::{EntityId, Model};

use crate::error::{GeometryError, GeometryResult};
use crate::transform::Transform;
use crate::units::UnitScale;

/// Resolve an `IfcLinearPlacement` by evaluating its basis curve.
///
/// `evaluator` supplies the capability; this function supplies the IFC
/// reading and the unit handling. The distance is converted to metres
/// before it crosses the boundary, because the kernel is unitless.
///
/// Refuses, rather than approximating, when:
///
/// - the basis curve is not one this bridge can lower to a `Curve3`
/// - the evaluator reports it cannot measure distance on that curve
/// - the authored value is a parameter but the caller wanted a distance
/// - roll is undefined because the tangent is vertical
pub fn derive_placement_transform(
    model: &Model,
    units: &UnitScale,
    placement: EntityId,
    expression: &PointByDistance,
    evaluator: &dyn CurveEvaluator,
) -> GeometryResult<Transform> {
    let curve = basis_curve3(model, units, placement, expression.basis_curve)?;

    // `IfcCurveMeasureSelect` says which method of measurement the file
    // means. Carry that across rather than collapsing it to a number: a
    // parameter passed as a distance places the product plausibly wrong.
    let at = match expression.distance_along {
        CurveMeasure::Length(value) => KernelMeasure::Distance(units.length(value)),
        CurveMeasure::Parameter(value) => KernelMeasure::Parameter(value),
    };

    let frame = evaluator
        .frame_at(&curve, at)
        .map_err(|error| GeometryError::Unsupported {
            entity: placement,
            type_name: "IFCLINEARPLACEMENT".into(),
            detail: refusal_detail(&error),
        })?;

    Ok(offset_frame(frame, expression, units))
}

/// The basis curve as a neutral `Curve3`.
///
/// An alignment centreline is the case that matters: `IfcGradientCurve`
/// pairs a plan with a vertical profile, which `ifc-alignment` already
/// composes exactly. Other curve families are refused by name here rather
/// than lowered approximately, because a placement derived from a curve we
/// guessed at is worse than one we declined to derive.
fn basis_curve3(
    model: &Model,
    units: &UnitScale,
    placement: EntityId,
    basis: EntityId,
) -> GeometryResult<Curve3> {
    let entity = model.get(basis).ok_or(GeometryError::MissingEntity {
        referrer: placement,
        missing: basis,
    })?;
    let alignment_units = AlignmentUnits {
        length_to_metres: units.length_to_metres,
        angle_to_radians: units.angle_to_radians,
    };
    // IFC lets the BasisCurve be the alignment itself or its curve
    // representation. Both name the same centreline, so both resolve; a
    // file that uses one is not less valid than one that uses the other.
    match entity.type_name.as_ref() {
        "IFCALIGNMENT" | "IFCGRADIENTCURVE" => {
            ifc_alignment::gradient_curve3(model, basis, alignment_units).map_err(|_error| {
                GeometryError::Unsupported {
                    entity: placement,
                    type_name: entity.type_name.to_string(),
                    detail: "basis curve does not compose an exact centreline",
                }
            })
        }
        other => Err(GeometryError::Unsupported {
            entity: placement,
            type_name: other.to_owned(),
            detail: "deriving a placement frame needs an alignment centreline as basis curve",
        }),
    }
}

/// Apply the authored lateral, vertical and longitudinal offsets.
///
/// The frame axes carry the convention: `x` is the tangent, `z` is right,
/// `y` is up. Offsets are applied along those axes, so a lateral offset
/// moves across the carriageway regardless of heading.
fn offset_frame(frame: Frame3, expression: &PointByDistance, units: &UnitScale) -> Transform {
    let lateral = units.length(expression.offset_lateral.unwrap_or(0.0));
    let vertical = units.length(expression.offset_vertical.unwrap_or(0.0));
    let longitudinal = units.length(expression.offset_longitudinal.unwrap_or(0.0));

    let origin = [
        frame.origin.x + frame.z.x * lateral + frame.y.x * vertical + frame.x.x * longitudinal,
        frame.origin.y + frame.z.y * lateral + frame.y.y * vertical + frame.x.y * longitudinal,
        frame.origin.z + frame.z.z * lateral + frame.y.z * vertical + frame.x.z * longitudinal,
    ];
    Transform {
        basis: [
            [frame.x.x, frame.x.y, frame.x.z],
            [frame.y.x, frame.y.y, frame.y.z],
            [frame.z.x, frame.z.y, frame.z.z],
        ],
        origin,
    }
}

/// Name why the evaluator declined.
///
/// The kernel distinguishes an unsupported operation from a degenerate
/// input, and that difference is actionable: the first means the file needs
/// a different curve family, the second means the placement itself is
/// unusable. Collapsing both to one message would hide that.
fn refusal_detail(error: &GeomError) -> &'static str {
    match error {
        GeomError::Unsupported { .. } => {
            "the evaluator cannot measure distance on this curve family"
        }
        GeomError::Degenerate { .. } => {
            "roll is undefined here: the curve tangent is parallel to the up reference"
        }
        _ => "the evaluator refused this placement",
    }
}
