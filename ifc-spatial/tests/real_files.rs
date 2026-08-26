//! Traversal against real IFC files from the fixture corpus.
//!
//! Synthetic models prove the logic; these prove it survives exporter output,
//! which is where assumptions about the canonical hierarchy actually break.

use ifc_model::Codec;
use ifc_spatial::{SpatialKind, SpatialTree};
use std::path::PathBuf;

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../test/fixtures")
}

fn load(rel: &str) -> Option<ifc_model::Model> {
    let path = fixtures().join(rel);
    let bytes = std::fs::read(path).ok()?;
    ifc_step::StepCodec.read_bytes(&bytes).ok()
}

#[test]
fn a_real_file_yields_one_project_rooted_tree() {
    let Some(model) = load("ifclite-geometry/issue_098_wall_W.ifc") else {
        eprintln!("skipped: fixture not present");
        return;
    };
    let tree = SpatialTree::build(&model);

    let projects: Vec<_> = tree.of_kind(SpatialKind::Project).collect();
    assert_eq!(projects.len(), 1, "a conformant export has one project");
    assert_eq!(
        tree.roots(),
        [projects[0].id],
        "and it is the only root: {:?}",
        tree.roots()
    );
    assert!(
        tree.dangling().is_empty(),
        "a valid file has no dangling containment: {:?}",
        tree.dangling()
    );
}

/// A real export that uses **only** `IfcRelAggregates`: project, site,
/// building and 28 storeys, with no containment relationship anywhere.
///
/// Discovered from the corpus, not assumed. The tree must still produce the
/// full hierarchy; asserting "elements exist" here would be asserting a
/// property of one exporter rather than of the traversal.
#[test]
fn a_file_without_containment_relationships_still_builds_its_hierarchy() {
    let Some(model) = load("ifclite-geometry/issue_098_wall_W.ifc") else {
        eprintln!("skipped: fixture not present");
        return;
    };
    let tree = SpatialTree::build(&model);
    let root = tree.roots()[0];

    assert_eq!(
        model.ids_of_type("IFCRELCONTAINEDINSPATIALSTRUCTURE").len(),
        0,
        "fixture premise: this file places nothing"
    );
    let storeys: Vec<_> = tree.of_kind(SpatialKind::Storey).collect();
    assert_eq!(storeys.len(), 28, "every storey is in the tree");
    for storey in &storeys {
        assert!(
            tree.ancestors(storey.id).contains(&root),
            "storey {:?} must reach the project",
            storey.id
        );
    }
    assert!(
        tree.elements_recursive(root).is_empty(),
        "and no elements are claimed, because the file places none"
    );
}

/// A real export that *does* use containment.
#[test]
fn elements_are_reachable_from_the_project_in_a_real_file() {
    let Some(model) = load("ifclite-geometry/mapped_instances_nested.ifc") else {
        eprintln!("skipped: fixture not present");
        return;
    };
    let tree = SpatialTree::build(&model);
    let root = tree.roots()[0];

    let elements = tree.elements_recursive(root);
    assert!(
        !elements.is_empty(),
        "this fixture does place elements, so the walk must find them"
    );

    for element in &elements {
        let entity = model.get(*element).expect("element is in the model");
        assert!(
            !SpatialKind::classify(&entity.type_name).is_container(),
            "{} was returned as an element",
            entity.type_name
        );
        assert!(
            tree.container_of(*element).is_some(),
            "element {element:?} has no container"
        );
    }
}

#[test]
fn every_storey_element_resolves_back_to_that_storey() {
    let Some(model) = load("ifclite-geometry/mapped_instances_nested.ifc") else {
        eprintln!("skipped: fixture not present");
        return;
    };
    let tree = SpatialTree::build(&model);

    let mut checked = 0usize;
    for container in tree.containers() {
        for element in &container.elements {
            assert_eq!(
                tree.container_of(*element),
                Some(container.id),
                "round trip failed for {element:?}"
            );
            checked += 1;
        }
    }
    assert!(checked > 0, "the fixture must exercise containment");
}

/// Traversal must not invent or lose entities relative to the file.
#[test]
fn the_tree_only_contains_entities_from_the_model() {
    for fixture in [
        "ifclite-geometry/issue_098_wall_W.ifc",
        "ifclite-geometry/mapped_instances_nested.ifc",
        "ifclite-geometry/bath_csg_solid.ifc",
    ] {
        let Some(model) = load(fixture) else {
            eprintln!("skipped: {fixture} not present");
            continue;
        };
        let tree = SpatialTree::build(&model);
        for node in tree.containers() {
            assert!(
                model.get(node.id).is_some(),
                "{fixture}: invented container"
            );
            for child in &node.children {
                assert!(model.get(*child).is_some(), "{fixture}: invented child");
            }
            for element in &node.elements {
                assert!(model.get(*element).is_some(), "{fixture}: invented element");
            }
        }
    }
}
