//! Meshing or colouring a subset gives each product exactly the entry the
//! whole-model call gives it (#608), on every committed fixture, with and
//! without the model's maintained reverse index.

#![cfg(all(feature = "step", feature = "mesh"))]

use std::path::{Path, PathBuf};

use ifc::{
    product_colours, product_meshes, product_meshes_for, Codec, EntityId, Model, ProductMesh,
    RepresentationPurpose, StepCodec,
};

/// Subset calls per fixture and call; the corpus has fixtures with
/// thousands of products, and each subset call is a full lookup.
const PER_FIXTURE: usize = 12;

fn fixtures(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .expect("fixture directory")
        .map(|entry| entry.expect("entry").path())
        .collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            fixtures(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "ifc") {
            out.push(path);
        }
    }
}

fn corpus() -> Vec<PathBuf> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../test/fixtures");
    let mut out = Vec::new();
    fixtures(&root, &mut out);
    assert!(
        out.len() >= 50,
        "the fixture walk found {} files",
        out.len()
    );
    out
}

fn read(path: &Path) -> Model {
    StepCodec::lenient()
        .read_bytes(&std::fs::read(path).expect("fixture"))
        .unwrap_or_else(|e| panic!("{} must parse leniently: {e:?}", path.display()))
}

/// What a mesh fixes regardless of triangle order.
fn shape<E: std::fmt::Debug>(mesh: &Result<ProductMesh, E>) -> String {
    match mesh {
        Ok(mesh) => format!(
            "{:?} {} {} {:?} {:?}",
            mesh.world,
            mesh.positions.len(),
            mesh.indices.len(),
            mesh.colour,
            mesh.closure
        ),
        Err(error) => format!("{error:?}"),
    }
}

/// Every subset entry equals the whole-model entry of the same product.
fn assert_subsets_agree(model: &Model, label: &str) {
    for purpose in [RepresentationPurpose::Body, RepresentationPurpose::Surface] {
        let whole = product_meshes_for(model, None, purpose);
        let again = product_meshes_for(model, None, purpose);
        for ((id, entry), (_, twin)) in whole.iter().zip(&again).take(PER_FIXTURE) {
            let subset = product_meshes_for(model, Some(&[*id]), purpose);
            assert_eq!(subset.len(), 1, "{label}");
            let want = format!("{:?}", (id, entry));
            if want == format!("{:?}", (id, twin)) {
                assert_eq!(
                    format!("{:?}", subset[0]),
                    want,
                    "{label}: mesh of {id:?} ({purpose:?})"
                );
            } else {
                // The reference backend orders some triangles differently
                // from call to call; everything else must still agree.
                assert_eq!(
                    shape(&subset[0].1),
                    shape(entry),
                    "{label}: mesh of {id:?} ({purpose:?})"
                );
            }
        }
    }
    assert_eq!(
        product_meshes(model, None).len(),
        product_meshes_for(model, None, RepresentationPurpose::Body).len(),
        "{label}"
    );
    let whole = product_colours(model, None);
    for (id, entry) in whole.iter().take(PER_FIXTURE) {
        let subset = product_colours(model, Some(&[*id]));
        assert_eq!(
            format!("{:?}", subset[0]),
            format!("{:?}", (id, entry)),
            "{label}: colour of {id:?}"
        );
    }
    let ids: Vec<EntityId> = whole.iter().map(|(id, _)| *id).take(PER_FIXTURE).collect();
    let together = product_colours(model, Some(&ids));
    assert_eq!(
        format!("{together:?}"),
        format!("{:?}", &whole[..ids.len()]),
        "{label}: colours of the leading products together"
    );
}

#[test]
fn a_subset_entry_equals_the_whole_model_entry_on_every_fixture() {
    for path in corpus() {
        let label = path.display().to_string();
        let model = read(&path);
        assert_subsets_agree(&model, &format!("{label} (fresh)"));
        model.reverse_index();
        assert_subsets_agree(&model, &format!("{label} (indexed)"));
    }
}
