//! `IfcCurveSegment` (IFC4X3): a placed piece of a parent curve.
//!
//! # What the entity states
//!
//! `IfcCurveSegment(Transition, Placement, SegmentStart, SegmentLength,
//! ParentCurve)`. The piece of `ParentCurve` from `SegmentStart` for
//! `SegmentLength` is moved rigidly so that its start point sits on
//! `Placement.Location` and its start tangent along `Placement.RefDirection`
//! ("as insertion point SegmentStart is the reference point of Placement;
//! RefDirection ... is bound to the parametrization sense of the segment").
//! The parent's own `Position` therefore cancels: only the parent's SHAPE
//! between the two measures matters. A negative `SegmentLength` walks the
//! parent backwards ("the sign of this value defines the sense agreement").
//!
//! # Measures are lengths
//!
//! Informal proposition 1: "until implementer agreements have resulted in a
//! defined parametric space for all possible types in attribute ParentCurve,
//! the values for attributes SegmentStart and SegmentLength shall be of type
//! IfcLengthMeasure", i.e. arc length along the parent. An
//! `IfcParameterValue` names a parameter space IFC has not defined and is a
//! typed refusal, not a guess.
//!
//! # What each parent lowers to
//!
//! - `IfcLine`: a two-point polyline, `|SegmentLength|` long.
//! - `IfcCircle`: a circle through the placement, trimmed by angle
//!   `|SegmentLength| / Radius`, turning left forwards and right backwards.
//! - a 2D `IfcPolyline`: the polyline cut at both arc lengths.
//! - the six `IfcSpiral` subtypes: a planar `Curve3::Intrinsic` carrying the
//!   spiral's law rebased to the segment (see `spiral.rs`).
//! - a 2D `IfcPolynomialCurve` (the `CUBIC` transition): its Bezier, placed,
//!   trimmed at `TrimSelector::ArcLength(SegmentLength)`; the parameter
//!   there inverts a non-elementary integral, which the kernel resolves
//!   (see `polynomial.rs`).
//!
//! A zero-length segment, which IFC4.3 puts at the end of every alignment
//! layout, is its placement exactly: a planar intrinsic curve of length
//! zero.

use axiolid_core::Frame3;
use axiolid_curve::{BSplineCurve3, Circle3, CurvatureLaw, Curve3, Intrinsic3, Polyline3};
use axiolid_model::{
    CurveRelation, CurveSegment, GeometryNode, NodeId, TrimSelector,
    TrimmingPreference as KernelPreference,
};
use ifc_model::{EntityId, Value};

use super::{frame3, lower_curve_node, polynomial, spiral, transition};
use crate::curve::composite::CompositeCurveSegment;
use crate::curve::polyline::Polyline;
use crate::error::GeometryResult;
use crate::lower::session::LoweringSession;
use crate::resource::placement::axis_placement_transform;
use crate::resource::point::CartesianPoint;
use crate::transform::Transform;

const TYPE: &str = "IFCCURVESEGMENT";

/// Why an `IfcParameterValue` measure is refused.
pub(crate) const PARAMETER_MEASURE: &str =
    "SegmentStart/SegmentLength given as IfcParameterValue: IFC4.3 ADD2 defines no parametric \
     space for IfcCurveSegment parents yet (informal proposition 1 requires IfcLengthMeasure)";

/// Why an `IfcAxis2PlacementLinear` placement is refused.
pub(crate) const LINEAR_PLACEMENT: &str =
    "an IfcAxis2PlacementLinear placement stands at a distance along a basis curve; the neutral \
     model has no distance-along-curve point relation to anchor it (#307)";

/// Why an `IfcPolynomialCurve` met on its own is refused.
pub(crate) const STANDALONE_POLYNOMIAL: &str =
    "an IfcPolynomialCurve is unbounded (-inf < u < inf) and the neutral vocabulary has no \
     unbounded polynomial curve; it lowers exactly as the ParentCurve of an IfcCurveSegment";

