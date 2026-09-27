//! Compile-and-run proof for the `ifc-geometry` code on the docs site.
//!
//! The lines between `// docs:snippet <name>` and `// docs:end` are copied
//! verbatim into the pages by `cargo run -p xtask -- docs`.
#![cfg(feature = "compile-reference-backend")]

use std::path::PathBuf;

use ifc_geometry::GeometryError;
use ifc_model::{Codec, Model};
use ifc_step::StepCodec;

fn coverage_fixture() -> Model {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../test/fixtures/synthetic-coverage/meshing_coverage.ifc");
    StepCodec
        .read_path(&path)
        .expect("the coverage fixture parses")
}

/// `docs/architecture/axiolid-boundary.md` -- compiling one product.
#[test]
fn documented_compile_example_meshes_a_product() -> Result<(), GeometryError> {
    let model = coverage_fixture();
    let product = model
        .ids()
        .find(|&id| {
            model
                .get(id)
                .and_then(|e| e.text(2))
                .is_some_and(|name| name == "polygonal-face-set-quads")
        })
        .expect("the fixture names its products");
    // docs:snippet compile-product-mesh
    use axiolid_core::Tolerance;
    use ifc_geometry::compile::compile_product_mesh; // feature = "compile-reference-backend"

    let mesh = compile_product_mesh(&model, product, Tolerance::MILLIMETRE)?;
    // docs:end
    assert!(mesh.is_some_and(|mesh| mesh.triangles().len() > 0));
    Ok(())
}
