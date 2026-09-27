//! `IfcCompositeCurve` profile boundaries as exact contours (#43).
//!
//! # What is lowered
//!
//! A composite is an ordered list of `IfcCompositeCurveSegment`s, each wrapping
//! a parent curve and a `SameSense` flag. Parents lowered here:
//!
//! - `IfcPolyline`: one straight `Line2` segment per edge.
//! - `IfcTrimmedCurve` over `IfcCircle` or `IfcLine`: one exact segment with a
//!   parameter domain. The arc stays an arc; nothing is chorded here.
//! - A nested `IfcCompositeCurve`, bounded by [`MAX_NESTING`].
//!
//! Anything else keeps a typed `Unsupported` naming the parent's own type, so
//! a gap in coverage is a diagnostic rather than a wrong shape.
//!
//! # Orientation
//!
//! A segment's traversal direction is the product of every flag above it:
//! the enclosing composite segment's `SameSense`, the trimmed curve's
//! `SenseAgreement`, and the same for each nesting level. The neutral
//! `ProfileSegment` carries that product as `same_sense` against a domain
//! written in the curve's own increasing parameter, which is exactly what the
//! kernel's flattener and exact lowerer both consume.
//!
//! # Gaps are refused, never closed
//!
//! Consecutive segments must meet, and the last must return to the first.
//! A gap wider than [`GAP_TOLERANCE`] is `Degenerate`, naming the segment
//! whose start misses the previous end. Closing it silently would put a
//! straight edge into the profile that the file never authored.

use std::f64::consts::TAU;

use axiolid_core::{Frame2, Interval, Point2, Vec2};
use axiolid_curve::{Circle2, Curve2, Line2};
use axiolid_profile::{Contour, ProfileSegment};
use ifc_model::{EntityId, Model};

use super::polyline_points;
use crate::curve::composite::{CompositeCurve, CompositeCurveSegment};
use crate::curve::conic::Circle;
use crate::curve::line::Line;
use crate::curve::trimmed::{TrimPoint, TrimmedCurve};
use crate::error::{GeometryError, GeometryResult};
use crate::resource::direction::resolve_unit;
use crate::resource::placement::Axis2Placement2D;
use crate::resource::point::cartesian_point_3d;
use crate::slots::Slots;
use crate::units::UnitScale;

/// How far two consecutive segment ends may be apart and still meet, in metres.
///
/// IFC's own default model precision is 1e-5 in the project length unit
/// (`NVL(ParentContext.Precision, 1.E-5)`). Every real composite surveyed for
/// #43 closes to well under that; a gap as wide as the arc test's 0.1 m is a
/// fault, not rounding. The value is in metres because lowering has already
/// converted every coordinate, so one constant serves metre and millimetre
/// files alike.
const GAP_TOLERANCE: f64 = 1e-5;

/// Maximum depth of composite-in-composite nesting.
///
/// Real files nest at most one level. The bound exists because a hostile file
/// can make a composite contain itself, which would otherwise recurse forever.
const MAX_NESTING: usize = 8;

/// Lower a closed `IfcCompositeCurve` profile boundary into one contour.
pub(super) fn composite_contour(
    model: &Model,
    id: EntityId,
    units: &UnitScale,
) -> GeometryResult<Contour> {
    let mut walk = Walk {
        model,
        units,
        segments: Vec::new(),
        owners: Vec::new(),
    };
    walk.composite(id, true, 0)?;
    walk.check_closed(id)?;
    Ok(Contour::new(walk.segments))
}

/// Accumulates oriented segments while walking a (possibly nested) composite.
struct Walk<'a> {
    model: &'a Model,
    units: &'a UnitScale,
    segments: Vec<ProfileSegment>,
    /// The entity each segment came from, for naming a gap's culprit.
    owners: Vec<EntityId>,
}

