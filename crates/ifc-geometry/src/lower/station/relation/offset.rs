//! Offset pieces of a relation basis (#414): the length of an offset curve
//! beside each span of its basis, read from stored data.
//!
//! Axiolid measures a station along an offset in the OFFSET's own length
//! from its start (axiolid/kernel#289, ADR 0082 amendment 2026-10-10), not
//! in its basis's. The compiler (`station::basis::flatten`) splits the basis
//! at every seam of every piece's curve (a joint, every polyline vertex, a
//! corner knot) and, for an offset by distances, at every station, and
//! offsets each span as one piece (`axiolid_evaluate::station::offset_pieces`):
//! every seam of the basis and every break of the law is a joint of the
//! offset, a seam under the #263 rule. This module repeats that split and
//! states each piece's length in closed form:
//!
//! - beside a straight span (a line, or one edge of a polyline) a constant
//!   offset is a line of the span's length; an offset linear in distance
//!   (`IfcOffsetCurveByDistances` between two stations) is a line too, of
//!   length `sqrt((L + dg)^2 + dl^2 + dv^2)` for the changes `dl`, `dv`, `dg`
//!   of the lateral, vertical and longitudinal offsets across it, since its
//!   section frame (tangent, left, up) is orthonormal and fixed along it;
//! - beside a circular span a constant offset that stays in the circle's
//!   plane is a circle of radius `r - a`, `a` the displacement towards the
//!   centre, so its length is `(r - a) / r` times the span's (the kernel's
//!   own closed form). The displacement is `d` along the left normal
//!   (`IfcOffsetCurve2D`), along `V x T` (`IfcOffsetCurve3D` with `V`
//!   along the circle's axis), or the lateral offset (a constant offset by
//!   distances of a level circle); the left normal points to the centre of
//!   a circle running anticlockwise about its axis seen from `+Z`, and away
//!   from it on one running clockwise or traversed backwards.
//!
//! The sign convention needs no conversion. IFC4 ADD2 TC1 and IFC4.3 ADD2
//! (8.9.3.40, `IfcOffsetCurve2D.Distance`): "A positive value of distance
//! defines an offset in the direction which is normal to the curve in the
//! sense of an anti-clockwise rotation through 90 degrees from the tangent
//! vector T at the given point. (This is in the direction of orthogonal
//! complement(T).)" That is the LEFT of the tangent, Axiolid's `+lateral`
//! and its `OffsetLaw::Planar` ("the offset curve 2D's anti-clockwise
//! rotation through 90 degrees from the tangent"). `IfcOffsetCurve3D`
//! (8.9.3.41): "in the direction V x T where V is the fixed reference
//! direction and T is the unit tangent"; Axiolid's `OffsetLaw::Directed` is
//! `d normalise(V x T)`. Every IFC curve lowers to a 3D curve, a 2D one in
//! `z = 0` of its frame, so an `IfcOffsetCurve2D` lowers with its frame's
//! `+Z` as `V` (`lower::curve`): for a tangent in that plane `Z x T` is the
//! anticlockwise normal, the same offset.
//!
//! The kernel reads an offset of a polyline edge numerically (its quadrature
//! of a constant speed, to rounding), where this module states the closed
//! form; the tests pin the two equal within Axiolid's `ARC_LENGTH_TOLERANCE`.
//! Everything else is a quadrature and refused by name ([`super::INEXACT`]):
//! an offset of an ellipse, a B-spline, a clothoid or a chain; a variable
//! offset of a circle; an offset of a circle placed at a station or tilted
//! out of the offset's plane; and an offset of a plan-measured (gradient)
//! curve, whose own plan length depends on the grade wherever the offset
//! has a vertical part. The kernel's own named refusals are repeated where
//! the stored data shows them: an offset of an offset ([`OFFSET_OF_OFFSET`]),
//! a planar (2D) offset law on a 3D curve ([`PLANAR_3D`]; IFC lowering
//! never stores one) and a circle whose offset radius collapses
//! ([`COLLAPSE`]). An offset across a corner of its
//! basis, whose two sides do not meet, and a 3D offset whose tangent runs
//! along its reference direction, are refused by the kernel when it
//! resolves the station, by name.

use axiolid_core::{Scalar, Vec3};
use axiolid_curve::{Curve2, Curve3, OffsetFrame, OffsetLaw, PathOffsets};
use axiolid_model::{Station, StationFrame};

use super::{pieces_between, Flat, Piece, INEXACT};
use crate::lower::session::AtomicCurve;

/// Why an offset of an offset is refused.
pub(crate) const OFFSET_OF_OFFSET: &str =
    "the basis is an offset of an offset curve; Axiolid measures a station along an offset of \
     atomic curves only (offset the base once, by the summed distance)";

/// Why a planar offset law on a 3D curve is refused.
pub(crate) const PLANAR_3D: &str =
    "the basis is a 2D offset of a 3D curve with no reference direction, which has no normal in \
     its plane (IfcOffsetCurve2D DimIs2D: the underlying curve shall be two-dimensional); a 3D \
     offset needs a reference direction";

