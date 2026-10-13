//! Trims of an atomic curve (#346, #436): the span a trim keeps, read from
//! its selectors as the compiler reads them (`station::basis::trim_parameters`),
//! stated in the curve's station measure where its data states both ends,
//! else on an ellipse or a B-spline bounded at the parameters or points the
//! selectors name; see the parent module's documentation.

use axiolid_core::{Point3, Scalar};
use axiolid_curve::{Curve2, Curve3, PathBound};
use axiolid_model::{OrientedCurveStation, TrimSelector, TrimmingPreference};

use super::{piece, Piece, INEXACT, POINT_TOLERANCE};
use crate::lower::session::AtomicCurve;

/// The span of native parameters `lo < hi` of `curve` in its station
/// measure, as `StationPiece::between_parameters` locates it: a closed
/// conic's from `lo` taken modulo one turn, past its measure's end where
/// the span crosses the parameter seam.
fn measures_between(
    curve: &AtomicCurve,
    lo: Scalar,
    hi: Scalar,
) -> Result<(Scalar, Scalar), &'static str> {
    if !closed_conic(curve) {
        return Ok((measure_at(curve, lo)?, measure_at(curve, hi)?));
    }
    let turn = std::f64::consts::TAU;
    let from = lo.rem_euclid(turn);
    let to = from + (hi - lo);
    let start = measure_at(curve, from)?;
    let end = if to <= turn {
        measure_at(curve, to)?
    } else {
        whole_length(curve).ok_or(INEXACT)? + measure_at(curve, to - turn)?
    };
    Ok((start, end))
}

/// A whole curve's station length, where its data states it.
pub(super) fn whole_length(curve: &AtomicCurve) -> Option<Scalar> {
    match curve {
        AtomicCurve::Two(Curve2::Ellipse(_) | Curve2::BSpline(_))
        | AtomicCurve::Three(Curve3::Ellipse(_) | Curve3::BSpline(_)) => None,
        _ => crate::lower::station::seams::stated_length(curve),
    }
}

/// The station measure at native parameter `t` (`StationCurve::measure_at`):
/// `t |d|` on a line, `r t` on a circle, the edges walked on a polyline,
/// `t` on a curve parameterised by its measure.
fn measure_at(curve: &AtomicCurve, t: Scalar) -> Result<Scalar, &'static str> {
    Ok(match curve {
        AtomicCurve::Two(Curve2::Line(line)) => t * line.direction.length(),
        AtomicCurve::Three(Curve3::Line(line)) => t * line.direction.length(),
        AtomicCurve::Two(Curve2::Circle(circle)) => t * circle.radius,
        AtomicCurve::Three(Curve3::Circle(circle)) => t * circle.radius,
        AtomicCurve::Two(Curve2::Polyline(polyline)) => polyline_measure(
            &polyline
                .points
                .iter()
                .map(|p| Point3::new(p.x, p.y, 0.0))
                .collect::<Vec<_>>(),
            polyline.closed,
            t,
        )?,
        AtomicCurve::Three(Curve3::Polyline(polyline)) => {
            polyline_measure(&polyline.points, polyline.closed, t)?
        }
        AtomicCurve::Two(Curve2::Intrinsic(_) | Curve2::Chain(_))
        | AtomicCurve::Three(Curve3::Intrinsic(_) | Curve3::Elevated(_)) => t,
        _ => return Err(INEXACT),
    })
}

/// The arc length from a polyline's start to parameter `t`, whose integer
/// part selects the edge (the closing edge last on a closed polyline).
pub(super) fn polyline_measure(
    points: &[Point3],
    closed: bool,
    t: Scalar,
) -> Result<Scalar, &'static str> {
    let mut edges: Vec<Scalar> = points.windows(2).map(|w| (w[1] - w[0]).length()).collect();
    if let (true, Some(first), Some(last)) = (closed, points.first(), points.last()) {
        edges.push((*first - *last).length());
    }
    if !t.is_finite() || t < 0.0 || t > edges.len() as Scalar {
        return Err(INEXACT);
    }
    let whole = (t.floor() as usize).min(edges.len().saturating_sub(1));
    let fraction = t - whole as Scalar;
    Ok(edges[..whole].iter().sum::<Scalar>() + fraction * edges.get(whole).copied().unwrap_or(0.0))
}