impl Walk<'_> {
    fn entity(&self, referrer: EntityId, id: EntityId) -> GeometryResult<&ifc_model::Entity> {
        self.model.get(id).ok_or(GeometryError::MissingEntity {
            referrer,
            missing: id,
        })
    }

    /// Append every segment of one composite, traversed forward when `forward`.
    ///
    /// A reversed composite is walked last segment first, and each of its
    /// segments is itself reversed: reversing a chain reverses both its order
    /// and every member's direction.
    fn composite(&mut self, id: EntityId, forward: bool, depth: usize) -> GeometryResult<()> {
        if depth > MAX_NESTING {
            return Err(GeometryError::ChainTooDeep {
                entity: id,
                kind: "composite curve",
                limit: MAX_NESTING,
            });
        }
        let entity = self.entity(id, id)?;
        let mut segment_refs = CompositeCurve::new(id, entity).segment_refs()?;
        if !forward {
            segment_refs.reverse();
        }
        for segment_ref in segment_refs {
            let segment_entity = self.entity(id, segment_ref)?;
            let segment = CompositeCurveSegment::new(segment_ref, segment_entity);
            if segment.is_reparametrised() {
                return Err(GeometryError::Unsupported {
                    entity: segment_ref,
                    type_name: "IFCREPARAMETRISEDCOMPOSITECURVESEGMENT".to_string(),
                    detail: "reparametrised composite segments are not lowered as profile \
                             boundaries",
                });
            }
            let parent = segment.parent_curve_ref()?;
            let sense = segment.same_sense()? == forward;
            self.parent(segment_ref, parent, sense, depth)?;
        }
        Ok(())
    }

    /// Append one segment's parent curve, traversed forward when `forward`.
    fn parent(
        &mut self,
        segment: EntityId,
        id: EntityId,
        forward: bool,
        depth: usize,
    ) -> GeometryResult<()> {
        let type_name = self.entity(segment, id)?.type_name.to_ascii_uppercase();
        match type_name.as_str() {
            "IFCPOLYLINE" => self.polyline(id, forward),
            "IFCTRIMMEDCURVE" => self.trimmed(id, forward),
            "IFCCOMPOSITECURVE" => self.composite(id, forward, depth + 1),
            _ => Err(GeometryError::Unsupported {
                entity: id,
                type_name,
                detail: "composite profile segments lower IfcPolyline, IfcTrimmedCurve over \
                         IfcCircle or IfcLine, and nested IfcCompositeCurve only",
            }),
        }
    }

    /// One straight segment per polyline edge.
    ///
    /// A reversed polyline is walked from its last point back to its first.
    /// Zero-length edges (a repeated point) carry no boundary and are dropped
    /// rather than handed to the kernel as a degenerate line.
    fn polyline(&mut self, id: EntityId, forward: bool) -> GeometryResult<()> {
        let mut points = polyline_points(self.model, id, self.units)?;
        if points.len() < 2 {
            return Err(Slots::new(id, self.entity(id, id)?)
                .degenerate("composite polyline segment has fewer than 2 points"));
        }
        if !forward {
            points.reverse();
        }
        for pair in points.windows(2) {
            let (origin, next) = (pair[0], pair[1]);
            if origin == next {
                continue;
            }
            self.push(
                id,
                ProfileSegment {
                    curve: Curve2::Line(Line2 {
                        origin,
                        direction: next - origin,
                    }),
                    domain: Interval::UNIT,
                    same_sense: true,
                },
            );
        }
        Ok(())
    }

    /// One exact segment for a trimmed circle or line.
    fn trimmed(&mut self, id: EntityId, forward: bool) -> GeometryResult<()> {
        let entity = self.entity(id, id)?;
        let view = TrimmedCurve::new(id, entity);
        let spec = view.spec()?;
        let preference = view.master_representation();
        let (trim1, trim2) = spec.endpoints();
        let basis_ref = spec.basis_curve;
        let basis_type = self.entity(id, basis_ref)?.type_name.to_ascii_uppercase();
        let basis = match basis_type.as_str() {
            "IFCCIRCLE" => self.circle(basis_ref)?,
            "IFCLINE" => self.line(basis_ref)?,
            _ => {
                return Err(GeometryError::Unsupported {
                    entity: basis_ref,
                    type_name: basis_type,
                    detail: "trimmed profile segments lower IfcCircle and IfcLine bases only",
                })
            }
        };

        let resolve = |trim: crate::curve::trimmed::Trim| -> GeometryResult<f64> {
            match trim.preferred(preference) {
                Some(TrimPoint::Parameter(raw)) => Ok(basis.parameter(raw, self.units)),
                Some(TrimPoint::Cartesian(point)) => {
                    let c = cartesian_point_3d(self.model, id, point)?;
                    let at = Point2::new(self.units.length(c[0]), self.units.length(c[1]));
                    Ok(basis.project(at))
                }
                None => Err(Slots::new(id, entity)
                    .degenerate("a trim end carries neither a parameter nor a point")),
            }
        };
        let t1 = resolve(trim1)?;
        let mut t2 = resolve(trim2)?;
        let sense = spec.sense_agreement;

        // IFC traverses from Trim1 to Trim2 in the direction SenseAgreement
        // names. On a circle that wraps: 270 deg -> 90 deg with .T. is a half
        // turn through 0, not a backwards half turn. Unwrap the end so the
        // interval runs the authored way, then write it in increasing order.
        if let Basis::Circle(_) = basis {
            if sense {
                while t2 <= t1 {
                    t2 += TAU;
                }
            } else {
                while t2 >= t1 {
                    t2 -= TAU;
                }
            }
        }
        if t1 == t2 {
            return Err(Slots::new(id, entity).degenerate("trimmed segment has zero extent"));
        }
        let domain = Interval::new(t1.min(t2), t1.max(t2));
        // Which way along the increasing parameter the file walks this segment:
        // SenseAgreement for the trimmed curve itself, then the enclosing
        // segment's orientation on top.
        let increasing = (t2 > t1) == forward;
        self.push(
            id,
            ProfileSegment {
                curve: basis.into_curve(),
                domain,
                same_sense: increasing,
            },
        );
        Ok(())
    }

    fn circle(&self, id: EntityId) -> GeometryResult<Basis> {
        let entity = self.entity(id, id)?;
        let view = Circle::new(id, entity);
        let radius = self.units.length(view.radius()?);
        if !(radius.is_finite() && radius > 0.0) {
            return Err(Slots::new(id, entity).degenerate("circle radius must be positive"));
        }
        let frame = self.frame(id, view.position_ref()?)?;
        Ok(Basis::Circle(Circle2 { frame, radius }))
    }

    fn line(&self, id: EntityId) -> GeometryResult<Basis> {
        let entity = self.entity(id, id)?;
        let view = Line::new(id, entity);
        let p = cartesian_point_3d(self.model, id, view.point_ref()?)?;
        let origin = Point2::new(self.units.length(p[0]), self.units.length(p[1]));
        let vector_ref = view.direction_vector_ref()?;
        let vector = Slots::new(vector_ref, self.entity(id, vector_ref)?);
        let unit = resolve_unit(self.model, id, vector.req_ref(0, "Orientation")?)?;
        // The magnitude is a length: a line parameter counts multiples of it.
        let magnitude = self.units.length(vector.req_f64(1, "Magnitude")?);
        let direction = Vec2::new(unit[0], unit[1]) * magnitude;
        if !(direction.is_finite() && direction.length() > 0.0) {
            return Err(vector.degenerate("line direction must be finite and non-zero"));
        }
        Ok(Basis::Line(Line2 { origin, direction }))
    }

    /// The 2D placement of a profile conic, in metres.
    ///
    /// A 3D placement is refused: a profile lies in its own XY plane, so a
    /// tilted axis would have no meaning, and dropping it silently would
    /// place the arc somewhere the file never said.
    fn frame(&self, owner: EntityId, position_ref: EntityId) -> GeometryResult<Frame2> {
        let position = self.entity(owner, position_ref)?;
        if !position
            .type_name
            .eq_ignore_ascii_case("IFCAXIS2PLACEMENT2D")
        {
            return Err(GeometryError::Unsupported {
                entity: position_ref,
                type_name: position.type_name.to_ascii_uppercase(),
                detail: "a profile conic needs an IfcAxis2Placement2D",
            });
        }
        let view = Axis2Placement2D::new(position_ref, position);
        let origin = view.location(self.model)?;
        let x = view.ref_direction(self.model)?;
        Ok(Frame2 {
            origin: Point2::new(self.units.length(origin[0]), self.units.length(origin[1])),
            x: Vec2::new(x[0], x[1]),
            y: Vec2::new(-x[1], x[0]),
        })
    }

    fn push(&mut self, owner: EntityId, segment: ProfileSegment) {
        self.segments.push(segment);
        self.owners.push(owner);
    }

    /// Every segment's start meets the previous end, and the chain closes.
    fn check_closed(&self, id: EntityId) -> GeometryResult<()> {
        if self.segments.is_empty() {
            return Err(GeometryError::Degenerate {
                entity: id,
                type_name: "IFCCOMPOSITECURVE".to_string(),
                detail: "composite profile boundary has no segments".to_string(),
            });
        }
        let ends: Vec<(Point2, Point2)> = self.segments.iter().map(endpoints).collect();
        for index in 0..ends.len() {
            let previous = ends[(index + ends.len() - 1) % ends.len()].1;
            let start = ends[index].0;
            let gap = previous.distance(start);
            // A NaN gap (from a non-finite coordinate) must refuse, not pass.
            if gap.is_nan() || gap > GAP_TOLERANCE {
                let what = if index == 0 {
                    "the composite does not close: the last segment ends"
                } else {
                    "segment starts"
                };
                return Err(GeometryError::Degenerate {
                    entity: self.owners[index],
                    type_name: "IFCCOMPOSITECURVE".to_string(),
                    detail: format!(
                        "{what} {gap} m from the previous end, over the {GAP_TOLERANCE} m gap \
                         tolerance (composite {id}); gaps are refused, never closed"
                    ),
                });
            }
        }
        Ok(())
    }
}

