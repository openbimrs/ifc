//! `IfcGradientCurve` and `IfcSegmentedReferenceCurve` (IFC4X3).
//!
//! # Gradient curve: plan plus height, both exact
//!
//! "Gradient curve is a type of 3D curve representation that is based on its
//! 2D projection (BaseCurve) and a height defined by its gradient segments";
//! "the value of the parameter equals the parameter value of BaseCurve". So
//! it lowers to `Curve3::Elevated`: the plan as ONE curve parameterised by
//! arc length, and the height as an `ElevationLaw` over distance along that
//! plan. Nothing is integrated or sampled.
//!
//! **Plan.** `BaseCurve` must be an `IfcCompositeCurve` of `IfcCurveSegment`s
//! with 2D placements whose parents have an elementary law (line, circle,
//! spiral; see `segment.rs`) or are a 2D `IfcPolynomialCurve` (the `CUBIC`
//! transition; see `polynomial.rs`). Without a polynomial the plan is one
//! `Curve2::Intrinsic` with a piecewise curvature law; with one it is a
//! `Curve2::Chain`, the polynomial a parametric piece read by arc length.
//! Either is anchored at the first segment's placement and every later
//! piece by arc length. Each later placement is still checked: its heading
//! in closed form at every seam after a curvature law (a kink is refused),
//! its position after a line or arc, whose end point is closed form. After
//! a spiral the end point is a Fresnel-type integral, after a polynomial
//! the end point and heading are an elliptic-integral inverse, so the
//! authored placement is accepted as stated, not verified -- the same rule
//! `ifc-alignment` applies to the business layout.
//!
//! **Profile.** The `Segments` are `IfcCurveSegment`s in the
//! (distance along, height) plane: `Placement.Location` is
//! `(StartDistAlong, StartHeight)` and lengths are arc lengths along the
//! parent, as for every curve segment.
//!
//! - An `IfcLine` parent rising at `RefDirection = (dx, dz)` covers
//!   `|SegmentLength| dx` of plan distance at grade `dz/dx`, exactly.
//! - An `IfcPolynomialCurve` parabola is a height function of distance only
//!   when the placement keeps its start tangent; its end abscissa inverts a
//!   non-elementary arc-length integral, so it is read from the next
//!   segment's placement (or the closing segment, or `EndPoint`) and the
//!   stated `SegmentLength` is checked against the closed-form parabola arc
//!   length.
//! - An `IfcCircle` parent is `ElevationLaw::CircularArc` (#258): the
//!   circle through the placement, turning left (a sag) for a positive
//!   `SegmentLength`. Its plan extent `R (sin t1 - sin t0)`, with
//!   `t1 = t0 + |SegmentLength| / R`, and its end height are closed form,
//!   so both seams are checked.
//! - An `IfcSpiral` parent (the vertical `IfcClothoid`) is
//!   `ElevationLaw::Intrinsic`: its curvature against its own arc length,
//!   rebased to the segment. Its plan extent and end height are
//!   Fresnel-type integrals, so the extent is read from the next start, as
//!   for the parabola, and the seam height there is accepted as authored.
//!
//! # Segmented reference curve: refused
//!
//! Axiolid now has a banked curve to carry cant, and `ifc-alignment` lowers
//! the business cant layout onto it. The geometric form is still refused:
//! its segments stand at stations along the base curve
//! (`IfcAxis2PlacementLinear`, #307) or carry cant in placement axes, and
//! IFC4.3 ADD2 gives no normative mapping from a segment's `ParentCurve` to
//! the cant law ("the superelevation rate of change is directly
//! proportionate to the curve segment parent curve curvature gradient").
//! Reading one would be a guess.

use std::f64::consts::TAU;

use axiolid_core::{Frame2, Point2, Vec2};
use axiolid_curve::{
    Chain2, ChainPiece2, CurvatureLaw, Curve2, Curve3, Elevated3, ElevationLaw, Intrinsic2,
};
use axiolid_model::{GeometryNode, NodeId};
use ifc_alignment::{AlignmentUnits, SeamTolerance};
use ifc_model::EntityId;

