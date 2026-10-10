//! Seams and length of a basis that lowers to a curve relation (#346,
//! #414), read from its stored data.
//!
//! Axiolid resolves a station along a curve relation since `axiolid-model`
//! 0.3.7 (axiolid/kernel#285, ADR 0082 amendment): a composite, a trim, a
//! surface curve whose 3D curve governs and a curve placed at a station,
//! nested in any combination, are flattened into spans of atomic curves laid
//! end to end, and the distance runs through them, each span in its own
//! curve's convention (plan distance when every span is elevated or banked,
//! arc length when none is; a mix is refused). Every interior joint is a
//! seam, smooth or not, and its `SeamSide` picks the piece. Since
//! `axiolid-mesh-compile` 0.3.19 (axiolid/kernel#289, the ADR's 2026-10-10
//! amendment) an offset is a basis too, flattened into offset pieces
//! measured in their own length ([`offset`]).
//!
//! That flattening lives in `axiolid-mesh-compile` and its pieces in
//! `axiolid-evaluate` (`station::composite`), execution providers this
//! crate does not link (ADR 0004). This module repeats the part a station
//! needs from stored data: the pieces' lengths, so the joints' distances,
//! and the seams of each piece's curve inside it. A station within the
//! model's precision of one is then snapped onto it and reads the incoming
//! piece, as on an atomic basis (`super::seams`). The reading follows the
//! compiler's (`station::basis::flatten`) rule for rule:
//!
//! - a composite's segments in order, a segment whose sense disagrees
//!   traversed backwards (its pieces reversed, in reverse order);
//! - an untrimmed line inside a composite is its parameter domain `[0, 1]`;
//! - a trim of an atomic curve between the parameters its selectors name:
//!   a parameter as is, an arc length from parameter `0` in the trim's
//!   sense, a point inverted onto a line or a circle (the selector the
//!   trimming preference picks); a closed conic from its start selector in
//!   the trim's sense, at most one turn; reversed where the sense
//!   disagrees;
//! - a trim of a relation between parameter selectors read as distances
//!   along it, which needs the relation measured in arc length; a trim
//!   whose basis is itself an offset is refused by name, as the kernel
//!   refuses it;
//! - a curve placed at a station, or a surface curve whose 3D curve
//!   governs, as its source: a rigid placement keeps the measure;
//! - an offset (`IfcOffsetCurve2D`, `IfcOffsetCurve3D`) as one offset piece
//!   per span of its basis's pieces between their seams, and an offset by
//!   distances (`IfcOffsetCurveByDistances`) likewise per interval between
//!   its stations ([`offset`]).
//!
//! A piece's length is read only where stored data states it exactly: a
//! line's `|direction|` times the parameter span, a circle's radius times
//! the angle, a polyline's edges, the parameter span of a curve
//! parameterised by its measure (an intrinsic curve, a chain, an elevated
//! curve's plan distance), and an offset's closed forms ([`offset`]). An
//! ellipse or a B-spline is measured by quadrature, which `axiolid-evaluate`
//! also reports inexact (`CompositeBasis::exact_seams`): for a composite
//! holding one the joints are unknown here ([`INEXACT`]), as an atomic
//! B-spline's corner knot is, and a station along it carries the kernel's
//! seam-snapping window instead (#423, `super`'s module documentation).
//!
//! The joints' points are not compared here: whether pieces meet is
//! evaluation. A gap, an undeclared reversed piece, or an offset across a
//! corner of its basis, whose two sides do not meet, is refused by name
//! when the kernel resolves the station.
//!
//! # The pieces themselves (#418)
//!
//! [`relation_run`] hands out the flattening: each piece's curve as stored,
//! its span in the curve's station measure, whether it runs backwards, and
//! the stations it is placed at, innermost first. A derived linear
//! placement (`constraint::placement::derive`) builds Axiolid's neutral
//! `CurvePath` from it, resolving those stations through the caller's
//! curve evaluator, which is the one step here that is evaluation. A closed
//! conic's trim is located as `axiolid-evaluate`'s
//! `StationPiece::between_parameters` locates it: from its start parameter
//! taken modulo one turn, so the span starts within the curve's measure.
//! An offset piece (#414) keeps the base span it offsets and its law
//! ([`offset::OffsetPiece`]), the neutral `PathOffset`'s two parts.

