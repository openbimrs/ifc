//! An `IfcPolynomialCurve` bounded by an `IfcCurveSegment` (#90).
//!
//! `IfcPolynomialCurve` is unbounded (`-inf < u < inf`), and an
//! `IfcCurveSegment` cuts it by ARC LENGTH: `SegmentStart` and
//! `SegmentLength` are `IfcLengthMeasure`s along the parent (informal
//! proposition 1). For a cubic parabola `x = u`, `y = u^3 / (6 R L)` --
//! IFC4.3's `CUBIC` transition -- the parameter where the arc length
//! reaches `L` inverts an elliptic integral. So the lowering stores the
//! polynomial exactly and leaves that inverse to an evaluator:
//!
//! - the parent between `u = 0` and a bound `U` past the end, as a
//!   Bezier: the power-to-Bernstein conversion of its coefficients, exact
//!   up to rounding, not a fit;
//! - trimmed from parameter `0` to `TrimSelector::ArcLength(L)`, which the
//!   kernel resolves by quadrature and a root find to a tolerance it states.
//!
//! # What is required, and why
//!
//! - **`SegmentStart = 0`**. The segment is placed rigidly so the parent's
//!   point at `SegmentStart` sits on `Placement`; at arc length `0` that is
//!   `P(0)`, closed form. At any other arc length it is the same elliptic
//!   inverse, so a non-zero start is refused rather than computed here.
//! - **Forwards** (`SegmentLength > 0`). Walking the parent backwards from
//!   `u = 0` is legal IFC but has not been needed; refused by name.
//! - **2D, with one coordinate linear in `u`**: `CoefficientsX` or
//!   `CoefficientsY` of degree one with a non-zero slope `c`. Then the speed
//!   is at least `|c|`, so the arc length `L` is reached by `U = L / |c|`
//!   and the stored span provably contains the trim. A curve with no such
//!   coordinate has no closed-form bound, and a 3D polynomial no plane for
//!   the placement to fix; both are refused.

use axiolid_core::{Point2, Vec2};
use axiolid_curve::{BSplineCurve2, KnotSpec};
use ifc_model::EntityId;

use crate::error::GeometryResult;
use crate::lower::session::LoweringSession;

use super::segment::Segment;

const TYPE: &str = "IFCPOLYNOMIALCURVE";

/// The parent in its segment's local frame: origin at `P(0)`, `+x` along
/// the tangent there, parameter `u` over `[0, U]`, in metres.
///
/// # Errors
///
/// Refuses, by name, every case the module documentation lists, and
/// non-finite coefficients or control points.
pub(crate) fn local_bezier(
    session: &LoweringSession<'_>,
    segment: &Segment,
) -> GeometryResult<BSplineCurve2> {
    let parent = segment.parent;
    let refuse = |detail: &'static str| session.unsupported(parent, TYPE, detail);
    if segment.start != 0.0 {
        return Err(refuse(
            "an IfcPolynomialCurve parent cut from a non-zero SegmentStart: the placed start \
             point inverts a non-elementary arc-length integral",
        ));
    }
    if segment.length < 0.0 {
        return Err(refuse(
            "an IfcPolynomialCurve parent walked backwards (negative SegmentLength)",
        ));
    }
    let (x, y) = coefficients(session, parent)?;
    // Lengths in the project unit; the parameter u is dimensionless.
    let f = session.units().length_to_metres;
    let slope = |c: &[f64]| {
        let linear = c.iter().skip(2).all(|v| *v == 0.0);
        let s = c.get(1).copied().unwrap_or(0.0) * f;
        (linear && s != 0.0).then_some(s.abs())
    };
    let Some(speed) = slope(&x).or_else(|| slope(&y)) else {
        return Err(refuse(
            "an IfcPolynomialCurve parent needs CoefficientsX or CoefficientsY of degree one to \
             bound its arc-length trim in closed form",
        ));
    };
    let bound = segment.length / speed;
    let degree = x.len().max(y.len()).max(2) - 1;
    let power = |c: &[f64]| -> Vec<f64> {
        (0..=degree)
            .map(|i| c.get(i).copied().unwrap_or(0.0) * f * bound.powi(i as i32))
            .collect()
    };
    let (px, py) = (bernstein(&power(&x)), bernstein(&power(&y)));
    // Rigid map: P(0) to the origin, the tangent at u = 0 to +x.
    let origin = Point2::new(px[0], py[0]);
    let tangent = Vec2::new(
        x.get(1).copied().unwrap_or(0.0),
        y.get(1).copied().unwrap_or(0.0),
    );
    let tangent = tangent / tangent.length();
    let normal = Vec2::new(-tangent.y, tangent.x);
    let control_points: Vec<Point2> = px
        .iter()
        .zip(&py)
        .map(|(x, y)| {
            let d = Point2::new(*x, *y) - origin;
            Point2::new(d.dot(tangent), d.dot(normal))
        })
        .collect();
    if !bound.is_finite()
        || control_points
            .iter()
            .any(|p| !(p.x.is_finite() && p.y.is_finite()))
    {
        return Err(session.degenerate(
            parent,
            TYPE,
            "the polynomial's Bezier control points are not finite",
        ));
    }
    Ok(BSplineCurve2 {
        degree: u16::try_from(degree).map_err(|_| refuse("polynomial degree out of range"))?,
        control_points,
        knots: vec![0.0, bound],
        multiplicities: vec![
            u32::try_from(degree + 1)
                .map_err(|_| refuse("polynomial degree out of range"))?;
            2
        ],
        weights: None,
        closed: false,
        self_intersect: None,
        knot_spec: KnotSpec::PiecewiseBezier,
    })
}

/// `CoefficientsX` and `CoefficientsY` of a 2D parent, checked finite.
fn coefficients(
    session: &LoweringSession<'_>,
    parent: EntityId,
) -> GeometryResult<(Vec<f64>, Vec<f64>)> {
    let slots = session.slots(parent)?;
    let read = |index: usize, name: &'static str| {
        slots
            .opt(index)
            .map(|_| slots.req_f64_list(index, name))
            .transpose()
    };
    let x = read(1, "CoefficientsX")?;
    let y = read(2, "CoefficientsY")?;
    let (Some(x), Some(y), None) = (x, y, slots.opt(3)) else {
        return Err(session.unsupported(
            parent,
            TYPE,
            "an IfcPolynomialCurve parent needs CoefficientsX and CoefficientsY and no \
             CoefficientsZ: a 3D polynomial has no plane for the segment's placement to fix",
        ));
    };
    if !x.iter().chain(&y).all(|v| v.is_finite()) {
        return Err(session.degenerate(parent, TYPE, "coefficients must be finite"));
    }
    Ok((x, y))
}

/// Bernstein coefficients of a polynomial in `t in [0, 1]` from its power
/// coefficients `a`: `b_j = sum_{i <= j} C(j, i) / C(n, i) a_i`.
fn bernstein(a: &[f64]) -> Vec<f64> {
    let n = a.len() - 1;
    let choose = |n: usize, k: usize| -> f64 {
        (0..k).fold(1.0, |acc, i| acc * (n - i) as f64 / (i + 1) as f64)
    };
    (0..=n)
        .map(|j| {
            (0..=j)
                .map(|i| choose(j, i) / choose(n, i) * a[i])
                .sum::<f64>()
        })
        .collect()
}
