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
//! Everything else is a quadrature ([`super::INEXACT`]): an offset of an
//! ellipse, a B-spline, a clothoid or a chain; a variable offset of a
//! circle; an offset of a circle placed at a station or tilted out of the
//! offset's plane; and an offset of a plan-measured (gradient) curve, whose
//! own plan length depends on the grade wherever the offset has a vertical
//! part. Its joints are unknown here, so a station along it carries the
//! kernel's seam-snapping window, which the kernel resolves against its own
//! certified joint distances (#423, axiolid/kernel#294), as do the sections
//! of a run along it (#429); an offset run along it continues to the end
//! the kernel measures (#433). A station along such an offset run
//! (`OffsetsAtStations`, #443) is flattened as the kernel flattens it
//! ([`by_stations`]). A derived placement reads such a piece open-ended, its length the
//! evaluator's (#427, `relation_pieces`). Its base is split at every seam
//! of its curve the kernel splits it at, smooth or not (`kernel_cuts`), so
//! the pieces are the kernel's. The kernel's own named
//! refusals are repeated where
//! the stored data shows them: an offset of an offset ([`OFFSET_OF_OFFSET`]),
//! a planar (2D) offset law on a 3D curve ([`PLANAR_3D`]; IFC lowering
//! never stores one) and a circle whose offset radius collapses
//! ([`COLLAPSE`]). An offset across a corner of its
//! basis, whose two sides do not meet, and a 3D offset whose tangent runs
//! along its reference direction, are refused by the kernel when it
//! resolves the station, by name.

use axiolid_core::{Scalar, Vec3};
use axiolid_curve::{
    ChainPiece2, Curve2, Curve3, ElevationLaw, OffsetFrame, OffsetLaw, PathOffsets,
};
use axiolid_model::{BasisEnd, RunStation, StationFrame};

use super::{pieces_between, Flat, Piece, INEXACT};
use crate::lower::session::AtomicCurve;

