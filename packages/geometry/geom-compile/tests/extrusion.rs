//! Gates for profile flattening and extrusion.
//!
//! Signed volume is the single check that catches wrong winding, missing caps,
//! and inverted sides simultaneously: it is positive exactly when the solid is
//! closed and outward-oriented, and its magnitude is `area * depth`.

use geom_compile::extrude::extrude;
use geom_compile::profile::{profile_rings, triangulate, Rings};
use geom_core::{Tolerance, Vec3};
use geom_mesh::TriMesh;
use geom_profile::{CircleProfile, Profile, RectangleProfile};

/// Six times the signed volume, via the divergence theorem.
fn six_volume(m: &TriMesh) -> f64 {
    m.indices
        .chunks_exact(3)
        .map(|t| {
            let (a, b, c) = (
                m.positions[t[0] as usize],
                m.positions[t[1] as usize],
                m.positions[t[2] as usize],
            );
            a.dot(b.cross(c))
        })
        .sum()
}

fn volume(m: &TriMesh) -> f64 {
    six_volume(m) / 6.0
}

/// Loop ranges for a `Rings` laid out as outer ++ holes.
#[allow(clippy::single_range_in_vec_init)]
fn loops(rings: &Rings) -> Vec<core::ops::Range<usize>> {
    let mut out = vec![0..rings.outer.len()];
    let mut start = rings.outer.len();
    for h in &rings.holes {
        out.push(start..start + h.len());
        start += h.len();
    }
    out
}

fn rect(x: f64, y: f64, thickness: Option<f64>) -> Profile {
    Profile::Rectangle(RectangleProfile {
        x,
        y,
        thickness,
        outer_radius: None,
        inner_radius: None,
    })
}

/// A solid box: volume must equal x * y * depth, and be POSITIVE.
///
/// A negative result means the solid is inside-out, which `geom-boolmesh`
/// refuses -- so this assertion is what keeps the two crates composable.
#[test]
fn a_box_extrudes_to_its_exact_volume_with_outward_winding() {
    let rings = profile_rings(&rect(4.0, 0.2, None), 1e-3, Tolerance::METRE).expect("rings");
    let (pts, tris) = triangulate(&rings).expect("triangulate");
    let mesh = extrude(&pts, &tris, &loops(&rings), Vec3::Z, 3.0).expect("extrude");

    let v = volume(&mesh);
    assert!(v > 0.0, "extruded solid must be outward-oriented, got {v}");
    assert!((v - 4.0 * 0.2 * 3.0).abs() < 1e-9, "expected 2.4, got {v}");
    assert!(mesh.validate_structure().is_ok());
}

/// A hollow section must lose exactly the hole's volume. This is the case the
/// hand-rolled ear clipper could not do, and the reason earcut was adopted.
#[test]
fn a_hollow_section_loses_exactly_the_hole_volume() {
    let rings = profile_rings(&rect(10.0, 6.0, Some(1.0)), 1e-3, Tolerance::METRE).expect("rings");
    assert_eq!(rings.holes.len(), 1, "hollow rectangle must produce a hole");

    let (pts, tris) = triangulate(&rings).expect("triangulate");
    let mesh = extrude(&pts, &tris, &loops(&rings), Vec3::Z, 2.0).expect("extrude");

    // Outer 10x6, inner 8x4 -> ring area 60 - 32 = 28.
    let expected = 28.0 * 2.0;
    let v = volume(&mesh);
    assert!(v > 0.0, "hollow solid must be outward-oriented, got {v}");
    assert!((v - expected).abs() < 1e-9, "expected {expected}, got {v}");
}

/// A disk's volume converges on pi r^2 h as the chord budget tightens, and the
/// approximation is always an INSCRIBED polygon, so it never overshoots.
#[test]
fn a_disk_converges_from_below_as_the_chord_budget_tightens() {
    let exact = core::f64::consts::PI * 4.0 * 1.5;
    let mut previous = 0.0;
    for chord in [1e-1, 1e-2, 1e-3, 1e-4] {
        let profile = Profile::Circle(CircleProfile {
            radius: 2.0,
            thickness: None,
        });
        let rings = profile_rings(&profile, chord, Tolerance::METRE).expect("rings");
        let (pts, tris) = triangulate(&rings).expect("triangulate");
        let mesh = extrude(&pts, &tris, &loops(&rings), Vec3::Z, 1.5).expect("extrude");
        let v = volume(&mesh);

        assert!(
            v > 0.0 && v < exact,
            "inscribed volume {v} must be under {exact}"
        );
        assert!(v > previous, "tightening the budget must not lose volume");
        previous = v;
    }
    // The segment count is clamped at 512, so the tightest achievable relative
    // volume error is set by that clamp, not by the requested chord budget:
    // an inscribed n-gon loses O(1/n^2) of the area. 512 segments gives ~2.5e-5.
    // Asserting a tighter bound would be asserting against the clamp we chose.
    assert!(
        (exact - previous).abs() / exact < 1e-4,
        "tightest budget should be within 1e-4 relative, got {previous} vs {exact}"
    );
}

/// Every directed edge must appear exactly once.
///
/// This is the gate that volume CANNOT provide: a cap lying in the z = 0 plane
/// contributes nothing to the divergence integral, so a flipped bottom cap is
/// invisible to a volume check. Directed-edge parity is sensitive to winding
/// everywhere, and it is precisely what `boolmesh::Manifold::new` enforces on
/// input -- so this test is the local proxy for "will the boolean accept it".
fn assert_edge_manifold(mesh: &TriMesh, what: &str) {
    use std::collections::HashMap;
    let mut directed: HashMap<(u32, u32), i32> = HashMap::new();
    for t in mesh.indices.chunks_exact(3) {
        for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
            *directed.entry((a, b)).or_default() += 1;
        }
    }
    for (&(a, b), &count) in &directed {
        assert_eq!(
            count, 1,
            "{what}: directed edge {a}->{b} appears {count} times"
        );
        let opposite = directed.get(&(b, a)).copied().unwrap_or(0);
        assert_eq!(
            opposite, 1,
            "{what}: edge {a}->{b} has {opposite} opposing half-edges, so the \
             surface is not closed and consistently wound"
        );
    }
}

#[test]
fn a_box_is_edge_manifold() {
    let rings = profile_rings(&rect(4.0, 0.2, None), 1e-3, Tolerance::METRE).expect("rings");
    let (pts, tris) = triangulate(&rings).expect("triangulate");
    let mesh = extrude(&pts, &tris, &loops(&rings), Vec3::Z, 3.0).expect("extrude");
    assert_edge_manifold(&mesh, "solid box");
}

#[test]
fn a_hollow_section_is_edge_manifold() {
    let rings = profile_rings(&rect(10.0, 6.0, Some(1.0)), 1e-3, Tolerance::METRE).expect("rings");
    let (pts, tris) = triangulate(&rings).expect("triangulate");
    let mesh = extrude(&pts, &tris, &loops(&rings), Vec3::Z, 2.0).expect("extrude");
    assert_edge_manifold(&mesh, "hollow section");
}
