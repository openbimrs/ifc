//! Intersecting curved grid axes through a caller-supplied evaluator
//! (#362), feature `compile`.
//!
//! A circle, an ellipse, a bent polyline, an arc of a conic, or an
//! `IfcOffsetCurve2D` of one of them has no closed-form intersection with
//! an arbitrary other axis that this crate should own. The caller's
//! [`CurveEvaluator`] supplies points and unit tangents at a curve's native
//! parameter -- the one measure every curve family admits -- and this
//! module solves for the intersection of the two offset curves by Newton
//! iteration from seeds sampled along each axis.
//!
//! # What is solved
//!
//! An axis curve `C(u)` with unit tangent `T(u)` and the grid offset `o`
//! (to the left of the axis, reverted by `SameSense`) is the curve
//! `V(u) = C(u) + k * left(T(u))`, where `k` folds the axis's offset, any
//! `IfcOffsetCurve2D` distances under it, and the signs of the senses
//! between them. A straight partner is `q + v * d`. The intersection is
//! the root of `V_1(u) - V_2(v)`, searched only within each curved axis's
//! parameter range: a full turn for a circle or ellipse, the trim range for
//! an `IfcTrimmedCurve`, `[0, n - 1]` for an `n`-point polyline.
//!
//! IFC's `IntersectingAxes` are "two grid axes which intersects at exactly
//! one intersection"; finding none within the range, or two distinct ones,
//! is [`GeometryError::GridAxesDoNotIntersect`] rather than a pick.

use std::f64::consts::TAU;

use axiolid_curve::Curve3;
use axiolid_curve_evaluate_contract::{CurveEvaluator, CurveMeasure};
use axiolid_model::GeometryNode;
use ifc_model::{EntityId, Model};

use super::resolve::{left_of, normalise, AxisInput, StraightAxis};
use crate::constraint::tolerance::model_precision;
use crate::curve::offset::OffsetCurve2D;
use crate::curve::trimmed::TrimmedCurve;
use crate::error::{GeometryError, GeometryResult};
use crate::lower::curve::lower_curve_node;
use crate::lower::session::LoweringSession;
use crate::transform::Transform;
use crate::units::UnitScale;

/// Samples taken along a curved axis to seed the iteration.
const SAMPLES: usize = 64;
/// Seeds actually iterated from, the closest pairs first.
const SEEDS: usize = 12;
/// Newton steps per seed.
const ITERATIONS: usize = 60;

/// One axis as the solver sees it, in metres.
enum Track {
    /// `q + v * d`, in the axis's grid sense, already offset.
    Straight {
        point: [f64; 2],
        direction: [f64; 2],
    },
    /// `C(u) + k * left(T(u))` over `[lo, hi]`.
    Curved {
        curve: Box<Curve3>,
        lo: f64,
        hi: f64,
        periodic: bool,
        /// Coefficient of `left(T)`, where `T` follows the parameter.
        k: f64,
        /// The axis's grid sense relative to the parameter direction.
        sense: f64,
    },
}

/// A curved axis curve read down to its evaluable carrier.
struct Carrier {
    curve: Curve3,
    lo: f64,
    hi: f64,
    periodic: bool,
    /// The IFC curve's sense relative to the carrier's parameter.
    sense: f64,
    /// `IfcOffsetCurve2D` distances, as a coefficient of `left(T)`.
    offset: f64,
}