use axiolid_core::{Point3, Scalar};
use axiolid_curve::{Curve2, Curve3, OffsetLaw};
use axiolid_model::{
    CurveRelation, GeometryNode, MasterRepresentation, NodeId, OrientedCurveStation, TrimSelector,
    TrimmingPreference,
};

use super::seams::{tangent_seams, Seam, BANKED};
use crate::lower::session::{AtomicCurve, LoweringSession};

mod offset;

pub(crate) use offset::{OffsetPiece, COLLAPSE, OFFSET_OF_OFFSET, PLANAR_3D};

/// Why a relation basis's joints are unknown here: a piece's length is a
/// quadrature. A station along it carries the kernel's seam-snapping window
/// (#423); a run along it, and a derived placement on it (#427), are
/// refused.
pub(crate) const INEXACT: &str =
    "the basis curve relation holds a piece whose length is not stated by its data (an \
     ellipse, a B-spline, a trim whose ends are not a parameter, an arc length on a curve \
     parameterised by it, or a point on a line or a circle, or an offset other than one of a \
     line or a polyline, or a constant one of a circle in its own plane), so the distances of \
     its joints, where IFC4.3 ADD2 (8.9.3.48.3) reads the previous segment, cannot be located \
     within the model's precision";

/// Why a relation the kernel measures no station along is refused.
pub(crate) const UNSUPPORTED: &str =
    "the basis curve relation is a parameter-space curve, a surface curve whose p-curve \
     governs, or an instanced curve; Axiolid measures a station along a composite, a trim, a \
     surface curve whose 3D curve governs, a curve placed at a station and an offset only";

/// Why a trim of an offset curve is refused.
pub(crate) const TRIM_OF_OFFSET: &str =
    "the basis is a trim of an offset curve; an offset curve takes its parameterisation from \
     its basis curve (IfcOffsetCurve2D, IfcOffsetCurve3D), which is not its own length, and \
     Axiolid reads no station along that parameter";

/// Why a basis mixing plan-measured and arc-length pieces is refused.
pub(crate) const MIXED: &str =
    "the basis curve relation joins pieces measured in plan distance (a gradient curve) and \
     pieces measured in arc length, so no one distance runs through it";

/// Why a trim of a plan-measured relation is refused.
const PLAN_TRIM: &str =
    "the basis is a trim of a curve relation measured in plan distance; its parameter is its \
     arc length, not the plan distance it is measured in";

/// Most relations and placements a basis may be nested through, as the
/// kernel's own limit.
const MAX_DEPTH: usize = 64;

/// Relative tolerance within which a point selector lies on its curve
/// (Axiolid's `JOINT_TOLERANCE`, at the scale of the points named).
const POINT_TOLERANCE: Scalar = 1e-9;

/// Axiolid's arc-length tolerance: a joint closer than this to a distance
/// is on it, and a clipped piece shorter than it is dropped.
const ARC_LENGTH_TOLERANCE: Scalar = 1e-12;

fn slack(distance: Scalar) -> Scalar {
    ARC_LENGTH_TOLERANCE * distance.abs().max(1.0)
}

/// A relation basis's tangent seams (every interior joint among them) and
/// its length, or why they cannot be read.
pub(crate) fn relation_seams(
    session: &LoweringSession<'_>,
    node: NodeId,
) -> Result<(Vec<Seam>, Scalar), &'static str> {
    relation_run(session, node).map(|run| (run.seams, run.length))
}

