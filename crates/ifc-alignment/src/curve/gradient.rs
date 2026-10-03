//! The 3D centreline: a horizontal layout paired with its vertical profile.
//!
//! This is the `IfcGradientCurve` idea. The plan and the profile are both
//! exact and stay exact: `Curve3::Elevated` composes them rather than
//! re-encoding either, so a clothoid stays a clothoid and a parabolic
//! vertical curve stays a parabola.
//!
//! Height is a function of distance along the PLAN, not along the 3D curve.
//! The two diverge by sqrt(1 + g^2) wherever grade is non-zero, and the
//! kernel names that convention in the type, so nothing here re-derives it.
//!
//! `Elevated3.plan` is a single `Curve2`, so the plan is the whole
//! horizontal layout as ONE curve parameterised by arc length
//! ([`lower_horizontal_plan`]): an intrinsic curve with a piecewise
//! curvature law, or an arc-length chain when the layout holds a `CUBIC`,
//! never the per-segment composite. Its parameter is distance along the
//! plan from the first horizontal segment's start. Seams the plan could not
//! verify in closed form are reported on the result.

use axiolid_curve::{Curve2, Curve3, Elevated3};
use axiolid_model::{GeometryGraphBuilder, GeometryNode};
use ifc_model::{EntityId, Model};

use super::assemble::{finish, LoweredAlignmentCurve};
use super::elevation::{indexed_from_plan_start, profile_law_within};
use super::plan::lower_horizontal_plan;
use super::seam::HorizontalSeam;
use super::tolerance::SeamTolerance;
use crate::cant::CantLayout;
use crate::error::{AlignmentError, AlignmentResult};
use crate::horizontal::AlignmentUnits;
use crate::vertical::read_vertical_segment;
use crate::view::AlignmentView;

/// Lower an `IfcAlignment` to an exact 3D centreline.
///
/// Pairs the alignment horizontal layout with its vertical profile as a
/// `Curve3::Elevated`. Both halves stay exact and either can be recovered
/// unchanged.
///
/// # Errors
///
/// Refuses an alignment without both layouts, a vertical segment family
/// with no determined elevation law (`CLOTHOID`), a non-contiguous profile,
/// and any plan
/// [`lower_horizontal_plan`] refuses (a segment without an exact law, a
/// heading kink, or a closed-form position gap). Seams after a transition
/// spiral are reported in [`LoweredAlignmentCurve::seams`], not refused.
pub fn lower_gradient_curve(
    model: &Model,
    entity: EntityId,
    units: AlignmentUnits,
) -> AlignmentResult<LoweredAlignmentCurve> {
    let view = AlignmentView::for_model(model)?;
    let alignment = model
        .get(entity)
        .ok_or(AlignmentError::MissingEntity { entity })?;
    if !view.schema.is_a(&alignment.type_name, "IfcAlignment") {
        return Err(AlignmentError::WrongType {
            entity,
            expected: "IfcAlignment",
            actual: alignment.type_name.to_string(),
        });
    }

    let horizontal = sole_layout(&view, entity, "IfcAlignmentHorizontal")?;
    let vertical = sole_layout(&view, entity, "IfcAlignmentVertical")?;

    let cant = optional_cant(model, &view, entity, units)?;
    let composed = compose(model, &view, horizontal, vertical, units, cant.as_ref())?;

    let mut builder = GeometryGraphBuilder::new();
    let root = builder
        .push(GeometryNode::Curve3(Curve3::Elevated(composed.centreline)))
        .map_err(|error| AlignmentError::Graph {
            detail: error.to_string(),
        })?;

    let mut sources = vec![horizontal, vertical];
    sources.extend(composed.vertical_ids);
    let mut lowered = finish(builder, root, sources)?;
    lowered.seams = composed.seams;
    Ok(lowered)
}

/// The single nested layout of a given type.
///
/// IFC allows an alignment to nest several layouts; pairing an arbitrary one
/// would silently pick a road other than the one the caller meant.
pub(super) fn sole_layout(
    view: &AlignmentView,
    alignment: EntityId,
    expected: &'static str,
) -> AlignmentResult<EntityId> {
    let children = view.nested_children(alignment, expected)?;
    match children.as_slice() {
        [only] => Ok(*only),
        [] => Err(AlignmentError::SemanticViolation {
            entity: Some(alignment),
            rule: "a gradient curve needs both a horizontal and a vertical layout",
        }),
        _ => Err(AlignmentError::SemanticViolation {
            entity: Some(alignment),
            rule: "an alignment with several layouts of one kind is ambiguous to compose",
        }),
    }
}

