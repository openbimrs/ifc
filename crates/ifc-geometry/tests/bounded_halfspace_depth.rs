//! A small polygonal boundary clips a tall wall to its full height, or the
//! mesh is flagged as off its exact result (#519).
//!
//! `tall_wall_small_bounded_clip_ifc2x3.ifc` is a 10125 x 350 mm wall
//! extruded 16165.033 mm, minus an `IfcPolygonalBoundedHalfSpace`: the
//! horizontal plane z = 11404.747 mm, `AgreementFlag` `.F.` (the material
//! above it), bounded by the 238.549 x 350 mm strip at the wall's end
//! (x in [9886.451, 10125]). The clip removes that strip from z = 11404.747
//! mm to the top, 4760.286 mm, and the wall keeps a 238.549 mm notch there.
//! IfcOpenShell 0.8.5 meshes it to 12 vertices and 56.887388853 m³.
//!
//! `Position`'s Z is along the plane normal, so the clip lowers to Axiolid's
//! `BoundedHalfSpace` as authored. Axiolid's exact compiler sizes that prism
//! from the subject and builds the analytic body. Its mesh compiler sizes
//! the slab from the boundary alone, at twice its largest coordinate
//! (axiolid-construct 0.3.18 `half_space.rs`, axiolid/kernel#325): 0.7 m
//! here, so the mesh removes only the strip's lowest 0.7 m and the notch
//! reaches the wall's top again. That is wrong, but not silent: the mesh's
//! deviation report, measured against the exact body, does not meet the
//! requested tolerance. The mesh is accepted as that known kernel state
//! only together with that flag, and as no other volume.
#![cfg(feature = "compile-reference-backend")]

use std::path::PathBuf;

use axiolid_contracts::ExecutionOptions;
use axiolid_core::Tolerance;
use axiolid_mesh::TriMesh;
use axiolid_mesh_compile::ReferenceExactCompiler;
use ifc_geometry::compile::{self, compile_product_mesh};
use ifc_geometry::lower::{lower_product_representation, LoweringSession, RepresentationPurpose};
use ifc_geometry::units;
use ifc_model::{Codec, EntityId, Model};
use ifc_step::StepCodec;

const WALL: EntityId = EntityId(52);

/// The wall's extrusion, in metres.
const LENGTH: f64 = 10.125;
const THICKNESS: f64 = 0.35;
const HEIGHT: f64 = 16.165_033_064_627_303;
/// The clip plane's height and the boundary's length along the wall.
const PLANE: f64 = 11.404_747_431_413_967;
const STRIP: f64 = 0.238_549_352_729_28;

/// The clipped wall: the extrusion minus the strip above the plane.
fn analytic() -> f64 {
    LENGTH * THICKNESS * HEIGHT - STRIP * THICKNESS * (HEIGHT - PLANE)
}

/// What axiolid/kernel#325's slab removes: the strip to a depth of twice
/// the boundary's largest coordinate (0.35 m) above the plane.
fn under_cut() -> f64 {
    LENGTH * THICKNESS * HEIGHT - STRIP * THICKNESS * (2.0 * THICKNESS)
}

fn volume(mesh: &TriMesh) -> f64 {
    assert!(!mesh.indices.is_empty(), "compiled mesh is empty");
    let base = mesh.positions[0];
    mesh.indices
        .chunks_exact(3)
        .map(|t| {
            let [a, b, c] = [t[0], t[1], t[2]].map(|i| mesh.positions[i as usize] - base);
            a.dot(b.cross(c)) / 6.0
        })
        .sum()
}

/// The heights of the mesh's vertices on the wall's clipped end, x = 10.125
/// m, ascending, to a micrometre.
fn end_heights(mesh: &TriMesh) -> Vec<f64> {
    let mut z: Vec<f64> = mesh
        .positions
        .iter()
        .filter(|p| (p.x - LENGTH).abs() < 1e-9)
        .map(|p| (p.z * 1e6).round() / 1e6)
        .collect();
    z.sort_by(f64::total_cmp);
    z.dedup();
    z
}

/// The mesh is the analytic body, its end cut at the plane; or the known
/// under-cut of axiolid/kernel#325, its end cut 0.7 m ABOVE the plane and
/// standing to the wall's top again. Returns whether it is the analytic one.
fn assert_analytic_or_under_cut(mesh: &TriMesh) -> bool {
    let round = |z: f64| (z * 1e6).round() / 1e6;
    let v = volume(mesh);
    let heights = end_heights(mesh);
    if (v - analytic()).abs() < 1e-9 {
        assert_eq!(heights, [0.0, round(PLANE)], "volume {v}");
        return true;
    }
    assert!(
        (v - under_cut()).abs() < 1e-9,
        "mesh volume {v}: neither the analytic {} nor the #325 under-cut {}",
        analytic(),
        under_cut()
    );
    assert_eq!(
        heights,
        [0.0, round(PLANE), round(PLANE + 2.0 * THICKNESS), round(HEIGHT)],
        "the under-cut must lie above the plane"
    );
    false
}

fn model() -> Model {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../test/fixtures/synthetic-compile/tall_wall_small_bounded_clip_ifc2x3.ifc");
    StepCodec.read_path(&path).expect("fixture parses")
}

/// The exact compiler sizes the prism from the wall and builds the notch.
#[test]
fn the_exact_body_is_cut_to_the_top_of_the_wall() {
    let model = model();
    let scale = units::resolve(&model);
    let mut session = LoweringSession::new(&model, &scale);
    let root = lower_product_representation(&mut session, WALL, RepresentationPurpose::Body)
        .expect("lowers")
        .expect("a body");
    let lowered = session.finish(root).expect("a valid graph");
    let options = ExecutionOptions::new(Tolerance::MILLIMETRE);
    let (body, _) = ReferenceExactCompiler::new()
        .compile_exact_with_report(&lowered.graph, lowered.root, &options)
        .expect("a placed extrusion clipped by a bounded half-space compiles exactly");
    let exact = axiolid_measure::exact_properties(&body, Tolerance::MILLIMETRE)
        .expect("measurable")
        .signed_volume;
    assert!(
        (exact - analytic()).abs() < 1e-9,
        "exact volume {exact}, expected {}",
        analytic()
    );

    // The mesh: the analytic body, certified; or the known under-cut of
    // axiolid/kernel#325, never certified.
    // TODO(#519): once the kernel release with axiolid/kernel#325 is
    // adopted, accept only the analytic volume, and require the report to
    // meet the requested tolerance.
    let (outcome, report) = compile::default_backend()
        .compile_mesh_with_deviation(&lowered.graph, lowered.root, &options)
        .expect("compiles with a report");
    if assert_analytic_or_under_cut(&outcome.mesh) {
        assert!(report.meets_requested(), "{report:?}");
    } else {
        assert!(
            !report.meets_requested(),
            "the #325 under-cut must not be certified: {report:?}"
        );
    }
}

/// The product entry point gives the same mesh, so the pin above covers what
/// a consumer meshes.
#[test]
fn the_product_mesh_is_the_analytic_body_or_the_known_under_cut() {
    let mesh = compile_product_mesh(&model(), WALL, Tolerance::MILLIMETRE)
        .expect("compiles")
        .expect("the wall has a Body");
    // TODO(#519): only the analytic body once axiolid/kernel#325 is in.
    assert_analytic_or_under_cut(&mesh);
}
