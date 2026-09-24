//! A sweep's `StartParam`/`EndParam` over an `IfcCompositeCurve` directrix.
//!
//! # The composite's own parameter
//!
//! ISO 10303-42 (IFC4 `IfcCompositeCurve`, Figure 389) parameterises a
//! composite by accumulating each segment's PARAMETRIC length: an
//! `IfcPolyline` counts 1 per edge, a trimmed conic counts its angle span in
//! the file's plane-angle unit, a trimmed line its parameter span, and an
//! `IfcReparametrisedCompositeCurveSegment` its `ParamLength`. Revit writes
//! `365 = 5 x 1 + 4 x 90 degrees` for a pipe a few metres long.
//!
//! The kernel's `parameter_range` on a composite is arc length over its
//! flattened path, a different and tolerance-dependent quantity. Mapping one
//! onto the other would be approximate, so this module never hands the kernel
//! a composite range at all. It resolves the range structurally, in raw file
//! parameters (the range and every segment span share them):
//!
//! - the whole composite: the authored directrix, unchanged;
//! - a partial range: a new composite of the covered segments, the first and
//!   last cut at their own parent parameter.
//!
//! Only what the range needs is read. A segment whose parametric length is not
//! stated exactly (a point-trimmed arc, a spline, an indexed poly-curve) is
//! refused by name when a range is given, never guessed.

use std::f64::consts::TAU;

use axiolid_core::Point3;
use axiolid_curve::{Curve3, Polyline3};
use axiolid_model::{
    CurveRelation, CurveSegment, GeometryNode, NodeId, Transition, TrimSelector,
    TrimmingPreference as KernelPreference,
};
use ifc_model::EntityId;

use super::{lower_curve_node, scale_parameter, transition, world_point};
use crate::curve::composite::{CompositeCurve, CompositeCurveSegment};
use crate::curve::polyline::Polyline;
use crate::curve::trimmed::TrimmedCurve;
use crate::error::GeometryResult;
use crate::lower::session::LoweringSession;
use crate::transform::Transform;

/// Nesting depth past which a composite-of-composites is refused.
const MAX_NESTING: usize = 32;

/// Relative slack when comparing a range against the parametric length.
///
/// Exporters write the full range as a decimal sum (`365.00000000000034`
/// against a computed `365.0000000000018`); this absorbs that rounding and
/// nothing an author could mean.
const RELATIVE_SLACK: f64 = 1e-9;

/// Is `kind` one of the composite families lowered as a `Composite`?
pub(crate) fn is_composite(kind: &str) -> bool {
    matches!(
        kind,
        "IFCCOMPOSITECURVE"
            | "IFCCOMPOSITECURVEONSURFACE"
            | "IFCBOUNDARYCURVE"
            | "IFCOUTERBOUNDARYCURVE"
    )
}