use super::segment::{circle_radius, read_segment, refuse_parent, segment_curvature, Segment};
use super::{polynomial, spiral};
use crate::curve::composite::CompositeCurve;
use crate::error::GeometryResult;
use crate::lower::session::LoweringSession;
use crate::resource::placement::axis_placement_transform;
use crate::transform::Transform;

const GRADIENT: &str = "IFCGRADIENTCURVE";
const SEGMENT: &str = "IFCCURVESEGMENT";

/// Why an `IfcSegmentedReferenceCurve` is refused.
pub(crate) const SEGMENTED_REFERENCE: &str =
    "IfcSegmentedReferenceCurve states cant through segments placed at stations along its base \
     curve and parent curves with no normative mapping to a cant law; the business cant layout \
     lowers to a banked curve through ifc-alignment (#311)";

/// Largest heading difference, in radians, accepted at a plan seam.
const HEADING_TOLERANCE: f64 = 1e-6;

/// Largest start-tangent difference, in radians, for a vertical parabola.
const TANGENT_TOLERANCE: f64 = 1e-9;

/// Refuse an `IfcSegmentedReferenceCurve` by name.
pub(super) fn segmented_reference_curve(
    session: &LoweringSession<'_>,
    id: EntityId,
) -> GeometryResult<NodeId> {
    Err(session.unsupported(id, "IFCSEGMENTEDREFERENCECURVE", SEGMENTED_REFERENCE))
}

/// Lower an `IfcGradientCurve` to an exact `Curve3::Elevated`.
pub(super) fn gradient_curve(
    session: &mut LoweringSession<'_>,
    id: EntityId,
    frame: Transform,
) -> GeometryResult<NodeId> {
    if !keeps_vertical(&frame) {
        return Err(session.unsupported(
            id,
            GRADIENT,
            "a plan plus a height carries only frames that keep the vertical axis: a tilted, \
             scaled or mirrored frame cannot be applied exactly",
        ));
    }
    let tolerance = Tolerance::for_session(session, id)?;
    let slots = session.slots(id)?;
    let base = slots.req_ref(2, "BaseCurve")?;
    let end_point = slots.opt_ref(3);
    let vertical = CompositeCurve::new(id, session.entity(id, id)?).segment_refs()?;

    let plan = plan(session, id, base, tolerance)?;
    let end = match end_point {
        Some(placement) => {
            let entity = session.entity(id, placement)?;
            let origin = axis_placement_transform(session.model(), placement, entity)?
                .to_metres(session.units())
                .origin;
            Some([origin[0], origin[1]])
        }
        None => None,
    };
    let elevation = profile(session, id, &vertical, end, plan.length, tolerance)?;

    // The frame keeps the vertical: rotate and move the plan, lift the height.
    let origin = frame.apply([plan.start.origin.x, plan.start.origin.y, 0.0]);
    let x = frame.apply_direction([plan.start.x.x, plan.start.x.y, 0.0]);
    let start = Frame2 {
        origin: Point2::new(origin[0], origin[1]),
        x: Vec2::new(x[0], x[1]),
        y: Vec2::new(-x[1], x[0]),
    };
    let placed = match plan.pieces {
        PlanPieces::Law(curvature) => {
            Curve2::Intrinsic(Intrinsic2::new(start, curvature, plan.length))
        }
        PlanPieces::Chain(pieces) => Curve2::Chain(Chain2::new(start, pieces)),
    };
    let Some(elevation) = lifted(elevation, origin[2]) else {
        return Err(session.degenerate(id, GRADIENT, "the profile law cannot be lifted"));
    };
    session.node_for(
        id,
        GeometryNode::Curve3(Curve3::Elevated(Elevated3::new(placed, elevation))),
    )
}

/// The plan: where it starts, its pieces, and its arc length.
struct Plan {
    start: Frame2,
    pieces: PlanPieces,
    length: f64,
}

/// One curvature law over the whole plan, or a chain when a piece has none.
enum PlanPieces {
    Law(CurvatureLaw),
    Chain(Vec<ChainPiece2>),
}