/// Intersect two axes of which at least one is curved; the point in FILE
/// units and the first axis's unit tangent there.
pub(super) fn intersect(
    model: &Model,
    units: &UnitScale,
    id: EntityId,
    inputs: &[AxisInput; 2],
    straight: [Option<StraightAxis>; 2],
    evaluator: &dyn CurveEvaluator,
) -> GeometryResult<([f64; 2], [f64; 2])> {
    let scale = units.length_to_metres;
    if !(scale.is_finite() && scale > 0.0) {
        return Err(GeometryError::Units(format!(
            "length factor {scale} cannot place a grid intersection"
        )));
    }
    let tracks = [
        track(model, units, &inputs[0], straight[0])?,
        track(model, units, &inputs[1], straight[1])?,
    ];
    let precision = units.length(model_precision(model)?);
    let refuse = |detail: &'static str| GeometryError::GridAxesDoNotIntersect {
        intersection: id,
        axes: [inputs[0].axis, inputs[1].axis],
        detail,
    };
    let evaluate = |error| GeometryError::Unsupported {
        entity: id,
        type_name: "IFCVIRTUALGRIDINTERSECTION".into(),
        detail: evaluator_refusal(&error),
    };

    let mut found: Vec<([f64; 2], f64)> = Vec::new();
    for (u, v) in seeds(&tracks, evaluator).map_err(evaluate)? {
        let Some((u, point)) = newton(&tracks, evaluator, u, v).map_err(evaluate)? else {
            continue;
        };
        let duplicate = found.iter().any(|(other, _)| {
            let gap = ((other[0] - point[0]).powi(2) + (other[1] - point[1]).powi(2)).sqrt();
            gap <= precision.max(1e-7 * point[0].abs().max(point[1].abs()).max(1.0))
        });
        if !duplicate {
            found.push((point, u));
        }
    }
    let (point, u) = match found[..] {
        [] => return Err(refuse("do not meet within their extent")),
        [only] => only,
        _ => {
            return Err(refuse(
                "meet more than once, where IfcVirtualGridIntersection requires exactly one \
                 intersection",
            ))
        }
    };
    let tangent = axis_tangent(&tracks[0], evaluator, u).map_err(evaluate)?;
    Ok(([point[0] / scale, point[1] / scale], tangent))
}

/// Build the solver's view of one axis.
fn track(
    model: &Model,
    units: &UnitScale,
    input: &AxisInput,
    straight: Option<StraightAxis>,
) -> GeometryResult<Track> {
    let axis_sense = if input.same_sense { 1.0 } else { -1.0 };
    let offset = units.length(input.offset);
    if let Some(line) = straight {
        let direction = line.direction.map(|v| v * axis_sense);
        let left = left_of(direction);
        return Ok(Track::Straight {
            point: [
                units.length(line.point[0]) + offset * left[0],
                units.length(line.point[1]) + offset * left[1],
            ],
            direction,
        });
    }
    let carrier = carrier(model, units, input.curve)?;
    // The axis runs along the IFC curve's sense, reverted by SameSense; its
    // offset is to the left of that, so left(sense * T) = sense * left(T).
    let sense = carrier.sense * axis_sense;
    Ok(Track::Curved {
        curve: Box::new(carrier.curve),
        lo: carrier.lo,
        hi: carrier.hi,
        periodic: carrier.periodic,
        k: carrier.offset + offset * sense,
        sense,
    })
}

/// Read a curved axis curve down to an evaluable carrier and its range.
fn carrier(model: &Model, units: &UnitScale, curve: EntityId) -> GeometryResult<Carrier> {
    let entity = model.get(curve).ok_or(GeometryError::MissingEntity {
        referrer: curve,
        missing: curve,
    })?;
    let unsupported = |detail: &'static str| GeometryError::Unsupported {
        entity: curve,
        type_name: entity.type_name.to_string(),
        detail,
    };
    match entity.type_name.as_ref() {
        "IFCCIRCLE" | "IFCELLIPSE" => Ok(Carrier {
            curve: lower(model, units, curve)?,
            lo: 0.0,
            hi: TAU,
            periodic: true,
            sense: 1.0,
            offset: 0.0,
        }),
        "IFCPOLYLINE" => {
            let points = crate::curve::polyline::Polyline::new(curve, entity).point_refs()?;
            Ok(Carrier {
                curve: lower(model, units, curve)?,
                lo: 0.0,
                hi: (points.len() - 1) as f64,
                periodic: false,
                sense: 1.0,
                offset: 0.0,
            })
        }
        "IFCTRIMMEDCURVE" => {
            let view = TrimmedCurve::new(curve, entity);
            let basis = view.basis_curve_ref()?;
            let basis_kind = model
                .get(basis)
                .map(|e| e.type_name.to_string())
                .unwrap_or_default();
            if !matches!(basis_kind.as_str(), "IFCCIRCLE" | "IFCELLIPSE") {
                return Err(unsupported(
                    "a trimmed grid axis is intersected only on a line, circle or ellipse",
                ));
            }
            let (Some(t1), Some(t2)) = (view.trim1()?.parameter, view.trim2()?.parameter) else {
                return Err(unsupported(
                    "a trimmed curved grid axis needs parameter trims to bound the search \
                     for its intersection",
                ));
            };
            let [t1, t2] = [units.angle(t1), units.angle(t2)];
            let agrees = view.sense_agreement()?;
            // ISO 10303-42: with SenseAgreement the arc runs from Trim1 the
            // positive way round to Trim2; without, the negative way.
            let (lo, sweep) = if agrees {
                (t1, (t2 - t1).rem_euclid(TAU))
            } else {
                (t2, (t1 - t2).rem_euclid(TAU))
            };
            let sweep = if sweep == 0.0 { TAU } else { sweep };
            Ok(Carrier {
                curve: lower(model, units, basis)?,
                lo,
                hi: lo + sweep,
                periodic: false,
                sense: if agrees { 1.0 } else { -1.0 },
                offset: 0.0,
            })
        }
        "IFCOFFSETCURVE2D" => {
            let view = OffsetCurve2D::new(curve, entity);
            let basis = view.basis_curve_ref()?;
            let mut inner = carrier(model, units, basis)?;
            // Offset to the left of the basis's own sense.
            inner.offset += inner.sense * units.length(view.distance()?);
            Ok(inner)
        }
        _ => Err(unsupported(
            "this curve family has no parameter range this bridge can state, so it cannot \
             be intersected as a grid axis",
        )),
    }
}

