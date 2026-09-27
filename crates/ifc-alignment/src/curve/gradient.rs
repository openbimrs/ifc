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
//! One limit is structural, not an omission here. `Elevated3.plan` is a
//! single `Curve2`, and the neutral vocabulary has no composite `Curve2`:
//! a multi-segment plan lowers to a `CurveRelation::Composite`, which is a
//! graph node rather than a curve. Such a layout is refused by name.

use axiolid_curve::{Curve2, Curve3, Elevated3};
use axiolid_model::{CurveRelation, GeometryGraphBuilder, GeometryNode};
use ifc_model::{EntityId, Model};

use super::assemble::{finish, LoweredAlignmentCurve};
use super::elevation::profile_law;
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
/// with no exact elevation law, a non-contiguous profile, and a plan that
/// lowers to a composite rather than a single curve.
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

    // The plan must be a single curve: `Elevated3.plan` is one `Curve2`.
    let cant = optional_cant(model, &view, entity, units)?;
    let (curve, ids) = compose(model, &view, horizontal, vertical, units, cant.as_ref())?;

    let mut builder = GeometryGraphBuilder::new();
    let root =
        builder
            .push(GeometryNode::Curve3(curve))
            .map_err(|error| AlignmentError::Graph {
                detail: error.to_string(),
            })?;

    let mut sources = vec![horizontal, vertical];
    sources.extend(ids);
    finish(builder, root, sources)
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

/// The plan as one `Curve2`, or a refusal naming why it is not.
///
/// A single-segment layout lowers to a trim over one basis curve, which is
/// the curve wanted here. A multi-segment layout lowers to a composite, and
/// the neutral vocabulary has no composite `Curve2` to put in `plan`.
/// Flattening it to a B-spline would discard the exact spirals this
/// composition exists to preserve, so it is refused instead.
fn sole_plan_curve(
    model: &Model,
    horizontal: EntityId,
    units: AlignmentUnits,
    cant: Option<&CantLayout>,
) -> AlignmentResult<Curve2> {
    let lowered = super::assemble::lower_horizontal_layout(model, horizontal, units, cant)?;
    // A layout always lowers to a composite, even with one segment. Resolve
    // through it rather than scanning the graph: picking an arbitrary curve
    // node would silently elevate a fragment of the road.
    let Some(GeometryNode::CurveRelation(CurveRelation::Composite { segments })) =
        lowered.graph.get(lowered.root)
    else {
        return Err(AlignmentError::Unsupported {
            entity: horizontal,
            type_name: "IfcAlignmentHorizontal".to_owned(),
            detail: "plan did not lower to a composite curve",
        });
    };
    let [only] = segments.as_slice() else {
        return Err(AlignmentError::Unsupported {
            entity: horizontal,
            type_name: "IfcAlignmentHorizontal".to_owned(),
            detail: "a multi-segment plan has no single Curve2 to elevate; the neutral vocabulary has no composite Curve2",
        });
    };
    // The segment is a trim over the basis curve carrying the real geometry.
    if let Some(GeometryNode::CurveRelation(CurveRelation::Trimmed { basis, .. })) =
        lowered.graph.get(only.curve)
    {
        if let Some(GeometryNode::Curve2(curve)) = lowered.graph.get(*basis) {
            return Ok(curve.clone());
        }
    }
    Err(AlignmentError::Unsupported {
        entity: horizontal,
        type_name: "IfcAlignmentHorizontal".to_owned(),
        detail: "plan has no single basis curve to elevate",
    })
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
) -> AlignmentResult<(Curve3, Vec<EntityId>)> {
    let plan = sole_plan_curve(model, horizontal, units, cant)?;
    let ids = view.segment_chain(vertical, "IfcAlignmentVerticalSegment")?;
    let mut segments = Vec::with_capacity(ids.len());
    for id in &ids {
        segments.push(read_vertical_segment(model, *id, units)?);
    }
    let elevation = profile_law(&segments)?;
    Ok((Curve3::Elevated(Elevated3::new(plan, elevation)), ids))
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
    let (curve, _) = compose(model, &view, horizontal, vertical, units, cant.as_ref())?;
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