/// One horizontal segment of the plan.
enum PlanPiece {
    /// A curvature law in the segment's own arc length.
    Law(CurvatureLaw),
    /// An `IfcPolynomialCurve` in its local frame, read by arc length.
    Polynomial(Curve2),
}

/// Seam tolerance: the model's declared precision, floored at rounding.
#[derive(Debug, Clone, Copy)]
struct Tolerance(f64);

impl Tolerance {
    fn for_session(session: &LoweringSession<'_>, id: EntityId) -> GeometryResult<Self> {
        let units = AlignmentUnits {
            length_to_metres: session.units().length_to_metres,
            angle_to_radians: session.units().angle_to_radians,
        };
        SeamTolerance::for_model(session.model(), units)
            .map(|t| Self(t.length()))
            .map_err(|_| {
                session.degenerate(
                    id,
                    GRADIENT,
                    "the model's declared Precision is not a usable seam tolerance",
                )
            })
    }

    fn same(self, a: f64, b: f64) -> bool {
        (a - b).abs() <= self.0.max(1e-9 * a.abs().max(b.abs()).max(1.0))
    }
}

/// The frame is rigid and keeps world Z as its Z.
fn keeps_vertical(frame: &Transform) -> bool {
    const EPSILON: f64 = 1e-12;
    let [x, y, z] = frame.basis;
    let near = |v: [f64; 3], w: [f64; 3]| v.iter().zip(w).all(|(a, b)| (a - b).abs() <= EPSILON);
    let unit_xy = ((x[0] * x[0] + x[1] * x[1]) - 1.0).abs() <= EPSILON;
    unit_xy
        && near(z, [0.0, 0.0, 1.0])
        && x[2].abs() <= EPSILON
        && near(y, [-x[1], x[0], 0.0])
        && frame.origin.iter().all(|v| v.is_finite())
}

/// The plan: one curve over every horizontal segment, parameterised by arc
/// length.
fn plan(
    session: &LoweringSession<'_>,
    owner: EntityId,
    base: EntityId,
    tolerance: Tolerance,
) -> GeometryResult<Plan> {
    if session.type_name(base)? != "IFCCOMPOSITECURVE" {
        return Err(session.unsupported(
            owner,
            GRADIENT,
            "BaseCurve must be an IfcCompositeCurve of IfcCurveSegments",
        ));
    }
    let segments = curve_segments(session, owner, base)?;
    let (body, closing) = split_closing(session, &segments)?;
    let Some(first) = body.first() else {
        return Err(session.degenerate(base, "IFCCOMPOSITECURVE", "no segment of positive length"));
    };

    let mut pieces = Vec::with_capacity(body.len());
    let mut station = 0.0;
    for (index, segment) in body.iter().enumerate() {
        let run = segment.length.abs();
        let piece = if segment.parent_kind == "IFCPOLYNOMIALCURVE" {
            PlanPiece::Polynomial(Curve2::BSpline(polynomial::local_bezier(session, segment)?))
        } else {
            PlanPiece::Law(segment_curvature(session, segment)?)
        };
        // After a polynomial neither the end point nor the end heading is
        // closed form: the next placement is accepted as authored.
        if let (PlanPiece::Law(law), Some(next)) = (&piece, body.get(index + 1).or(closing)) {
            check_plan_seam(session, segment, law, run, next, tolerance)?;
        }
        pieces.push((run, piece));
        station += run;
    }
    let malformed = || {
        session.degenerate(
            base,
            "IFCCOMPOSITECURVE",
            "the horizontal segments did not assemble into a well-formed plan curve",
        )
    };
    let x = first.placement.basis[0];
    let start = Frame2 {
        origin: Point2::new(first.placement.origin[0], first.placement.origin[1]),
        x: Vec2::new(x[0], x[1]),
        y: Vec2::new(-x[1], x[0]),
    };
    let pieces = if pieces
        .iter()
        .all(|(_, piece)| matches!(piece, PlanPiece::Law(_)))
    {
        let mut laws = Vec::with_capacity(pieces.len());
        let mut breaks = Vec::with_capacity(pieces.len());
        let mut at = 0.0;
        for (index, (run, piece)) in pieces.into_iter().enumerate() {
            if let PlanPiece::Law(law) = piece {
                if index > 0 {
                    breaks.push(at);
                }
                laws.push(law);
            }
            at += run;
        }
        let curvature = if laws.len() == 1 {
            laws.remove(0)
        } else {
            CurvatureLaw::piecewise(breaks, laws)
        };
        let curve = Intrinsic2::new(start, curvature, station);
        if !curve.curvature.is_well_formed()
            || curve.total_turning().is_none_or(|turn| !turn.is_finite())
        {
            return Err(malformed());
        }
        PlanPieces::Law(curve.curvature)
    } else {
        let pieces: Vec<ChainPiece2> = pieces
            .into_iter()
            .map(|(length, piece)| match piece {
                PlanPiece::Law(curvature) => ChainPiece2::Intrinsic { curvature, length },
                PlanPiece::Polynomial(curve) => ChainPiece2::Parametric {
                    curve,
                    start: 0.0,
                    length,
                },
            })
            .collect();
        if !Chain2::new(start, pieces.clone()).is_well_formed() {
            return Err(malformed());
        }
        PlanPieces::Chain(pieces)
    };
    Ok(Plan {
        start,
        pieces,
        length: station,
    })
}

