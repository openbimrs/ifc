//! Where a basis curve's tangent may jump, and its stated length, read from
//! its stored data.
//!
//! Nothing here evaluates a curve: a polyline's vertex distances are sums of
//! its edge lengths, an elevation law's seams are its stored breaks and its
//! grades at a break are the closed forms `ElevationLaw::grade_at` states.
//! Where only an evaluator could tell (a B-spline's arc length to a multiple
//! knot, the end grade of a vertical clothoid), the seam is not shown
//! continuous and counts as one, or the curve's seams are unknown and a
//! station on it is refused by name.
//!
//! The tangent is continuous by construction on a line, a conic, an
//! intrinsic curve and an arc-length chain (`axiolid_curve::chain`: "a chain
//! is tangent-continuous at every join by construction").

use axiolid_core::Scalar;
use axiolid_curve::{Curve2, Curve3, ElevationLaw};
use ifc_alignment::{AlignmentUnits, SeamTolerance};
use ifc_model::EntityId;

use crate::error::GeometryResult;
use crate::lower::session::{AtomicCurve, LoweringSession};

/// Why a curve relation's seams are unknown.
pub(crate) const RELATION: &str =
    "the basis curve lowers to a curve relation (a composite, trimmed or offset curve), whose \
     tangent discontinuities cannot be located from stored data; IFC4.3 ADD2 (8.9.3.48.3) \
     reads a station on one with the previous segment's tangent and the neutral station reads \
     the next one's";

/// Why a B-spline with a possible kink is refused.
const SPLINE: &str =
    "the basis B-spline has an interior knot of multiplicity at least its degree, where its \
     tangent may jump; the knot's distance along it is an arc-length integral, so a station \
     cannot be shown clear of it";

/// Why a curve family with no stated seams is refused.
const UNKNOWN: &str =
    "the basis curve family has no stated tangent continuity, so a station on it cannot be \
     shown clear of a tangent discontinuity";

/// Why a banked basis is refused. It cannot be produced by this crate
/// (`IfcSegmentedReferenceCurve` is #311); the guard keeps the frame map
/// honest if it ever is: a banked section rolls, IFC's vertical offset does
/// not.
pub(crate) const BANKED: &str =
    "the basis is a banked curve, whose neutral section frame rolls with the cant, while \
     IFC4.3 ADD2 (8.9.3.48.3) reads OffsetVertical in the unrolled vertical plane of the \
     tangent";

/// Two directions closer than this sine are one direction.
const PARALLEL: Scalar = 1e-12;

/// Two grades closer than this are one grade (they are ratios, as in
/// `ifc_alignment::SeamTolerance`).
const SAME_GRADE: Scalar = 1e-9;

/// The model's declared `Precision`, in metres, capped at 1 mm.
pub(crate) fn precision(
    session: &LoweringSession<'_>,
    owner: EntityId,
    owner_type: &str,
) -> GeometryResult<f64> {
    let units = AlignmentUnits {
        length_to_metres: session.units().length_to_metres,
        angle_to_radians: session.units().angle_to_radians,
    };
    SeamTolerance::for_model(session.model(), units)
        .map(|tolerance| tolerance.length())
        .map_err(|_| {
            session.degenerate(
                owner,
                owner_type,
                "the model's declared Precision is not a usable station tolerance",
            )
        })
}

/// Distances from the start where the tangent is not shown continuous,
/// ascending; `Err` naming why they cannot be located.
pub(crate) fn tangent_seams(curve: &AtomicCurve) -> Result<Vec<Scalar>, &'static str> {
    match curve {
        AtomicCurve::Two(curve) => seams2(curve),
        AtomicCurve::Three(curve) => seams3(curve),
    }
}

fn seams2(curve: &Curve2) -> Result<Vec<Scalar>, &'static str> {
    match curve {
        Curve2::Line(_)
        | Curve2::Circle(_)
        | Curve2::Ellipse(_)
        | Curve2::Intrinsic(_)
        | Curve2::Chain(_) => Ok(Vec::new()),
        Curve2::Polyline(polyline) => Ok(polyline_seams(
            &polyline
                .points
                .iter()
                .map(|p| [p.x, p.y, 0.0])
                .collect::<Vec<_>>(),
            polyline.closed,
        )),
        Curve2::BSpline(spline) => spline_seams(spline.degree, &spline.multiplicities),
        _ => Err(UNKNOWN),
    }
}

fn seams3(curve: &Curve3) -> Result<Vec<Scalar>, &'static str> {
    match curve {
        Curve3::Line(_) | Curve3::Circle(_) | Curve3::Ellipse(_) | Curve3::Intrinsic(_) => {
            Ok(Vec::new())
        }
        Curve3::Polyline(polyline) => Ok(polyline_seams(
            &polyline
                .points
                .iter()
                .map(|p| [p.x, p.y, p.z])
                .collect::<Vec<_>>(),
            polyline.closed,
        )),
        Curve3::BSpline(spline) => spline_seams(spline.degree, &spline.multiplicities),
        Curve3::Elevated(elevated) => {
            let mut seams = seams2(&elevated.plan)?;
            elevation_seams(&elevated.elevation, 0.0, &mut seams);
            seams.sort_by(f64::total_cmp);
            Ok(seams)
        }
        Curve3::Banked(_) => Err(BANKED),
        _ => Err(UNKNOWN),
    }
}

