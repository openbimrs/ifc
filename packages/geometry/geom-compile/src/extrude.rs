//! Linear extrusion of a triangulated profile into a closed solid.
//!
//! The result must be watertight and outward-oriented, because that is exactly
//! what `geom-boolmesh` demands of its inputs. Getting the winding wrong here
//! produces a mesh that looks valid and computes wrong booleans -- the failure
//! mode that has already cost two debugging sessions.

use geom_core::{Point2, Point3, Scalar, Vec3};
use geom_kernel::{GeomError, GeomResult};
use geom_mesh::TriMesh;

/// Extrude a triangulated 2D profile along `direction` by `depth`.
///
/// The profile lies in the local z = 0 plane. Caps use the triangulation;
/// sides are quads split into two triangles per boundary edge.
///
/// `boundary` lists the closed loops of the profile as index ranges into
/// `points`: each loop is a contiguous run, matching `profile::Rings` layout.
pub fn extrude(
    points: &[Point2],
    triangles: &[[u32; 3]],
    loops: &[core::ops::Range<usize>],
    direction: Vec3,
    depth: Scalar,
) -> GeomResult<TriMesh> {
    if !depth.is_finite() || depth <= 0.0 {
        return Err(GeomError::InvalidInput(format!(
            "extrusion depth must be positive and finite, got {depth}"
        )));
    }
    if !direction.is_finite() || direction.length() <= 0.0 {
        return Err(GeomError::InvalidInput(
            "extrusion direction must be a finite non-zero vector".to_owned(),
        ));
    }
    let offset = direction.normalize() * depth;
    if !offset.is_finite() {
        return Err(GeomError::Degenerate(
            "extrusion direction could not be normalised".to_owned(),
        ));
    }

    let n = points.len();
    let mut positions = Vec::with_capacity(n * 2);
    // Base ring first, then the offset ring: vertex i has its twin at i + n.
    positions.extend(points.iter().map(|p| Point3::new(p.x, p.y, 0.0)));
    positions.extend(points.iter().map(|p| Point3::new(p.x, p.y, 0.0) + offset));

    let mut indices: Vec<u32> = Vec::with_capacity(triangles.len() * 6 + n * 6);
    let top = n as u32;

    // Caps. The profile triangulation is counter-clockwise seen from +z, which
    // is outward for the TOP cap and inward for the bottom, so the bottom is
    // emitted reversed.
    for t in triangles {
        indices.extend_from_slice(&[t[0] + top, t[1] + top, t[2] + top]);
        indices.extend_from_slice(&[t[0], t[2], t[1]]);
    }

    // Sides. Each boundary edge (a -> b) becomes the quad a, b, b', a'.
    for range in loops {
        let len = range.len();
        if len < 3 {
            return Err(GeomError::InvalidInput(format!(
                "extrusion loop needs at least 3 vertices, got {len}"
            )));
        }
        for k in 0..len {
            let a = (range.start + k) as u32;
            let b = (range.start + (k + 1) % len) as u32;
            indices.extend_from_slice(&[a, b, b + top]);
            indices.extend_from_slice(&[a, b + top, a + top]);
        }
    }

    Ok(TriMesh::new(positions, indices))
}

/// Triangulate rings and extrude them in one step.
///
/// The loop layout must match `triangulate`'s vertex order exactly, so the
/// two are derived from the same `Rings` value here rather than by a caller
/// reconstructing the ranges.
pub fn extrude_profile(
    rings: &crate::profile::Rings,
    direction: Vec3,
    depth: Scalar,
    _tolerance: geom_core::Tolerance,
) -> GeomResult<TriMesh> {
    let (points, triangles) = crate::profile::triangulate(rings)?;
    let mut loops = Vec::with_capacity(1 + rings.holes.len());
    let mut start = 0usize;
    loops.push(start..rings.outer.len());
    start += rings.outer.len();
    for hole in &rings.holes {
        loops.push(start..start + hole.len());
        start += hole.len();
    }
    extrude(&points, &triangles, &loops, direction, depth)
}