/// Why an offset of an offset is refused: Axiolid's own rule
/// (axiolid/kernel#289, `axiolid-evaluate`'s `StationOffset::new`), whatever
/// the two offsets are (`IfcOffsetCurve2D`, `IfcOffsetCurve3D`,
/// `IfcOffsetCurveByDistances`, in any combination).
pub(crate) const OFFSET_OF_OFFSET: &str =
    "the basis is an offset of an offset curve (an IfcOffsetCurve2D, IfcOffsetCurve3D or \
     IfcOffsetCurveByDistances whose BasisCurve is itself an offset curve); Axiolid measures a \
     station along an offset of atomic curves only (axiolid/kernel#289: offset the base once, \
     by the summed law)";

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
/// first piece's start to its end at the last piece's end. A piece whose
/// length has no closed form here is open (#427): its end is the
/// evaluator's own length of the offset.
pub(super) fn offset_pieces<'s>(
    base: &[Piece<'s>],
    law: OffsetLaw,
) -> Result<Vec<Piece<'s>>, &'static str> {
    if base.iter().any(|piece| piece.offset.is_some()) {
        return Err(OFFSET_OF_OFFSET);
    }
    let total: Scalar = base.iter().map(Piece::length).sum();
    let mut out = Vec::new();
    let mut run = 0.0;
    for piece in base {
        let cuts = kernel_cuts(piece)?;
        let spans: Vec<(Piece<'s>, OffsetLaw)> = if !piece.is_stated() {
            // A base whose own end (or start, #436) is the evaluator's:
            // offset whole, which needs neither a cut nor where a linear
            // law is along it; a constant one (the offsets an offset by
            // distances carries on to its basis's end, #443) is the same
            // wherever it is read.
            if !cuts.is_empty() || !constant(law) {
                return Err(INEXACT);
            }
            vec![(piece.clone(), law)]
        } else {
            let mut bounds = vec![0.0];
            bounds.extend(cuts);
            bounds.push(piece.length());
            bounds
                .windows(2)
                .map(|pair| {
                    let (a, b) = (pair[0], pair[1]);
                    let law = match law {
                        OffsetLaw::Linear { start, end, frame } if start != end => {
                            OffsetLaw::Linear {
                                start: start.lerp(end, (run + a) / total),
                                end: start.lerp(end, (run + b) / total),
                                frame,
                            }
                        }
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
                    (span, law)
                })
                .collect()
        };
        for (span, law) in spans {
            let length = if !span.is_stated() {
                None
            } else {
                offset_length(&span, law)?
            };
            out.push(Piece {
                curve: piece.curve,
                start: 0.0,
                end: length.unwrap_or(Scalar::INFINITY),
                open: length.is_none(),
                start_bound: None,
                end_bound: None,
                reversed: false,
                // Beside a plan-measured curve the offset is measured in its
                // own plan length (axiolid/kernel#294).
                plan: piece.plan,
                seams: Vec::new(),
                placements: Vec::new(),
                offset: Some(Box::new(OffsetPiece { base: span, law })),
            });
        }
        run += piece.length();
    }
    Ok(out)
}

/// Whether `law` is the same all along: a constant offset, or a linear one
/// whose two ends agree.
fn constant(law: OffsetLaw) -> bool {
    match law {
        OffsetLaw::Linear { start, end, .. } => start == end,
        _ => true,
    }
}

/// Where a station of an offset by distances stands along its basis.
#[derive(Debug, Clone, Copy)]
enum At {
    /// At this distance, stated.
    Distance(Scalar),
    /// At the basis's end, which the kernel measures (#433).
    End,
}

/// An offset by distances along `base`, flattened (`OffsetByStations`, or
/// its general form `OffsetsAtStations`, #429, #433, #443): the basis
/// between each pair of consecutive stations (an atomic one measured from
/// its own start, a line unbounded), offset by the offsets interpolated
/// between them, as `axiolid-mesh-compile`'s `offset_by_stations` builds
/// them.
///
/// The kernel resolves a station at the basis's start to `0` and one at its
/// end to the basis's length by its own measure, then snaps every station
/// within its seam-snapping window, and splits the basis there. Here:
///
/// - a station at the start stands at `0`;
/// - a station at the end, last, closes a span that runs to the basis's
///   end: clipped from the stated pieces where every length is stated, and
///   otherwise reaching into its last piece whose length is a quadrature,
///   left open there (#427), the evaluator's end;
/// - a windowed station stands at its own distance where no seam of the
///   basis lies within its window, the kernel's snap then a no-op. A seam
///   within it, which the kernel would snap onto, or one past a piece whose
///   length is a quadrature, leaves where the span ends to the kernel
///   ([`INEXACT`]).
///
/// An offset of an offset is refused first, as the kernel refuses it
/// ([`OFFSET_OF_OFFSET`]).
pub(super) fn by_stations<'s>(
    base: Flat<'s>,
    stations: &[RunStation],
    frame: StationFrame,
) -> Result<Vec<Piece<'s>>, &'static str> {
    if let Flat::Pieces(pieces) = &base {
        if pieces.iter().any(|piece| piece.offset.is_some()) {
            return Err(OFFSET_OF_OFFSET);
        }
    }
    if stations.len() < 2 {
        return Err(INEXACT);
    }
    let frame = match frame {
        StationFrame::Section => OffsetFrame::Section,
        StationFrame::Plan => OffsetFrame::Plan,
        _ => return Err(INEXACT),
    };
    let offsets = |station: &RunStation| {
        let offsets = &station.station.offsets;
        PathOffsets::new(offsets.lateral, offsets.vertical, offsets.longitudinal)
    };
    // The whole basis, read only by a run that carries a window or a
    // station at its end, so a plain run is flattened as it always was.
    let general = stations
        .iter()
        .any(|station| station.basis_end.is_some() || station.seam_window > 0.0);
    let whole = if general {
        match &base {
            Flat::Atomic(curve, placements) => Flat::Atomic(curve, placements.clone()),
            Flat::Pieces(pieces) => Flat::Pieces(pieces.clone()),
        }
        .into_pieces()?
    } else {
        Vec::new()
    };
    let seams = if general {
        snap_seams(&whole)?
    } else {
        Vec::new()
    };
    let places = stations
        .iter()
        .map(|station| match station.basis_end {
            None => {
                let distance = station.station.distance;
                let window = station.seam_window;
                let snaps = window > 0.0
                    && seams
                        .iter()
                        .any(|seam| (seam - distance).abs() <= window + super::slack(distance));
                if snaps {
                    Err(INEXACT)
                } else {
                    Ok(At::Distance(distance))
                }
            }
            Some(BasisEnd::Start) => Ok(At::Distance(0.0)),
            Some(BasisEnd::End) => Ok(At::End),
            Some(_) => Err(INEXACT),
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut out = Vec::new();
    for (pair, at) in stations.windows(2).zip(places.windows(2)) {
        let span = match (at[0], at[1]) {
            (At::Distance(a), At::Distance(b)) => match &base {
                Flat::Atomic(curve, placements) => {
                    vec![super::piece(curve, a, b, placements.clone())?]
                }
                Flat::Pieces(pieces) => pieces_between(pieces, a, b)?,
            },
            (At::Distance(a), At::End) => to_end(&whole, a)?,
            // A station at the end before another: the kernel refuses the
            // run's order by name.
            (At::End, _) => return Err(INEXACT),
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

/// The pieces of `pieces`, the whole basis, from the distance `from` to its
/// end. Where its last piece's length is a quadrature (open, #427, or ending
/// at a parameter or a point, #436) and every piece before it is stated, the
/// span keeps that piece from its stated start (shifted by `from` where
/// `from` lies inside it), so its end stays the evaluator's, as the
/// kernel's own `pieces_between(from, length)` ends at the length it
/// measures. Any other leaves where the span starts unknown ([`INEXACT`]).
fn to_end<'s>(pieces: &[Piece<'s>], from: Scalar) -> Result<Vec<Piece<'s>>, &'static str> {
    let Some((last, before)) = pieces.split_last() else {
        return Err(INEXACT);
    };
    if last.is_stated() {
        let length: Scalar = pieces.iter().map(Piece::length).sum();
        return pieces_between(pieces, from, length);
    }
    if last.reversed || last.start_bound.is_some() || !before.iter().all(Piece::is_stated) {
        return Err(INEXACT);
    }
    let prefix: Scalar = before.iter().map(Piece::length).sum();
    let mut out = if from < prefix - super::slack(prefix) {
        pieces_between(before, from, prefix)?
    } else {
        Vec::new()
    };
    let mut tail = last.clone();
    if from > prefix + super::slack(prefix) {
        tail.start = last.start + (from - prefix);
    }
    out.push(tail);
    Ok(out)
}

/// The distances along `pieces`, the whole basis, of every seam the kernel
/// may snap a windowed station onto (`axiolid_evaluate::station::window`):
/// every joint and every seam of a stated piece's curve inside it
/// ([`kernel_cuts`], a superset of the seams whose frame may jump), up to
/// the first piece whose length is a quadrature, whose start is the last
/// one stated. Such a piece is an ellipse's, a B-spline's without a corner
/// knot (one with a corner knot is refused before this, `SPLINE`) or an
/// offset's of a span without seams, so it holds none.
fn snap_seams(pieces: &[Piece<'_>]) -> Result<Vec<Scalar>, &'static str> {
    let mut out = Vec::new();
    let mut at = 0.0;
    for (index, piece) in pieces.iter().enumerate() {
        if index > 0 {
            out.push(at);
        }
        if !piece.is_stated() {
            break;
        }
        out.extend(kernel_cuts(piece)?.into_iter().map(|cut| at + cut));
        at += piece.length();
    }
    Ok(out)
}

/// The distances into `piece`, in its direction, of every seam of its
/// curve strictly inside it, smooth or not: the compiler splits an offset's
/// base there (`StationPiece::seams`, `axiolid_evaluate::station_seams2`
/// and `station_seams3`), so each offset piece's base spans no seam. They
/// are a polyline's vertices, an intrinsic curve's curvature (and torsion)
/// seams, a chain's joins and its intrinsic pieces' curvature seams, and an
/// elevated curve's plan seams (on an intrinsic or chain plan) and every
/// break of its profile, read from stored data as the kernel reads them.
/// A circle or an ellipse has none, nor has a B-spline without a corner
/// knot; one with a corner knot is refused before this
/// (`super::super::seams::SPLINE`).
fn kernel_cuts(piece: &Piece<'_>) -> Result<Vec<Scalar>, &'static str> {
    let (seams, length) = curve_cuts(piece.curve);
    let mut seams: Vec<Scalar> = seams
        .into_iter()
        .filter(|s| {
            s.is_finite()
                && *s > super::slack(0.0)
                && length.is_none_or(|l| *s < l - super::slack(l))
        })
        .collect();
    seams.sort_by(Scalar::total_cmp);
    seams.dedup_by(|b, a| *b - *a <= super::slack(*b));
    if !piece.is_stated() {
        // An open piece runs to its curve's end from its stated start: every
        // seam past that start is inside (#443; a whole curve's start is
        // `0`). A bounded one (#436) is an ellipse's or a B-spline's,
        // refused before this if it has a seam (`SPLINE`): it has none to
        // cut at.
        if piece.reversed || piece.start_bound.is_some() {
            return Ok(seams);
        }
        let from = piece.start + super::slack(piece.start);
        return Ok(seams.into_iter().filter(|s| *s > from).collect());
    }
    let mut cuts: Vec<Scalar> = seams
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
    Ok(cuts)
}

/// Every seam of `curve` in its station measure, unsorted, and its length
/// where stated (module documentation of [`kernel_cuts`]).
fn curve_cuts(curve: &AtomicCurve) -> (Vec<Scalar>, Option<Scalar>) {
    let length = crate::lower::station::seams::stated_length(curve);
    let seams = match curve {
        AtomicCurve::Two(Curve2::Polyline(polyline)) => {
            polyline_vertices(&polyline.points, polyline.closed, |a, b| (b - a).length())
        }
        AtomicCurve::Three(Curve3::Polyline(polyline)) => {
            polyline_vertices(&polyline.points, polyline.closed, |a, b| (b - a).length())
        }
        AtomicCurve::Two(plan) => plan_seams(plan),
        AtomicCurve::Three(Curve3::Intrinsic(intrinsic)) => {
            let mut seams = intrinsic.curvature.seams_within(intrinsic.length);
            seams.extend(intrinsic.torsion.seams_within(intrinsic.length));
            seams
        }
        AtomicCurve::Three(Curve3::Elevated(elevated)) => {
            let mut seams = plan_seams(&elevated.plan);
            if elevated.elevation.is_well_formed() {
                profile_breaks(&elevated.elevation, 0.0, &mut seams);
            }
            seams
        }
        _ => Vec::new(),
    };
    (seams, length)
}

/// The seams of a plan curve read by its intrinsic definition: an
/// intrinsic curve's curvature seams, a chain's joins and its intrinsic
/// pieces' curvature seams.
fn plan_seams(curve: &Curve2) -> Vec<Scalar> {
    match curve {
        Curve2::Intrinsic(intrinsic) => intrinsic.curvature.seams_within(intrinsic.length),
        Curve2::Chain(chain) => {
            let mut seams = chain.joins().unwrap_or_default();
            let mut begin = 0.0;
            for piece in &chain.pieces {
                if let ChainPiece2::Intrinsic { curvature, length } = piece {
                    seams.extend(
                        curvature
                            .seams_within(*length)
                            .into_iter()
                            .filter(|s| *s > 0.0 && s < length)
                            .map(|s| begin + s),
                    );
                }
                begin += piece.length();
            }
            seams
        }
        _ => Vec::new(),
    }
}

/// Every break of a piecewise profile, nested ones shifted by where their
/// piece starts.
fn profile_breaks(law: &ElevationLaw, offset: Scalar, out: &mut Vec<Scalar>) {
    if let ElevationLaw::Piecewise { breaks, laws } = law {
        for (index, piece) in laws.iter().enumerate() {
            let start = if index == 0 { 0.0 } else { breaks[index - 1] };
            profile_breaks(piece, offset + start, out);
        }
        out.extend(breaks.iter().map(|b| offset + b));
    }
}

/// A polyline's vertices where each non-degenerate edge starts after an
/// earlier non-degenerate one (`polyline_seams`), at their running sums.
fn polyline_vertices<P: Copy>(
    points: &[P],
    closed: bool,
    length: impl Fn(P, P) -> Scalar,
) -> Vec<Scalar> {
    let mut edges: Vec<Scalar> = points.windows(2).map(|w| length(w[0], w[1])).collect();
    if let (true, Some(first), Some(last)) = (closed, points.first(), points.last()) {
        edges.push(length(*last, *first));
    }
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
    vertices
}

/// The length of the offset by `law` of the span `piece`, holding no seam
/// of its curve, where stored data states it.
///
/// `Ok(None)` where it has no closed form (an offset beside a gradient
/// curve, a spiral, an ellipse or a B-spline, a variable offset of a
/// circle, a cusp): the piece is then open and the evaluator measures it
/// (#427), or refuses it by name.
fn offset_length(piece: &Piece<'_>, law: OffsetLaw) -> Result<Option<Scalar>, &'static str> {
    match closed_form_length(piece, law) {
        Ok(length) => Ok(Some(length)),
        Err(reason) if reason == INEXACT => Ok(None),
        Err(reason) => Err(reason),
    }
}

/// [`offset_length`], `INEXACT` where it has no closed form.
fn closed_form_length(piece: &Piece<'_>, law: OffsetLaw) -> Result<Scalar, &'static str> {
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