/// Why another parent family is refused.
const OTHER_PARENT: &str =
    "IfcCurveSegment ParentCurve family: only IfcLine, IfcCircle, a 2D IfcPolyline, a 2D \
     IfcPolynomialCurve and the IfcSpiral subtypes are lowered";

/// Why a scaled or mirrored frame is refused.
const NOT_RIGID: &str =
    "a scaled or mirrored frame changes an intrinsic curve's curvature and length; only rigid \
     frames are carried exactly";

/// An `IfcCurveSegment`, read and converted to metres.
#[derive(Debug, Clone)]
pub(crate) struct Segment {
    /// The segment entity.
    pub(crate) id: EntityId,
    /// `Placement` in metres, before the caller's frame.
    pub(crate) placement: Transform,
    /// Whether `Placement` is an `IfcAxis2Placement2D`.
    pub(crate) planar_placement: bool,
    /// `SegmentStart`, metres of arc length along the parent.
    pub(crate) start: f64,
    /// `SegmentLength`, signed metres of arc length.
    pub(crate) length: f64,
    /// `ParentCurve`.
    pub(crate) parent: EntityId,
    /// Upper-cased parent type.
    pub(crate) parent_kind: String,
}

/// Read segment `id`.
pub(crate) fn read_segment(session: &LoweringSession<'_>, id: EntityId) -> GeometryResult<Segment> {
    let slots = session.slots(id)?;
    let placement_ref = slots.req_ref(1, "Placement")?;
    let placement_entity = session.entity(id, placement_ref)?;
    let placement_kind = placement_entity.type_name.to_ascii_uppercase();
    let placement = match placement_kind.as_str() {
        "IFCAXIS2PLACEMENT2D" | "IFCAXIS2PLACEMENT3D" => {
            axis_placement_transform(session.model(), placement_ref, placement_entity)?
                .to_metres(session.units())
        }
        "IFCAXIS2PLACEMENTLINEAR" => {
            return Err(session.unsupported(id, TYPE, LINEAR_PLACEMENT));
        }
        _ => {
            return Err(session.unsupported(
                id,
                TYPE,
                "an IfcAxis1Placement has no RefDirection to fix the segment's start tangent",
            ));
        }
    };
    let start = measure(session, id, slots.req(2, "SegmentStart")?)?;
    let length = measure(session, id, slots.req(3, "SegmentLength")?)?;
    let parent = slots.req_ref(4, "ParentCurve")?;
    Ok(Segment {
        id,
        placement,
        planar_placement: placement_kind == "IFCAXIS2PLACEMENT2D",
        start,
        length,
        parent,
        parent_kind: session.type_name(parent)?,
    })
}

/// One `IfcCurveMeasureSelect`, as metres of arc length.
fn measure(session: &LoweringSession<'_>, id: EntityId, value: &Value) -> GeometryResult<f64> {
    let Value::Typed { type_name, value } = value else {
        return Err(session.degenerate(
            id,
            TYPE,
            "SegmentStart/SegmentLength must state its IfcCurveMeasureSelect type",
        ));
    };
    let name = type_name.to_ascii_uppercase();
    if name == "IFCPARAMETERVALUE" {
        return Err(session.unsupported(id, TYPE, PARAMETER_MEASURE));
    }
    let raw = value.as_f64().filter(|_| name.ends_with("LENGTHMEASURE"));
    let Some(raw) = raw else {
        return Err(session.degenerate(
            id,
            TYPE,
            format!("SegmentStart/SegmentLength must be an IfcLengthMeasure, found {name}"),
        ));
    };
    let metres = session.units().length(raw);
    if metres.is_finite() {
        Ok(metres)
    } else {
        Err(session.degenerate(id, TYPE, "SegmentStart/SegmentLength must be finite"))
    }
}

