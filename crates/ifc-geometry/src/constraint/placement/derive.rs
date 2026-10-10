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
//!
//! # What an `IfcParameterValue` means here
//!
//! `IfcPointByDistanceExpression.DistanceAlong` is an `IfcCurveMeasureSelect`:
//! a length, or a parameter of the basis curve (IFC4.3 ADD2 8.9.3.48). A
//! parameter is only as defined as the basis curve's parameterisation:
//!
//! - `IfcPolyline` counts one per segment (8.9.3.51), and so does a
//!   line-only `IfcIndexedPolyCurve` (see the `polyline` submodule); the
//!   neutral polyline uses the same convention, so the parameter crosses
//!   unchanged. Only where every segment is one unit long does it equal a
//!   length.
//! - `IfcGradientCurve` takes the parameter of its `BaseCurve` (8.9.3.34.1),
//!   an `IfcCompositeCurve` of `IfcCurveSegment`s. A composite accumulates
//!   the parametric ranges of its parent curves (8.9.3.20.1, after
//!   ISO 10303-42), which are not lengths (an `IfcCircle` counts its angle,
//!   8.9.3.18.1; an `IfcClothoid` `u = s / (A sqrt(pi))`, 8.9.3.19.1), while
//!   `IfcCurveSegment` (8.9.3.28.1) states that no parametric space is yet
//!   defined for its parent curves and measures segments by length. The
//!   parameter of an alignment centreline is therefore undefined, and is
//!   refused by name rather than handed to an evaluator, whose own parameter
//!   on that curve is plan distance (axiolid ADR 0082), a different quantity.
//!   An `IfcAlignment` basis names the same centreline and is refused alike.
//!   This matches the station lowering, which refuses a parameter too.
//!
//! # The derived frame (#355)
//!
//! IFC4.3 ADD2 fixes the frame, so it is built here from the evaluator's
//! point and unit tangent alone, never from the evaluator's own axis
//! labelling. That labelling is `x` the tangent, `y` up and `z` to the
//! right (the contract's text, corrected by axiolid/kernel#242 to what the
//! reference provider always returned), so the frame below is `(x, -z, y)`
//! of it for a reference-up evaluator; reading only `x` keeps the frame
//! right for any provider whatever it calls its other axes, and a banked
//! curve's roll never reaches it.
//!
//! - `x` is the tangent: `IfcAxis2PlacementLinear.RefDirection` defaults to
//!   it (8.9.3.4), and `OffsetLongitudinal` runs along it.
//! - `y` is the horizontal left, `normalise(Z x tangent)`:
//!   `IfcPointByDistanceExpression.OffsetLateral` is horizontal and
//!   "positive values indicate to the left of the basis curve".
//! - `z = x x y` is up, perpendicular to the tangent in its vertical plane:
//!   that is where `OffsetVertical` points, and what `Axis` defaults to,
//!   "the normal vector of the basis curve that lies within the vertical
//!   plane containing the RefDirection".
//!
//! Explicit `Axis`/`RefDirection` on the placement are components in that
//! `(tangent, left, up)` frame (8.9.3.4), the reading the station lowering
//! uses too, and [`derive_linear_placement_transform`] composes them.
//!
//! # Tangent discontinuities (#409)
//!
//! IFC4.3 ADD2 8.9.3.48.3: "If DistanceAlong coincides with a point of
//! tangential discontinuity (within precision limits), then the tangent of
//! the previous segment governs." The derivation reads such a placement as
//! the station lowering does (`lower::station`, #346): the seams are read
//! from the basis curve's stored data, a `DistanceAlong` within the model's
//! declared `Precision` (capped at 1 mm) or rounding of one is snapped to
//! the seam's own distance, and the evaluator is asked for the frame there
//! with [`CurveEvaluator::frame_at_on`] and [`SeamSide::Incoming`]. Off a
//! seam it is asked side-lessly, as before. The derived placement thus
//! equals the lowered station's frame on and near a seam, and `Verify`
//! compares a cache against the incoming frame.
//!
//! A native parameter on a polyline vertex is located by its distance, an
//! exact running sum of edge lengths, and read at that distance: the
//! contract lets a provider refuse the incoming side of a polyline's native
//! parameter, and the distance names the same place.
//!
//! An evaluator that does not implement seam sides refuses `Incoming` with
//! `SEAM_SIDE_UNSUPPORTED`; that is [`GeometryError::SeamSideUnsupported`]
//! naming the placement, its basis curve and the seam, never the outgoing
//! frame in its place.
//!
//! # Curve-relation bases (#418)
//!
//! A plain `IfcCompositeCurve`, an `IfcTrimmedCurve` (and the other
//! composites and surface curves the curve lowering reads as relations), and
//! a composite of `IfcCurveSegment`s placed by `IfcAxis2PlacementLinear`s
//! lower to a curve relation, which the station lowering measures end to
//! end through its pieces (#346). The derivation reads the same basis as
//! Axiolid's neutral `CurvePath` (axiolid/kernel#290), built from the stored
//! relation in the `path` submodule, through the evaluator's `path_*`
//! queries: [`CurveEvaluator::path_frame_at`] off a seam, and
//! [`CurveEvaluator::path_frame_at_on`] with [`SeamSide::Incoming`] on one.
//! Every joint of the relation is a seam, as for the station, so a
//! `DistanceAlong` within precision of a joint reads the segment that ends
//! there, IFC's previous segment. The distance is the relation's: arc
//! length, or plan distance where every piece is a gradient curve. A native
//! parameter is refused by name: a relation's pieces have no parameter that
//! runs through them, and the station lowering refuses one too.
//!
//! An evaluator that does not implement curve paths refuses them with
//! `CURVE_PATH_UNSUPPORTED`; that is [`GeometryError::CurvePathUnsupported`]
//! naming the placement and its basis curve, never a frame read on one
//! piece in its place. An offset curve is read the same way (#414): its
//! pieces are `PathCurve::Offset`s of its basis's spans, measured in the
//! offset's own length, as the station lowering measures it. Where that
//! length is a quadrature (an offset beside a gradient curve, a spiral or a
//! B-spline, and a relation with an ellipse or a B-spline piece), the path
//! cannot be stated without an execution provider and the placement is
//! refused by [`GeometryError::PathPieceLengthUnstated`] (#427), never
//! derived from an estimate.
//!
//! # Which distance
//!
//! `DistanceAlong` on an alignment centreline is plan distance (an
//! `IfcGradientCurve` takes its `BaseCurve`'s measure, 8.9.3.34.1), and arc
//! length along a straight-segment curve. An evaluator states which
//! distance it measures per curve ([`CurveEvaluator::distance_convention`]);
//! one that measures something else is refused rather than trusted.
//!
//! # Checking a cached position (#354)
//!
//! `IfcLinearPlacement.CartesianPosition` is, in IFC4.3's words, "an
//! optional fallback for the RelativePlacement attribute" for importers that
//! do not support linear placement. The linear expression is authoritative.
//! When a caller supplies an evaluator, [`CachedPositionPolicy`] decides what
//! happens to a cached position; see there.

