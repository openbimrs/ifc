//! Exact curve lowering.
//!
//! # Scope
//!
//! Covers `IfcPolyline`, `IfcLine`, `IfcCircle`, `IfcTrimmedCurve`,
//! `IfcCompositeCurve`, and the explicit-knot
//! `IfcBSplineCurveWithKnots` / `IfcRationalBSplineCurveWithKnots`
//! subtypes. Convention-only `IfcBSplineCurve` and other families report a
//! typed `Unsupported` naming the entity, so a gap is a diagnostic rather
//! than a wrong shape.
//!
//! # Trim parameters are not all lengths
//!
//! `IfcTrimmedCurve` carries `IfcParameterValue`s in the *basis curve's own*
//! parameterisation. For an `IfcLine` that parameter is a length along the
//! direction vector and scales with the length unit. For an `IfcCircle` it is
//! an **angle** in the model's plane-angle unit. Applying the length factor to
//! a conic parameter silently rescales every arc: on a millimetre file the
//! 0.082 rad arc in `swept_disk_composite_arc_crankbar.ifc` would become 82
//! radians and wrap the circle thirteen times. The basis curve therefore
//! decides which unit conversion applies.
//!
//! # Why the frame is applied here and not deferred
//!
//! Curves are lowered as world-space geometry, matching `lower::brep`: the
//! caller's frame is applied to points and to conic frames as they are built.
//! Deferring would require every consumer to carry a parallel transform stack.

use axiolid_core::{Frame3, Point3, Vec3};
use axiolid_curve::{BSplineCurve3, Circle3, Curve3, KnotSpec, Line3, Polyline3};
use axiolid_model::{
    CurveRelation, CurveSegment, GeometryNode, NodeId, Transition, TrimSelector,
    TrimmingPreference as KernelPreference,
};
use ifc_model::EntityId;

use crate::curve::bspline::{BSplineCurve, KnotType};
use crate::curve::composite::{CompositeCurve, CompositeCurveSegment, TransitionCode};
use crate::curve::conic::Circle;
use crate::curve::line::Line;
use crate::curve::polyline::Polyline;
use crate::curve::trimmed::{TrimmedCurve, TrimmingPreference};
use crate::error::GeometryResult;
use crate::lower::session::LoweringSession;
use crate::resource::direction::resolve_unit;
use crate::resource::placement::axis_placement_transform;
use crate::resource::point::CartesianPoint;
use crate::transform::Transform;

/// Family label used for curve memoization.
const KIND: &str = "curve";

/// Lower any supported curve, returning its node.
pub fn lower_curve_node(
    session: &mut LoweringSession<'_>,
    id: EntityId,
    frame: Transform,
) -> GeometryResult<NodeId> {
    if let Some(node) = session.memoized(id, KIND, frame) {
        return Ok(node);
    }
    session.enter(id, KIND)?;
    let result = build(session, id, frame);
    session.exit(id);
    let node = result?;
    session.memoize(id, KIND, frame, node);
    Ok(node)
}

fn build(
    session: &mut LoweringSession<'_>,
    id: EntityId,
    frame: Transform,
) -> GeometryResult<NodeId> {
    let type_name = session.type_name(id)?;
    match type_name.as_str() {
        "IFCPOLYLINE" => polyline(session, id, frame),
        "IFCLINE" => line(session, id, frame),
        "IFCCIRCLE" => circle(session, id, frame),
        "IFCTRIMMEDCURVE" => trimmed(session, id, frame),
        "IFCCOMPOSITECURVE" => composite(session, id, frame),
        "IFCBSPLINECURVEWITHKNOTS" | "IFCRATIONALBSPLINECURVEWITHKNOTS" => {
            bspline(session, id, frame)
        }
        other => Err(session.unsupported(id, other, "curve family")),
    }
}