/// Check where `next` starts against where `segment` ends.
fn check_plan_seam(
    session: &LoweringSession<'_>,
    segment: &Segment,
    law: &CurvatureLaw,
    run: f64,
    next: &Segment,
    tolerance: Tolerance,
) -> GeometryResult<()> {
    let heading = angle(segment.placement.basis[0]);
    let turning = Intrinsic2::new(unit_frame(), law.clone(), run)
        .total_turning()
        .unwrap_or(f64::NAN);
    let difference = (angle(next.placement.basis[0]) - heading - turning).rem_euclid(TAU);
    let off = difference.min(TAU - difference);
    if off.is_nan() || off > HEADING_TOLERANCE {
        return Err(session.unsupported(
            next.id,
            SEGMENT,
            "consecutive horizontal segments do not share a tangent direction; one exact plan \
             curve cannot carry a kink",
        ));
    }
    // A line or arc ends at a closed-form point; a spiral does not.
    if let Some(k) = law.constant_value() {
        let (along, across) = if k == 0.0 {
            (run, 0.0)
        } else {
            ((k * run).sin() / k, (1.0 - (k * run).cos()) / k)
        };
        let [x, y] = [segment.placement.basis[0], segment.placement.basis[1]];
        let p = segment.placement.origin;
        let end = [
            p[0] + along * x[0] + across * y[0],
            p[1] + along * x[1] + across * y[1],
        ];
        let q = next.placement.origin;
        if !(tolerance.same(end[0], q[0]) && tolerance.same(end[1], q[1])) {
            return Err(session.unsupported(
                next.id,
                SEGMENT,
                "a horizontal segment does not start where the previous line or arc ends; one \
                 exact plan curve cannot carry a gap",
            ));
        }
    }
    Ok(())
}

/// Every member of `composite`, read as an `IfcCurveSegment` with a 2D
/// placement.
fn curve_segments(
    session: &LoweringSession<'_>,
    owner: EntityId,
    composite: EntityId,
) -> GeometryResult<Vec<Segment>> {
    let refs = CompositeCurve::new(composite, session.entity(owner, composite)?).segment_refs()?;
    refs.into_iter()
        .map(|id| {
            if session.type_name(id)? != SEGMENT {
                return Err(session.unsupported(
                    owner,
                    GRADIENT,
                    "gradient curve segments must be IfcCurveSegments",
                ));
            }
            let segment = read_segment(session, id)?;
            if !segment.planar_placement {
                return Err(session.unsupported(
                    id,
                    SEGMENT,
                    "a gradient curve's segments are placed by IfcAxis2Placement2D",
                ));
            }
            Ok(segment)
        })
        .collect()
}