use axiolid_contracts::GeomError;
use axiolid_core::Frame3;
use axiolid_curve::Curve3;
use axiolid_curve_evaluate_contract::{
    CurveEvaluator, CurveMeasure as KernelMeasure, DistanceConvention, SeamSide,
    SEAM_SIDE_UNSUPPORTED,
};
use axiolid_model::GeometryNode;
use ifc_alignment::{
    AlignmentUnits, CurveMeasure, LinearPlacement, PointByDistance, SeamTolerance,
};
use ifc_model::{EntityId, Model, Value};

use crate::constraint::tolerance;
use crate::error::{GeometryError, GeometryResult};
use crate::lower::curve::lower_curve_node;
use crate::lower::session::LoweringSession;
use crate::lower::station::seams::{nearest, seams3, Seam};
use crate::resource::direction::resolve_unit;
use crate::transform::Transform;
use crate::units::UnitScale;

mod path;
mod polyline;

pub use path::basis_curve_path;

/// What lowering does with an `IfcLinearPlacement`'s cached
/// `CartesianPosition` when the caller supplies a [`CurveEvaluator`] (#354).
///
/// Without an evaluator nothing changes: the cache is used as it is, and a
/// placement without one is refused. With one, an uncached placement is
/// derived, and a cached one is handled by this policy.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum CachedPositionPolicy {
    /// Derive the position from the linear expression and compare the cache
    /// with it. The default.
    ///
    /// - The cache's location within the model's tolerance (see
    ///   [`cached_position_tolerance`]) of the derived origin: the DERIVED
    ///   frame is used, because the linear expression is authoritative.
    /// - Beyond it: [`GeometryError::CachedPlacementMismatch`] naming the
    ///   placement and both positions. A stale cache, e.g. after the
    ///   alignment was edited, would otherwise place the product silently
    ///   wrong, and a caller who supplied an evaluator can derive the truth.
    /// - A derivation the bridge refuses (an `IfcParameterValue` on an
    ///   alignment, a basis curve it cannot lower) is that refusal: a cache
    ///   that cannot be checked is not checked.
    ///
    /// Only the location is compared. The cache's axes are not: IFC defaults
    /// `Axis` to the curve normal in the vertical plane, tilted with the
    /// grade, and exporters that write the cache with a plumb `Z` are not
    /// stale.
    #[default]
    Verify,
    /// Use a cached position as it is, without deriving or comparing; derive
    /// only placements that have no cache.
    ///
    /// For a model whose caches are trusted, or whose linear expressions
    /// this bridge cannot derive (an `IfcParameterValue` on an alignment is
    /// refused by name, #347) while the cache still places the product as
    /// the exporter computed it.
    Trust,
}

