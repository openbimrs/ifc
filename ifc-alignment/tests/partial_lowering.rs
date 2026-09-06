//! A production-shaped alignment (line -> clothoid -> arc -> clothoid -> line)
//! must lower its exact segments and name its refused ones, rather than
//! failing wholesale.

use ifc_alignment::{
    lower_horizontal_layout, lower_horizontal_layout_partial, AlignmentError, AlignmentUnits,
};
use ifc_model::{Codec, EntityId};
use ifc_step::StepCodec;

/// The `IfcAlignmentHorizontal` in the spiral fixture.
const HORIZONTAL: EntityId = EntityId(191);

fn load(name: &str) -> ifc_model::Model {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../test/fixtures/synthetic-surfaces")
        .join(name);
    StepCodec.read_path(&path).expect("fixture parses")
}

fn units() -> AlignmentUnits {
    AlignmentUnits {
        length_to_metres: 1.0,
        angle_to_radians: 1.0,
    }
}

fn model() -> ifc_model::Model {
    load("synthetic_alignment_spiral.ifc")
}

#[test]
fn the_all_or_nothing_entry_point_still_refuses_a_spiral_layout() {
    // Backward compatibility: the strict entry point keeps its contract.
    let model = model();
    let error = lower_horizontal_layout(&model, HORIZONTAL, units())
        .expect_err("a CLOTHOID has no exact neutral primitive");
    assert!(
        matches!(&error, AlignmentError::Unsupported { type_name, .. } if type_name == "CLOTHOID"),
        "expected a CLOTHOID refusal, got {error:?}"
    );
}

#[test]
fn partial_lowering_keeps_the_exact_segments_and_names_the_refused_ones() {
    let model = model();
    let result =
        lower_horizontal_layout_partial(&model, HORIZONTAL, units()).expect("layout is readable");

    assert_eq!(result.segment_count, 5);
    assert_eq!(
        result.lowered_count(),
        3,
        "line, arc and line lower exactly"
    );
    assert!(!result.is_complete());

    // Both clothoids are named, in authored order, with their entity ids.
    let refused: Vec<(u64, &str)> = result
        .refused
        .iter()
        .map(|r| (r.entity.0, r.type_name.as_str()))
        .collect();
    assert_eq!(refused, vec![(103, "CLOTHOID"), (107, "CLOTHOID")]);

    // Every refusal states the capability that is missing, not a generic failure.
    for refusal in &result.refused {
        assert!(matches!(refusal.reason, AlignmentError::Unsupported { .. }));
    }
}

#[test]
fn a_refusal_splits_the_layout_into_separate_runs_rather_than_bridging_it() {
    let model = model();
    let result =
        lower_horizontal_layout_partial(&model, HORIZONTAL, units()).expect("layout is readable");

    // line | CLOTHOID | arc | CLOTHOID | line  =>  three runs of one segment.
    // Continuity is never asserted across a segment that was not lowered.
    assert_eq!(result.runs.len(), 3);
    let sources: Vec<Vec<u64>> = result
        .runs
        .iter()
        .map(|run| run.sources.iter().map(|id| id.0).collect())
        .collect();
    assert_eq!(sources, vec![vec![101], vec![105], vec![109]]);

    // No run may contain a spiral. Asserting against the fixture's known
    // CLOTHOID ids rather than against `result.refused` keeps this honest: a
    // regression that silently lowered spirals would empty `refused` and make
    // a self-referential check pass vacuously.
    const FIXTURE_SPIRALS: [u64; 2] = [103, 107];
    for run in &result.runs {
        for source in &run.sources {
            assert!(
                !FIXTURE_SPIRALS.contains(&source.0),
                "run contains spiral segment {source:?}, which has no exact lowering"
            );
        }
    }
    for spiral in FIXTURE_SPIRALS {
        assert!(
            result.refused.iter().any(|r| r.entity.0 == spiral),
            "spiral {spiral} must be reported as refused, not silently lowered"
        );
    }
    let lowered_total: usize = result.runs.iter().map(|run| run.sources.len()).sum();
    assert_eq!(lowered_total, result.lowered_count());
    assert_eq!(lowered_total + result.refused.len(), result.segment_count);
}

#[test]
fn a_fully_lowerable_layout_reports_complete_with_one_run() {
    // The pre-existing line->arc fixture has no spiral, so partial lowering
    // must agree with the strict entry point.
    let model = load("synthetic_alignment_layout.ifc");

    let result = lower_horizontal_layout_partial(&model, EntityId(121), units())
        .expect("layout is readable");
    assert!(result.is_complete());
    assert_eq!(result.refused, vec![]);
    assert_eq!(result.runs.len(), 1);
    assert_eq!(result.lowered_count(), 2);

    // Same content the strict entry point produces. Graph identity differs by
    // construction (each graph carries its own id), so compare the parts that
    // describe the geometry rather than the container's identity.
    let strict = lower_horizontal_layout(&model, EntityId(121), units()).expect("lowers");
    assert_eq!(result.runs[0].sources, strict.sources);
    // `NodeId` embeds the owning graph's id, so two independently built graphs
    // never compare equal by value even when they describe identical geometry.
    // Normalise that id out and compare the structure itself.
    let shape = |curve: &ifc_alignment::LoweredAlignmentCurve| -> Vec<String> {
        let graph_id = regex_free_graph_id(&format!("{:?}", curve.graph));
        curve
            .graph
            .iter()
            .map(|(_, node)| format!("{node:?}").replace(&graph_id, "GraphId(N)"))
            .collect()
    };
    assert_eq!(shape(&result.runs[0]), shape(&strict));
}

/// Extract the `GraphId(n)` token from a graph's debug rendering.
///
/// `NodeId` embeds its owning graph's id and neither field is public, so
/// normalising it out of a debug string is the only way to compare two
/// independently built graphs structurally.
fn regex_free_graph_id(debug: &str) -> String {
    let start = debug.find("GraphId(").expect("graph debug names its id");
    let end = debug[start..].find(')').expect("graph id is closed") + start + 1;
    debug[start..end].to_owned()
}