/// Split off the closing zero-length segment; refuse a misplaced one.
fn split_closing<'s>(
    session: &LoweringSession<'_>,
    segments: &'s [Segment],
) -> GeometryResult<(&'s [Segment], Option<&'s Segment>)> {
    let (body, closing) = match segments.split_last() {
        Some((last, body)) if last.length == 0.0 => (body, Some(last)),
        _ => (segments, None),
    };
    if let Some(misplaced) = body.iter().find(|s| s.length == 0.0) {
        return Err(session.degenerate(
            misplaced.id,
            SEGMENT,
            "a zero-length IfcCurveSegment is allowed only as the last segment of its layout",
        ));
    }
    Ok((body, closing))
}

/// The vertical profile as one law over distance along the plan.
fn profile(
    session: &LoweringSession<'_>,
    owner: EntityId,
    refs: &[EntityId],
    end_point: Option<[f64; 2]>,
    plan_length: f64,
    tolerance: Tolerance,
) -> GeometryResult<ElevationLaw> {
    let segments = refs
        .iter()
        .map(|id| {
            let segment = read_segment(session, *id)?;
            if segment.planar_placement {
                Ok(segment)
            } else {
                Err(session.unsupported(
                    *id,
                    SEGMENT,
                    "a vertical segment is placed by IfcAxis2Placement2D in (distance, height)",
                ))
            }
        })
        .collect::<GeometryResult<Vec<_>>>()?;
    let (body, closing) = split_closing(session, &segments)?;
    let Some(first) = body.first() else {
        return Err(session.degenerate(owner, GRADIENT, "no vertical segment of positive length"));
    };
    if !tolerance.same(first.placement.origin[0], 0.0) {
        return Err(session.unsupported(
            first.id,
            SEGMENT,
            "the vertical profile must start where the base curve starts",
        ));
    }

    let mut laws = Vec::with_capacity(body.len());
    let mut breaks = Vec::with_capacity(body.len());
    let mut reach = 0.0;
    for (index, segment) in body.iter().enumerate() {
        let next = body
            .get(index + 1)
            .or(closing)
            .map(|s| [s.placement.origin[0], s.placement.origin[1]])
            .or(end_point);
        let d0 = segment.placement.origin[0];
        let (law, extent) = vertical_piece(session, segment, next.map(|p| p[0] - d0), tolerance)?;
        // A spiral's end height is a Fresnel-type integral: accepted as the
        // next start states it. Every other piece ends in closed form.
        let end_height = match (&law, law.height_at(extent)) {
            (ElevationLaw::Intrinsic { .. }, _) => None,
            (_, Some(height)) => Some(height),
            (_, None) => {
                return Err(session.degenerate(
                    segment.id,
                    SEGMENT,
                    "the vertical segment has no finite end height",
                ))
            }
        };
        let end = d0 + extent;
        if let Some(next) = next {
            let height_ok = end_height.is_none_or(|height| tolerance.same(height, next[1]));
            if !(tolerance.same(end, next[0]) && height_ok) {
                return Err(session.unsupported(
                    segment.id,
                    SEGMENT,
                    "a vertical segment does not end where the next one starts; one elevation \
                     law cannot carry a gap",
                ));
            }
        }
        if index > 0 {
            breaks.push(d0);
        }
        laws.push(law);
        reach = end;
    }
    if !tolerance.same(reach, plan_length) {
        return Err(session.unsupported(
            owner,
            GRADIENT,
            "the vertical profile must end where the base curve ends",
        ));
    }
    let law = if laws.len() == 1 {
        laws.remove(0)
    } else {
        ElevationLaw::Piecewise { breaks, laws }
    };
    if law.is_well_formed() {
        Ok(law)
    } else {
        Err(session.degenerate(
            owner,
            GRADIENT,
            "the vertical profile is not a well-formed law",
        ))
    }
}