/// A caller-supplied evaluator and what to do with a cached position.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Derivation<'e> {
    pub(crate) evaluator: &'e dyn CurveEvaluator,
    pub(crate) cached: CachedPositionPolicy,
}

/// Resolve an `IfcLinearPlacement` with an evaluator, in metres, relative
/// to the frame its basis curve is stated in (#357).
///
/// `resolved` is the placement as `ifc-alignment` read it, in FILE units;
/// `cached` its `CartesianPosition`, if any, as a transform in metres
/// relative to the same frame; `parent` that frame's world transform in
/// metres. The cache is compared in world coordinates, so a mismatch names
/// world positions.
pub(crate) fn resolve_linear(
    model: &Model,
    units: &UnitScale,
    placement: EntityId,
    resolved: &LinearPlacement,
    cached: Option<Transform>,
    derivation: Derivation<'_>,
    parent: &Transform,
) -> GeometryResult<Transform> {
    if let (Some(cached), CachedPositionPolicy::Trust) = (cached, derivation.cached) {
        return Ok(cached);
    }
    let derived = linear_transform(model, units, placement, resolved, derivation.evaluator)?;
    if let Some(cached) = cached {
        check_cache(
            model,
            units,
            placement,
            &parent.compose(&cached),
            &parent.compose(&derived),
        )?;
    }
    Ok(derived)
}

/// Refuse a cached position farther than the model's tolerance from the
/// derived one.
fn check_cache(
    model: &Model,
    units: &UnitScale,
    placement: EntityId,
    cached: &Transform,
    derived: &Transform,
) -> GeometryResult<()> {
    let precision = cached_position_tolerance(model, units)?;
    let [a, b] = [cached.origin, derived.origin];
    let distance = tolerance::distance(a, b);
    // The floating-point rounding term the alignment seams use
    // (`ifc_alignment::SeamTolerance`): 1e-9 relative, floor 1.
    let tolerance = tolerance::point_tolerance(precision, a, b);
    if distance.is_finite() && distance <= tolerance {
        return Ok(());
    }
    Err(GeometryError::CachedPlacementMismatch {
        placement,
        cached: a,
        derived: b,
        distance,
        tolerance,
    })
}