/// Whether the curve is a circle or an ellipse, whose trim may cross its
/// parameter seam.
fn closed_conic(curve: &AtomicCurve) -> bool {
    matches!(
        curve,
        AtomicCurve::Two(Curve2::Circle(_) | Curve2::Ellipse(_))
            | AtomicCurve::Three(Curve3::Circle(_) | Curve3::Ellipse(_))
    )
}

/// The piece a trim of the atomic `curve` keeps (module documentation):
/// stated in its station measure where its data states both ends, else,
/// on an ellipse or a B-spline (#436), bounded at the native parameters or
/// points its selectors name, which the evaluator resolves.
pub(super) fn trimmed<'s>(
    curve: &'s AtomicCurve,
    start: &[TrimSelector],
    end: &[TrimSelector],
    sense: bool,
    preference: TrimmingPreference,
    placements: Vec<&'s OrientedCurveStation>,
) -> Result<Piece<'s>, &'static str> {
    match trim_parameters(curve, start, end, sense, preference)? {
        Trim::Parameters(lo, hi) => match measures_between(curve, lo, hi) {
            Ok((from, to)) => piece(curve, from, to, placements),
            // The distance to an interior parameter of an ellipse or a
            // B-spline is a quadrature: the evaluator locates the span as
            // `StationPiece::between_parameters` does, from the same `lo`
            // and `hi` (`PathPiece::between_parameters`, #436).
            Err(_) if quadrature(curve) => bounded(
                curve,
                TrimEnd::Parameter(lo),
                TrimEnd::Parameter(hi),
                placements,
            ),
            Err(reason) => Err(reason),
        },
        Trim::Bounded(from, to) => bounded(curve, from, to, placements),
    }
}

/// The span of `curve` from `from` to `to`, in its direction, each end
/// stated or left to the evaluator as a [`PathBound`] (#436).
fn bounded<'s>(
    curve: &'s AtomicCurve,
    from: TrimEnd,
    to: TrimEnd,
    placements: Vec<&'s OrientedCurveStation>,
) -> Result<Piece<'s>, &'static str> {
    let mut out = piece(curve, -Scalar::INFINITY, Scalar::INFINITY, placements)?;
    match from {
        TrimEnd::Measure(m) => out.start = m,
        other => out.start_bound = other.bound(),
    }
    match to {
        TrimEnd::Measure(m) => out.end = m,
        other => out.end_bound = other.bound(),
    }
    Ok(out)
}

/// Whether `curve` is an ellipse or a B-spline, whose station measure at
/// an interior parameter is a quadrature.
fn quadrature(curve: &AtomicCurve) -> bool {
    matches!(
        curve,
        AtomicCurve::Two(Curve2::Ellipse(_) | Curve2::BSpline(_))
            | AtomicCurve::Three(Curve3::Ellipse(_) | Curve3::BSpline(_))
    )
}

/// What a trim of an atomic curve keeps.
enum Trim {
    /// The native parameters `lo < hi` it spans (`lo` may lie outside a
    /// closed conic's first turn, `hi` past it).
    Parameters(Scalar, Scalar),
    /// On an ellipse or a B-spline, the ends its selectors name in the
    /// curve's direction, at least one of them an arc length or a point
    /// (#436).
    Bounded(TrimEnd, TrimEnd),
}

/// One trim end read from its selectors.
#[derive(Debug, Clone, Copy, PartialEq)]
enum TrimEnd {
    /// A native parameter of the curve.
    Parameter(Scalar),
    /// A distance in the curve's station measure: an arc length on an
    /// ellipse or a B-spline whose measure starts at parameter `0`, in the
    /// curve's own sense (#436).
    Measure(Scalar),
    /// A point of an ellipse or a B-spline, which only an evaluator
    /// inverts (#436).
    Point(Point3),
}

impl TrimEnd {
    fn bound(self) -> Option<PathBound> {
        match self {
            Self::Parameter(t) => Some(PathBound::Parameter(t)),
            Self::Point(p) => Some(PathBound::Point(p)),
            Self::Measure(_) => None,
        }
    }

    fn is_finite(self) -> bool {
        match self {
            Self::Parameter(v) | Self::Measure(v) => v.is_finite(),
            Self::Point(p) => p.is_finite(),
        }
    }
}

