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
                // TODO(#610): the reference backend orders some triangles
                // differently from call to call; everything else must still
                // agree.
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
        // The first Body call builds the model's index, so this pass is the
        // first use; the next runs on a model that already keeps it.
        assert_subsets_agree(&model, &format!("{label} (first use)"));
        assert!(model.reverse_index_if_built().is_some(), "{label}");
        assert_subsets_agree(&model, &format!("{label} (indexed)"));
    }
}

/// Only the Body carries a colour, so meshing another purpose builds no
/// resolver and leaves the model without a reverse index.
#[test]
fn meshing_a_non_body_purpose_builds_no_index() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../test/fixtures/synthetic-colour/material_colour.ifc");
    let model = read(&path);
    let meshes = product_meshes_for(&model, None, RepresentationPurpose::Surface);
    assert!(!meshes.is_empty());
    assert!(model.reverse_index_if_built().is_none());
    product_meshes(&model, None);
    assert!(model.reverse_index_if_built().is_some());
}

#[cfg(all(feature = "authoring", feature = "ifc4x3"))]
#[path = "../benches/authoring/walls.rs"]
#[allow(dead_code)]
mod walls;

/// A change looked at inside `Model::preview` meshes and colours what the
/// same change applied for real does (#608): the resolver must see the
/// previewed records, not an index of the model as it was.
#[cfg(all(feature = "authoring", feature = "ifc4x3"))]
#[test]
fn a_preview_meshes_what_the_change_would_mesh() {
    use ifc::{apply_authoring, Entity, Value};

    fn change(model: &mut Model, scene: &walls::Scene) -> Vec<String> {
        let out = apply_authoring(
            model,
            ifc::schema::ifc4x3(),
            &walls::ops(scene, 0..1, "x"),
            5,
        )
        .expect("the wall applies");
        let wall = out.ids[5].expect("the wall");
        let solid = out.ids[3].expect("its solid");
        let colour = model.push(Entity::new(
            "IFCCOLOURRGB",
            vec![
                Value::Null,
                Value::Real(0.2),
                Value::Real(0.4),
                Value::Real(0.6),
            ],
        ));
        let shading = model.push(Entity::new(
            "IFCSURFACESTYLESHADING",
            vec![Value::Ref(colour), Value::Null],
        ));
        let style = model.push(Entity::new(
            "IFCSURFACESTYLE",
            vec![
                Value::Null,
                Value::Enum("BOTH".into()),
                Value::List(vec![Value::Ref(shading)]),
            ],
        ));
        model.push(Entity::new(
            "IFCSTYLEDITEM",
            vec![
                Value::Ref(solid),
                Value::List(vec![Value::Ref(style)]),
                Value::Null,
            ],
        ));
        // A neighbour in the model, to show the lookup is per product.
        let neighbour = model.ids_of_type("IfcWall")[0];
        vec![
            format!("{:?}", product_meshes(model, Some(&[wall]))),
            format!("{:?}", product_colours(model, Some(&[wall]))),
            format!("{:?}", product_colours(model, Some(&[neighbour]))),
        ]
    }

    let (mut model, scene) = walls::model(20);
    model.header_mut().schema = vec!["IFC4X3_ADD2".into()];
    model.reverse_index();
    let before = (model.len(), model.revision());

    let previewed = model.preview(|m| change(m, &scene));
    assert_eq!(
        (model.len(), model.revision()),
        before,
        "the preview left a trace"
    );
    assert!(
        previewed[1].contains("0.2, 0.4, 0.6"),
        "the previewed style colours the new wall: {}",
        previewed[1]
    );
    assert!(previewed[2].contains("Ok(None)"), "{}", previewed[2]);

    let mut applied = model.clone();
    assert_eq!(change(&mut applied, &scene), previewed);
}