/// A basis flattened into its pieces, with their seams and length.
#[derive(Debug, Clone)]
pub(crate) struct Run<'s> {
    /// The pieces, in the order the distance runs through them. Read by the
    /// derived placement along a relation, which needs `compile`; a
    /// lowering-only build (the facade's `geometry-wire`, #367) reads the
    /// seams and length alone.
    #[cfg_attr(not(feature = "compile"), allow(dead_code))]
    pub pieces: Vec<Piece<'s>>,
    /// Every interior joint and every seam of a piece's curve inside it,
    /// ascending.
    pub seams: Vec<Seam>,
    /// The sum of the pieces' lengths.
    pub length: Scalar,
    /// Whether the pieces are measured in plan distance.
    #[cfg_attr(not(feature = "compile"), allow(dead_code))]
    pub plan: bool,
}

/// The basis `node` flattened (module documentation): a curve relation into
/// its pieces, an atomic curve into one piece, whole (a line its parameter
/// domain `[0, 1]`), or why it cannot be.
pub(crate) fn relation_run<'s>(
    session: &'s LoweringSession<'_>,
    node: NodeId,
) -> Result<Run<'s>, &'static str> {
    let pieces = flatten(session, node, 0)?.into_pieces()?;
    let Some(first) = pieces.first() else {
        return Err(INEXACT);
    };
    let plan = first.plan;
    if pieces.iter().any(|piece| piece.plan != plan) {
        return Err(MIXED);
    }
    let mut seams = Vec::new();
    let mut at = 0.0;
    for (index, piece) in pieces.iter().enumerate() {
        if index > 0 {
            // Every joint is a seam to the kernel, never smooth.
            seams.push(Seam {
                distance: at,
                reverses: false,
            });
        }
        seams.extend(piece.inner_seams().map(|seam| Seam {
            distance: at + seam.distance,
            ..seam
        }));
        at += piece.length();
    }
    seams.sort_by(|a, b| a.distance.total_cmp(&b.distance));
    Ok(Run {
        pieces,
        seams,
        length: at,
        plan,
    })
}

/// A span `[start, end]` of an atomic curve in its station measure.
#[derive(Debug, Clone)]
pub(crate) struct Piece<'s> {
    /// The curve, as stored.
    pub curve: &'s AtomicCurve,
    /// Where the span starts, in the curve's station measure.
    pub start: Scalar,
    /// Where it ends.
    pub end: Scalar,
    /// Whether the distance runs from `end` to `start`.
    pub reversed: bool,
    /// Measured in plan distance (an elevated or banked curve).
    plan: bool,
    /// The curve's own seams, in its measure.
    seams: Vec<Seam>,
    /// The stations the curve is placed at, innermost first.
    pub placements: Vec<&'s OrientedCurveStation>,
    /// On an offset piece, the base span it offsets and its law; `curve`
    /// is then the base's, and `[start, end]` the offset's own measure
    /// `[0, L]`.
    pub offset: Option<Box<OffsetPiece<'s>>>,
}

impl Piece<'_> {
    fn length(&self) -> Scalar {
        self.end - self.start
    }

    fn reversed(mut self) -> Self {
        self.reversed = !self.reversed;
        self
    }

    /// The curve's seams strictly inside the piece, at their distance
    /// into it in the composite's direction.
    fn inner_seams(&self) -> impl Iterator<Item = Seam> + '_ {
        self.seams
            .iter()
            .filter(|seam| {
                seam.distance > self.start + slack(self.start)
                    && seam.distance < self.end - slack(self.end)
            })
            .map(|seam| Seam {
                distance: if self.reversed {
                    self.end - seam.distance
                } else {
                    seam.distance - self.start
                },
                ..*seam
            })
    }
}

/// A node flattened: one atomic curve, placed at the stations listed
/// (innermost first), or spans end to end.
enum Flat<'s> {
    Atomic(&'s AtomicCurve, Vec<&'s OrientedCurveStation>),
    Pieces(Vec<Piece<'s>>),
}