/// Why an offset through a circle's centre is refused.
pub(crate) const COLLAPSE: &str =
    "the basis offsets a circular arc by its radius or more towards its centre, so the offset \
     collapses (its radius r - d is zero or negative)";

/// Axiolid's `CUSP_TOLERANCE`: the smallest scale `(r - a) / r` of an
/// offset circle, or speed of an offset relative to its base's, that is not
/// a collapse or a cusp.
const CUSP_TOLERANCE: Scalar = 1e-6;

/// Two directions closer than this sine are one direction: the kernel's
/// test of a circle's axis against `+Z` or a reference direction.
const PARALLEL: Scalar = 1e-12;

/// An offset piece: the span of an atomic curve it is beside (reversed or
/// placed as the flattening left it) and its law, the two parts of
/// Axiolid's neutral `PathOffset`.
#[derive(Debug, Clone)]
// Read by the derived placement along an offset, which needs `compile`;
// a lowering-only build (#367) only measures the pieces.
#[cfg_attr(not(feature = "compile"), allow(dead_code))]
pub(crate) struct OffsetPiece<'s> {
    /// The base span, holding no seam of its curve inside it.
    pub base: Piece<'s>,
    /// The displacement from it.
    pub law: OffsetLaw,
}

/// The offset of `base`, pieces laid end to end, by `law`: one piece per
/// span of a base piece between the seams of its curve, as
/// `offset_pieces` builds them. A linear law runs from its start at the
/// first piece's start to its end at the last piece's end.
pub(super) fn offset_pieces<'s>(
    base: &[Piece<'s>],
    law: OffsetLaw,
) -> Result<Vec<Piece<'s>>, &'static str> {
    let total: Scalar = base.iter().map(Piece::length).sum();
    let mut out = Vec::new();
    let mut run = 0.0;
    for piece in base {
        if piece.offset.is_some() {
            return Err(OFFSET_OF_OFFSET);
        }
        if piece.plan {
            return Err(INEXACT);
        }
        let mut cuts = vec![0.0];
        cuts.extend(vertex_cuts(piece));
        cuts.push(piece.length());
        for pair in cuts.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            let law = match law {
                OffsetLaw::Linear { start, end, frame } => OffsetLaw::Linear {
                    start: start.lerp(end, (run + a) / total),
                    end: start.lerp(end, (run + b) / total),
                    frame,
                },
                other => other,
            };
            let mut span = piece.clone();
            if piece.reversed {
                span.start = piece.end - b;
                span.end = piece.end - a;
            } else {
                span.start = piece.start + a;
                span.end = piece.start + b;
            }
            let length = offset_length(&span, law)?;
            out.push(Piece {
                curve: piece.curve,
                start: 0.0,
                end: length,
                reversed: false,
                plan: false,
                seams: Vec::new(),
                placements: Vec::new(),
                offset: Some(Box::new(OffsetPiece { base: span, law })),
            });
        }
        run += piece.length();
    }
    Ok(out)
}

/// An offset by distances along `base`, flattened: the basis between each
/// pair of consecutive stations (an atomic one measured from its own start,
/// a line unbounded), offset by the offsets interpolated between them.
pub(super) fn by_stations<'s>(
    base: Flat<'s>,
    stations: &[Station],
    frame: StationFrame,
) -> Result<Vec<Piece<'s>>, &'static str> {
    if stations.len() < 2 {
        return Err(INEXACT);
    }
    let frame = match frame {
        StationFrame::Section => OffsetFrame::Section,
        StationFrame::Plan => OffsetFrame::Plan,
        _ => return Err(INEXACT),
    };
    let offsets = |station: &Station| {
        PathOffsets::new(
            station.offsets.lateral,
            station.offsets.vertical,
            station.offsets.longitudinal,
        )
    };
    let mut out = Vec::new();
    for pair in stations.windows(2) {
        let (a, b) = (pair[0].distance, pair[1].distance);
        let span = match &base {
            Flat::Atomic(curve, placements) => {
                vec![super::piece(curve, a, b, placements.clone())?]
            }
            Flat::Pieces(pieces) => pieces_between(pieces, a, b)?,
        };
        let law = OffsetLaw::Linear {
            start: offsets(&pair[0]),
            end: offsets(&pair[1]),
            frame,
        };
        out.extend(offset_pieces(&span, law)?);
    }
    Ok(out)
}

