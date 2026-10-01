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
//! horizontal layout as ONE intrinsic curve with a piecewise curvature law
//! ([`lower_horizontal_plan`]), not the per-segment composite. Its
//! parameter is distance along the plan from the first horizontal
//! segment's start. Seams the plan could not verify in closed form are
//! reported on the result.

use axiolid_curve::{Curve3, Elevated3};
use axiolid_model::{GeometryGraphBuilder, GeometryNode};
use ifc_model::{EntityId, Model};

use super::assemble::{finish, LoweredAlignmentCurve};
use super::elevation::profile_law;
use super::plan::lower_horizontal_plan;
use super::seam::HorizontalSeam;
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
/// with no exact elevation law, a non-contiguous profile, and any plan
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
    let (curve, ids, seams) = compose(model, &view, horizontal, vertical, units, cant.as_ref())?;

    let mut builder = GeometryGraphBuilder::new();
    let root =
        builder
            .push(GeometryNode::Curve3(curve))
            .map_err(|error| AlignmentError::Graph {
                detail: error.to_string(),
            })?;

    let mut sources = vec![horizontal, vertical];
    sources.extend(ids);
    let mut lowered = finish(builder, root, sources)?;
    lowered.seams = seams;
    Ok(lowered)
}

/// The single nested layout of a given type.
///
/// IFC allows an alignment to nest several layouts; pairing an arbitrary one
/// would silently pick a road other than the one the caller meant.
fn sole_layout(
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
fn compose(
    model: &Model,
    view: &AlignmentView<'_>,
    horizontal: EntityId,
    vertical: EntityId,
    units: AlignmentUnits,
    cant: Option<&CantLayout>,
) -> AlignmentResult<(Curve3, Vec<EntityId>, Vec<HorizontalSeam>)> {
    let plan = lower_horizontal_plan(model, horizontal, units, cant)?;
    let ids = view.segment_chain(vertical, "IfcAlignmentVerticalSegment")?;
    let mut segments = Vec::with_capacity(ids.len());
    for id in &ids {
        segments.push(read_vertical_segment(model, *id, units)?);
    }
    let elevation = profile_law(&segments)?;
    Ok((
        Curve3::Elevated(Elevated3::new(plan.curve, elevation)),
        ids,
        plan.seams,
    ))
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
    let (curve, _, _) = compose(model, &view, horizontal, vertical, units, cant.as_ref())?;
    Ok(curve)
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