/// `IfcPolyline`: ordered points, closed when the last repeats the first.
fn polyline(
    session: &mut LoweringSession<'_>,
    id: EntityId,
    frame: Transform,
) -> GeometryResult<NodeId> {
    let entity = session.entity(id, id)?;
    let view = Polyline::new(id, entity);
    let closed = view.closes_by_repeating_first_point()?;
    let refs = view.point_refs()?;

    let mut points = Vec::with_capacity(refs.len());
    for point_ref in &refs {
        points.push(Point3::from_array(world_point(
            session, id, *point_ref, frame,
        )?));
    }
    // A closing duplicate is represented by the `closed` flag, not by a
    // repeated vertex: leaving both produces a zero-length final segment.
    if closed && points.len() > 1 {
        points.pop();
    }
    session.node_for(
        id,
        GeometryNode::Curve3(Curve3::Polyline(Polyline3 { points, closed })),
    )
}

/// `IfcLine`: origin point plus an `IfcVector` direction.
///
/// The vector's magnitude is the parameterisation scale and is preserved:
/// normalizing it would silently reparameterise every trim taken on the line.
fn line(
    session: &mut LoweringSession<'_>,
    id: EntityId,
    frame: Transform,
) -> GeometryResult<NodeId> {
    let entity = session.entity(id, id)?;
    let view = Line::new(id, entity);
    let origin = world_point(session, id, view.point_ref()?, frame)?;
    let vector_ref = view.direction_vector_ref()?;
    let direction = world_vector(session, id, vector_ref, frame)?;
    session.node_for(
        id,
        GeometryNode::Curve3(Curve3::Line(Line3 {
            origin: Point3::from_array(origin),
            direction: Vec3::from_array(direction),
        })),
    )
}

/// `IfcCircle`: a radius in the XY plane of its own placement.
fn circle(
    session: &mut LoweringSession<'_>,
    id: EntityId,
    frame: Transform,
) -> GeometryResult<NodeId> {
    let entity = session.entity(id, id)?;
    let view = Circle::new(id, entity);
    let radius = session.units().length(view.radius()?);
    let position_ref = view.position_ref()?;
    let position = session.entity(id, position_ref)?;
    let local = axis_placement_transform(session.model(), position_ref, position)?
        .to_metres(session.units());
    let placed = frame.compose(&local);
    session.node_for(
        id,
        GeometryNode::Curve3(Curve3::Circle(Circle3 {
            frame: frame3(&placed),
            radius,
        })),
    )
}

/// `IfcTrimmedCurve`: a basis curve plus two trim selectors.
///
/// The basis curve decides how a parameter selector is scaled: length for a
/// line, plane angle for a conic. See the module note.
fn trimmed(
    session: &mut LoweringSession<'_>,
    id: EntityId,
    frame: Transform,
) -> GeometryResult<NodeId> {
    let entity = session.entity(id, id)?;
    let view = TrimmedCurve::new(id, entity);
    let basis_ref = view.basis_curve_ref()?;
    let basis = lower_curve_node(session, basis_ref, frame)?;

    let basis_kind = session.type_name(basis_ref)?;
    let sense_agreement = view.sense_agreement()?;
    let preference = view.master_representation();
    let spec = view.spec()?;
    let (t1, t2) = spec.endpoints();

    let start = selectors(session, id, t1, &basis_kind, frame)?;
    let end = selectors(session, id, t2, &basis_kind, frame)?;
    if start.is_empty() || end.is_empty() {
        return Err(session.degenerate(
            id,
            "IFCTRIMMEDCURVE",
            "a trim end carries neither a parameter nor a point",
        ));
    }

    session.node_for(
        id,
        GeometryNode::CurveRelation(CurveRelation::Trimmed {
            basis,
            start,
            end,
            sense_agreement,
            preference: match preference {
                TrimmingPreference::Cartesian => KernelPreference::Cartesian,
                TrimmingPreference::Parameter => KernelPreference::Parameter,
                TrimmingPreference::Unspecified => KernelPreference::Unspecified,
            },
        }),
    )
}

