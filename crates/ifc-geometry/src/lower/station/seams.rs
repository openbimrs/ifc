//! Where a basis curve's tangent may jump, and its stated length, read from
//! its stored data.
//!
//! Nothing here evaluates a curve: a polyline's vertex distances are sums of
//! its edge lengths, an elevation law's seams are its stored breaks and its
//! grades at a break are the closed forms `ElevationLaw::grade_at` states.
//! Where only an evaluator could tell (a B-spline's arc length to a multiple
//! knot, the end grade of a vertical clothoid), the seam is not shown
//! continuous and counts as one, or the curve's seams are unknown: a
//! station on it then carries the kernel's seam-snapping window (#423), and
//! a run along it is refused by name.
//!
//! The distances are the ones Axiolid's `exact_station_seams2` /
//! `exact_station_seams3` (`axiolid-evaluate` 0.3.7, ADR 0082 amendment)
//! report for the same data, read the same way: running sums of segment
//! lengths and stored breaks. That crate is an execution provider this
//! crate does not link (ADR 0004), so the reading is repeated here and
//! pinned against it in the tests. It lists every polyline vertex and every
//! profile break; this list keeps only those where the tangent is not shown
//! continuous, the ones where a station's side or a mitre matters. Like it,
//! this refuses a B-spline's corner knot, whose distance is a quadrature.
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

/// Why the seams of a B-spline with a possible kink are unknown here.
pub(crate) const SPLINE: &str =
    "the basis B-spline has an interior knot of multiplicity at least its degree, where its \
     tangent may jump; the knot's distance along it is an arc-length integral, so a station \
     cannot be shown clear of it or snapped to it within the model's precision";

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

/// The smallest cosine of half the turn at a seam, `|t_in + t_out| / 2`,
/// across which a run of stations is mitred: Axiolid's `MITRE_TOLERANCE`
/// (`axiolid-evaluate` 0.3.7, `station::seam`), below which "the tangents
/// nearly reverse and the mitre plane would stretch a section without
/// bound". Repeated here because that crate is not linked; a test pins the
/// two equal.
pub(crate) const MITRE_TOLERANCE: Scalar = 1e-6;

/// A place along a basis curve where its tangent is not shown continuous.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Seam {
    /// Distance from the start in the curve's station measure.
    pub distance: Scalar,
    /// Whether the curve turns back on itself there, by its stored data:
    /// a polyline's edges, or an elevated curve's plan edges, whose half
    /// turn has a cosine at most [`MITRE_TOLERANCE`]. Read in plan on an
    /// elevated curve, where a grade only makes the cosine larger, so this
    /// never misses a reversal Axiolid's mitre refuses.
    pub reverses: bool,
}

impl Seam {
    fn kink(distance: Scalar) -> Self {
        Self {
            distance,
            reverses: false,
        }
    }
}

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

/// How far apart two distances may be and still name the same place, IFC's
/// "within precision limits" (8.9.3.48.3): the declared `precision` (see
/// [`precision`]), or rounding at the distance's magnitude where that is
/// larger.
pub(crate) fn window(precision: f64, distance: f64) -> f64 {
    precision.max(1e-9 * distance.abs().max(1.0))
}

/// The seam of `seams` that `distance` lies on within [`window`], the
/// nearest if two are that close.
pub(crate) fn nearest(seams: &[Seam], distance: f64, precision: f64) -> Option<Seam> {
    let tolerance = window(precision, distance);
    seams
        .iter()
        .filter(|seam| (seam.distance - distance).abs() <= tolerance)
        .min_by(|a, b| {
            (a.distance - distance)
                .abs()
                .total_cmp(&(b.distance - distance).abs())
        })
        .copied()
}

/// Where the tangent is not shown continuous, ascending; `Err` naming why
/// the seams cannot be located.
pub(crate) fn tangent_seams(curve: &AtomicCurve) -> Result<Vec<Seam>, &'static str> {
    match curve {
        AtomicCurve::Two(curve) => seams2(curve),
        AtomicCurve::Three(curve) => seams3(curve),
    }
}

fn seams2(curve: &Curve2) -> Result<Vec<Seam>, &'static str> {
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

/// [`tangent_seams`] of a neutral 3D curve; also what a derived linear
/// placement (`constraint::placement::derive`, #409) locates its seam by.
pub(crate) fn seams3(curve: &Curve3) -> Result<Vec<Seam>, &'static str> {
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
            seams.sort_by(|a, b| a.distance.total_cmp(&b.distance));
            Ok(seams)
        }
        Curve3::Banked(_) => Err(BANKED),
        _ => Err(UNKNOWN),
    }
}

/// Every interior polyline vertex where the edge direction turns, the
/// closing edge of a closed polyline included, and whether it turns back.
/// A closed polyline's first vertex is its start and end, not a seam along
/// it.
fn polyline_seams(points: &[[f64; 3]], closed: bool) -> Vec<Seam> {
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
                let half = 0.5
                    * length([
                        before[0] / before_length + current[0] / current_length,
                        before[1] / before_length + current[1] / current_length,
                        before[2] / before_length + current[2] / current_length,
                    ]);
                seams.push(Seam {
                    distance,
                    reverses: half <= MITRE_TOLERANCE,
                });
            }
        }
        distance += current_length;
        previous = Some(current);
    }
    seams
}

/// A B-spline is `C^(degree - m)` at a knot of multiplicity `m`; at
/// `m >= degree` its tangent may jump, at a distance only quadrature finds.
fn spline_seams(degree: u16, multiplicities: &[u32]) -> Result<Vec<Seam>, &'static str> {
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
fn elevation_seams(law: &ElevationLaw, start: Scalar, out: &mut Vec<Seam>) {
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
            // The plan runs on through a grade break: both tangents point
            // forwards in plan, so they never reverse.
            out.push(Seam::kink(start + seam));
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
