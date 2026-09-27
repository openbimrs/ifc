//! The straight-edged outline of an arbitrary profile, in metres (#166).
//!
//! [`super::describe_profile`] names an arbitrary profile's boundary curves by
//! id. A rule check that bounds a section -- openings in a wall whose outline
//! is a free polyline at a mitred join -- needs the vertices themselves, and
//! must not link a kernel to get them. This reads them for boundaries whose
//! every edge is straight, which a vertex list states exactly.
//!
//! # Curved edges are refused, never chorded
//!
//! A vertex list cannot state an arc. Chording one would hand the caller a
//! polygon that differs from the file by an error nobody chose, so an
//! `IfcArcIndex` segment, and any curve family other than `IfcPolyline` and
//! `IfcIndexedPolyCurve`, is [`GeometryError::Unsupported`] naming the curve.
//!
//! # Rings
//!
//! Each boundary is returned as a ring: authored order, the closing vertex not
//! repeated. IFC closes a polyline by repeating its first point, and a closed
//! indexed curve by ending on its first index; both are dropped here, as the
//! profile lowering drops them, so the ring is the loop the kernel receives.

use ifc_model::{EntityId, Model};

use super::{describe_profile, ProfileParameters};
use crate::curve::{IndexedPolyCurve, PolySegment, Polyline};
use crate::error::{GeometryError, GeometryResult};
use crate::resource::point::CartesianPointList;
use crate::units::UnitScale;

/// Two vertices closer than this, in metres, are the same vertex.
///
/// The value the profile lowering uses for the closing duplicate, so both
/// agree on whether a ring repeats its first vertex.
const SAME_VERTEX: f64 = 1e-12;

/// An arbitrary profile's boundaries as rings of vertices.
///
/// Coordinates are in the profile's own 2D coordinate system, in metres.
/// Arbitrary profiles have no `Position`, so these are the curve coordinates
/// the file authored, converted once.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct ProfileOutline {
    /// The `IfcProfileDef` entity.
    pub profile: EntityId,
    /// `OuterCurve`'s vertices, in authored order, the first not repeated.
    pub outer: Vec<[f64; 2]>,
    /// Each `IfcArbitraryProfileDefWithVoids.InnerCurves` ring, in file
    /// order. Empty for an `IfcArbitraryClosedProfileDef`.
    pub inner: Vec<Vec<[f64; 2]>>,
}

/// Read an arbitrary closed profile's outline, in metres.
///
/// Accepts `IfcArbitraryClosedProfileDef` and
/// `IfcArbitraryProfileDefWithVoids` whose boundaries are `IfcPolyline`s or
/// line-only `IfcIndexedPolyCurve`s. Refused, each naming the entity:
///
/// - any other profile family, and any other boundary curve family, as
///   [`GeometryError::Unsupported`];
/// - an `IfcArcIndex` segment, as [`GeometryError::Unsupported`]: the
///   outline states straight edges only;
/// - a 3D point, a ring of fewer than three distinct vertices, or indexed
///   segments that do not join, as [`GeometryError::Degenerate`];
/// - whatever [`describe_profile`] refuses for the profile itself.
///
/// Kernel-free.
pub fn profile_outline(
    model: &Model,
    units: &UnitScale,
    profile: EntityId,
) -> GeometryResult<ProfileOutline> {
    let description = describe_profile(model, units, profile)?;
    let (outer, inner): (EntityId, &[EntityId]) = match &description.parameters {
        ProfileParameters::ArbitraryClosed { outer_curve } => (*outer_curve, &[]),
        ProfileParameters::ArbitraryWithVoids {
            outer_curve,
            inner_curves,
        } => (*outer_curve, inner_curves),
        _ => {
            return Err(GeometryError::Unsupported {
                entity: profile,
                type_name: description.type_name,
                detail: "only IfcArbitraryClosedProfileDef and IfcArbitraryProfileDefWithVoids \
                         have an authored outline; describe_profile states the other families",
            })
        }
    };
    Ok(ProfileOutline {
        profile,
        outer: ring(model, units, outer)?,
        inner: inner
            .iter()
            .map(|curve| ring(model, units, *curve))
            .collect::<GeometryResult<_>>()?,
    })
}