/// Convert one trim end into kernel selectors, scaling by basis curve kind.
fn selectors(
    session: &mut LoweringSession<'_>,
    owner: EntityId,
    trim: crate::curve::trimmed::Trim,
    basis_kind: &str,
    frame: Transform,
) -> GeometryResult<Vec<TrimSelector>> {
    let mut out = Vec::new();
    if let Some(raw) = trim.parameter {
        out.push(TrimSelector::Parameter(scale_parameter(
            session, basis_kind, raw,
        )));
    }
    if let Some(point_ref) = trim.cartesian {
        let placed = world_point(session, owner, point_ref, frame)?;
        out.push(TrimSelector::Point3(Point3::from_array(placed)));
    }
    Ok(out)
}

/// Scale one trim parameter into kernel units, by basis curve kind.
///
/// Three parameterisations, not two:
///
/// - A conic (`IfcCircle`, `IfcEllipse`) parameterises by ANGLE.
/// - A polyline parameterises by POINT INDEX, and a composite curve by
///   SEGMENT INDEX. Both are dimensionless ordinals, so scaling them by the
///   length unit turns a millimetre file's `2.0` into `0.002` and collapses
///   the trim onto the curve's start.
/// - Everything else parameterises by LENGTH and converts.
pub(crate) fn scale_parameter(session: &LoweringSession<'_>, basis_kind: &str, raw: f64) -> f64 {
    match basis_kind {
        "IFCCIRCLE" | "IFCELLIPSE" => session.units().angle(raw),
        "IFCPOLYLINE" | "IFCCOMPOSITECURVE" => raw,
        _ => session.units().length(raw),
    }
}

/// `IfcCompositeCurve`: ordered segments, each wrapping a parent curve.
fn composite(
    session: &mut LoweringSession<'_>,
    id: EntityId,
    frame: Transform,
) -> GeometryResult<NodeId> {
    let entity = session.entity(id, id)?;
    let view = CompositeCurve::new(id, entity);
    let segment_refs = view.segment_refs()?;
    if segment_refs.is_empty() {
        return Err(session.degenerate(id, "IFCCOMPOSITECURVE", "no segments"));
    }

    let mut segments = Vec::with_capacity(segment_refs.len());
    for segment_ref in &segment_refs {
        let segment_entity = session.entity(id, *segment_ref)?;
        let segment = CompositeCurveSegment::new(*segment_ref, segment_entity);
        let parent = segment.parent_curve_ref()?;
        // Segment order is the curve's own traversal order and must be
        // preserved: sorting or deduplicating segments reorders the path.
        segments.push(CurveSegment {
            curve: lower_curve_node(session, parent, frame)?,
            same_sense: segment.same_sense()?,
            transition: transition(segment.transition()?),
        });
    }
    session.node_for(
        id,
        GeometryNode::CurveRelation(CurveRelation::Composite { segments }),
    )
}