/// The segment's exact curvature law in its own arc length, `[0, |length|]`.
///
/// Signed in the segment's placement plane: positive turns towards the
/// placement's local Y. Refuses parents with no elementary law.
pub(crate) fn segment_curvature(
    session: &LoweringSession<'_>,
    segment: &Segment,
) -> GeometryResult<CurvatureLaw> {
    let kind = segment.parent_kind.as_str();
    let law =
        match kind {
            "IFCLINE" => CurvatureLaw::straight(),
            "IFCCIRCLE" => CurvatureLaw::circular(1.0 / circle_radius(session, segment)?),
            _ if spiral::is_spiral(kind) => {
                spiral::spiral_law(session, segment.parent, kind, segment.length.abs())?
            }
            // Lowered on its own, but its corners have no curvature law.
            "IFCPOLYLINE" => return Err(session.unsupported(
                segment.parent,
                kind,
                "an IfcPolyline parent has corners, which one intrinsic plan curve cannot carry",
            )),
            _ => return Err(refuse_parent(session, segment)),
        };
    spiral::piece_law(&law, segment.start, segment.length).ok_or_else(|| {
        session.degenerate(
            segment.id,
            TYPE,
            "the parent law could not be rebased to the segment",
        )
    })
}

/// The typed refusal for a parent this module does not lower.
pub(crate) fn refuse_parent(
    session: &LoweringSession<'_>,
    segment: &Segment,
) -> crate::GeometryError {
    session.unsupported(segment.parent, &segment.parent_kind, OTHER_PARENT)
}

/// `IfcCircle.Radius` in metres, checked positive.
pub(crate) fn circle_radius(
    session: &LoweringSession<'_>,
    segment: &Segment,
) -> GeometryResult<f64> {
    let radius = session
        .units()
        .length(session.slots(segment.parent)?.req_f64(1, "Radius")?);
    if radius.is_finite() && radius > 0.0 {
        Ok(radius)
    } else {
        Err(session.degenerate(
            segment.parent,
            "IFCCIRCLE",
            "Radius must be finite and positive",
        ))
    }
}

/// Lower `IfcCurveSegment` `id` as a world-space curve.
pub(super) fn lower_segment(
    session: &mut LoweringSession<'_>,
    id: EntityId,
    frame: Transform,
) -> GeometryResult<NodeId> {
    let segment = read_segment(session, id)?;
    let placed = frame.compose(&segment.placement);
    let Some(start) = rigid_frame(&placed) else {
        return Err(session.unsupported(id, TYPE, NOT_RIGID));
    };
    let run = segment.length.abs();
    if run == 0.0 {
        return intrinsic(session, id, start, CurvatureLaw::straight(), 0.0);
    }
    match segment.parent_kind.as_str() {
        "IFCLINE" => {
            let end = start.origin + start.x * run;
            session.node_for(
                id,
                GeometryNode::Curve3(Curve3::Polyline(Polyline3 {
                    points: vec![start.origin, end],
                    closed: false,
                })),
            )
        }
        "IFCCIRCLE" => arc(session, &segment, start, run),
        "IFCPOLYLINE" => polyline(session, &segment, start),
        "IFCPOLYNOMIALCURVE" => polynomial(session, &segment, start, run),
        _ => {
            let law = segment_curvature(session, &segment)?;
            intrinsic(session, id, start, law, run)
        }
    }
}

/// A planar intrinsic curve in `start`'s XY plane.
fn intrinsic(
    session: &mut LoweringSession<'_>,
    id: EntityId,
    start: Frame3,
    law: CurvatureLaw,
    run: f64,
) -> GeometryResult<NodeId> {
    let curve = Intrinsic3::new(start, law, CurvatureLaw::straight(), run);
    if curve.total_turning().is_none_or(|turn| !turn.is_finite()) {
        return Err(session.degenerate(id, TYPE, "the segment's turning integral is not finite"));
    }
    session.node_for(id, GeometryNode::Curve3(Curve3::Intrinsic(curve)))
}

