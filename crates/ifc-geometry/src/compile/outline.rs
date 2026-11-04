//! An arbitrary profile's outline with its arcs sampled by the backend (#660).
//!
//! [`crate::profile_outline`] is exact and kernel-free, and so refuses an
//! `IfcArcIndex`: a vertex list cannot state an arc. This is the sibling for a
//! caller that wants a polygon anyway and says how closely. Each arc segment
//! is lowered as the exact circle the file states and handed to a
//! [`PolylineCompiler`], which chord-samples it within the caller's
//! [`Tolerance`] and certifies how far the polygon may lie from the arc. No
//! code here samples an arc; the bridge only asks (ADR 0004).
//!
//! # What is exact and what is sampled
//!
//! Every authored point appears once, bitwise the file's value in metres,
//! and each sampled arc's two end vertices are the arc's authored end points
//! (an `IfcArcIndex` mid point is on the arc but not a ring vertex), not the
//! backend's evaluation of the circle there. The backend's own vertices
//! are kept only strictly between them. [`SampledArc`] records which
//! vertices those are and the bound for that arc, so a caller can tell the
//! edges that are approximate from the ones that are not.
//!
//! A boundary is compiled one segment at a time, never as the whole
//! composite curve: the backend would keep each earlier segment's *evaluated*
//! last vertex at a joint, and report no segment, so neither "authored points
//! are exact" nor "which edges are approximate" could be stated.
//!
//! A boundary without an arc is read as [`crate::profile_outline`] reads it,
//! with no arcs and a bound of zero.

use axiolid_contracts::ExecutionOptions;
use axiolid_model::{CurveRelation, GeometryNode};
use ifc_model::{EntityId, Model};

use super::{refused, ChordDeviation, PolylineCompiler, Tolerance};
use crate::curve::PolySegment;
use crate::error::{GeometryError, GeometryResult};
use crate::input::profile::{arc_boundary, boundary_curves, ring};
use crate::lower::{lower_curve_node, LoweringSession};
use crate::transform::Transform;
use crate::units;

#[cfg(feature = "compile-reference-backend")]
use super::default_backend;

/// An arbitrary profile's boundaries, arcs sampled.
///
/// Coordinates are in the profile's own 2D coordinate system, in metres.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct SampledProfileOutline {
    /// The `IfcProfileDef` entity.
    pub profile: EntityId,
    /// `OuterCurve`, sampled.
    pub outer: SampledRing,
    /// Each `InnerCurves` ring, in file order. Empty for an
    /// `IfcArbitraryClosedProfileDef`.
    pub inner: Vec<SampledRing>,
}

/// One boundary curve as a ring of vertices.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct SampledRing {
    /// The boundary curve the ring was read from.
    pub curve: EntityId,
    /// Profile 2D frame, metres, authored order, closing vertex not repeated.
    pub vertices: Vec<[f64; 2]>,
    /// One entry per `IfcArcIndex` segment, in segment order.
    pub arcs: Vec<SampledArc>,
    /// `requested` is `tolerance.linear()`; `bound` is the largest arc bound,
    /// zero when the ring has no arcs.
    pub deviation: ChordDeviation,
}

/// Where one sampled `IfcArcIndex` segment sits in its ring.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct SampledArc {
    /// 0-based index into the curve's `Segments`.
    pub segment: usize,
    /// Index in [`SampledRing::vertices`] of the arc's authored start point.
    pub start: usize,
    /// Sampled vertices after `start`; the arc ends at vertex
    /// `(start + samples + 1) % vertices.len()`, an authored point.
    pub samples: usize,
    /// The backend's certified bound for this arc plus the distance its end
    /// vertices were snapped, in metres.
    pub bound: f64,
}

/// Read an arbitrary profile's outline in metres, sampling each arc with
/// `backend` within `tolerance`.
///
/// Accepts `IfcArbitraryClosedProfileDef` and
/// `IfcArbitraryProfileDefWithVoids` whose boundaries are `IfcPolyline`s or
/// `IfcIndexedPolyCurve`s. Refused, each naming the entity:
///
/// - any other profile family, and any other boundary curve family, as
///   [`GeometryError::Unsupported`];
/// - what [`crate::profile_outline`] refuses as `Degenerate`: a 3D point, a
///   ring of fewer than three distinct vertices, segments that do not join,
///   an open indexed curve, and an `IfcArcIndex` of coincident points;
/// - a sampled arc vertex off the profile plane by more than the linear
///   tolerance, as [`GeometryError::Degenerate`];
/// - an arc the backend refuses, returns fewer than two vertices for,
///   runs backwards, or ends more than the linear tolerance from the
///   authored end points, as [`GeometryError::CompilationRefused`] naming the
///   boundary curve.
///
/// # Errors
///
/// As above, plus whatever [`crate::describe_profile`] and unit resolution
/// refuse.
pub fn compile_profile_outline_with<B: PolylineCompiler>(
    backend: &B,
    model: &Model,
    profile: EntityId,
    tolerance: Tolerance,
) -> GeometryResult<SampledProfileOutline> {
    let scale = units::resolve(model)?;
    let (outer, inner) = boundary_curves(model, &scale, profile)?;
    let sample = |curve: EntityId| sampled_ring(backend, model, &scale, curve, tolerance);
    Ok(SampledProfileOutline {
        profile,
        outer: sample(outer)?,
        inner: inner
            .into_iter()
            .map(sample)
            .collect::<GeometryResult<_>>()?,
    })
}

