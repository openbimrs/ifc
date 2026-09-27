//! Exact vertex sets for the solid leaves whose extent is a polytope.
//!
//! The axis-aligned box of a polytope is the box of its vertices, and an
//! affine map sends vertices to vertices, so bounding the transformed
//! vertices is exact under any placement, rotated or not. That is the whole
//! test for admission here: a leaf is bounded only when its boundary is
//! straight-edged.
//!
//! - A linear extrusion of a polygon is the polygon plus the polygon moved
//!   by the extrusion offset: twice the profile's vertices.
//! - A block is its eight corners.
//!
//! Anything curved (a circle or rounded-corner profile, an arc in a contour,
//! a parameterised section with fillets) returns `None`. Its local box would
//! still contain it, but the box's corners are not on the shape, so after a
//! rotation they over-state the world box. The caller then falls back to the
//! compiled mesh.
//!
//! Only well-formed values are admitted. A zero depth or a degenerate
//! rectangle also falls back, so the compiler reports it rather than this
//! module returning a plausible box for geometry that does not build.

use axiolid_core::{Point2, Point3, Scalar, Vec3};
use axiolid_curve::Curve2;
use axiolid_model::{GeometryGraph, GeometryNode, NodeId};
use axiolid_profile::{Contour, Profile};

/// Vertices of a linear extrusion of `profile`, in the extrusion's frame.
///
/// The offset is the unit direction times `depth`, the convention the
/// neutral `SolidOperation::Extrusion` shares with its compiler.
pub(super) fn extrusion(
    graph: &GeometryGraph,
    profile: NodeId,
    direction: Vec3,
    depth: Scalar,
) -> Option<Vec<Point3>> {
    let GeometryNode::Profile(profile) = graph.get(profile)? else {
        return None;
    };
    if !depth.is_finite() || depth <= 0.0 || !direction.is_finite() {
        return None;
    }
    let offset = direction.try_normalize()? * depth;
    let outline = outline(profile)?;
    let base = outline.iter().map(|p| Point3::new(p.x, p.y, 0.0));
    let top = base.clone().map(|p| p + offset);
    Some(base.chain(top).collect())
}

/// The eight corners of a block, or `None` for a non-positive extent.
///
/// The neutral block is centred on its origin, as the compiler builds it.
/// `lower::csg` puts the half-extent shift that gives `IfcBlock` its corner
/// anchor on the enclosing `Instance`, so reading the corners here the way
/// the compiler does keeps this box and the compiled mesh identical.
pub(super) fn block(x: Scalar, y: Scalar, z: Scalar) -> Option<[Point3; 8]> {
    if ![x, y, z].iter().all(|v| v.is_finite() && *v > 0.0) {
        return None;
    }
    let (hx, hy, hz) = (x / 2.0, y / 2.0, z / 2.0);
    Some([
        Point3::new(-hx, -hy, -hz),
        Point3::new(hx, -hy, -hz),
        Point3::new(-hx, hy, -hz),
        Point3::new(hx, hy, -hz),
        Point3::new(-hx, -hy, hz),
        Point3::new(hx, -hy, hz),
        Point3::new(-hx, hy, hz),
        Point3::new(hx, hy, hz),
    ])
}

/// The vertices of a straight-edged profile's outer boundary.
///
/// Holes never extend a bound, so only the outer boundary is read.
fn outline(profile: &Profile) -> Option<Vec<Point2>> {
    match profile {
        // Centred on the profile origin; a hollow section has the same
        // outer boundary. Corner radii cut the corners off, so a rounded
        // rectangle is not a polygon.
        Profile::Rectangle(rectangle) => {
            let rounded = rectangle.outer_radius.is_some_and(|r| r != 0.0);
            let (x, y) = (rectangle.x, rectangle.y);
            if rounded || !(x.is_finite() && y.is_finite() && x > 0.0 && y > 0.0) {
                return None;
            }
            let (hx, hy) = (x / 2.0, y / 2.0);
            Some(vec![
                Point2::new(-hx, -hy),
                Point2::new(hx, -hy),
                Point2::new(hx, hy),
                Point2::new(-hx, hy),
            ])
        }
        Profile::Contour(contour) => straight_contour(&contour.outer),
        Profile::Derived { basis, transform } => Some(
            outline(basis)?
                .into_iter()
                .map(|p| transform.transform_point2(p))
                .collect(),
        ),
        _ => None,
    }
}

/// Endpoints of a contour made only of line segments.
fn straight_contour(contour: &Contour) -> Option<Vec<Point2>> {
    let mut points = Vec::with_capacity(contour.segments.len() * 2);
    for segment in &contour.segments {
        let Curve2::Line(line) = &segment.curve else {
            return None;
        };
        let domain = segment.domain;
        points.push(line.origin + line.direction * domain.start);
        points.push(line.origin + line.direction * domain.end);
    }
    let finite = points.iter().all(|p| p.is_finite());
    // Fewer than three segments encloses no area; the compiler names that.
    (finite && contour.segments.len() >= 3).then_some(points)
}