impl<'s> Flat<'s> {
    /// As pieces: an atomic curve whole (a line its domain `[0, 1]`).
    fn into_pieces(self) -> Result<Vec<Piece<'s>>, &'static str> {
        match self {
            Self::Atomic(curve, placements) => {
                let end = match curve {
                    AtomicCurve::Two(Curve2::Line(line)) => line.direction.length(),
                    AtomicCurve::Three(Curve3::Line(line)) => line.direction.length(),
                    _ => whole_length(curve).ok_or(INEXACT)?,
                };
                Ok(vec![piece(curve, 0.0, end, placements)?])
            }
            Self::Pieces(pieces) => Ok(pieces),
        }
    }
}

fn reversed(pieces: Vec<Piece<'_>>) -> Vec<Piece<'_>> {
    pieces.into_iter().rev().map(Piece::reversed).collect()
}

fn piece<'s>(
    curve: &'s AtomicCurve,
    start: Scalar,
    end: Scalar,
    placements: Vec<&'s OrientedCurveStation>,
) -> Result<Piece<'s>, &'static str> {
    let seams = tangent_seams(curve)?;
    let plan = matches!(
        curve,
        AtomicCurve::Three(Curve3::Elevated(_) | Curve3::Banked(_))
    );
    if matches!(curve, AtomicCurve::Three(Curve3::Banked(_))) {
        return Err(BANKED);
    }
    Ok(Piece {
        curve,
        start,
        end,
        reversed: false,
        plan,
        seams,
        placements,
        offset: None,
    })
}

fn flatten<'s>(
    session: &'s LoweringSession<'_>,
    node: NodeId,
    depth: usize,
) -> Result<Flat<'s>, &'static str> {
    if depth > MAX_DEPTH {
        return Err(UNSUPPORTED);
    }
    if let Some(curve) = session.atomic_curve(node) {
        return Ok(Flat::Atomic(curve, Vec::new()));
    }
    match session.relation(node) {
        // The source carried by the placement, applied after any it has.
        Some(GeometryNode::InstanceAtStation(placed)) => {
            Ok(match flatten(session, placed.source, depth + 1)? {
                Flat::Atomic(curve, mut placements) => {
                    placements.push(&placed.station);
                    Flat::Atomic(curve, placements)
                }
                Flat::Pieces(mut pieces) => {
                    for piece in &mut pieces {
                        piece.placements.push(&placed.station);
                    }
                    Flat::Pieces(pieces)
                }
            })
        }
        Some(GeometryNode::CurveRelation(CurveRelation::Composite { segments })) => {
            let mut out = Vec::new();
            for segment in segments {
                let pieces = flatten(session, segment.curve, depth + 1)?.into_pieces()?;
                out.extend(if segment.same_sense {
                    pieces
                } else {
                    reversed(pieces)
                });
            }
            Ok(Flat::Pieces(out))
        }
        Some(GeometryNode::CurveRelation(CurveRelation::Trimmed {
            basis,
            start,
            end,
            sense_agreement,
            preference,
        })) => {
            if matches!(
                session.relation(*basis),
                Some(GeometryNode::CurveRelation(
                    CurveRelation::Offset { .. } | CurveRelation::OffsetByStations { .. }
                ))
            ) {
                return Err(TRIM_OF_OFFSET);
            }
            let pieces = match flatten(session, *basis, depth + 1)? {
                Flat::Atomic(curve, placements) => {
                    let (lo, hi) =
                        trim_parameters(curve, start, end, *sense_agreement, *preference)?;
                    let (from, to) = measures_between(curve, lo, hi)?;
                    vec![piece(curve, from, to, placements)?]
                }
                Flat::Pieces(pieces) => {
                    if pieces.iter().any(|piece| piece.plan) {
                        return Err(PLAN_TRIM);
                    }
                    let a = parameter_only(start, *preference)?;
                    let b = parameter_only(end, *preference)?;
                    pieces_between(&pieces, a.min(b), a.max(b))?
                }
            };
            Ok(Flat::Pieces(if *sense_agreement {
                pieces
            } else {
                reversed(pieces)
            }))
        }
        Some(GeometryNode::CurveRelation(CurveRelation::SurfaceCurve {
            curve_3d,
            master: MasterRepresentation::Curve3d,
            ..
        })) => flatten(session, *curve_3d, depth + 1),
        Some(GeometryNode::CurveRelation(CurveRelation::Offset {
            basis,
            distance,
            reference_direction,
        })) => {
            let law = match reference_direction {
                None => OffsetLaw::Planar {
                    distance: *distance,
                },
                Some(direction) => OffsetLaw::Directed {
                    distance: *distance,
                    reference_direction: *direction,
                },
            };
            let base = flatten(session, *basis, depth + 1)?.into_pieces()?;
            Ok(Flat::Pieces(offset::offset_pieces(&base, law)?))
        }
        Some(GeometryNode::CurveRelation(CurveRelation::OffsetByStations {
            basis,
            stations,
            frame,
        })) => {
            let base = flatten(session, *basis, depth + 1)?;
            Ok(Flat::Pieces(offset::by_stations(base, stations, *frame)?))
        }
        _ => Err(UNSUPPORTED),
    }
}

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