/// The tolerance a cached `CartesianPosition` is checked at, in metres
/// (#354).
///
/// IFC defines `IfcGeometricRepresentationContext.Precision` as "the
/// tolerance under which two given points are still assumed to be
/// identical", so that is the tolerance: the coarsest `Precision` any 3D
/// root context declares (sub-contexts derive theirs from it), in the
/// project length unit and converted to metres. A model that declares none
/// gets IFC's own default, `1.E-5` project units, which is what
/// `IfcGeometricRepresentationSubContext.Precision` derives to without a
/// parent value (`NVL(ParentContext.Precision, 1.E-5)`, IFC4 ADD2 TC1).
///
/// # Errors
///
/// A declared `Precision` that is not a finite positive number is
/// [`GeometryError::Degenerate`] naming the context: a check against it
/// would accept or refuse everything.
pub fn cached_position_tolerance(model: &Model, units: &UnitScale) -> GeometryResult<f64> {
    Ok(units.length(tolerance::model_precision(model)?))
}

/// Resolve an `IfcLinearPlacement` from its linear expression alone, in
/// metres, ignoring any cached `CartesianPosition`.
///
/// Reads `RelativePlacement`: its `IfcPointByDistanceExpression` through
/// [`derive_placement_transform`], and its optional `Axis`/`RefDirection`,
/// composed in the `(tangent, left, up)` frame (module documentation).
///
/// The result is in the coordinate system the basis curve is stated in --
/// the object coordinate system of the alignment that carries it -- not
/// in the world: `PlacementRelTo` and the alignment's own `ObjectPlacement`
/// are not composed here (#357). [`super::product_world_transform_with_evaluator`]
/// and the evaluator-taking lowering compose that chain.
///
/// # Errors
///
/// As [`derive_placement_transform`]; a malformed placement (wrong types,
/// `Axis` without `RefDirection`) is [`GeometryError::Unsupported`] naming
/// it, a missing direction [`GeometryError::MissingEntity`], and parallel
/// axes [`GeometryError::Degenerate`].
pub fn derive_linear_placement_transform(
    model: &Model,
    units: &UnitScale,
    placement: EntityId,
    evaluator: &dyn CurveEvaluator,
) -> GeometryResult<Transform> {
    let resolved = resolve_in_file_units(model, units, placement)?;
    linear_transform(model, units, placement, &resolved, evaluator)
}

/// Read an `IfcLinearPlacement` through `ifc-alignment`, keeping lengths in
/// file units: [`derive_placement_transform`] converts them itself.
pub(crate) fn resolve_in_file_units(
    model: &Model,
    units: &UnitScale,
    placement: EntityId,
) -> GeometryResult<LinearPlacement> {
    let file_units = AlignmentUnits {
        length_to_metres: 1.0,
        angle_to_radians: units.angle_to_radians,
    };
    ifc_alignment::resolve_linear_placement(model, placement, file_units).map_err(|_error| {
        GeometryError::Unsupported {
            entity: placement,
            type_name: "IFCLINEARPLACEMENT".into(),
            detail: "linear placement is malformed; ifc-alignment refused it",
        }
    })
}

/// The derived frame of `resolved`, with explicit axes applied, in metres.
fn linear_transform(
    model: &Model,
    units: &UnitScale,
    placement: EntityId,
    resolved: &LinearPlacement,
    evaluator: &dyn CurveEvaluator,
) -> GeometryResult<Transform> {
    let base = derive_placement_transform(
        model,
        units,
        placement,
        &resolved.relative_placement,
        evaluator,
    )?;
    if !resolved.has_explicit_axes {
        return Ok(base);
    }
    let (axis, ref_direction) = explicit_axes(model, placement)?;
    let local =
        Transform::from_axes([0.0; 3], Some(axis), Some(ref_direction)).ok_or_else(|| {
            GeometryError::Degenerate {
                entity: placement,
                type_name: "IFCLINEARPLACEMENT".into(),
                detail: "IfcAxis2PlacementLinear Axis and RefDirection are parallel (WR2)".into(),
            }
        })?;
    Ok(base.compose(&local))
}