/// One vertical segment: its elevation law in local distance, and the plan
/// distance it covers.
///
/// `to_next` is the plan distance to the next stated start, when there is
/// one; a parabola and a spiral need it.
fn vertical_piece(
    session: &LoweringSession<'_>,
    segment: &Segment,
    to_next: Option<f64>,
    tolerance: Tolerance,
) -> GeometryResult<(ElevationLaw, f64)> {
    let [dx, dz] = [segment.placement.basis[0][0], segment.placement.basis[0][1]];
    if dx.is_nan() || dx <= 0.0 {
        return Err(session.unsupported(
            segment.id,
            SEGMENT,
            "a vertical segment's start tangent must advance along the base curve",
        ));
    }
    let z0 = segment.placement.origin[1];
    let kind = segment.parent_kind.as_str();
    match kind {
        "IFCLINE" => Ok((
            ElevationLaw::constant_grade(z0, dz / dx),
            segment.length.abs() * dx,
        )),
        "IFCPOLYNOMIALCURVE" => parabola(session, segment, to_next, tolerance)
            .map(|(coefficients, extent)| (ElevationLaw::Polynomial { coefficients }, extent)),
        "IFCCIRCLE" => vertical_arc(session, segment, z0, dx, dz),
        _ if spiral::is_spiral(kind) => {
            let curvature = segment_curvature(session, segment)?;
            let Some(extent) = to_next.filter(|e| e.is_finite() && *e > 0.0) else {
                return Err(session.unsupported(
                    segment.parent,
                    kind,
                    "a vertical spiral's plan extent is a Fresnel-type integral and no next \
                     segment, closing segment or EndPoint states it",
                ));
            };
            Ok((ElevationLaw::intrinsic(z0, dz / dx, curvature), extent))
        }
        _ => Err(refuse_parent(session, segment)),
    }
}

/// A vertical `IfcCircle`: the circle through the placement, as
/// `ElevationLaw::CircularArc`, and the plan distance it covers.
///
/// A positive `SegmentLength` travels the circle in its own sense, turning
/// left in the (distance, height) plane: a sag, positive radius. The arc
/// turns by `|SegmentLength| / R` from `t0 = atan(dz / dx)` and covers
/// `R (sin t1 - sin t0)` of plan distance, both closed form. An arc that
/// turns vertical or back on itself before its end has no height there and
/// is refused.
fn vertical_arc(
    session: &LoweringSession<'_>,
    segment: &Segment,
    z0: f64,
    dx: f64,
    dz: f64,
) -> GeometryResult<(ElevationLaw, f64)> {
    let radius = circle_radius(session, segment)?.copysign(segment.length);
    let start = dz.atan2(dx);
    let end = start + segment.length.abs() / radius;
    if end.abs() >= std::f64::consts::FRAC_PI_2 {
        return Err(session.unsupported(
            segment.id,
            SEGMENT,
            "the vertical arc turns vertical before its end, so it has no height there",
        ));
    }
    let extent = radius * (end.sin() - start.sin());
    Ok((ElevationLaw::circular_arc(z0, dz / dx, radius), extent))
}