/// The composed centreline, without a surrounding graph.
///
/// Shared by the graph lowering and by callers that need the curve itself
/// to hand to an evaluator. One composition, so the two cannot drift.
pub(super) fn compose(
    model: &Model,
    view: &AlignmentView<'_>,
    horizontal: EntityId,
    vertical: EntityId,
    units: AlignmentUnits,
    cant: Option<&CantLayout>,
) -> AlignmentResult<Composed> {
    let plan = lower_horizontal_plan(model, horizontal, units, cant)?;
    let ids = view.segment_chain(vertical, "IfcAlignmentVerticalSegment")?;
    let mut segments = Vec::with_capacity(ids.len());
    for id in &ids {
        segments.push(read_vertical_segment(model, *id, units)?);
    }
    // The profile's seams are checked at the precision the file declares
    // (#141), the same rule `vertical_profile_law` applies.
    let tolerance = SeamTolerance::for_model(model, units)?;
    let profile = profile_law_within(&segments, tolerance)?;
    // `profile_law` is indexed from the first StartDistAlong; the plan from
    // its own start. Re-index so both halves read the same distance, and
    // refuse a profile that does not cover the whole plan.
    let start = segments.first().map_or(0.0, |s| s.start_dist_along);
    let end = segments
        .last()
        .map_or(0.0, |s| s.start_dist_along + s.horizontal_length);
    let plan_length = match &plan.curve {
        Curve2::Intrinsic(intrinsic) => Some(intrinsic.length),
        Curve2::Chain(chain) => chain.length(),
        _ => None,
    }
    .ok_or_else(|| AlignmentError::Graph {
        detail: "the horizontal plan did not lower to an arc-length curve".to_owned(),
    })?;
    let elevation = indexed_from_plan_start(profile, start, end, plan_length, vertical, tolerance)?;
    Ok(Composed {
        centreline: Elevated3::new(plan.curve, elevation),
        plan_length,
        vertical_ids: ids,
        seams: plan.seams,
    })
}

/// The composed centreline and what the callers report about it.
pub(super) struct Composed {
    /// Plan and profile, both exact.
    pub(super) centreline: Elevated3,
    /// Arc length of the plan.
    pub(super) plan_length: f64,
    /// The vertical segments, in authored order.
    pub(super) vertical_ids: Vec<EntityId>,
    /// The plan's seams.
    pub(super) seams: Vec<HorizontalSeam>,
}

/// The exact 3D centreline of `entity`, an `IfcAlignment`.
///
/// Same composition the graph lowering uses, returned directly so a caller
/// can pass it to a `CurveEvaluator`. Returning the curve rather than a
/// point keeps evaluation the caller's choice: this crate stores exact
/// geometry and never computes on it.
pub fn gradient_curve3(
    model: &Model,
    entity: EntityId,
    units: AlignmentUnits,
) -> AlignmentResult<Curve3> {
    let view = AlignmentView::for_model(model)?;
    let alignment = model
        .get(entity)
        .ok_or(AlignmentError::MissingEntity { entity })?;
    if !view.schema.is_a(&alignment.type_name, "IfcAlignment") {
        return Err(AlignmentError::WrongType {
            entity,
            expected: "IfcAlignment",
            actual: alignment.type_name.to_string(),
        });
    }
    let horizontal = sole_layout(&view, entity, "IfcAlignmentHorizontal")?;
    let vertical = sole_layout(&view, entity, "IfcAlignmentVertical")?;
    let cant = optional_cant(model, &view, entity, units)?;
    let composed = compose(model, &view, horizontal, vertical, units, cant.as_ref())?;
    Ok(Curve3::Elevated(composed.centreline))
}

/// The alignment cant layout, when it has exactly one.
///
/// Cant is optional: a road alignment has none, and its absence is not an
/// error. Several cant layouts are ambiguous, which is.
fn optional_cant(
    model: &Model,
    view: &AlignmentView,
    alignment: EntityId,
    units: AlignmentUnits,
) -> AlignmentResult<Option<CantLayout>> {
    let children = view.nested_children(alignment, "IfcAlignmentCant")?;
    match children.as_slice() {
        [] => Ok(None),
        [only] => CantLayout::resolve(model, *only, units).map(Some),
        _ => Err(AlignmentError::SemanticViolation {
            entity: Some(alignment),
            rule: "an alignment with several cant layouts is ambiguous to compose",
        }),
    }
}
