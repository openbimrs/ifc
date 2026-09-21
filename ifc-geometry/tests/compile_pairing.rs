//! Lowering and compilation must stay paired.
//!
//! `ifc-geometry` lowers 68 representation-item families; the Axiolid mesh
//! compiler evaluates ten solid families. Those two sets agree today, and
//! nothing but this test keeps them agreeing: either side can evolve and
//! turn a previously-compiling product into a refusal without any signal.
//!
//! The assertion is deliberately not "everything compiles". A refusal is a
//! legitimate outcome -- the compiler rejects unbounded operations, for one
//! -- so what gets pinned is that every product reaches a TYPED answer:
//! triangles, or `CompilationRefused` naming the reason. A panic, a hang, or
//! a silently empty mesh is the failure this catches.
#![cfg(feature = "compile-reference-backend")]

use axiolid_core::Tolerance;
use ifc_geometry::compile::compile_product_mesh;
use ifc_geometry::lower::geometric_products;
use ifc_model::{Codec, Model};
use ifc_step::StepCodec;
use std::path::PathBuf;

fn fixture(subdir: &str, name: &str) -> Model {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../test/fixtures")
        .join(subdir)
        .join(name);
    StepCodec
        .read_path(&path)
        .unwrap_or_else(|e| panic!("fixture {} must parse: {e:?}", path.display()))
}

/// Fixtures chosen to span the compiler's hard paths, not the easy ones.
///
/// CSG, a half-space clip, a surface-curve sweep, overlapping boolean
/// openings, and nested mapped items each exercise a different compiler
/// branch. The expected count is the number of products carrying a body
/// representation, which is what `compile_product_mesh` answers for.
const CORPUS: &[(&str, &str, usize)] = &[
    ("ifclite-geometry", "bath_csg_solid.ifc", 1),
    ("ifclite-geometry", "issue_098_wall_W.ifc", 15),
    ("ifclite-geometry", "issue_1155_halfspace_flyaway.ifc", 1),
    (
        "ifclite-geometry",
        "issue_1485_duct_elbow_surface_curve_swept.ifc",
        2,
    ),
    (
        "ifclite-geometry",
        "issue_2019_wall_two_overlapping_openings.ifc",
        4,
    ),
    ("ifclite-geometry", "mapped_instances_nested.ifc", 3),
    // Authored to be refused: a UNION whose right operand is a half-space is
    // unbounded, so the compiler cannot produce a finite mesh. Without this
    // entry the refusal branch below never executes and its assertion is
    // unfalsifiable -- every other fixture in this corpus compiles cleanly.
    ("synthetic-compile", "union_over_halfspace_unbounded.ifc", 1),
];

#[test]
fn every_lowered_product_reaches_a_typed_compilation_answer() {
    for (subdir, name, expected_bodies) in CORPUS {
        let model = fixture(subdir, name);
        let mut bodies = 0usize;
        for product in geometric_products(&model) {
            match compile_product_mesh(&model, product, Tolerance::MILLIMETRE) {
                Ok(Some(mesh)) => {
                    bodies += 1;
                    assert!(
                        mesh.triangle_count() > 0,
                        "{name}: {product} compiled to an empty mesh; an empty \
                         result is indistinguishable from success and must be a \
                         refusal instead"
                    );
                }
                // No body representation: ordinary for a spatial container.
                Ok(None) => {}
                Err(error) => {
                    bodies += 1;
                    assert!(
                        error.entity().is_some(),
                        "{name}: refusal for {product} lost its entity \
                         attribution: {error}"
                    );
                }
            }
        }
        assert_eq!(
            bodies, *expected_bodies,
            "{name}: body-representation count changed; lowering or fixture \
             drifted"
        );
    }
}