/// One boundary curve as a ring of distinct vertices, in metres.
fn ring(model: &Model, units: &UnitScale, curve: EntityId) -> GeometryResult<Vec<[f64; 2]>> {
    let entity = model.get(curve).ok_or(GeometryError::MissingEntity {
        referrer: curve,
        missing: curve,
    })?;
    let type_name = entity.type_name.to_ascii_uppercase();
    let raw = match type_name.as_str() {
        "IFCPOLYLINE" => polyline(model, curve, entity)?,
        "IFCINDEXEDPOLYCURVE" => indexed(model, curve, entity)?,
        _ => {
            return Err(GeometryError::Unsupported {
                entity: curve,
                type_name,
                detail: "outline vertices are read from IfcPolyline and line-only \
                         IfcIndexedPolyCurve boundaries; other curve families may be curved",
            })
        }
    };
    let mut vertices: Vec<[f64; 2]> = raw
        .into_iter()
        .map(|[x, y]| [units.length(x), units.length(y)])
        .collect();
    if let (Some(first), Some(last)) = (vertices.first(), vertices.last()) {
        if vertices.len() >= 2 && distance(*first, *last) < SAME_VERTEX {
            vertices.pop();
        }
    }
    if vertices.len() < 3 {
        return Err(GeometryError::Degenerate {
            entity: curve,
            type_name,
            detail: format!(
                "a profile boundary needs at least 3 distinct vertices, found {}",
                vertices.len()
            ),
        });
    }
    Ok(vertices)
}

/// An `IfcPolyline`'s points, in file units. Each must be 2D.
fn polyline(
    model: &Model,
    curve: EntityId,
    entity: &ifc_model::Entity,
) -> GeometryResult<Vec<[f64; 2]>> {
    Polyline::new(curve, entity)
        .points(model)?
        .iter()
        .map(|point| match point.coordinates()?.as_slice() {
            [x, y] => Ok([*x, *y]),
            other => Err(GeometryError::Degenerate {
                entity: point.id(),
                type_name: "IFCCARTESIANPOINT".to_owned(),
                detail: format!(
                    "a profile boundary point is 2D (OuterCurve.Dim = 2), this one has {} \
                     coordinates",
                    other.len()
                ),
            }),
        })
        .collect()
}

/// A line-only `IfcIndexedPolyCurve`'s path, in file units.
///
/// Without `Segments` the path is every point in list order. With them, each
/// `IfcLineIndex` run must start where the previous one ended
/// (`IfcConsecutiveSegments`), and an `IfcArcIndex` is refused.
fn indexed(
    model: &Model,
    curve: EntityId,
    entity: &ifc_model::Entity,
) -> GeometryResult<Vec<[f64; 2]>> {
    let view = IndexedPolyCurve::new(curve, entity);
    let list = match view.points(model)? {
        CartesianPointList::TwoD(list) => list.coordinates()?,
        CartesianPointList::ThreeD(list) => {
            return Err(GeometryError::Degenerate {
                entity: list.id(),
                type_name: "IFCCARTESIANPOINTLIST3D".to_owned(),
                detail: "a profile boundary is 2D (OuterCurve.Dim = 2), this point list is 3D"
                    .to_owned(),
            })
        }
    };
    if !view.has_explicit_segments() {
        return Ok(list);
    }
    let mut indices: Vec<usize> = Vec::new();
    for segment in view.segments(list.len())? {
        let PolySegment::Line(run) = segment else {
            return Err(GeometryError::Unsupported {
                entity: curve,
                type_name: entity.type_name.to_ascii_uppercase(),
                detail: "an IfcArcIndex segment is a circular arc, which a vertex outline \
                         cannot state; it is refused rather than chorded",
            });
        };
        match indices.last() {
            None => indices.extend_from_slice(&run),
            Some(end) if Some(end) == run.first() => indices.extend_from_slice(&run[1..]),
            Some(_) => {
                return Err(GeometryError::Degenerate {
                    entity: curve,
                    type_name: entity.type_name.to_ascii_uppercase(),
                    detail: "Segments are not consecutive (WHERE rule Consecutive), so the \
                             boundary is not one ring"
                        .to_owned(),
                })
            }
        }
    }
    Ok(indices.into_iter().map(|index| list[index]).collect())
}

fn distance(a: [f64; 2], b: [f64; 2]) -> f64 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}