/// `RelativePlacement.Axis` and `.RefDirection`, as unit components.
fn explicit_axes(model: &Model, placement: EntityId) -> GeometryResult<([f64; 3], [f64; 3])> {
    // IFC4X3_ADD2: IfcLinearPlacement.RelativePlacement is slot 1;
    // IfcAxis2PlacementLinear has Location, Axis, RefDirection at 0..2.
    let reference = |owner: EntityId, slot: usize| -> GeometryResult<EntityId> {
        model
            .get(owner)
            .and_then(|entity| entity.attributes.get(slot))
            .and_then(Value::as_ref_id)
            .ok_or(GeometryError::Unsupported {
                entity: owner,
                type_name: "IFCAXIS2PLACEMENTLINEAR".into(),
                detail: "IfcAxis2PlacementLinear states Axis and RefDirection only in part",
            })
    };
    let linear = reference(placement, 1)?;
    let axis = resolve_unit(model, linear, reference(linear, 1)?)?;
    let ref_direction = resolve_unit(model, linear, reference(linear, 2)?)?;
    Ok((axis, ref_direction))
}

/// Resolve an `IfcPointByDistanceExpression` by evaluating its basis curve.
///
/// `evaluator` supplies the capability; this function supplies the IFC
/// reading and the unit handling. `expression` is in FILE units (read it
/// with `ifc_alignment` and a length factor of 1, as
/// [`derive_linear_placement_transform`] does); the distance and offsets are
/// converted to metres before they cross the boundary, because the kernel is
/// unitless.
///
/// The frame is `(tangent, left, up)` and the offsets follow IFC4.3: see
/// the module documentation. The placement's own `Axis`/`RefDirection` are
/// not read here; [`derive_linear_placement_transform`] applies them.
///
/// Refuses, rather than approximating, when:
///
/// - the basis curve is not one this bridge can lower to a `Curve3`
/// - the evaluator reports it cannot measure distance on that curve, or
///   measures a different distance than IFC states for it
/// - the authored value is an `IfcParameterValue` on a basis curve whose
///   IFC parameterisation is undefined (an alignment centreline)
/// - the tangent is vertical, so there is no horizontal left
/// - `DistanceAlong` lies on a tangent discontinuity and the evaluator
///   cannot read its incoming side ([`GeometryError::SeamSideUnsupported`],
///   module documentation)
/// - the basis curve is a curve relation and the evaluator reads single
///   curves only ([`GeometryError::CurvePathUnsupported`]), or the
///   authored value is an `IfcParameterValue` along one
pub fn derive_placement_transform(
    model: &Model,
    units: &UnitScale,
    placement: EntityId,
    expression: &PointByDistance,
    evaluator: &dyn CurveEvaluator,
) -> GeometryResult<Transform> {
    let parameter_requested = matches!(expression.distance_along, CurveMeasure::Parameter(_));
    let (curve, convention) = match basis_curve(
        model,
        units,
        placement,
        expression.basis_curve,
        parameter_requested,
        evaluator,
    )? {
        Basis::Curve(curve, convention) => (curve, convention),
        Basis::Path(basis) => {
            return path::derive_on_path(model, units, placement, expression, &basis, evaluator)
        }
    };

    // `IfcCurveMeasureSelect` says which method of measurement the file
    // means. Carry that across rather than collapsing it to a number: a
    // parameter passed as a distance places the product plausibly wrong.
    let at = match expression.distance_along {
        CurveMeasure::Length(value) => {
            // A distance is only as good as the evaluator's agreement on
            // which distance it is (the contract says to consult this).
            if evaluator.distance_convention(&curve) != convention {
                return Err(GeometryError::Unsupported {
                    entity: placement,
                    type_name: "IFCLINEARPLACEMENT".into(),
                    detail: "the evaluator measures a different distance along this basis \
                             curve than IFC states (plan distance on an alignment, arc \
                             length on a polyline)",
                });
            }
            KernelMeasure::Distance(units.length(value))
        }
        CurveMeasure::Parameter(value) => KernelMeasure::Parameter(value),
    };

    let refused = |error: GeomError| GeometryError::Unsupported {
        entity: placement,
        type_name: "IFCLINEARPLACEMENT".into(),
        detail: refusal_detail(&error),
    };
    let frame = match seam_at(model, units, placement, &curve, at)? {
        // On a tangent discontinuity the previous segment governs
        // (8.9.3.48.3): the incoming side, read at the seam's own distance,
        // as `lower::station` stores it (module documentation).
        Some(seam) => {
            if !at.is_distance() && evaluator.distance_convention(&curve) != convention {
                return Err(GeometryError::Unsupported {
                    entity: placement,
                    type_name: "IFCLINEARPLACEMENT".into(),
                    detail: SEAM_PARAMETER_CONVENTION,
                });
            }
            evaluator
                .frame_at_on(&curve, KernelMeasure::Distance(seam), SeamSide::Incoming)
                .map_err(|error| match error {
                    GeomError::UnsupportedInput { input, .. } if input == SEAM_SIDE_UNSUPPORTED => {
                        GeometryError::SeamSideUnsupported {
                            placement,
                            basis: expression.basis_curve,
                            distance: seam,
                        }
                    }
                    other => refused(other),
                })?
        }
        None => evaluator.frame_at(&curve, at).map_err(refused)?,
    };

    offset_frame(&frame, expression, units).ok_or(GeometryError::Unsupported {
        entity: placement,
        type_name: "IFCLINEARPLACEMENT".into(),
        detail: path::VERTICAL_BASIS,
    })
}

