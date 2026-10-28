//! A small polygonal boundary clips a tall wall to its full height (#519).
//!
//! `tall_wall_small_bounded_clip_ifc2x3.ifc` is a 10125 x 350 mm wall
//! extruded 16165.033 mm, minus an `IfcPolygonalBoundedHalfSpace`: the
//! horizontal plane z = 11404.747 mm, `AgreementFlag` `.F.` (the material
//! above it), bounded by the 238.549 x 350 mm strip at the wall's end
//! (x in [9886.451, 10125]). The clip removes that strip from z = 11404.747
//! mm to the top, 4760.286 mm, and the wall keeps a 238.549 mm notch there.
//! IfcOpenShell 0.8.5 meshes it to 12 vertices and 56.887388853 m³.
//!
//! The clip lowers to Axiolid's `BoundedHalfSpaceAlongZ`, which carries no
//! depth. Up to axiolid-construct 0.3.18 the mesh compiler sized the slab
//! from the boundary alone, at twice its largest coordinate (0.7 m here),
//! so the mesh removed only the strip's lowest 0.7 m. Since 0.3.19
//! (axiolid-mesh-compile 0.3.27, axiolid/kernel#325) both compilers size it
//! from the subject: the mesh is the analytic body, certified.
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

/// The clipped wall: the extrusion minus the strip above the plane,
/// 56.887388852960 m³.
fn analytic() -> f64 {
    LENGTH * THICKNESS * HEIGHT - STRIP * THICKNESS * (HEIGHT - PLANE)
}

#[test]
fn the_analytic_volume_is_the_kernel_tests() {
    assert!(
        (analytic() - 56.887_388_852_960).abs() < 1e-11,
        "{}",
        analytic()
    );
}

fn volume(mesh: &TriMesh) -> f64 {
    assert!(!mesh.indices.is_empty(), "compiled mesh is empty");
    let base = mesh.positions[0];
    mesh.indices
        .as_chunks::<3>()
        .0
        .iter()
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

/// The mesh is the analytic body, its end cut at the plane and nowhere
/// above it.
fn assert_analytic(mesh: &TriMesh) {
    let round = |z: f64| (z * 1e6).round() / 1e6;
    let v = volume(mesh);
    assert!(
        (v - analytic()).abs() < 1e-9,
        "mesh volume {v}, expected the analytic {}",
        analytic()
    );
    assert_eq!(end_heights(mesh), [0.0, round(PLANE)], "volume {v}");
}

fn model() -> Model {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../test/fixtures/synthetic-compile/tall_wall_small_bounded_clip_ifc2x3.ifc");
    StepCodec.read_path(&path).expect("fixture parses")
}

/// Both compilers size the cut from the wall and build the notch.
#[test]
fn the_exact_body_and_the_mesh_are_cut_to_the_top_of_the_wall() {
    let model = model();
    let scale = units::resolve(&model).expect("project units");
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

    // The mesh: the analytic body, certified against the exact one.
    let (outcome, report) = compile::default_backend()
        .compile_mesh_with_deviation(&lowered.graph, lowered.root, &options)
        .expect("compiles with a report");
    assert_analytic(&outcome.mesh);
    assert!(report.meets_requested(), "{report:?}");
}

/// The product entry point gives the same mesh, so the pin above covers what
/// a consumer meshes.
#[test]
fn the_product_mesh_is_the_analytic_body() {
    let mesh = compile_product_mesh(&model(), WALL, Tolerance::MILLIMETRE)
        .expect("compiles")
        .expect("the wall has a Body");
    assert_analytic(&mesh);
}
