//! `IfcIndexedPolyCurve` profile boundaries as exact contours (#335).
//!
//! # What is lowered
//!
//! The `OuterCurve` and `InnerCurves` of `IfcArbitraryClosedProfileDef` and
//! `IfcArbitraryProfileDefWithVoids`, given as an `IfcIndexedPolyCurve` over
//! an `IfcCartesianPointList2D`. IFC4 ADD2 TC1 and IFC4X3 ADD2 state the
//! curve identically:
//!
//! - "In the case that the list of Segments is not provided, all points in
//!   the IfcCartesianPointList are connected by straight line segments in
//!   the order they appear": one straight `Line2` edge per consecutive pair.
//! - An `IfcLineIndex` is a polyline run: one edge per consecutive pair of
//!   its indices.
//! - An `IfcArcIndex` is "the start point of the circular arc, ... a point on
//!   arc, ... the end point": the exact circle through the three points, as
//!   one `Circle2` segment with an angle domain. Nothing is chorded here.
//! - An `IfcArcIndex` whose three distinct points are collinear is, by the
//!   schema's own words, a polyline: "The three points shall not be
//!   co-linear. In case that this informal proposition is not maintained,
//!   the arc segment shall be treated as a polyline segment." It lowers as
//!   the straight path start -> mid -> end: one edge when the middle point
//!   lies between the others, two when it does not.
//!
//! Coincidence and collinearity are judged "after taking the Precision
//! factor into account, given by the applicable
//! IfcGeometricRepresentationContext": the model's declared `Precision`
//! (`constraint::tolerance`), `1.E-5` project units when none is declared.
//! The three points of an arc are classified by `tolerance::arc_points`,
//! which the general curve lowering shares (#396).
//!
//! The edges are written exactly as the `IfcCompositeCurve` reader writes the
//! same boundary (`super::composite`), so an indexed boundary and its
//! composite equivalent lower to the same contour.
//!
//! # Refused, never repaired
//!
//! Each refusal is [`GeometryError::Degenerate`] naming the curve:
//!
//! - An open curve. Closure is the schema's: with `Segments`, "the last index
//!   of the last Segment and the first index of the first Segment are
//!   identical"; without, "the first and the last Cartesian point ... are
//!   identical". A profile boundary "has to be a closed curve", so closing
//!   one would add an edge the file never authored.
//! - Segments that are not consecutive (WHERE rule `Consecutive`).
//! - An arc with two coincident points: no circle and no polyline segment is
//!   defined, and the schema states no fallback for it.
//! - `SelfIntersect` TRUE. The flag is "for information only", but it states
//!   that the curve crosses itself, and "The OuterCurve shall not intersect".
//! - A 3D point list (`OuterCurve.Dim = 2`).
//!
//! # Half-space boundaries (#393)
//!
//! IFC4X3 ADD2 admits an `IfcIndexedPolyCurve` as
//! `IfcPolygonalBoundedHalfSpace.PolygonalBoundary` (`BoundaryType`); IFC4
//! ADD2 TC1 does not, but the curve itself is defined identically in both, so
//! an IFC4 file using it lowers the same way and `BoundaryType` is left to
//! validation. Under [`BoundaryRole::HalfSpace`] the reading, closure,
//! collinear-arc fallback, exact arcs (#398: the kernel's bounded half-space
//! takes a `Profile` contour, axiolid/kernel#277) and refusals above are
//! unchanged, except that a 3D point list is admitted when every `z` is
//! exactly 0, as a 3D point of a boundary `IfcPolyline` is; any other `z` is
//! off the plane `BoundaryDim` requires and is refused.

use std::f64::consts::TAU;

use axiolid_core::{Frame2, Interval, Point2, Vec2};
use axiolid_curve::{Circle2, Curve2, Line2};
use axiolid_profile::{Contour, ProfileSegment};
use ifc_model::{EntityId, Model};

use super::role::BoundaryRole;
use crate::constraint::tolerance::{
    arc_points, model_precision_metres, points_coincide, ArcPoints,
};
use crate::curve::{IndexedPolyCurve, PolySegment};
use crate::error::{GeometryError, GeometryResult};
use crate::resource::point::CartesianPointList;
use crate::units::UnitScale;

const TYPE_NAME: &str = "IFCINDEXEDPOLYCURVE";