/// The distance of the tangent discontinuity `at` lies on, in metres, if
/// any (#409).
///
/// The same reading as `lower::station`: the seams are read from the
/// curve's stored data (`lower::station::seams`), and `at` is on one within
/// the model's declared `Precision` (capped at 1 mm) or rounding. A native
/// parameter is located by its distance: on the neutral polyline, the only
/// curve a parameter reaches here, that is an exact running sum.
fn seam_at(
    model: &Model,
    units: &UnitScale,
    placement: EntityId,
    curve: &Curve3,
    at: KernelMeasure,
) -> GeometryResult<Option<f64>> {
    let distance = match at {
        KernelMeasure::Distance(distance) => Some(distance),
        KernelMeasure::Parameter(parameter) => polyline_distance(curve, parameter),
        _ => None,
    };
    let Some(distance) = distance.filter(|d| d.is_finite()) else {
        // Not a place along the curve: the evaluator refuses it by name.
        return Ok(None);
    };
    let seams = seams3(curve).map_err(|reason| GeometryError::Unsupported {
        entity: placement,
        type_name: "IFCLINEARPLACEMENT".into(),
        detail: reason,
    })?;
    snap(model, units, placement, &seams, distance)
}

/// The distance of the seam of `seams` that `distance` lies on within the
/// model's precision, if any.
fn snap(
    model: &Model,
    units: &UnitScale,
    placement: EntityId,
    seams: &[Seam],
    distance: f64,
) -> GeometryResult<Option<f64>> {
    if seams.is_empty() || !distance.is_finite() {
        return Ok(None);
    }
    let precision = SeamTolerance::for_model(
        model,
        AlignmentUnits {
            length_to_metres: units.length_to_metres,
            angle_to_radians: units.angle_to_radians,
        },
    )
    .map_err(|_| GeometryError::Degenerate {
        entity: placement,
        type_name: "IFCLINEARPLACEMENT".into(),
        detail: "the model's declared Precision is not a usable seam tolerance".into(),
    })?
    .length();
    Ok(nearest(seams, distance, precision).map(|seam| seam.distance))
}

/// The distance along a neutral polyline at its native `parameter` (one
/// unit per segment, see the `polyline` submodule); `None` off its range or
/// on another curve.
fn polyline_distance(curve: &Curve3, parameter: f64) -> Option<f64> {
    let Curve3::Polyline(polyline) = curve else {
        return None;
    };
    let lengths: Vec<f64> = polyline
        .points
        .windows(2)
        .map(|w| (w[1] - w[0]).length())
        .collect();
    let segments = lengths.len() as f64;
    if !parameter.is_finite() || parameter < 0.0 || parameter > segments {
        return None;
    }
    let whole = (parameter.floor() as usize).min(lengths.len().saturating_sub(1));
    let before: f64 = lengths[..whole].iter().sum();
    Some(before + (parameter - whole as f64) * lengths.get(whole).copied().unwrap_or(0.0))
}

/// Why a native parameter on a seam is refused when the evaluator measures
/// another distance there.
const SEAM_PARAMETER_CONVENTION: &str =
    "DistanceAlong is a parameter on a tangent discontinuity of the basis curve, which is \
     read at the seam's distance from the incoming side (IFC4.3 ADD2 8.9.3.48.3), and the \
     evaluator measures a different distance along this curve";

