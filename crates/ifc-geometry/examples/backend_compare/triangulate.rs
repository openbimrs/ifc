//! Planar polygon triangulation for the example kernel: ear clipping, with
//! holes bridged into the outer ring first.
//!
//! Plain O(n²) ear clipping, which is ample for the corpus and short enough
//! to read. A polygon it cannot triangulate (self-intersecting, or a hole
//! no bridge can reach) is refused as degenerate, never approximated.

use axiolid_contracts::{GeomError, GeomResult};
use axiolid_core::Point2;

/// The shoelace area of a ring of point indices, positive when the ring
/// runs counter-clockwise.
pub fn signed_area(points: &[Point2], ring: &[usize]) -> f64 {
    let n = ring.len();
    (0..n)
        .map(|i| points[ring[i]].perp_dot(points[ring[(i + 1) % n]]))
        .sum::<f64>()
        / 2.0
}

/// Orient rings for a region: the first (outer) counter-clockwise, every
/// other (a hole) clockwise. Only the index order changes.
pub fn orient(points: &[Point2], mut rings: Vec<Vec<usize>>) -> Vec<Vec<usize>> {
    for (index, ring) in rings.iter_mut().enumerate() {
        let counter_clockwise = signed_area(points, ring) > 0.0;
        if counter_clockwise != (index == 0) {
            ring.reverse();
        }
    }
    rings
}

/// Triangulate the region `rings` bounds, oriented as [`orient`] leaves
/// them. Returns counter-clockwise index triples into `points`.
pub fn triangulate(
    points: &[Point2],
    rings: &[Vec<usize>],
    epsilon: f64,
) -> GeomResult<Vec<[usize; 3]>> {
    let (outer, holes) = rings
        .split_first()
        .ok_or_else(|| GeomError::Degenerate("polygon has no boundary".into()))?;
    let mut ring = outer.clone();
    let mut holes: Vec<&Vec<usize>> = holes.iter().collect();
    // Rightmost hole first, as the classic construction does. Each bridge is
    // checked against the ring merged so far, earlier bridges included, and
    // against the holes still pending, so no two bridges cross.
    holes.sort_by(|a, b| rightmost_x(points, b).total_cmp(&rightmost_x(points, a)));
    for (index, hole) in holes.iter().enumerate() {
        ring = bridge(points, &ring, hole, &holes[index + 1..])?;
    }
    ear_clip(points, ring, epsilon)
}

fn rightmost_x(points: &[Point2], ring: &[usize]) -> f64 {
    ring.iter()
        .map(|&i| points[i].x)
        .fold(f64::NEG_INFINITY, f64::max)
}

/// Splice `hole` into `ring` along a bridge from the hole's rightmost
/// vertex to the nearest ring vertex it can see.
fn bridge(
    points: &[Point2],
    ring: &[usize],
    hole: &[usize],
    pending: &[&Vec<usize>],
) -> GeomResult<Vec<usize>> {
    let start = (0..hole.len())
        .max_by(|&a, &b| points[hole[a]].x.total_cmp(&points[hole[b]].x))
        .ok_or_else(|| GeomError::Degenerate("hole has no vertices".into()))?;
    let m = points[hole[start]];
    let mut candidates: Vec<usize> = (0..ring.len()).collect();
    candidates.sort_by(|&a, &b| {
        m.distance_squared(points[ring[a]])
            .total_cmp(&m.distance_squared(points[ring[b]]))
    });
    let edges = || {
        std::iter::once(ring)
            .chain(std::iter::once(hole))
            .chain(pending.iter().map(|h| h.as_slice()))
            .flat_map(|r| (0..r.len()).map(move |i| (points[r[i]], points[r[(i + 1) % r.len()]])))
    };
    let visible = candidates
        .into_iter()
        .find(|&c| {
            let p = points[ring[c]];
            edges().all(|(a, b)| !crosses(m, p, a, b))
        })
        .ok_or_else(|| GeomError::Degenerate("no bridge reaches a hole".into()))?;
    let mut merged = Vec::with_capacity(ring.len() + hole.len() + 2);
    merged.extend_from_slice(&ring[..=visible]);
    merged.extend((0..=hole.len()).map(|k| hole[(start + k) % hole.len()]));
    merged.extend_from_slice(&ring[visible..]);
    Ok(merged)
}

/// Whether segments `pq` and `ab` cross at a point interior to both.
/// Segments that share an endpoint do not cross.
fn crosses(p: Point2, q: Point2, a: Point2, b: Point2) -> bool {
    if p == a || p == b || q == a || q == b {
        return false;
    }
    let side = |o: Point2, s: Point2, t: Point2| (s - o).perp_dot(t - o);
    let (d1, d2) = (side(p, q, a), side(p, q, b));
    let (d3, d4) = (side(a, b, p), side(a, b, q));
    d1 * d2 < 0.0 && d3 * d4 < 0.0
}

/// Ear clipping of one counter-clockwise ring of point indices.
///
/// A collinear vertex is clipped as a zero-area ear only when no proper ear
/// is left, so the triangles cover the polygon exactly.
fn ear_clip(points: &[Point2], mut ring: Vec<usize>, epsilon: f64) -> GeomResult<Vec<[usize; 3]>> {
    let mut triangles = Vec::with_capacity(ring.len().saturating_sub(2));
    while ring.len() > 3 {
        let ear = find_ear(points, &ring, epsilon, false)
            .or_else(|| find_ear(points, &ring, epsilon, true))
            .ok_or_else(|| GeomError::Degenerate("polygon is not simple".into()))?;
        let n = ring.len();
        triangles.push([ring[(ear + n - 1) % n], ring[ear], ring[(ear + 1) % n]]);
        ring.remove(ear);
    }
    if let [a, b, c] = ring[..] {
        triangles.push([a, b, c]);
    }
    Ok(triangles)
}

fn find_ear(points: &[Point2], ring: &[usize], epsilon: f64, flat: bool) -> Option<usize> {
    let n = ring.len();
    (0..n).find(|&i| {
        let (a, b, c) = (
            points[ring[(i + n - 1) % n]],
            points[ring[i]],
            points[ring[(i + 1) % n]],
        );
        let turn = (b - a).perp_dot(c - b);
        let shaped = if flat {
            turn.abs() <= epsilon
        } else {
            turn > epsilon
        };
        // Compared by position, not index: a bridged hole repeats two
        // positions, and those must not block the ears they bound.
        shaped
            && !ring.iter().any(|&other| {
                let p = points[other];
                p != a && p != b && p != c && inside_triangle(p, a, b, c)
            })
    })
}

/// Inside or on the boundary of the counter-clockwise triangle `abc`.
fn inside_triangle(p: Point2, a: Point2, b: Point2, c: Point2) -> bool {
    (b - a).perp_dot(p - a) >= 0.0
        && (c - b).perp_dot(p - b) >= 0.0
        && (a - c).perp_dot(p - c) >= 0.0
}