/// Lower a composite sweep directrix with the range already applied.
///
/// `range` is `(StartParam, EndParam)` exactly as authored. The returned
/// node needs no further `parameter_range`.
pub(crate) fn lower_composite_directrix(
    session: &mut LoweringSession<'_>,
    sweep: EntityId,
    sweep_type: &str,
    directrix: EntityId,
    frame: Transform,
    range: Option<(f64, f64)>,
) -> GeometryResult<NodeId> {
    let Some((start, end)) = range else {
        return lower_curve_node(session, directrix, frame);
    };
    let segments = segments(session, sweep, sweep_type, directrix, 0)?;
    let total: f64 = segments
        .iter()
        .map(|segment| segment.composite_length)
        .sum();
    let slack = RELATIVE_SLACK * total.max(1.0);
    let (lo, hi) = (start.min(end), start.max(end));

    if !(lo.is_finite() && hi.is_finite()) || lo < -slack || hi > total + slack {
        return Err(session.degenerate(
            sweep,
            sweep_type,
            format!(
                "StartParam/EndParam ({start}, {end}) exceed the directrix's parametric \
                 length {total}; the range is in the composite's own parameter \
                 (1 per polyline edge, the angle of each arc), not a length"
            ),
        ));
    }
    if hi - lo <= slack {
        return Err(session.degenerate(
            sweep,
            sweep_type,
            format!("StartParam/EndParam ({start}, {end}) select an empty sweep"),
        ));
    }
    if lo <= slack && hi >= total - slack {
        return lower_curve_node(session, directrix, frame);
    }

    let mut kept = Vec::new();
    let mut offset = 0.0;
    for segment in &segments {
        let (from, to) = (offset, offset + segment.composite_length);
        offset = to;
        let (cut_lo, cut_hi) = (lo.max(from), hi.min(to));
        if cut_hi - cut_lo <= slack {
            continue;
        }
        if cut_lo <= from + slack && cut_hi >= to - slack {
            kept.push(CurveSegment {
                curve: lower_curve_node(session, segment.parent, frame)?,
                same_sense: segment.same_sense,
                transition: segment.transition,
            });
            continue;
        }
        // Composite-local offsets, rescaled to the parent's own parameter
        // (they differ only on a reparametrised segment), then read in the
        // parent's traversal direction.
        let scale = segment.native_length / segment.composite_length;
        let (u0, u1) = ((cut_lo - from) * scale, (cut_hi - from) * scale);
        let (v0, v1) = if segment.same_sense {
            (u0, u1)
        } else {
            (segment.native_length - u1, segment.native_length - u0)
        };
        let mut pieces = cut(session, sweep, sweep_type, segment, frame, v0, v1)?;
        // Pieces come in the parent's order; a reversed segment walks them
        // backwards, each piece itself reversed by `same_sense`.
        if !segment.same_sense {
            pieces.reverse();
        }
        let Some(last) = pieces.len().checked_sub(1) else {
            // `cut_hi - cut_lo > slack` was checked above, so a cut always
            // spans geometry; an empty cut would be a bug here, not a file.
            return Err(session.degenerate(
                segment.parent,
                "IFCCOMPOSITECURVESEGMENT",
                "a sweep range cut this segment to nothing",
            ));
        };
        for (index, curve) in pieces.into_iter().enumerate() {
            kept.push(CurveSegment {
                curve,
                same_sense: segment.same_sense,
                transition: if index == last {
                    segment.transition
                } else {
                    Transition::Continuous
                },
            });
        }
    }
    session.node_for(
        directrix,
        GeometryNode::CurveRelation(CurveRelation::Composite { segments: kept }),
    )
}

/// One composite segment, with its parametric length resolved.
struct Segment {
    parent: EntityId,
    same_sense: bool,
    transition: Transition,
    /// The parent curve's own parametric span.
    native_length: f64,
    /// What the segment contributes to the composite's parameter:
    /// `ParamLength` on a reparametrised segment, else `native_length`.
    composite_length: f64,
    piece: Piece,
}

/// How a parent curve's parameter maps onto geometry.
enum Piece {
    /// `IfcPolyline`: parameter `i` is point `i`, as authored.
    Polyline { points: Vec<EntityId> },
    /// `IfcTrimmedCurve` with parameter trims on both ends.
    Trimmed {
        basis: EntityId,
        basis_kind: String,
        /// Raw parameter where the parent's own traversal starts: `Trim1`
        /// on a conic, the sense-selected end of the sorted trims on a line.
        start: f64,
        /// `+1` when the parent runs with the basis parameter, `-1` against.
        direction: f64,
        /// Raw period of a closed conic basis; `None` for a line.
        period: Option<f64>,
    },
    /// A composite nested as a segment's parent.
    Nested,
}

fn segments(
    session: &mut LoweringSession<'_>,
    sweep: EntityId,
    sweep_type: &str,
    composite: EntityId,
    depth: usize,
) -> GeometryResult<Vec<Segment>> {
    if depth > MAX_NESTING {
        return Err(session.unsupported(
            sweep,
            sweep_type,
            "a sweep range over composites nested this deeply",
        ));
    }
    let entity = session.entity(sweep, composite)?;
    let refs = CompositeCurve::new(composite, entity).segment_refs()?;
    let mut out = Vec::with_capacity(refs.len());
    for segment_ref in refs {
        let entity = session.entity(sweep, segment_ref)?;
        let view = CompositeCurveSegment::new(segment_ref, entity);
        let parent = view.parent_curve_ref()?;
        let (native_length, piece) = parametric_span(session, sweep, sweep_type, parent, depth)?;
        let composite_length = view.param_length()?.unwrap_or(native_length);
        out.push(Segment {
            parent,
            same_sense: view.same_sense()?,
            transition: transition(view.transition()?),
            native_length,
            composite_length,
            piece,
        });
    }
    Ok(out)
}