/// A basis curve as the evaluator reads it.
// Large only by the path, which lives for one derivation.
#[allow(clippy::large_enum_variant)]
enum Basis {
    /// One neutral curve and the distance IFC states along it.
    Curve(Curve3, DistanceConvention),
    /// A curve relation, as a path (#418).
    Path(path::PathBasis),
}

/// The IFC curve types that lower to a curve relation, read as a path
/// (#418): composites, trims, surface curves whose 3D curve governs, and
/// offset curves (#414).
const RELATION_BASES: &[&str] = &[
    "IFCCOMPOSITECURVE",
    "IFCCOMPOSITECURVEONSURFACE",
    "IFCBOUNDARYCURVE",
    "IFCOUTERBOUNDARYCURVE",
    "IFCTRIMMEDCURVE",
    "IFCSURFACECURVE",
    "IFCINTERSECTIONCURVE",
    "IFCSEAMCURVE",
    "IFCOFFSETCURVE2D",
    "IFCOFFSETCURVE3D",
    "IFCOFFSETCURVEBYDISTANCES",
];

/// The basis curve as a neutral `Curve3`, or as a curve path.
///
/// An alignment centreline is the case that matters: `IfcGradientCurve`
/// pairs a plan with a vertical profile, which `ifc-alignment` already
/// composes exactly. Straight-segment curves (`IfcPolyline`, a line-only
/// `IfcIndexedPolyCurve`) lower to the neutral polyline, whose arc length is
/// an exact finite sum. A composite or trimmed curve is read as the curve
/// path the station lowering measures (module documentation). Other curve
/// families -- an ellipse, a B-spline -- are refused by name here rather
/// than lowered approximately, because a placement derived from a curve we
/// guessed at is worse than one we declined to derive.
fn basis_curve(
    model: &Model,
    units: &UnitScale,
    placement: EntityId,
    basis: EntityId,
    parameter_requested: bool,
    evaluator: &dyn CurveEvaluator,
) -> GeometryResult<Basis> {
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
        // The curve representation lowers exactly through the same path a
        // representation item takes. `ifc_alignment::gradient_curve3` reads an
        // `IfcAlignment` only, so handing it the curve always refused.
        // Refused before lowering: the answer does not depend on the curve.
        "IFCGRADIENTCURVE" | "IFCALIGNMENT" if parameter_requested => {
            Err(GeometryError::Unsupported {
                entity: basis,
                type_name: entity.type_name.to_string(),
                detail: UNDEFINED_ALIGNMENT_PARAMETER,
            })
        }
        "IFCGRADIENTCURVE" => Ok(Basis::Curve(
            gradient_curve(model, units, basis)?,
            DistanceConvention::PlanDistance,
        )),
        "IFCALIGNMENT" => ifc_alignment::gradient_curve3(model, basis, alignment_units)
            .map(|curve| Basis::Curve(curve, DistanceConvention::PlanDistance))
            .map_err(|_error| GeometryError::Unsupported {
                entity: placement,
                type_name: entity.type_name.to_string(),
                detail: "basis curve does not compose an exact centreline",
            }),
        "IFCPOLYLINE" => Ok(Basis::Curve(
            polyline::polyline(model, units, basis, entity)?,
            DistanceConvention::ArcLength3d,
        )),
        "IFCINDEXEDPOLYCURVE" => Ok(Basis::Curve(
            polyline::indexed_polycurve(model, units, basis, entity, parameter_requested)?,
            DistanceConvention::ArcLength3d,
        )),
        // Refused before lowering: the answer does not depend on the curve.
        kind if RELATION_BASES.contains(&kind) && parameter_requested => {
            Err(GeometryError::Unsupported {
                entity: basis,
                type_name: kind.to_owned(),
                detail: path::RELATION_PARAMETER,
            })
        }
        kind if RELATION_BASES.contains(&kind) => {
            path::path_basis(model, units, placement, basis, kind, evaluator).map(Basis::Path)
        }
        other => Err(GeometryError::Unsupported {
            entity: placement,
            type_name: other.to_owned(),
            detail: "deriving a placement frame needs an alignment centreline, a \
                     straight-segment polyline or a composite or trimmed curve as basis curve",
        }),
    }
}