/// The distance of every interior polyline vertex where the edge direction
/// turns, the closing edge of a closed polyline included. A closed
/// polyline's first vertex is its start and end, not a seam along it.
fn polyline_seams(points: &[[f64; 3]], closed: bool) -> Vec<Scalar> {
    let edge = |a: [f64; 3], b: [f64; 3]| [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let length = |v: [f64; 3]| (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    let mut edges: Vec<[f64; 3]> = points.windows(2).map(|w| edge(w[0], w[1])).collect();
    if let (true, Some(first), Some(last)) = (closed, points.first(), points.last()) {
        edges.push(edge(*last, *first));
    }
    let mut seams = Vec::new();
    let mut distance = 0.0;
    let mut previous: Option<[f64; 3]> = None;
    for current in edges {
        let current_length = length(current);
        if current_length == 0.0 {
            continue;
        }
        if let Some(before) = previous {
            let before_length = length(before);
            let cross = [
                before[1] * current[2] - before[2] * current[1],
                before[2] * current[0] - before[0] * current[2],
                before[0] * current[1] - before[1] * current[0],
            ];
            let dot = before[0] * current[0] + before[1] * current[1] + before[2] * current[2];
            let sine = length(cross) / (before_length * current_length);
            if sine > PARALLEL || dot <= 0.0 {
                seams.push(distance);
            }
        }
        distance += current_length;
        previous = Some(current);
    }
    seams
}

/// A B-spline is `C^(degree - m)` at a knot of multiplicity `m`; at
/// `m >= degree` its tangent may jump, at a distance only quadrature finds.
fn spline_seams(degree: u16, multiplicities: &[u32]) -> Result<Vec<Scalar>, &'static str> {
    let interior = multiplicities
        .get(1..multiplicities.len().saturating_sub(1))
        .unwrap_or(&[]);
    if interior.iter().any(|m| *m >= u32::from(degree)) {
        Err(SPLINE)
    } else {
        Ok(Vec::new())
    }
}

/// Every break of `law`, shifted by `start`, where the grade is not shown
/// continuous. A seam belongs to the piece that starts there.
fn elevation_seams(law: &ElevationLaw, start: Scalar, out: &mut Vec<Scalar>) {
    let ElevationLaw::Piecewise { breaks, laws } = law else {
        return;
    };
    let starts: Vec<Scalar> = std::iter::once(0.0).chain(breaks.iter().copied()).collect();
    for (index, piece) in laws.iter().enumerate() {
        let piece_start = starts.get(index).copied().unwrap_or(0.0);
        elevation_seams(piece, start + piece_start, out);
        let Some(next) = laws.get(index + 1) else {
            continue;
        };
        let Some(&seam) = breaks.get(index) else {
            continue;
        };
        let left = end_grade(piece, seam - piece_start);
        let right = start_grade(next);
        let continuous = matches!(
            (left, right),
            (Some(l), Some(r)) if (l - r).abs() <= SAME_GRADE * l.abs().max(r.abs()).max(1.0)
        );
        if !continuous {
            out.push(start + seam);
        }
    }
}

/// The grade at the end of a piece spanning `span`, where it is closed form.
fn end_grade(law: &ElevationLaw, span: Scalar) -> Option<Scalar> {
    match law {
        ElevationLaw::Intrinsic { .. } => None,
        // A nested piecewise law past its last break reads its last piece.
        _ => law.grade_at(span),
    }
}

/// The grade at the start of a piece, which every law stores.
fn start_grade(law: &ElevationLaw) -> Option<Scalar> {
    match law {
        ElevationLaw::Intrinsic { grade, .. } | ElevationLaw::CircularArc { grade, .. } => {
            Some(*grade)
        }
        ElevationLaw::Piecewise { laws, .. } => laws.first().and_then(start_grade),
        _ => law.grade_at(0.0),
    }
}

/// The basis curve's length, where its data states it: an intrinsic
/// curve's or a chain's stored length, an elevated curve's plan, a
/// polyline's summed edges, a circle's circumference. `None` for a line
/// (unbounded) and for families whose length is an arc-length integral.
pub(crate) fn stated_length(curve: &AtomicCurve) -> Option<Scalar> {
    match curve {
        AtomicCurve::Two(curve) => length2(curve),
        AtomicCurve::Three(curve) => match curve {
            Curve3::Intrinsic(intrinsic) => Some(intrinsic.length),
            Curve3::Elevated(elevated) => length2(&elevated.plan),
            Curve3::Circle(circle) => Some(std::f64::consts::TAU * circle.radius),
            Curve3::Polyline(polyline) => Some(
                polyline
                    .points
                    .windows(2)
                    .map(|w| (w[1] - w[0]).length())
                    .sum::<Scalar>()
                    + closing(
                        polyline.closed,
                        polyline.points.first(),
                        polyline.points.last(),
                    ),
            ),
            _ => None,
        },
    }
    .filter(|length| length.is_finite() && *length >= 0.0)
}

fn length2(curve: &Curve2) -> Option<Scalar> {
    match curve {
        Curve2::Intrinsic(intrinsic) => Some(intrinsic.length),
        Curve2::Chain(chain) => chain.length(),
        Curve2::Circle(circle) => Some(std::f64::consts::TAU * circle.radius),
        Curve2::Polyline(polyline) => {
            let open: Scalar = polyline
                .points
                .windows(2)
                .map(|w| (w[1] - w[0]).length())
                .sum();
            let close = match (
                polyline.closed,
                polyline.points.first(),
                polyline.points.last(),
            ) {
                (true, Some(first), Some(last)) => (*first - *last).length(),
                _ => 0.0,
            };
            Some(open + close)
        }
        _ => None,
    }
}

fn closing(
    closed: bool,
    first: Option<&axiolid_core::Point3>,
    last: Option<&axiolid_core::Point3>,
) -> Scalar {
    match (closed, first, last) {
        (true, Some(first), Some(last)) => (*first - *last).length(),
        _ => 0.0,
    }
}