/// A trimmed segment's basis, with its parameter interpretation.
#[derive(Clone, Copy)]
enum Basis {
    Circle(Circle2),
    Line(Line2),
}

impl Basis {
    /// Convert an authored trim parameter into the kernel's parameter.
    ///
    /// A circle parameter is an angle in the project's plane-angle unit. A
    /// line parameter counts multiples of its `IfcVector`, which already
    /// carries the length, so it is dimensionless and passes through.
    fn parameter(self, raw: f64, units: &UnitScale) -> f64 {
        match self {
            Self::Circle(_) => units.angle(raw),
            Self::Line(_) => raw,
        }
    }

    /// The parameter of the point on this curve closest to `at`.
    ///
    /// A cartesian trim point is authored ON the basis, so this inverts the
    /// parameterisation rather than genuinely projecting; an off-curve point
    /// still lands on the curve, and the gap check then reports how far off
    /// the composite's joints are.
    fn project(self, at: Point2) -> f64 {
        match self {
            Self::Circle(c) => {
                let d = at - c.frame.origin;
                let angle = d.dot(c.frame.y).atan2(d.dot(c.frame.x));
                if angle < 0.0 {
                    angle + TAU
                } else {
                    angle
                }
            }
            Self::Line(l) => (at - l.origin).dot(l.direction) / l.direction.length_squared(),
        }
    }

    fn into_curve(self) -> Curve2 {
        match self {
            Self::Circle(c) => Curve2::Circle(c),
            Self::Line(l) => Curve2::Line(l),
        }
    }
}

/// Start and end of a segment, in traversal order.
fn endpoints(segment: &ProfileSegment) -> (Point2, Point2) {
    let at = |t: f64| match &segment.curve {
        Curve2::Line(l) => l.origin + l.direction * t,
        Curve2::Circle(c) => {
            let (sin, cos) = t.sin_cos();
            c.frame.origin + c.frame.x * (c.radius * cos) + c.frame.y * (c.radius * sin)
        }
        // Only lines and circles are ever pushed by this module.
        _ => unreachable!("composite lowering emits lines and circles only"),
    };
    let (a, b) = (at(segment.domain.start), at(segment.domain.end));
    if segment.same_sense {
        (a, b)
    } else {
        (b, a)
    }
}
