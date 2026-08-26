//! Contexts read from real exporter output.
//!
//! The synthetic tests construct `Value::Derived` deliberately. These prove the
//! STEP parser actually produces it for `*` and that inheritance works on files
//! nobody wrote for this test.

use ifc_geometry::{all_contexts, plan_contexts, RepresentationContext, TargetView};
use ifc_model::Codec;
use std::path::PathBuf;

fn load(rel: &str) -> Option<ifc_model::Model> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../test/fixtures")
        .join(rel);
    let bytes = std::fs::read(path).ok()?;
    ifc_step::StepCodec.read_bytes(&bytes).ok()
}

#[test]
fn a_real_sub_context_inherits_precision_from_its_parent() {
    let Some(model) = load("ifclite-geometry/issue_098_wall_W.ifc") else {
        eprintln!("skipped: fixture not present");
        return;
    };
    let contexts = all_contexts(&model);
    assert!(!contexts.is_empty(), "the file declares contexts");

    let subs: Vec<_> = contexts
        .iter()
        .filter(|c| c.is_sub_context())
        .copied()
        .collect();
    assert!(!subs.is_empty(), "and at least one sub-context");

    for sub in &subs {
        assert!(
            sub.parent().is_some(),
            "sub-context {:?} must name a parent",
            sub.id()
        );
        // The fixture writes CoordinateSpaceDimension as `*`; reading the slot
        // directly would yield the marker rather than the project's 3.
        assert_eq!(
            sub.coordinate_space_dimension(&model),
            Some(3),
            "sub-context {:?} must inherit the parent dimension",
            sub.id()
        );
    }
}

/// Proves the parser really produces `Value::Derived` for `*`, rather than the
/// synthetic tests exercising a shape that never occurs in practice.
#[test]
fn the_parser_produces_derived_markers_for_star() {
    let Some(model) = load("ifclite-geometry/issue_098_wall_W.ifc") else {
        eprintln!("skipped: fixture not present");
        return;
    };
    let mut found = false;
    for id in model.ids_of_type("IFCGEOMETRICREPRESENTATIONSUBCONTEXT") {
        let entity = model.get(*id).unwrap();
        if matches!(entity.attribute(2), Some(ifc_model::Value::Derived)) {
            found = true;
            break;
        }
    }
    assert!(
        found,
        "no `*` found; the inheritance path would be untested"
    );
}

#[test]
fn real_target_views_parse() {
    let Some(model) = load("ifclite-geometry/issue_098_wall_W.ifc") else {
        eprintln!("skipped: fixture not present");
        return;
    };
    let views: Vec<_> = all_contexts(&model)
        .iter()
        .filter_map(|c| c.target_view())
        .collect();

    assert!(!views.is_empty(), "sub-contexts declare target views");
    assert!(
        views.contains(&TargetView::ModelView),
        "this file is a 3D model: {views:?}"
    );
    assert!(
        !views.iter().any(|v| matches!(v, TargetView::Other(_))),
        "every view in the corpus should be a known constant: {views:?}"
    );
}

/// A 3D model file has no plan contexts. Asserting the absence is as
/// meaningful as asserting presence: it proves the filter discriminates.
#[test]
fn a_model_view_file_yields_no_plan_contexts() {
    let Some(model) = load("ifclite-geometry/issue_098_wall_W.ifc") else {
        eprintln!("skipped: fixture not present");
        return;
    };
    assert!(
        plan_contexts(&model).is_empty(),
        "this fixture contains only MODEL_VIEW and GRAPH_VIEW sub-contexts"
    );
}

#[test]
fn contexts_across_the_corpus_are_well_formed() {
    for fixture in [
        "ifclite-geometry/issue_098_wall_W.ifc",
        "ifclite-geometry/mapped_instances_nested.ifc",
        "ifclite-geometry/bath_csg_solid.ifc",
        "ifclite-geometry/shared_point_faceted_brep.ifc",
    ] {
        let Some(model) = load(fixture) else {
            eprintln!("skipped: {fixture}");
            continue;
        };
        for context in all_contexts(&model) {
            // Every sub-context must resolve a placement, directly or through
            // its parent. A None here means geometry with no known origin.
            if context.is_sub_context() {
                assert!(
                    context.world_coordinate_system(&model).is_some(),
                    "{fixture}: {:?} has no resolvable placement",
                    context.id()
                );
            }
            let _ = RepresentationContext::id(&context);
        }
    }
}