/// A circular arc starting at `start`, tangent to its X axis.
///
/// Forwards the arc turns left (the circle's own counter-clockwise sense in
/// the placement plane), backwards it turns right. The circle's frame puts
/// parameter 0 at the start point and increases along the travel, so the
/// trim is the plain angle `run / radius`.
fn arc(
    session: &mut LoweringSession<'_>,
    segment: &Segment,
    start: Frame3,
    run: f64,
) -> GeometryResult<NodeId> {
    let radius = circle_radius(session, segment)?;
    let side = if segment.length > 0.0 { 1.0 } else { -1.0 };
    let centre = start.origin + start.y * (side * radius);
    let circle = Circle3 {
        frame: Frame3 {
            origin: centre,
            x: start.y * -side,
            y: start.x,
            z: start.z * side,
        },
        radius,
    };
    let basis = session.node_for(segment.id, GeometryNode::Curve3(Curve3::Circle(circle)))?;
    session.node_for(
        segment.id,
        GeometryNode::CurveRelation(CurveRelation::Trimmed {
            basis,
            start: vec![TrimSelector::Parameter(0.0)],
            end: vec![TrimSelector::Parameter(run / radius)],
            sense_agreement: true,
            preference: KernelPreference::Parameter,
        }),
    )
}

/// A 2D `IfcPolynomialCurve` parent: its Bezier placed in `start`'s XY
/// plane, trimmed where its arc length reaches `run`.
fn polynomial(
    session: &mut LoweringSession<'_>,
    segment: &Segment,
    start: Frame3,
    run: f64,
) -> GeometryResult<NodeId> {
    let local = polynomial::local_bezier(session, segment)?;
    let control_points = local
        .control_points
        .iter()
        .map(|p| start.origin + start.x * p.x + start.y * p.y)
        .collect();
    let basis = session.node_for(
        segment.id,
        GeometryNode::Curve3(Curve3::BSpline(BSplineCurve3 {
            degree: local.degree,
            control_points,
            knots: local.knots,
            multiplicities: local.multiplicities,
            weights: None,
            closed: false,
            self_intersect: local.self_intersect,
            knot_spec: local.knot_spec,
        })),
    )?;
    session.node_for(
        segment.id,
        GeometryNode::CurveRelation(CurveRelation::Trimmed {
            basis,
            start: vec![TrimSelector::Parameter(0.0)],
            end: vec![TrimSelector::ArcLength(run)],
            sense_agreement: true,
            preference: KernelPreference::Parameter,
        }),
    )
}

/// A 2D `IfcPolyline` parent cut at both arc lengths and placed.
fn polyline(
    session: &mut LoweringSession<'_>,
    segment: &Segment,
    start: Frame3,
) -> GeometryResult<NodeId> {
    let parent = segment.parent;
    let refs = Polyline::new(parent, session.entity(segment.id, parent)?).point_refs()?;
    let mut points = Vec::with_capacity(refs.len());
    for point_ref in refs {
        let view = CartesianPoint::new(point_ref, session.entity(parent, point_ref)?);
        if view.dimension()? != 2 {
            return Err(session.unsupported(
                parent,
                "IFCPOLYLINE",
                "a 3D IfcPolyline parent has no plane to fix the segment's start normal",
            ));
        }
        let [x, y, _] = view.coordinates_3d()?;
        points.push([session.units().length(x), session.units().length(y)]);
    }
    let Some(cut) = cut_polyline(&points, segment.start, segment.length) else {
        return Err(session.degenerate(
            parent,
            "IFCPOLYLINE",
            "the segment's arc-length range leaves the parent polyline or meets a zero-length edge",
        ));
    };
    // Rigid map: the parent point at SegmentStart to the placement origin,
    // the travel tangent there to its X axis.
    let [p0, tangent] = [cut.points[0], cut.tangent];
    let normal = [-tangent[1], tangent[0]];
    let placed = cut
        .points
        .iter()
        .map(|q| {
            let d = [q[0] - p0[0], q[1] - p0[1]];
            let along = d[0] * tangent[0] + d[1] * tangent[1];
            let across = d[0] * normal[0] + d[1] * normal[1];
            start.origin + start.x * along + start.y * across
        })
        .collect();
    session.node_for(
        segment.id,
        GeometryNode::Curve3(Curve3::Polyline(Polyline3 {
            points: placed,
            closed: false,
        })),
    )
}