/// Exact explicit-knot polynomial/rational B-spline curve.
fn bspline(
    session: &mut LoweringSession<'_>,
    id: EntityId,
    frame: Transform,
) -> GeometryResult<NodeId> {
    let type_name = session.type_name(id)?;
    let entity = session.entity(id, id)?;
    let view = BSplineCurve::new(id, entity);
    let degree = u16::try_from(view.degree()?)
        .map_err(|_| session.degenerate(id, &type_name, "Degree exceeds u16"))?;
    let knots = view.knots()?.ok_or_else(|| {
        session.unsupported(
            id,
            &type_name,
            "explicit knots required for lossless lowering",
        )
    })?;
    finite_values(session, id, &type_name, "Knots", &knots.values)?;
    let multiplicities = knots
        .multiplicities
        .into_iter()
        .map(|value| {
            u32::try_from(value)
                .map_err(|_| session.degenerate(id, &type_name, "multiplicity exceeds u32"))
        })
        .collect::<GeometryResult<Vec<_>>>()?;

    let refs = view.control_point_refs()?;
    let mut control_points = Vec::with_capacity(refs.len());
    for point_ref in refs {
        let placed = world_point(session, id, point_ref, frame)?;
        finite_values(
            session,
            id,
            &type_name,
            "transformed control point",
            &placed,
        )?;
        control_points.push(Point3::from_array(placed));
    }
    let weights = view.weights()?;
    if let Some(values) = weights.as_deref() {
        finite_values(session, id, &type_name, "WeightsData", values)?;
    }
    let closed = view.closed_curve().ok_or_else(|| {
        session.unsupported(
            id,
            &type_name,
            "unknown ClosedCurve is not lossless in bool",
        )
    })?;
    session.node_for(
        id,
        GeometryNode::Curve3(Curve3::BSpline(BSplineCurve3 {
            degree,
            control_points,
            knots: knots.values,
            multiplicities,
            weights,
            closed,
            self_intersect: view.self_intersect(),
            knot_spec: bspline_knot_spec(view.knot_spec()),
        })),
    )
}

fn bspline_knot_spec(source: KnotType) -> KnotSpec {
    match source {
        KnotType::Uniform => KnotSpec::Uniform,
        KnotType::QuasiUniform => KnotSpec::QuasiUniform,
        KnotType::PiecewiseBezier => KnotSpec::PiecewiseBezier,
        KnotType::Unspecified => KnotSpec::Unspecified,
    }
}

pub(super) fn finite_values(
    session: &LoweringSession<'_>,
    id: EntityId,
    type_name: &str,
    field: &'static str,
    values: &[f64],
) -> GeometryResult<()> {
    if values.iter().all(|value| value.is_finite()) {
        Ok(())
    } else {
        Err(session.degenerate(id, type_name, field))
    }
}

fn transition(code: TransitionCode) -> Transition {
    match code {
        TransitionCode::Discontinuous => Transition::Discontinuous,
        TransitionCode::Continuous => Transition::Continuous,
        TransitionCode::ContSameGradient => Transition::ContinuousSameGradient,
        TransitionCode::ContSameGradientSameCurvature => {
            Transition::ContinuousSameGradientSameCurvature
        }
    }
}

/// Resolve an `IfcCartesianPoint`, scale to metres, then place it.
fn world_point(
    session: &mut LoweringSession<'_>,
    owner: EntityId,
    point_ref: EntityId,
    frame: Transform,
) -> GeometryResult<[f64; 3]> {
    let entity = session.entity(owner, point_ref)?;
    let raw = CartesianPoint::new(point_ref, entity).coordinates_3d()?;
    let scaled = raw.map(|value| session.units().length(value));
    Ok(frame.apply(scaled))
}

/// Resolve an `IfcVector`: unit direction times magnitude, then rotate.
///
/// The magnitude is a length and scales; the direction is rotated only.
fn world_vector(
    session: &mut LoweringSession<'_>,
    owner: EntityId,
    vector_ref: EntityId,
    frame: Transform,
) -> GeometryResult<[f64; 3]> {
    let entity = session.entity(owner, vector_ref)?;
    let slots = crate::slots::Slots::new(vector_ref, entity);
    let direction_ref = slots.req_ref(0, "Orientation")?;
    let magnitude = session.units().length(slots.req_f64(1, "Magnitude")?);
    let unit = resolve_unit(session.model(), owner, direction_ref)?;
    let scaled = unit.map(|component| component * magnitude);
    Ok(frame.apply_direction(scaled))
}

/// Build a kernel frame from a placed transform.
fn frame3(t: &Transform) -> Frame3 {
    Frame3 {
        origin: Point3::from_array(t.origin),
        x: Vec3::from_array(t.basis[0]),
        y: Vec3::from_array(t.basis[1]),
        z: Vec3::from_array(t.basis[2]),
    }
}

#[cfg(test)]
mod tests;