/// Lower a closed `IfcIndexedPolyCurve` boundary into one contour.
pub(super) fn indexed_contour(
    model: &Model,
    id: EntityId,
    units: &UnitScale,
    role: BoundaryRole,
) -> GeometryResult<Contour> {
    let entity = model.get(id).ok_or(GeometryError::MissingEntity {
        referrer: id,
        missing: id,
    })?;
    let view = IndexedPolyCurve::new(id, entity);
    if view.self_intersect() == Some(true) {
        return Err(refuse(
            id,
            format!(
                "SelfIntersect is TRUE: the curve crosses itself, and {} shall not intersect",
                role.what()
            ),
        ));
    }
    let points = points_2d(model, &view, units, role)?;
    let mut boundary = Boundary {
        id,
        precision: model_precision_metres(model, units)?,
        segments: Vec::new(),
    };

    if view.has_explicit_segments() {
        let segments = view.segments(points.len())?;
        let (Some(first), Some(last)) = (segments.first(), segments.last()) else {
            return Err(refuse(id, "Segments is empty"));
        };
        if first.indices().first() != last.indices().last() {
            return Err(refuse(
                id,
                format!(
                    "the curve is open: the last index of the last segment is not the first \
                     index of the first, and {} must be closed",
                    role.what()
                ),
            ));
        }
        for pair in segments.windows(2) {
            if pair[0].indices().last() != pair[1].indices().first() {
                return Err(refuse(
                    id,
                    "Segments are not consecutive (WHERE rule Consecutive): a segment does \
                     not start at the index the previous one ends on",
                ));
            }
        }
        for segment in segments {
            match segment {
                PolySegment::Line(indices) => {
                    let run: Vec<Point2> = indices.into_iter().map(|i| points[i]).collect();
                    boundary.lines(&run);
                }
                PolySegment::Arc { start, mid, end } => {
                    boundary.arc(points[start], points[mid], points[end])?;
                }
            }
        }
    } else {
        let (Some(first), Some(last)) = (points.first(), points.last()) else {
            return Err(refuse(id, "the point list is empty"));
        };
        if points.len() < 2
            || !points_coincide(boundary.precision, first.to_array(), last.to_array())
        {
            return Err(refuse(
                id,
                format!(
                    "the curve is open: its first and last points are {} m apart, over the \
                     model's {} m precision, and {} must be closed",
                    first.distance(*last),
                    boundary.precision,
                    role.what()
                ),
            ));
        }
        boundary.lines(&points);
    }

    let straight = boundary
        .segments
        .iter()
        .all(|segment| matches!(segment.curve, Curve2::Line(_)));
    if boundary.segments.len() < 2 || (straight && boundary.segments.len() < 3) {
        return Err(refuse(id, "the boundary encloses no area"));
    }
    Ok(Contour::new(boundary.segments))
}

/// The point list as 2D metres; a non-finite value refuses.
///
/// A 3D list refuses for a profile. For a half-space boundary it is read
/// when every `z` is exactly 0, and refused, naming the list, otherwise.
fn points_2d(
    model: &Model,
    view: &IndexedPolyCurve<'_>,
    units: &UnitScale,
    role: BoundaryRole,
) -> GeometryResult<Vec<Point2>> {
    let raw = match view.points(model)? {
        CartesianPointList::TwoD(list) => list.coordinates()?,
        CartesianPointList::ThreeD(list) if role == BoundaryRole::HalfSpace => {
            let mut planar = Vec::new();
            for (index, row) in list.coordinates()?.into_iter().enumerate() {
                let [x, y] = role
                    .planar(list.id(), "IFCCARTESIANPOINTLIST3D", &row)
                    .map_err(|error| match error {
                        GeometryError::Degenerate {
                            entity,
                            type_name,
                            detail,
                        } => GeometryError::Degenerate {
                            entity,
                            type_name,
                            detail: format!("point {}: {detail}", index + 1),
                        },
                        other => other,
                    })?;
                planar.push([x, y]);
            }
            planar
        }
        CartesianPointList::ThreeD(list) => {
            return Err(GeometryError::Degenerate {
                entity: list.id(),
                type_name: "IFCCARTESIANPOINTLIST3D".to_owned(),
                detail: "a profile boundary is 2D (OuterCurve.Dim = 2), this point list is 3D"
                    .to_owned(),
            })
        }
    };
    raw.into_iter()
        .map(|[x, y]| {
            let point = Point2::new(units.length(x), units.length(y));
            if point.is_finite() {
                Ok(point)
            } else {
                Err(refuse(view.id(), "a point coordinate is not finite"))
            }
        })
        .collect()
}