/// A polyline piece in travel order, with its unit start tangent.
struct Cut {
    points: Vec<[f64; 2]>,
    tangent: [f64; 2],
}

/// The piece of `points` from arc length `start` for signed `length`.
fn cut_polyline(points: &[[f64; 2]], start: f64, length: f64) -> Option<Cut> {
    let mut stations = vec![0.0];
    for pair in points.windows(2) {
        let edge = (pair[1][0] - pair[0][0]).hypot(pair[1][1] - pair[0][1]);
        if edge.is_nan() || edge <= 0.0 {
            return None;
        }
        stations.push(stations.last()? + edge);
    }
    let total = *stations.last()?;
    let end = start + length;
    let slack = 1e-9 * total.max(1.0);
    if points.len() < 2 || start.min(end) < -slack || start.max(end) > total + slack {
        return None;
    }
    let at = |s: f64| -> [f64; 2] {
        let s = s.clamp(0.0, total);
        let i = stations
            .partition_point(|x| *x <= s)
            .clamp(1, points.len() - 1);
        let t = (s - stations[i - 1]) / (stations[i] - stations[i - 1]);
        let (a, b) = (points[i - 1], points[i]);
        [a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1])]
    };
    let (lo, hi) = (start.min(end), start.max(end));
    let mut cut = vec![at(lo)];
    cut.extend(
        stations
            .iter()
            .zip(points)
            .filter(|(s, _)| **s > lo + slack && **s < hi - slack)
            .map(|(_, p)| *p),
    );
    cut.push(at(hi));
    if length < 0.0 {
        cut.reverse();
    }
    let (a, b) = (cut[0], cut[1]);
    let span = (b[0] - a[0]).hypot(b[1] - a[1]);
    if span.is_nan() || span <= 0.0 {
        return None;
    }
    let tangent = [(b[0] - a[0]) / span, (b[1] - a[1]) / span];
    Some(Cut {
        points: cut,
        tangent,
    })
}

/// A unit, orthogonal, right-handed frame from `placed`, or `None`.
pub(crate) fn rigid_frame(placed: &Transform) -> Option<Frame3> {
    const EPSILON: f64 = 1e-9;
    let frame = frame3(placed);
    let unit = [frame.x, frame.y, frame.z]
        .iter()
        .all(|v| (v.length_squared() - 1.0).abs() <= EPSILON);
    let orthogonal = frame.x.dot(frame.y).abs() <= EPSILON
        && frame.x.dot(frame.z).abs() <= EPSILON
        && frame.y.dot(frame.z).abs() <= EPSILON;
    let right_handed = frame.x.cross(frame.y).dot(frame.z) >= 1.0 - EPSILON;
    let finite = frame.origin.is_finite() && placed.basis.iter().flatten().all(|v| v.is_finite());
    (unit && orthogonal && right_handed && finite).then_some(frame)
}

/// One member of an `IfcCompositeCurve` that is an `IfcCurveSegment`.
///
/// The zero-length segment IFC4.3 requires LAST in an alignment layout adds
/// no geometry and returns `None`; a zero-length segment anywhere else hides
/// a seam and is refused.
pub(super) fn composite_member(
    session: &mut LoweringSession<'_>,
    composite: EntityId,
    segment_ref: EntityId,
    last: bool,
    frame: Transform,
) -> GeometryResult<Option<CurveSegment>> {
    let segment = read_segment(session, segment_ref)?;
    if segment.length == 0.0 {
        if last {
            return Ok(None);
        }
        return Err(session.degenerate(
            segment_ref,
            TYPE,
            format!(
                "a zero-length IfcCurveSegment is allowed only as the last segment of {composite}"
            ),
        ));
    }
    let code = CompositeCurveSegment::new(segment_ref, session.entity(composite, segment_ref)?)
        .transition()?;
    Ok(Some(CurveSegment {
        curve: lower_curve_node(session, segment_ref, frame)?,
        same_sense: true,
        transition: transition(code),
    }))
}

#[cfg(test)]
mod tests;