/// The parent curve's parametric length, in raw file parameters.
fn parametric_span(
    session: &mut LoweringSession<'_>,
    sweep: EntityId,
    sweep_type: &str,
    parent: EntityId,
    depth: usize,
) -> GeometryResult<(f64, Piece)> {
    let kind = session.type_name(parent)?;
    match kind.as_str() {
        "IFCPOLYLINE" => {
            let entity = session.entity(sweep, parent)?;
            let points = Polyline::new(parent, entity).point_refs()?;
            Ok(((points.len() - 1) as f64, Piece::Polyline { points }))
        }
        "IFCTRIMMEDCURVE" => trimmed_span(session, sweep, sweep_type, parent),
        nested if is_composite(nested) => {
            let inner = segments(session, sweep, sweep_type, parent, depth + 1)?;
            let length = inner.iter().map(|segment| segment.composite_length).sum();
            Ok((length, Piece::Nested))
        }
        _ => Err(session.unsupported(
            sweep,
            sweep_type,
            "a sweep range over a composite segment whose parent is not a polyline, a \
             parameter-trimmed curve or a composite: its parametric length is not read",
        )),
    }
}

fn trimmed_span(
    session: &mut LoweringSession<'_>,
    sweep: EntityId,
    sweep_type: &str,
    parent: EntityId,
) -> GeometryResult<(f64, Piece)> {
    let entity = session.entity(sweep, parent)?;
    let view = TrimmedCurve::new(parent, entity);
    let basis = view.basis_curve_ref()?;
    let basis_kind = session.type_name(basis)?;
    let sense = view.sense_agreement()?;
    let (trim1, trim2) = view.spec()?.endpoints();
    let (Some(t1), Some(t2)) = (trim1.parameter, trim2.parameter) else {
        return Err(session.unsupported(
            sweep,
            sweep_type,
            "a sweep range over a composite segment trimmed only by points: its \
             parametric length is defined by parameter trims the file does not state",
        ));
    };
    let direction = if sense { 1.0 } else { -1.0 };
    let travelled = direction * (t2 - t1);
    let (span, period) = match basis_kind.as_str() {
        "IFCCIRCLE" | "IFCELLIPSE" => {
            // One turn in the file's own angle unit.
            let period = TAU / session.units().angle(1.0);
            let span = if travelled > 0.0 && travelled <= period * (1.0 + RELATIVE_SLACK) {
                travelled
            } else {
                travelled.rem_euclid(period)
            };
            (span, Some(period))
        }
        "IFCLINE" => (travelled.abs(), None),
        _ => {
            return Err(session.unsupported(
                sweep,
                sweep_type,
                "a sweep range over a trimmed composite segment whose basis is not a \
                 line or conic",
            ))
        }
    };
    if !(span.is_finite() && span > 0.0) {
        return Err(session.degenerate(
            parent,
            "IFCTRIMMEDCURVE",
            format!("trims ({t1}, {t2}) span no parameter"),
        ));
    }
    // A conic runs from Trim1 the way `SenseAgreement` says, wrapping past
    // the seam if it must. A line has no seam: an uncut trimmed line is
    // drawn over its sorted trims, reversed iff `SenseAgreement` is false, so
    // a cut piece reads it the same way or the two would disagree.
    let (start, direction) = match period {
        Some(_) => (t1, direction),
        None if sense => (t1.min(t2), 1.0),
        None => (t1.max(t2), -1.0),
    };
    Ok((
        span,
        Piece::Trimmed {
            basis,
            basis_kind,
            start,
            direction,
            period,
        },
    ))
}

