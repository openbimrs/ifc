//! Profile -> 2D polygon rings, then triangles.
//!
//! Curved boundaries are flattened to chords under an explicit
//! `TessellationOptions`-style budget; nothing here invents a default
//! tolerance.
//!
//! Triangulation of rings-with-holes is delegated to `earcut` (MIT/Apache-2.0,
//! pure Rust). ADR 0015 records why: hole bridging is a solved problem and a
//! hand-rolled version failed its own area gate on the two-hole case.
//! `geom_scalar::triangulate_simple` is retained as the differential oracle for
//! the hole-free case, so the adopted implementation is audited, not trusted.

use geom_core::{Point2, Scalar, Tolerance};
use geom_kernel::{GeomError, GeomResult};
use geom_profile::{CircleProfile, Profile, RectangleProfile};

/// Outer ring plus holes, all CCW/CW normalised by the caller's contract:
/// outer counter-clockwise, holes clockwise.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Rings {
    /// Outer boundary, counter-clockwise.
    pub outer: Vec<Point2>,
    /// Inner boundaries, clockwise.
    pub holes: Vec<Vec<Point2>>,
}

/// Number of chord segments needed to approximate a full circle of `radius`
/// within `chord_error`.
///
/// From the sagitta relation `e = r (1 - cos(pi/n))`. Clamped to [8, 512]: a
/// coarse budget must still produce a recognisable disk, and an absurdly tight
/// one must not explode the mesh.
fn circle_segments(radius: Scalar, chord_error: Scalar) -> usize {
    if radius <= 0.0 || chord_error <= 0.0 {
        return 8;
    }
    let ratio = 1.0 - (chord_error / radius);
    if ratio <= -1.0 {
        return 8;
    }
    let n = core::f64::consts::PI / ratio.clamp(-1.0, 1.0).acos();
    (n.ceil() as usize).clamp(8, 512)
}

/// Sample a circle counter-clockwise.
fn circle_ring(cx: Scalar, cy: Scalar, radius: Scalar, segments: usize) -> Vec<Point2> {
    (0..segments)
        .map(|i| {
            let t = (i as Scalar) * core::f64::consts::TAU / (segments as Scalar);
            Point2::new(cx + radius * t.cos(), cy + radius * t.sin())
        })
        .collect()
}

/// Flatten a profile into rings under an explicit chord budget.
///
/// Only the families a format adapter currently emits are handled. Everything
/// else returns `Unsupported` rather than a silently wrong approximation --
/// a wrong wall is far more expensive than a missing one.
pub fn profile_rings(
    profile: &Profile,
    chord_error: Scalar,
    tolerance: Tolerance,
) -> GeomResult<Rings> {
    match profile {
        Profile::Rectangle(r) => rectangle_rings(r, chord_error, tolerance),
        Profile::Circle(c) => circle_rings(c, chord_error),
        other => Err(GeomError::Unsupported {
            backend: crate::BACKEND_ID,
            operation: geom_kernel::Operation::ProfileTriangulation,
        })
        .inspect_err(|_| {
            let _ = other;
        }),
    }
}

/// Rectangle, optionally hollow. Corner radii are not yet approximated.
fn rectangle_rings(
    r: &RectangleProfile,
    _chord_error: Scalar,
    tolerance: Tolerance,
) -> GeomResult<Rings> {
    if !(r.x.is_finite() && r.y.is_finite()) || r.x <= 0.0 || r.y <= 0.0 {
        return Err(GeomError::InvalidInput(format!(
            "rectangle profile must have positive finite extents, got {} x {}",
            r.x, r.y
        )));
    }
    let (hx, hy) = (r.x / 2.0, r.y / 2.0);
    let outer = vec![
        Point2::new(-hx, -hy),
        Point2::new(hx, -hy),
        Point2::new(hx, hy),
        Point2::new(-hx, hy),
    ];
    let mut holes = Vec::new();
    if let Some(t) = r.thickness {
        if t <= 0.0 || 2.0 * t >= r.x || 2.0 * t >= r.y {
            return Err(GeomError::InvalidInput(format!(
                "hollow rectangle wall thickness {t} does not fit inside {} x {}",
                r.x, r.y
            )));
        }
        let (ix, iy) = (hx - t, hy - t);
        if !tolerance.eq(ix, 0.0) && !tolerance.eq(iy, 0.0) {
            // Clockwise: opposite winding to the outer ring marks it a hole.
            holes.push(vec![
                Point2::new(-ix, -iy),
                Point2::new(-ix, iy),
                Point2::new(ix, iy),
                Point2::new(ix, -iy),
            ]);
        }
    }
    Ok(Rings { outer, holes })
}

/// Circle, optionally annular.
fn circle_rings(c: &CircleProfile, chord_error: Scalar) -> GeomResult<Rings> {
    if !c.radius.is_finite() || c.radius <= 0.0 {
        return Err(GeomError::InvalidInput(format!(
            "circle profile radius must be positive and finite, got {}",
            c.radius
        )));
    }
    let segments = circle_segments(c.radius, chord_error);
    let outer = circle_ring(0.0, 0.0, c.radius, segments);
    let mut holes = Vec::new();
    if let Some(t) = c.thickness {
        if t <= 0.0 || t >= c.radius {
            return Err(GeomError::InvalidInput(format!(
                "annulus wall thickness {t} does not fit inside radius {}",
                c.radius
            )));
        }
        let inner = c.radius - t;
        let mut ring = circle_ring(0.0, 0.0, inner, circle_segments(inner, chord_error));
        ring.reverse(); // clockwise
        holes.push(ring);
    }
    Ok(Rings { outer, holes })
}

/// Triangulate rings into a flat index buffer over a single vertex list.
///
/// Delegates to `earcut`. The returned vertices are the concatenation
/// `outer ++ holes`, matching earcut's hole-index convention.
pub fn triangulate(rings: &Rings) -> GeomResult<(Vec<Point2>, Vec<[u32; 3]>)> {
    if rings.outer.len() < 3 {
        return Err(GeomError::InvalidInput(format!(
            "profile outer ring needs at least 3 vertices, got {}",
            rings.outer.len()
        )));
    }
    let mut verts: Vec<[Scalar; 2]> = rings.outer.iter().map(|p| [p.x, p.y]).collect();
    let mut hole_starts = Vec::with_capacity(rings.holes.len());
    for hole in &rings.holes {
        if hole.len() < 3 {
            return Err(GeomError::InvalidInput(format!(
                "profile hole needs at least 3 vertices, got {}",
                hole.len()
            )));
        }
        hole_starts.push(verts.len());
        verts.extend(hole.iter().map(|p| [p.x, p.y]));
    }

    let mut earcutter = earcut::Earcut::new();
    let mut flat: Vec<usize> = Vec::new();
    earcutter.earcut(verts.iter().copied(), &hole_starts, &mut flat);

    if flat.is_empty() || flat.len() % 3 != 0 {
        return Err(GeomError::Degenerate(format!(
            "triangulation produced {} indices for a {}-vertex profile",
            flat.len(),
            verts.len()
        )));
    }
    let tris = flat
        .chunks_exact(3)
        .map(|c| [c[0] as u32, c[1] as u32, c[2] as u32])
        .collect();
    let points = verts.into_iter().map(|v| Point2::new(v[0], v[1])).collect();
    Ok((points, tris))
}
