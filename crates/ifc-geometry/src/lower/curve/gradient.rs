//! `IfcGradientCurve` and `IfcSegmentedReferenceCurve` (IFC4X3).
//!
//! # Gradient curve: plan plus height, both exact
//!
//! "Gradient curve is a type of 3D curve representation that is based on its
//! 2D projection (BaseCurve) and a height defined by its gradient segments";
//! "the value of the parameter equals the parameter value of BaseCurve". So
//! it lowers to `Curve3::Elevated`: the plan as ONE `Curve2::Intrinsic` with
//! a piecewise curvature law, and the height as an `ElevationLaw` over
//! distance along that plan. Nothing is integrated or sampled.
//!
//! **Plan.** `BaseCurve` must be an `IfcCompositeCurve` of `IfcCurveSegment`s
//! with 2D placements whose parents have an elementary law (line, circle,
//! spiral; see `segment.rs`). The curve is anchored at the first segment's
//! placement and every later piece by arc length. Each later placement is
//! still checked: its heading in closed form at every seam (a kink is
//! refused), its position after a line or arc, whose end point is closed
//! form. After a spiral the end point is a Fresnel-type integral, so the
//! authored placement is accepted as stated, not verified -- the same rule
//! `ifc-alignment` applies to the business layout.
//!
//! **Profile.** The `Segments` are `IfcCurveSegment`s in the
//! (distance along, height) plane: `Placement.Location` is
//! `(StartDistAlong, StartHeight)` and lengths are arc lengths along the
//! parent, as for every curve segment. An `IfcLine` parent rising at
//! `RefDirection = (dx, dz)` covers `|SegmentLength| dx` of plan distance at
//! grade `dz/dx`, exactly. An `IfcPolynomialCurve` parabola is a height
//! function of distance only when the placement keeps its start tangent; its
//! end abscissa inverts a non-elementary arc-length integral, so it is read
//! from the next segment's placement (or the closing segment, or `EndPoint`)
//! and the stated `SegmentLength` is checked against the closed-form
//! parabola arc length. Vertical circular arcs and clothoids are not
//! polynomial in plan distance and stay refused (#258).
//!
//! # Segmented reference curve: refused (#93)
//!
//! It adds cant -- a roll of the cross-section about the centreline -- which
//! the pinned neutral vocabulary has no value for.

use std::f64::consts::TAU;

use axiolid_core::{Frame2, Point2, Vec2};
use axiolid_curve::{CurvatureLaw, Curve2, Curve3, Elevated3, ElevationLaw, Intrinsic2};
use axiolid_model::{GeometryNode, NodeId};
use ifc_alignment::{AlignmentUnits, SeamTolerance};
use ifc_model::EntityId;

use super::segment::{read_segment, refuse_parent, segment_curvature, Segment};
use crate::curve::composite::CompositeCurve;
use crate::error::GeometryResult;
use crate::lower::session::LoweringSession;
use crate::resource::placement::axis_placement_transform;
use crate::transform::Transform;

const GRADIENT: &str = "IFCGRADIENTCURVE";
const SEGMENT: &str = "IFCCURVESEGMENT";

/// Why an `IfcSegmentedReferenceCurve` is refused.
pub(crate) const SEGMENTED_REFERENCE: &str =
    "IfcSegmentedReferenceCurve adds cant (a roll of the section about the centreline); the \
     pinned neutral curve vocabulary has no roll law to carry it exactly (#93)";

/// Why a vertical circular arc or clothoid is refused.
const VERTICAL_NOT_POLYNOMIAL: &str =
    "a vertical IfcCircle or IfcClothoid is not polynomial in plan distance, and the pinned \
     ElevationLaw has only polynomial pieces (#258)";

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
    let placed = Intrinsic2::new(
        Frame2 {
            origin: Point2::new(origin[0], origin[1]),
            x: Vec2::new(x[0], x[1]),
            y: Vec2::new(-x[1], x[0]),
        },
        plan.curvature,
        plan.length,
    );
    let elevation = lifted(elevation, origin[2]);
    session.node_for(
        id,
        GeometryNode::Curve3(Curve3::Elevated(Elevated3::new(
            Curve2::Intrinsic(placed),
            elevation,
        ))),
    )
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