/// Lower one curve to its neutral `Curve3`, in metres.
fn lower(model: &Model, units: &UnitScale, curve: EntityId) -> GeometryResult<Curve3> {
    let mut session = LoweringSession::new(model, units);
    let root = lower_curve_node(&mut session, curve, Transform::identity())?;
    let lowered = session.finish(root)?;
    match lowered.graph.get(lowered.root) {
        Some(GeometryNode::Curve3(curve3)) => Ok(curve3.clone()),
        _ => Err(GeometryError::Unsupported {
            entity: curve,
            type_name: model
                .get(curve)
                .map(|e| e.type_name.to_string())
                .unwrap_or_default(),
            detail: "the grid axis curve did not lower to a single neutral Curve3",
        }),
    }
}

type Evaluated<T> = Result<T, axiolid_contracts::GeomError>;

/// The point on `track` at `u`.
fn point(track: &Track, evaluator: &dyn CurveEvaluator, u: f64) -> Evaluated<[f64; 2]> {
    match track {
        Track::Straight { point, direction } => {
            Ok([point[0] + u * direction[0], point[1] + u * direction[1]])
        }
        Track::Curved { curve, k, .. } => {
            let p = evaluator.point_at(curve, CurveMeasure::Parameter(u))?;
            let t = planar_tangent(curve, evaluator, u)?;
            let left = left_of(t);
            Ok([p.x + k * left[0], p.y + k * left[1]])
        }
    }
}

/// The axis's unit tangent at `u`, in its grid sense.
fn axis_tangent(track: &Track, evaluator: &dyn CurveEvaluator, u: f64) -> Evaluated<[f64; 2]> {
    match track {
        Track::Straight { direction, .. } => Ok(*direction),
        Track::Curved { curve, sense, .. } => {
            Ok(planar_tangent(curve, evaluator, u)?.map(|v| v * sense))
        }
    }
}

/// The evaluator's unit tangent, in the grid's XY plane.
fn planar_tangent(curve: &Curve3, evaluator: &dyn CurveEvaluator, u: f64) -> Evaluated<[f64; 2]> {
    let t = evaluator.tangent_at(curve, CurveMeasure::Parameter(u))?;
    normalise([t.x, t.y]).ok_or(axiolid_contracts::GeomError::Degenerate(
        "the grid axis has no tangent in the grid's XY plane".into(),
    ))
}