/// A vertical `IfcPolynomialCurve` of degree at most 2, as `z(t)`.
fn parabola(
    session: &LoweringSession<'_>,
    segment: &Segment,
    to_next: Option<f64>,
    tolerance: Tolerance,
) -> GeometryResult<(Vec<f64>, f64)> {
    let parent = segment.parent;
    let refuse = |detail: &'static str| session.unsupported(parent, "IFCPOLYNOMIALCURVE", detail);
    if !tolerance.same(segment.start, 0.0) || segment.length < 0.0 {
        return Err(refuse(
            "a vertical IfcPolynomialCurve cut from a non-zero SegmentStart, or walked \
             backwards: its placed start inverts a non-elementary arc-length integral",
        ));
    }
    let slots = session.slots(parent)?;
    let x = slots
        .opt(1)
        .map(|_| slots.req_f64_list(1, "CoefficientsX"))
        .transpose()?;
    let y = slots
        .opt(2)
        .map(|_| slots.req_f64_list(2, "CoefficientsY"))
        .transpose()?;
    let (Some(x), Some(y), None) = (x, y, slots.opt(3)) else {
        return Err(refuse(
            "a vertical IfcPolynomialCurve needs CoefficientsX and CoefficientsY and no \
             CoefficientsZ",
        ));
    };
    if x.len() != 2
        || x[1].is_nan()
        || x[1] <= 0.0
        || y.len() > 3
        || !x.iter().chain(&y).all(|v| v.is_finite())
    {
        return Err(refuse(
            "a vertical IfcPolynomialCurve must be x = x0 + x1 u (x1 > 0) and y of degree at \
             most 2: a higher degree has no closed-form arc length to check SegmentLength",
        ));
    }
    // z(t) = z0 + Y(u(t)) - Y(0), u = t / (f x1): heights scale by f.
    let f = session.units().length_to_metres;
    let b = |i: usize| y.get(i).copied().unwrap_or(0.0);
    let grade = b(1) / x[1];
    let bend = b(2) / (f * x[1] * x[1]);
    let tangent = grade.atan2(1.0);
    let placed = segment.placement.basis[0][1].atan2(segment.placement.basis[0][0]);
    if (tangent - placed).abs() > TANGENT_TOLERANCE {
        return Err(refuse(
            "the placement turns the parabola away from its own start tangent, so its height \
             is no longer a polynomial in distance along",
        ));
    }
    let Some(extent) = to_next.filter(|e| e.is_finite() && *e > 0.0) else {
        return Err(refuse(
            "a parabola's end abscissa inverts a non-elementary arc-length integral and no next \
             segment, closing segment or EndPoint states it",
        ));
    };
    if !tolerance.same(parabola_arc_length(grade, bend, extent), segment.length) {
        return Err(session.unsupported(
            segment.id,
            SEGMENT,
            "SegmentLength is not the parabola's arc length to the next segment's start",
        ));
    }
    Ok((vec![segment.placement.origin[1], grade, bend], extent))
}

/// Arc length of `z = g t + c t^2` over `[0, x]`, in closed form.
fn parabola_arc_length(g: f64, c: f64, x: f64) -> f64 {
    if c == 0.0 {
        return x * (1.0 + g * g).sqrt();
    }
    // With w = z' = g + 2 c t: integral sqrt(1 + w^2) dw / (2 c).
    let primitive = |w: f64| (w * (1.0 + w * w).sqrt() + w.asinh()) / 2.0;
    (primitive(g + 2.0 * c * x) - primitive(g)) / (2.0 * c)
}

/// The law raised by `dz` everywhere.
///
/// Every law this module builds is listed: a height left unlifted would put
/// the road `dz` off, so an unknown law is a degenerate result, not a
/// pass-through.
fn lifted(law: ElevationLaw, dz: f64) -> Option<ElevationLaw> {
    Some(match law {
        ElevationLaw::Polynomial { mut coefficients } => {
            if let Some(c) = coefficients.first_mut() {
                *c += dz;
            } else {
                coefficients.push(dz);
            }
            ElevationLaw::Polynomial { coefficients }
        }
        ElevationLaw::Piecewise { breaks, laws } => ElevationLaw::Piecewise {
            breaks,
            laws: laws
                .into_iter()
                .map(|law| lifted(law, dz))
                .collect::<Option<_>>()?,
        },
        ElevationLaw::CircularArc {
            height,
            grade,
            radius,
        } => ElevationLaw::circular_arc(height + dz, grade, radius),
        ElevationLaw::Intrinsic {
            height,
            grade,
            curvature,
        } => ElevationLaw::intrinsic(height + dz, grade, curvature),
        _ => return None,
    })
}

fn angle(x: [f64; 3]) -> f64 {
    x[1].atan2(x[0])
}

fn unit_frame() -> Frame2 {
    Frame2 {
        origin: Point2::new(0.0, 0.0),
        x: Vec2::new(1.0, 0.0),
        y: Vec2::new(0.0, 1.0),
    }
}

#[cfg(test)]
mod tests;