/// The parent's sub-curve between its own parameters `v0 < v1`.
///
/// Returned in the parent's traversal order. A periodic arc that crosses the
/// basis seam is returned as two pieces so no piece wraps.
fn cut(
    session: &mut LoweringSession<'_>,
    sweep: EntityId,
    sweep_type: &str,
    segment: &Segment,
    frame: Transform,
    v0: f64,
    v1: f64,
) -> GeometryResult<Vec<NodeId>> {
    match &segment.piece {
        Piece::Polyline { points } => {
            let edges = points.len() - 1;
            let at = |session: &mut LoweringSession<'_>, v: f64| -> GeometryResult<Point3> {
                let index = (v.floor() as usize).min(edges - 1);
                let t = v - index as f64;
                let a = world_point(session, segment.parent, points[index], frame)?;
                let b = world_point(session, segment.parent, points[index + 1], frame)?;
                Ok(Point3::from_array([
                    a[0] + (b[0] - a[0]) * t,
                    a[1] + (b[1] - a[1]) * t,
                    a[2] + (b[2] - a[2]) * t,
                ]))
            };
            let mut path = vec![at(session, v0)?];
            let first_vertex = v0.floor() as usize + 1;
            for (index, point) in points.iter().enumerate().skip(first_vertex) {
                if index as f64 >= v1 {
                    break;
                }
                path.push(Point3::from_array(world_point(
                    session,
                    segment.parent,
                    *point,
                    frame,
                )?));
            }
            path.push(at(session, v1)?);
            path.dedup();
            let node = session.node_for(
                segment.parent,
                GeometryNode::Curve3(Curve3::Polyline(Polyline3 {
                    points: path,
                    closed: false,
                })),
            )?;
            Ok(vec![node])
        }
        Piece::Trimmed {
            basis,
            basis_kind,
            start,
            direction,
            period,
        } => {
            let (q0, q1) = (start + direction * v0, start + direction * v1);
            let ranges = match period {
                None => vec![(q0, q1)],
                Some(period) => unwrapped(q0, q1, *period),
            };
            let basis_node = lower_curve_node(session, *basis, frame)?;
            // The kernel samples a trimmed basis over the sorted interval and
            // reverses when `sense_agreement` is false, so a piece's sense is
            // exactly its traversal direction along the basis parameter.
            let sense = *direction > 0.0;
            ranges
                .into_iter()
                .map(|(a, b)| {
                    let parameter = |raw| {
                        vec![TrimSelector::Parameter(scale_parameter(
                            session, basis_kind, raw,
                        ))]
                    };
                    let relation = CurveRelation::Trimmed {
                        basis: basis_node,
                        start: parameter(a),
                        end: parameter(b),
                        sense_agreement: sense,
                        preference: KernelPreference::Parameter,
                    };
                    session.node_for(segment.parent, GeometryNode::CurveRelation(relation))
                })
                .collect()
        }
        Piece::Nested => Err(session.unsupported(
            sweep,
            sweep_type,
            "a sweep range that ends inside a nested composite segment",
        )),
    }
}

/// Split a periodic interval traversed from `q0` to `q1` at the basis seam.
///
/// `[0, period]` is the basis domain; a piece never wraps past it, because a
/// trimmed conic read as `start > end` is ambiguous about which way it runs.
/// A piece that merely touches the seam is kept whole, and the empty sliver
/// on the far side of the seam is not emitted: the kernel refuses an empty
/// trimmed interval, and it carries no geometry.
fn unwrapped(q0: f64, q1: f64, period: f64) -> Vec<(f64, f64)> {
    let travelled = q1 - q0;
    let seam_slack = RELATIVE_SLACK * period;
    // Anchor a start that sits on the seam on the side it travels INTO, so a
    // backwards piece starting at 0 is read as starting at `period`.
    let mut a = q0.rem_euclid(period);
    if travelled < 0.0 && a <= seam_slack {
        a = period;
    } else if travelled > 0.0 && a >= period - seam_slack {
        a = 0.0;
    }
    let b = a + travelled;
    let pieces = if (-seam_slack..=period + seam_slack).contains(&b) {
        vec![(a, b.clamp(0.0, period))]
    } else if b > period {
        vec![(a, period), (0.0, b - period)]
    } else {
        vec![(a, 0.0), (period, b + period)]
    };
    pieces
        .into_iter()
        .filter(|(from, to)| (to - from).abs() > seam_slack)
        .collect()
}