/// Starting pairs `(u, v)`, closest first.
fn seeds(tracks: &[Track; 2], evaluator: &dyn CurveEvaluator) -> Evaluated<Vec<(f64, f64)>> {
    let samples = |track: &Track| -> Evaluated<Vec<(f64, [f64; 2])>> {
        let Track::Curved { lo, hi, .. } = track else {
            return Ok(Vec::new());
        };
        (0..=SAMPLES)
            .map(|i| {
                let u = lo + (hi - lo) * i as f64 / SAMPLES as f64;
                point(track, evaluator, u).map(|p| (u, p))
            })
            .collect()
    };
    let mut candidates: Vec<(f64, f64, f64)> = Vec::new();
    match tracks {
        [Track::Straight {
            point: q,
            direction: d,
        }, curved]
        | [curved, Track::Straight {
            point: q,
            direction: d,
        }] => {
            let curved_first = matches!(tracks[0], Track::Curved { .. });
            for (u, p) in samples(curved)? {
                let v = (p[0] - q[0]) * d[0] + (p[1] - q[1]) * d[1];
                let foot = [q[0] + v * d[0], q[1] + v * d[1]];
                let gap = (p[0] - foot[0]).hypot(p[1] - foot[1]);
                candidates.push(if curved_first {
                    (gap, u, v)
                } else {
                    (gap, v, u)
                });
            }
        }
        [a, b] => {
            let (sa, sb) = (samples(a)?, samples(b)?);
            for (u, p) in &sa {
                let best = sb
                    .iter()
                    .map(|(v, r)| ((p[0] - r[0]).hypot(p[1] - r[1]), *v))
                    .min_by(|x, y| x.0.total_cmp(&y.0));
                if let Some((gap, v)) = best {
                    candidates.push((gap, *u, v));
                }
            }
        }
    }
    candidates.sort_by(|x, y| x.0.total_cmp(&y.0));
    Ok(candidates
        .into_iter()
        .take(SEEDS)
        .map(|(_, u, v)| (u, v))
        .collect())
}

/// Keep `u` within a curved track's range: wrap a full turn, clamp an arc.
fn confine(track: &Track, u: f64) -> f64 {
    match track {
        Track::Straight { .. } => u,
        Track::Curved {
            lo, hi, periodic, ..
        } => {
            if *periodic {
                lo + (u - lo).rem_euclid(hi - lo)
            } else {
                u.clamp(*lo, *hi)
            }
        }
    }
}

/// Derivative of `point(track, u)` by central differences.
fn derivative(track: &Track, evaluator: &dyn CurveEvaluator, u: f64) -> Evaluated<[f64; 2]> {
    let (lo, hi, periodic) = match track {
        Track::Straight { direction, .. } => return Ok(*direction),
        Track::Curved {
            lo, hi, periodic, ..
        } => (*lo, *hi, *periodic),
    };
    let h = 1e-6 * (hi - lo).max(1.0);
    let (a, b) = if periodic {
        (u - h, u + h)
    } else {
        ((u - h).max(lo), (u + h).min(hi))
    };
    let (pa, pb) = (point(track, evaluator, a)?, point(track, evaluator, b)?);
    Ok([(pb[0] - pa[0]) / (b - a), (pb[1] - pa[1]) / (b - a)])
}

/// Newton iteration on `V_1(u) - V_2(v)`; the converged `u` and point, or
/// `None` when this seed does not converge inside both ranges.
fn newton(
    tracks: &[Track; 2],
    evaluator: &dyn CurveEvaluator,
    mut u: f64,
    mut v: f64,
) -> Evaluated<Option<(f64, [f64; 2])>> {
    for _ in 0..ITERATIONS {
        let (a, b) = (
            point(&tracks[0], evaluator, u)?,
            point(&tracks[1], evaluator, v)?,
        );
        let f = [a[0] - b[0], a[1] - b[1]];
        let scale = a.iter().chain(b.iter()).fold(1.0f64, |m, c| m.max(c.abs()));
        if f[0].hypot(f[1]) <= 1e-11 * scale {
            return Ok(Some((u, a)));
        }
        let (ja, jb) = (
            derivative(&tracks[0], evaluator, u)?,
            derivative(&tracks[1], evaluator, v)?,
        );
        // J = [ja, -jb]; solve J * [du, dv] = f.
        let det = -ja[0] * jb[1] + ja[1] * jb[0];
        if !det.is_finite() || det.abs() <= 1e-14 {
            return Ok(None);
        }
        let du = (-f[0] * jb[1] + f[1] * jb[0]) / det;
        let dv = (ja[0] * f[1] - ja[1] * f[0]) / det;
        u = confine(&tracks[0], u - du);
        v = confine(&tracks[1], v - dv);
    }
    Ok(None)
}

/// Why the evaluator refused, by kind.
fn evaluator_refusal(error: &axiolid_contracts::GeomError) -> &'static str {
    use axiolid_contracts::GeomError;
    match error {
        GeomError::Unsupported { .. } | GeomError::UnsupportedInput { .. } => {
            "the evaluator cannot evaluate this grid axis curve family"
        }
        GeomError::Degenerate(_) => "the grid axis curve is degenerate where it was evaluated",
        _ => "the evaluator refused a grid axis curve",
    }
}