/// [`compile_profile_outline_with`] with [`default_backend`].
///
/// # Errors
///
/// As [`compile_profile_outline_with`].
#[cfg(feature = "compile-reference-backend")]
pub fn compile_profile_outline(
    model: &Model,
    profile: EntityId,
    tolerance: Tolerance,
) -> GeometryResult<SampledProfileOutline> {
    compile_profile_outline_with(&default_backend(), model, profile, tolerance)
}

fn sampled_ring<B: PolylineCompiler>(
    backend: &B,
    model: &Model,
    scale: &units::UnitScale,
    curve: EntityId,
    tolerance: Tolerance,
) -> GeometryResult<SampledRing> {
    let none = ChordDeviation::new(tolerance.linear(), 0.0);
    let entity = model.get(curve).ok_or(GeometryError::MissingEntity {
        referrer: curve,
        missing: curve,
    })?;
    let boundary = if entity.type_name.eq_ignore_ascii_case("IFCINDEXEDPOLYCURVE") {
        arc_boundary(model, curve, entity)?
    } else {
        None
    };
    let Some(boundary) = boundary else {
        return Ok(SampledRing {
            curve,
            vertices: ring(model, scale, curve)?,
            arcs: Vec::new(),
            deviation: none,
        });
    };

    let mut session = LoweringSession::new(model, scale);
    let node = lower_curve_node(&mut session, curve, Transform::identity())?;
    let children: Vec<_> = match session.relation(node) {
        Some(GeometryNode::CurveRelation(CurveRelation::Composite { segments }))
            if segments.len() == boundary.segments.len() =>
        {
            segments.iter().map(|segment| segment.curve).collect()
        }
        _ => {
            return Err(GeometryError::Unsupported {
                entity: curve,
                type_name: entity.type_name.to_ascii_uppercase(),
                detail: "the lowered boundary is not one node per segment",
            })
        }
    };
    let lowered = session.finish(node)?;
    let options = ExecutionOptions::new(tolerance);
    let metres = |index: usize| {
        let [x, y] = boundary.list[index];
        [scale.length(x), scale.length(y)]
    };

    let mut vertices: Vec<[f64; 2]> = Vec::new();
    let mut arcs = Vec::new();
    let mut bound = 0.0_f64;
    for (segment, (part, child)) in boundary.segments.iter().zip(children).enumerate() {
        match part {
            PolySegment::Line(run) => {
                if vertices.is_empty() {
                    vertices.push(metres(run[0]));
                }
                vertices.extend(run[1..].iter().map(|&index| metres(index)));
            }
            PolySegment::Arc { start, end, .. } => {
                if vertices.is_empty() {
                    vertices.push(metres(*start));
                }
                let outcome = backend
                    .compile_polyline(&lowered.graph, child, &options)
                    .map_err(|error| refused(curve, error, &lowered.provenance))?;
                let points = &outcome.points;
                if points.len() < 2 {
                    return Err(GeometryError::CompilationRefused {
                        entity: curve,
                        reason: "the backend returned fewer than two vertices for an arc"
                            .to_owned(),
                    });
                }
                let authored_start = metres(*start);
                let authored_end = metres(*end);
                let to = |p: &super::Point3, q: [f64; 2]| (p.x - q[0]).hypot(p.y - q[1]).hypot(p.z);
                let (first, last) = (&points[0], &points[points.len() - 1]);
                if to(first, authored_end) < to(first, authored_start) {
                    return Err(GeometryError::CompilationRefused {
                        entity: curve,
                        reason: "the backend's arc polyline runs from the arc's end to its start"
                            .to_owned(),
                    });
                }
                let snapped = to(first, authored_start).max(to(last, authored_end));
                if snapped.is_nan() || snapped > tolerance.linear() {
                    return Err(GeometryError::CompilationRefused {
                        entity: curve,
                        reason: format!(
                            "the backend's arc ends are {snapped} m from the authored end \
                             points, beyond the linear tolerance {}",
                            tolerance.linear()
                        ),
                    });
                }
                let samples = &points[1..points.len() - 1];
                if let Some(off) = samples.iter().find(|p| p.z.abs() > tolerance.linear()) {
                    return Err(GeometryError::Degenerate {
                        entity: curve,
                        type_name: entity.type_name.to_ascii_uppercase(),
                        detail: format!(
                            "a sampled arc vertex leaves the profile plane (z = {})",
                            off.z
                        ),
                    });
                }
                let arc_bound = outcome.deviation.bound + snapped;
                arcs.push(SampledArc {
                    segment,
                    start: vertices.len() - 1,
                    samples: samples.len(),
                    bound: arc_bound,
                });
                bound = bound.max(arc_bound);
                vertices.extend(samples.iter().map(|p| [p.x, p.y]));
                // The authored end, not the backend's evaluation there.
                vertices.push(metres(*end));
            }
        }
    }
    // The boundary closes by index: its last vertex is its first.
    vertices.pop();
    if vertices.len() < 3 {
        return Err(GeometryError::Degenerate {
            entity: curve,
            type_name: entity.type_name.to_ascii_uppercase(),
            detail: format!(
                "a profile boundary needs at least 3 distinct vertices, found {}",
                vertices.len()
            ),
        });
    }
    Ok(SampledRing {
        curve,
        vertices,
        arcs,
        deviation: ChordDeviation::new(tolerance.linear(), bound),
    })
}