/// The pieces between `start < end` along them, clipped, as
/// `CompositeBasis::pieces_between` keeps them.
fn pieces_between<'s>(
    pieces: &[Piece<'s>],
    start: Scalar,
    end: Scalar,
) -> Result<Vec<Piece<'s>>, &'static str> {
    let length: Scalar = pieces.iter().map(Piece::length).sum();
    if !(start.is_finite() && end.is_finite()) || end <= start {
        return Err(INEXACT);
    }
    let (start, end) = (start.max(0.0), end.min(length));
    let mut out = Vec::new();
    let mut lo = 0.0;
    for piece in pieces {
        let hi = lo + piece.length();
        let (a, b) = (start.max(lo) - lo, end.min(hi) - lo);
        if b - a > slack(b) {
            let mut clipped = piece.clone();
            if piece.reversed {
                clipped.start = piece.end - b;
                clipped.end = piece.end - a;
            } else {
                clipped.start = piece.start + a;
                clipped.end = piece.start + b;
            }
            out.push(clipped);
        }
        lo = hi;
    }
    Ok(out)
}

/// A whole curve's station length, where its data states it.
fn whole_length(curve: &AtomicCurve) -> Option<Scalar> {
    match curve {
        AtomicCurve::Two(Curve2::Ellipse(_) | Curve2::BSpline(_))
        | AtomicCurve::Three(Curve3::Ellipse(_) | Curve3::BSpline(_)) => None,
        _ => super::seams::stated_length(curve),
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
fn polyline_measure(points: &[Point3], closed: bool, t: Scalar) -> Result<Scalar, &'static str> {
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

/// The parameters `lo < hi` a trim of an atomic curve spans, as the
/// compiler reads them (`station::basis::trim_parameters`).
fn trim_parameters(
    curve: &AtomicCurve,
    start: &[TrimSelector],
    end: &[TrimSelector],
    sense: bool,
    preference: TrimmingPreference,
) -> Result<(Scalar, Scalar), &'static str> {
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
        return Ok(if sense { (a, a + span) } else { (a - span, a) });
    }
    Ok((a.min(b), a.max(b)))
}