/// The plan: one intrinsic curve over every horizontal segment.
fn plan(
    session: &LoweringSession<'_>,
    owner: EntityId,
    base: EntityId,
    tolerance: Tolerance,
) -> GeometryResult<Intrinsic2> {
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

    let mut laws = Vec::with_capacity(body.len());
    let mut breaks = Vec::with_capacity(body.len());
    let mut station = 0.0;
    for (index, segment) in body.iter().enumerate() {
        let law = segment_curvature(session, segment)?;
        let run = segment.length.abs();
        if let Some(next) = body.get(index + 1).or(closing) {
            check_plan_seam(session, segment, &law, run, next, tolerance)?;
        }
        if index > 0 {
            breaks.push(station);
        }
        laws.push(law);
        station += run;
    }
    let curvature = if laws.len() == 1 {
        laws.remove(0)
    } else {
        CurvatureLaw::piecewise(breaks, laws)
    };
    let x = first.placement.basis[0];
    let curve = Intrinsic2::new(
        Frame2 {
            origin: Point2::new(first.placement.origin[0], first.placement.origin[1]),
            x: Vec2::new(x[0], x[1]),
            y: Vec2::new(-x[1], x[0]),
        },
        curvature,
        station,
    );
    if !curve.curvature.is_well_formed()
        || curve.total_turning().is_none_or(|turn| !turn.is_finite())
    {
        return Err(session.degenerate(
            base,
            "IFCCOMPOSITECURVE",
            "the horizontal segments did not assemble into a well-formed curvature law",
        ));
    }
    Ok(curve)
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
        let end = [d0 + extent, horner(&law, extent)];
        if let Some(next) = next {
            if !(tolerance.same(end[0], next[0]) && tolerance.same(end[1], next[1])) {
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
        laws.push(ElevationLaw::Polynomial { coefficients: law });
        reach = end[0];
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

/// One vertical segment: its height polynomial in local distance, and the
/// plan distance it covers.
///
/// `to_next` is the plan distance to the next stated start, when there is
/// one; only a parabola needs it.
fn vertical_piece(
    session: &LoweringSession<'_>,
    segment: &Segment,
    to_next: Option<f64>,
    tolerance: Tolerance,
) -> GeometryResult<(Vec<f64>, f64)> {
    let [dx, dz] = [segment.placement.basis[0][0], segment.placement.basis[0][1]];
    if dx.is_nan() || dx <= 0.0 {
        return Err(session.unsupported(
            segment.id,
            SEGMENT,
            "a vertical segment's start tangent must advance along the base curve",
        ));
    }
    let z0 = segment.placement.origin[1];
    match segment.parent_kind.as_str() {
        "IFCLINE" => Ok((vec![z0, dz / dx], segment.length.abs() * dx)),
        "IFCPOLYNOMIALCURVE" => parabola(session, segment, to_next, tolerance),
        "IFCCIRCLE" | "IFCCLOTHOID" => Err(session.unsupported(
            segment.parent,
            &segment.parent_kind,
            VERTICAL_NOT_POLYNOMIAL,
        )),
        _ => Err(refuse_parent(session, segment)),
    }
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
        return Err(refuse(super::segment::POLYNOMIAL_PARENT));
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
             segment, closing segment or EndPoint states it (#90)",
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

/// Evaluate ascending coefficients at `t`.
fn horner(coefficients: &[f64], t: f64) -> f64 {
    coefficients.iter().rev().fold(0.0, |acc, c| acc * t + c)
}

/// The law raised by `dz` everywhere.
fn lifted(law: ElevationLaw, dz: f64) -> ElevationLaw {
    match law {
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
            laws: laws.into_iter().map(|law| lifted(law, dz)).collect(),
        },
        other => other,
    }
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