/// Why an `IfcParameterValue` along an alignment centreline is refused.
///
/// See the module documentation for the IFC4.3 ADD2 clauses.
const UNDEFINED_ALIGNMENT_PARAMETER: &str =
    "an IfcParameterValue DistanceAlong on an alignment centreline: the parameter \
     space of a composite of IfcCurveSegments is undefined in IFC4.3 ADD2; state an \
     IfcLengthMeasure";

/// An `IfcGradientCurve` basis curve as its exact `Curve3::Elevated`, in metres.
///
/// A refusal names the gradient curve (or the nested entity) and its own
/// reason, such as a vertical arc with no neutral elevation law.
fn gradient_curve(model: &Model, units: &UnitScale, basis: EntityId) -> GeometryResult<Curve3> {
    let mut session = LoweringSession::new(model, units);
    let root = lower_curve_node(&mut session, basis, Transform::identity())?;
    let lowered = session.finish(root)?;
    match lowered.graph.get(lowered.root) {
        Some(GeometryNode::Curve3(curve)) => Ok(curve.clone()),
        _ => Err(GeometryError::Unsupported {
            entity: basis,
            type_name: "IFCGRADIENTCURVE".into(),
            detail: "the gradient curve did not lower to a single neutral Curve3",
        }),
    }
}

/// The IFC frame at the evaluated point, with the authored offsets applied.
///
/// Built from `frame.origin` and the unit tangent `frame.x` only (module
/// documentation): `left = normalise(Z x tangent)`, `up = tangent x left`.
/// `OffsetLateral` moves along `left`, `OffsetVertical` along `up` and
/// `OffsetLongitudinal` along the tangent. `None` when the tangent is
/// vertical and `left` is undefined.
fn offset_frame(
    frame: &Frame3,
    expression: &PointByDistance,
    units: &UnitScale,
) -> Option<Transform> {
    let tangent = [frame.x.x, frame.x.y, frame.x.z];
    let horizontal = (tangent[0] * tangent[0] + tangent[1] * tangent[1]).sqrt();
    if !horizontal.is_finite() || horizontal <= 1e-12 {
        return None;
    }
    // Z x tangent = (-t.y, t.x, 0).
    let left = [-tangent[1] / horizontal, tangent[0] / horizontal, 0.0];
    let up = [
        tangent[1] * left[2] - tangent[2] * left[1],
        tangent[2] * left[0] - tangent[0] * left[2],
        tangent[0] * left[1] - tangent[1] * left[0],
    ];

    let lateral = units.length(expression.offset_lateral.unwrap_or(0.0));
    let vertical = units.length(expression.offset_vertical.unwrap_or(0.0));
    let longitudinal = units.length(expression.offset_longitudinal.unwrap_or(0.0));
    let point = [frame.origin.x, frame.origin.y, frame.origin.z];
    let origin = std::array::from_fn(|i| {
        point[i] + left[i] * lateral + up[i] * vertical + tangent[i] * longitudinal
    });
    Some(Transform {
        basis: [tangent, left, up],
        origin,
    })
}

/// Name why the evaluator declined.
///
/// The kernel distinguishes an unsupported operation from a rejected
/// measure and from a degenerate curve, and that difference is actionable:
/// the first means the file needs a different curve family, the others mean
/// the placement itself is unusable. Collapsing them to one message would
/// hide that. The reference evaluator reports an undefined roll (tangent
/// parallel to the up reference) and a distance past the curve's end as
/// invalid input, and a degenerate curve piece as degenerate.
fn refusal_detail(error: &GeomError) -> &'static str {
    match error {
        GeomError::Unsupported { .. } | GeomError::UnsupportedInput { .. } => {
            "the evaluator cannot measure distance on this curve family"
        }
        GeomError::InvalidInput(_) => {
            "the evaluator rejected the measure: it lies off the curve, or roll is \
             undefined because the curve tangent is parallel to the up reference"
        }
        GeomError::Degenerate(_) => "the evaluator found the basis curve degenerate there",
        _ => "the evaluator refused this placement",
    }
}