/// The span a trim of an atomic curve keeps, as the compiler reads it
/// (`station::basis::trim_parameters`): between parameters `lo < hi`, a
/// closed conic's from the start selector in the trim's sense (possibly
/// across its parameter seam), any other curve's between its two
/// selectors. On an ellipse or a B-spline an end named by an arc length or
/// a point is left to the evaluator (#436): the span runs from the start
/// selector to the end selector in the trim's sense (from the end to the
/// start against it), on a closed conic forwards within one turn as the
/// compiler's periodic trim does. A span that does not run so (an open
/// B-spline trimmed against the order of its parameters) is refused by the
/// evaluator, by name.
fn trim_parameters(
    curve: &AtomicCurve,
    start: &[TrimSelector],
    end: &[TrimSelector],
    sense: bool,
    preference: TrimmingPreference,
) -> Result<Trim, &'static str> {
    let scale = start
        .iter()
        .chain(end)
        .map(|selector| match selector {
            TrimSelector::Point2(p) => p.abs().max_element(),
            TrimSelector::Point3(p) => p.abs().max_element(),
            _ => 0.0,
        })
        .fold(1.0, Scalar::max);
    let tolerance = POINT_TOLERANCE * scale;
    let a = trim_parameter(start, preference, curve, sense, tolerance)?;
    let b = trim_parameter(end, preference, curve, sense, tolerance)?;
    let (TrimEnd::Parameter(a), TrimEnd::Parameter(b)) = (a, b) else {
        let (from, to) = if sense { (a, b) } else { (b, a) };
        return Ok(Trim::Bounded(from, to));
    };
    if a == b {
        return Err(INEXACT);
    }
    if closed_conic(curve) {
        // `directrix::periodic_trim_interval` without a range.
        let period = std::f64::consts::TAU;
        let travelled = if sense { b - a } else { a - b };
        let span = if travelled > 0.0 && travelled <= period * (1.0 + 1e-12) {
            travelled
        } else {
            match travelled.rem_euclid(period) {
                0.0 => period,
                wrapped => wrapped,
            }
        };
        let (lo, hi) = if sense { (a, a + span) } else { (a - span, a) };
        return Ok(Trim::Parameters(lo, hi));
    }
    Ok(Trim::Parameters(a.min(b), a.max(b)))
}

/// One trim end read from its selectors (`directrix::parameter`): a
/// parameter, a point inverted onto a line or a circle; on an ellipse or a
/// B-spline an arc length or a point left to the evaluator (#436).
fn trim_parameter(
    selectors: &[TrimSelector],
    preference: TrimmingPreference,
    curve: &AtomicCurve,
    sense: bool,
    tolerance: Scalar,
) -> Result<TrimEnd, &'static str> {
    let is_measure =
        |s: &&TrimSelector| matches!(s, TrimSelector::Parameter(_) | TrimSelector::ArcLength(_));
    let first_measure = || {
        selectors
            .iter()
            .find(is_measure)
            .map_or(Ok(None), |s| measured(s, curve, sense))
    };
    let invert = || {
        selectors.iter().find_map(|selector| {
            let point = match selector {
                TrimSelector::Point2(p) => Point3::new(p.x, p.y, 0.0),
                TrimSelector::Point3(p) => *p,
                _ => return None,
            };
            match invert(curve, point, tolerance) {
                Some(t) => Some(TrimEnd::Parameter(t)),
                // Inverted by the evaluator, which refuses a point off the
                // curve or one several parameters name, by name.
                None if quadrature(curve) => Some(TrimEnd::Point(point)),
                None => None,
            }
        })
    };
    let selected = match preference {
        TrimmingPreference::Parameter => first_measure()?,
        TrimmingPreference::Cartesian => match invert() {
            Some(value) => Some(value),
            None => first_measure()?,
        },
        _ => match selectors.first() {
            Some(selector) if is_measure(&selector) => measured(selector, curve, sense)?,
            _ => invert(),
        },
    };
    selected.filter(|end| end.is_finite()).ok_or(INEXACT)
}

