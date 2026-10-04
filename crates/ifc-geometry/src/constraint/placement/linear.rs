//! Linear placement: where a product sits along an alignment.
//!
//! IFC4x3 places road and rail furniture by distance along a curve rather
//! than by a chain of nested boxes, so the `IfcLocalPlacement` walk does not
//! reach them and refuses by type.
//!
//! This module resolves the cached `CartesianPosition` an authoring tool
//! writes alongside the linear expression. Deriving the transform from the
//! basis curve needs a caller-supplied `CurveEvaluator` (the opt-in
//! `compile` feature, ADR 0004): with one, `derive::resolve_linear` derives
//! an uncached placement and checks a cached one (#353, #354); without one,
//! an uncached placement is refused by name, never approximated.
//!
//! ADR 0003 (amended 2026-09-15) permits the `ifc-alignment` dependency:
//! bridges may depend on bridges.

use ifc_model::{EntityId, Model};

use super::LinearResolution;
use crate::error::{GeometryError, GeometryResult};
use crate::resource::placement::axis_placement_transform;
use crate::transform::Transform;
use crate::units::UnitScale;

/// `IfcLinearPlacement` attribute slots.
///
/// IFC4X3_ADD2: inherited `PlacementRelTo` is slot 0; `RelativePlacement`
/// and `CartesianPosition` are this declaration own slots 1..2.
mod slot {
    /// `CartesianPosition`: the cached explicit placement, optional.
    pub const CARTESIAN_POSITION: usize = 2;
}

/// Is this entity an `IfcLinearPlacement`?
pub(crate) fn is_linear_placement(model: &Model, placement: EntityId) -> bool {
    model
        .get(placement)
        .is_some_and(|entity| entity.is_type("IFCLINEARPLACEMENT"))
}

/// Resolve an `IfcLinearPlacement` to a world transform, in metres.
///
/// # Errors
///
/// Without an evaluator in `linear`, refuses when the placement states only
/// the linear expression. Computing a frame from the basis curve needs
/// curve evaluation at a distance, which this crate never does itself: a
/// wrong sign on the offset puts a sign on the wrong side of a carriageway,
/// so a typed refusal names the evaluator-taking entry points instead.
pub(crate) fn linear_placement_transform(
    model: &Model,
    units: &UnitScale,
    placement: EntityId,
    linear: LinearResolution<'_>,
) -> GeometryResult<Transform> {
    let entity = model.get(placement).ok_or(GeometryError::MissingEntity {
        referrer: placement,
        missing: placement,
    })?;
    let cached = entity
        .attributes
        .get(slot::CARTESIAN_POSITION)
        .and_then(ifc_model::Value::as_ref_id);

    #[cfg(feature = "compile")]
    if let Some(derivation) = linear.derivation {
        let resolved = super::derive::resolve_in_file_units(model, units, placement)?;
        let cached = cached
            .map(|cartesian| cached_transform(model, units, placement, cartesian))
            .transpose()?;
        return super::derive::resolve_linear(
            model, units, placement, &resolved, cached, derivation,
        );
    }
    #[cfg(not(feature = "compile"))]
    let _ = linear;

    // Validate the linear expression even when the cached position is used:
    // a file whose expression is malformed is broken whether or not a
    // shortcut happens to be present.
    let alignment_units = ifc_alignment::AlignmentUnits {
        length_to_metres: units.length_to_metres,
        angle_to_radians: units.angle_to_radians,
    };
    ifc_alignment::resolve_linear_placement(model, placement, alignment_units).map_err(
        |_error| GeometryError::Unsupported {
            entity: placement,
            type_name: "IFCLINEARPLACEMENT".into(),
            detail: "linear placement is malformed; ifc-alignment refused it",
        },
    )?;

    let Some(cartesian) = cached else {
        // Without the cached shortcut the frame has to be derived by
        // evaluating the basis curve, which is computation and therefore
        // opt-in under ADR 0004. Point the caller at the capability rather
        // than approximating a position here.
        return Err(GeometryError::Unsupported {
            entity: placement,
            type_name: "IFCLINEARPLACEMENT".into(),
            detail: "placement states only IfcPointByDistanceExpression and no CartesianPosition; \
                     derive the frame with a CurveEvaluator (feature `compile`: \
                     LoweringSession::with_curve_evaluator or \
                     product_world_transform_with_evaluator)",
        });
    };
    cached_transform(model, units, placement, cartesian)
}

/// The cached `CartesianPosition` as a transform, in metres.
fn cached_transform(
    model: &Model,
    units: &UnitScale,
    placement: EntityId,
    cartesian: EntityId,
) -> GeometryResult<Transform> {
    let cartesian_entity = model.get(cartesian).ok_or(GeometryError::MissingEntity {
        referrer: placement,
        missing: cartesian,
    })?;
    Ok(axis_placement_transform(model, cartesian, cartesian_entity)?.to_metres(units))
}