/// The distances into `piece`, in its direction, of its curve's polyline
/// vertices strictly inside it: the compiler splits an offset's base there
/// (`StationPiece::seams`), whether the edges turn or not.
fn vertex_cuts(piece: &Piece<'_>) -> Vec<Scalar> {
    let edges: Vec<Scalar> = match piece.curve {
        AtomicCurve::Two(Curve2::Polyline(polyline)) => {
            let points = &polyline.points;
            let mut edges: Vec<Scalar> =
                points.windows(2).map(|w| (w[1] - w[0]).length()).collect();
            if let (true, Some(first), Some(last)) =
                (polyline.closed, points.first(), points.last())
            {
                edges.push((*first - *last).length());
            }
            edges
        }
        AtomicCurve::Three(Curve3::Polyline(polyline)) => {
            let points = &polyline.points;
            let mut edges: Vec<Scalar> =
                points.windows(2).map(|w| (w[1] - w[0]).length()).collect();
            if let (true, Some(first), Some(last)) =
                (polyline.closed, points.first(), points.last())
            {
                edges.push((*first - *last).length());
            }
            edges
        }
        _ => return Vec::new(),
    };
    // `polyline_seams`: the vertex where each non-degenerate edge starts
    // after an earlier non-degenerate one.
    let mut vertices = Vec::new();
    let (mut at, mut started) = (0.0, false);
    for edge in edges {
        if edge > 0.0 {
            if started {
                vertices.push(at);
            }
            started = true;
        }
        at += edge;
    }
    let mut cuts: Vec<Scalar> = vertices
        .into_iter()
        .filter(|v| {
            *v > piece.start + super::slack(piece.start) && *v < piece.end - super::slack(piece.end)
        })
        .map(|v| {
            if piece.reversed {
                piece.end - v
            } else {
                v - piece.start
            }
        })
        .collect();
    cuts.sort_by(Scalar::total_cmp);
    cuts
}

/// The length of the offset by `law` of the span `piece`, holding no seam
/// of its curve, where stored data states it.
fn offset_length(piece: &Piece<'_>, law: OffsetLaw) -> Result<Scalar, &'static str> {
    let two_d = matches!(piece.curve, AtomicCurve::Two(_));
    if matches!(law, OffsetLaw::Planar { .. }) && !two_d {
        return Err(PLANAR_3D);
    }
    let length = piece.length();
    let placed = !piece.placements.is_empty();
    match piece.curve {
        AtomicCurve::Two(Curve2::Line(_) | Curve2::Polyline(_)) => straight(length, law, true),
        AtomicCurve::Three(Curve3::Line(line)) => {
            straight(length, law, line.direction.z == 0.0 && !placed)
        }
        AtomicCurve::Three(Curve3::Polyline(polyline)) => {
            let level = polyline.points.iter().all(|p| p.z == polyline.points[0].z);
            straight(length, law, level && !placed)
        }
        AtomicCurve::Two(Curve2::Circle(circle)) if !placed => {
            let normal = Vec3::new(0.0, 0.0, circle.frame.x.perp_dot(circle.frame.y));
            arc(circle.radius, normal, length, piece.reversed, law)
        }
        AtomicCurve::Three(Curve3::Circle(circle)) if !placed => {
            let normal = circle.frame.x.cross(circle.frame.y);
            arc(circle.radius, normal, length, piece.reversed, law)
        }
        _ => Err(INEXACT),
    }
}

/// An offset of a straight span `length` long: a line beside it. `level`:
/// the span runs horizontally, so its plan frame is its section frame.
fn straight(length: Scalar, law: OffsetLaw, level: bool) -> Result<Scalar, &'static str> {
    match law {
        OffsetLaw::Planar { .. } | OffsetLaw::Directed { .. } => Ok(length),
        OffsetLaw::Linear { start, end, frame } => {
            if frame != OffsetFrame::Section && !level {
                return Err(INEXACT);
            }
            let along = length + (end.longitudinal - start.longitudinal);
            if along.is_nan() || along <= CUSP_TOLERANCE * length {
                return Err(INEXACT);
            }
            let across = end.lateral - start.lateral;
            let up = end.vertical - start.vertical;
            Ok((along * along + across * across + up * up).sqrt())
        }
        _ => Err(INEXACT),
    }
}

/// A constant offset of a circular span `length` long, of radius `radius`
/// and axis `normal` (`x cross y` of its frame): a circle of radius
/// `r - a`, `a` the displacement towards the centre.
fn arc(
    radius: Scalar,
    normal: Vec3,
    length: Scalar,
    reversed: bool,
    law: OffsetLaw,
) -> Result<Scalar, &'static str> {
    let axis = normal.normalize();
    let sense = if reversed { -1.0 } else { 1.0 };
    // The left lateral `Z x T` points to the centre where the circle runs
    // anticlockwise seen from `+Z`, `V x T` where it runs anticlockwise
    // about `V`.
    let along = |direction: Vec3| {
        let direction = direction.normalize();
        (direction.cross(axis).length() <= PARALLEL).then(|| direction.dot(axis).signum())
    };
    let towards = match law {
        OffsetLaw::Planar { distance } => distance * along(Vec3::Z).ok_or(INEXACT)?,
        OffsetLaw::Directed {
            distance,
            reference_direction,
        } => distance * along(reference_direction).ok_or(INEXACT)?,
        OffsetLaw::Linear { start, end, .. } => {
            if start != end || start.longitudinal != 0.0 {
                return Err(INEXACT);
            }
            start.lateral * along(Vec3::Z).ok_or(INEXACT)?
        }
        _ => return Err(INEXACT),
    } * sense;
    let scale = (radius - towards) / radius;
    if !(scale.is_finite() && scale > CUSP_TOLERANCE) {
        return Err(COLLAPSE);
    }
    Ok(scale * length)
}