/// One trim end read as a basis parameter (`directrix::parameter`).
fn trim_parameter(
    selectors: &[TrimSelector],
    preference: TrimmingPreference,
    curve: &AtomicCurve,
    sense: bool,
    tolerance: Scalar,
) -> Result<Scalar, &'static str> {
    let is_measure =
        |s: &&TrimSelector| matches!(s, TrimSelector::Parameter(_) | TrimSelector::ArcLength(_));
    let first_measure = || {
        selectors
            .iter()
            .find(is_measure)
            .map_or(Ok(None), |s| measured(s, curve, sense))
    };
    let invert = || {
        selectors.iter().find_map(|selector| match selector {
            TrimSelector::Point2(p) => invert(curve, Point3::new(p.x, p.y, 0.0), tolerance),
            TrimSelector::Point3(p) => invert(curve, *p, tolerance),
            _ => None,
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
    selected.filter(|t| t.is_finite()).ok_or(INEXACT)
}

/// A parameter or an arc length from parameter `0` in the trim's sense,
/// as a parameter, where the arc length is stated by the curve's data.
fn measured(
    selector: &TrimSelector,
    curve: &AtomicCurve,
    sense: bool,
) -> Result<Option<Scalar>, &'static str> {
    match selector {
        TrimSelector::Parameter(value) => Ok(Some(*value)),
        TrimSelector::ArcLength(length) => {
            let signed = if sense { *length } else { -*length };
            Ok(Some(match curve {
                AtomicCurve::Two(Curve2::Line(line)) => signed / line.direction.length(),
                AtomicCurve::Three(Curve3::Line(line)) => signed / line.direction.length(),
                AtomicCurve::Two(Curve2::Circle(circle)) => signed / circle.radius,
                AtomicCurve::Three(Curve3::Circle(circle)) => signed / circle.radius,
                AtomicCurve::Two(Curve2::Intrinsic(_) | Curve2::Chain(_))
                | AtomicCurve::Three(Curve3::Intrinsic(_)) => signed,
                _ => return Err(INEXACT),
            }))
        }
        _ => Ok(None),
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

/// A trim end of a relation basis: a parameter selector, read as a distance
/// along it (`directrix::parameter_only`).
fn parameter_only(
    selectors: &[TrimSelector],
    preference: TrimmingPreference,
) -> Result<Scalar, &'static str> {
    match preference {
        TrimmingPreference::Cartesian => Err(INEXACT),
        _ => match selectors
            .iter()
            .find(|s| matches!(s, TrimSelector::Parameter(_) | TrimSelector::ArcLength(_)))
        {
            Some(TrimSelector::Parameter(value)) if value.is_finite() => Ok(*value),
            _ => Err(INEXACT),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn polyline_measure_walks_edges() {
        let points = [
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(3.0, 0.0, 0.0),
            Point3::new(3.0, 4.0, 0.0),
        ];
        assert_eq!(polyline_measure(&points, false, 0.0), Ok(0.0));
        assert_eq!(polyline_measure(&points, false, 1.5), Ok(5.0));
        assert_eq!(polyline_measure(&points, false, 2.0), Ok(7.0));
        assert_eq!(polyline_measure(&points, true, 3.0), Ok(12.0));
        assert!(polyline_measure(&points, false, 2.5).is_err());
    }

    #[test]
    fn clipping_keeps_the_kernels_spans() {
        let line = AtomicCurve::Three(Curve3::Line(axiolid_curve::Line3 {
            origin: Point3::ZERO,
            direction: axiolid_core::Vec3::X,
        }));
        let piece = |start: Scalar, end: Scalar, reversed: bool| Piece {
            curve: &line,
            start,
            end,
            reversed,
            plan: false,
            seams: Vec::new(),
            placements: Vec::new(),
            offset: None,
        };
        let kept = pieces_between(&[piece(0.0, 4.0, false), piece(1.0, 7.0, true)], 2.0, 6.0)
            .expect("inside");
        assert_eq!(kept.len(), 2);
        assert_eq!((kept[0].start, kept[0].end), (2.0, 4.0));
        // Reversed: distances 0..2 into it are its measures 7..5.
        assert_eq!((kept[1].start, kept[1].end), (5.0, 7.0));
    }
}