/// Oriented contour segments, accumulated in traversal order.
struct Boundary {
    id: EntityId,
    /// The model's `Precision`, in metres.
    precision: f64,
    segments: Vec<ProfileSegment>,
}

impl Boundary {
    /// One straight edge per consecutive pair of `run`.
    ///
    /// A zero-length edge (a repeated point) carries no boundary and is
    /// dropped, as the composite reader drops it.
    fn lines(&mut self, run: &[Point2]) {
        for pair in run.windows(2) {
            let (origin, next) = (pair[0], pair[1]);
            if origin == next {
                continue;
            }
            self.segments.push(ProfileSegment {
                curve: Curve2::Line(Line2 {
                    origin,
                    direction: next - origin,
                }),
                domain: Interval::UNIT,
                same_sense: true,
            });
        }
    }

    /// The exact circular arc from `start` through `mid` to `end`, or the
    /// polyline through them when they are collinear.
    ///
    /// The circle's frame is the profile's own axes about the circumcentre,
    /// so an angle parameter reads exactly as the composite reader reads a
    /// trimmed `IfcCircle` placed without a `RefDirection`. The domain is
    /// written in increasing angle; `same_sense` says whether the arc runs
    /// with it (anticlockwise) or against it (clockwise).
    fn arc(&mut self, start: Point2, mid: Point2, end: Point2) -> GeometryResult<()> {
        let planar = |p: Point2| [p.x, p.y, 0.0];
        match arc_points(self.precision, planar(start), planar(mid), planar(end)) {
            ArcPoints::Coincident => {
                return Err(refuse(
                    self.id,
                    "an IfcArcIndex has two coincident points, so no circle is defined",
                ))
            }
            ArcPoints::NonFinite => {
                return Err(refuse(self.id, "an IfcArcIndex's points are not finite"))
            }
            // "In case that this informal proposition is not maintained, the
            // arc segment shall be treated as a polyline segment."
            ArcPoints::Collinear { through_mid: false } => {
                self.lines(&[start, end]);
                return Ok(());
            }
            ArcPoints::Collinear { through_mid: true } => {
                self.lines(&[start, mid, end]);
                return Ok(());
            }
            ArcPoints::Circular => {}
        }
        let chord = end - start;
        let u = mid - start;
        let turn = u.perp_dot(chord);
        let denominator = 2.0 * turn;
        let centre = start
            + Vec2::new(
                chord.y * u.length_squared() - u.y * chord.length_squared(),
                u.x * chord.length_squared() - chord.x * u.length_squared(),
            ) / denominator;
        let radius = start.distance(centre);
        if !(centre.is_finite() && radius.is_finite() && radius > 0.0) {
            return Err(refuse(
                self.id,
                "an IfcArcIndex's circumcentre is not finite",
            ));
        }
        let angle = |at: Point2| {
            let d = at - centre;
            let angle = d.y.atan2(d.x);
            if angle < 0.0 {
                angle + TAU
            } else {
                angle
            }
        };
        let t1 = angle(start);
        let mut t2 = angle(end);
        // `start -> mid -> end` turning left is an anticlockwise arc.
        let anticlockwise = turn > 0.0;
        if anticlockwise {
            while t2 <= t1 {
                t2 += TAU;
            }
        } else {
            while t2 >= t1 {
                t2 -= TAU;
            }
        }
        self.segments.push(ProfileSegment {
            curve: Curve2::Circle(Circle2 {
                frame: Frame2 {
                    origin: centre,
                    x: Vec2::X,
                    y: Vec2::Y,
                },
                radius,
            }),
            domain: Interval::new(t1.min(t2), t1.max(t2)),
            same_sense: anticlockwise,
        });
        Ok(())
    }
}

fn refuse(id: EntityId, detail: impl Into<String>) -> GeometryError {
    GeometryError::Degenerate {
        entity: id,
        type_name: TYPE_NAME.to_owned(),
        detail: detail.into(),
    }
}