/// A parameter, or an arc length from parameter `0` in the trim's sense:
/// as a parameter where the curve's data states it; on an ellipse or a
/// B-spline whose measure starts at parameter `0`, in the curve's own
/// sense, as the distance it is in the curve's measure (#436).
fn measured(
    selector: &TrimSelector,
    curve: &AtomicCurve,
    sense: bool,
) -> Result<Option<TrimEnd>, &'static str> {
    match selector {
        TrimSelector::Parameter(value) => Ok(Some(TrimEnd::Parameter(*value))),
        TrimSelector::ArcLength(length) => {
            let signed = if sense { *length } else { -*length };
            Ok(Some(TrimEnd::Parameter(match curve {
                AtomicCurve::Two(Curve2::Line(line)) => signed / line.direction.length(),
                AtomicCurve::Three(Curve3::Line(line)) => signed / line.direction.length(),
                AtomicCurve::Two(Curve2::Circle(circle)) => signed / circle.radius,
                AtomicCurve::Three(Curve3::Circle(circle)) => signed / circle.radius,
                AtomicCurve::Two(Curve2::Intrinsic(_) | Curve2::Chain(_))
                | AtomicCurve::Three(Curve3::Intrinsic(_)) => signed,
                // The arc length from parameter `0` forwards is the
                // curve's own measure there; backwards, or from a
                // parameter `0` that is not where the measure starts, it
                // is a quadrature from the start.
                _ if sense && *length >= 0.0 && measured_from_zero(curve) => {
                    return Ok(Some(TrimEnd::Measure(*length)));
                }
                _ => return Err(INEXACT),
            })))
        }
        _ => Ok(None),
    }
}

/// Whether `curve` is an ellipse, or a B-spline whose parameter domain
/// starts at `0`: its station measure is the arc length from parameter
/// `0`.
fn measured_from_zero(curve: &AtomicCurve) -> bool {
    let spline_start = |degree: u16, knots: &[Scalar], multiplicities: &[u32]| {
        knots
            .iter()
            .zip(multiplicities)
            .flat_map(|(knot, times)| std::iter::repeat_n(*knot, *times as usize))
            .nth(usize::from(degree))
    };
    match curve {
        AtomicCurve::Two(Curve2::Ellipse(_)) | AtomicCurve::Three(Curve3::Ellipse(_)) => true,
        AtomicCurve::Two(Curve2::BSpline(spline)) => {
            spline_start(spline.degree, &spline.knots, &spline.multiplicities) == Some(0.0)
        }
        AtomicCurve::Three(Curve3::BSpline(spline)) => {
            spline_start(spline.degree, &spline.knots, &spline.multiplicities) == Some(0.0)
        }
        _ => false,
    }
}

/// The parameter of `point` on a line or a circle, when it lies on it
/// within `tolerance`.
fn invert(curve: &AtomicCurve, point: Point3, tolerance: Scalar) -> Option<Scalar> {
    let (origin, direction) = match curve {
        AtomicCurve::Two(Curve2::Line(line)) => (
            Point3::new(line.origin.x, line.origin.y, 0.0),
            axiolid_core::Vec3::new(line.direction.x, line.direction.y, 0.0),
        ),
        AtomicCurve::Three(Curve3::Line(line)) => (line.origin, line.direction),
        AtomicCurve::Two(Curve2::Circle(circle)) => {
            let frame = circle.frame;
            let d = axiolid_core::Vec2::new(point.x - frame.origin.x, point.y - frame.origin.y);
            let (u, v) = (d.dot(frame.x), d.dot(frame.y));
            let on =
                point.z.abs() <= tolerance && ((u.hypot(v)) - circle.radius).abs() <= tolerance;
            return on.then(|| v.atan2(u));
        }
        AtomicCurve::Three(Curve3::Circle(circle)) => {
            let frame = circle.frame;
            let d = point - frame.origin;
            let (u, v, w) = (d.dot(frame.x), d.dot(frame.y), d.dot(frame.z));
            let on = w.abs() <= tolerance && (u.hypot(v) - circle.radius).abs() <= tolerance;
            return on.then(|| v.atan2(u));
        }
        _ => return None,
    };
    let speed = direction.length_squared();
    if !(speed.is_finite() && speed > 0.0) {
        return None;
    }
    let t = (point - origin).dot(direction) / speed;
    let foot = origin + direction * t;
    ((point - foot).length() <= tolerance).then_some(t)
}
