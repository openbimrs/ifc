//! A production-shaped alignment (line -> clothoid -> arc -> clothoid -> line)
//! must lower its exact segments and name its refused ones, rather than
//! failing wholesale.

use axiolid_curve::Curve2;
use axiolid_model::{CurveRelation, GeometryNode, Transition};
use ifc_alignment::{
    lower_horizontal_layout, lower_horizontal_layout_partial, AlignmentError, AlignmentUnits,
    SeamCheck,
};
use ifc_model::{Codec, EntityId};
use ifc_step::StepCodec;

/// The `IfcAlignmentHorizontal` in the spiral fixture.
const HORIZONTAL: EntityId = EntityId(191);

fn load(name: &str) -> ifc_model::Model {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../test/fixtures/synthetic-surfaces")
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

/// The strict entry point lowers a spiral layout to ONE composite (#239).
///
/// Position after a spiral has no closed form, so those seams are recorded
/// as authored rather than refused; seams after a line or arc are verified.
/// Only a verified seam claims `Continuous`.
#[test]
fn the_strict_entry_point_lowers_a_spiral_layout_with_its_seams_recorded() {
    let model = model();
    let lowered = lower_horizontal_layout(&model, HORIZONTAL, units(), None)
        .expect("a spiral no longer stops the strict path");
    assert_eq!(
        lowered.sources.iter().map(|id| id.0).collect::<Vec<_>>(),
        vec![101, 103, 105, 107, 109]
    );
    let checks: Vec<(u64, u64, f64, SeamCheck)> = lowered
        .seams
        .iter()
        .map(|seam| {
            (
                seam.previous.0,
                seam.next.0,
                seam.distance_along,
                seam.position,
            )
        })
        .collect();
    assert_eq!(
        checks,
        vec![
            (101, 103, 100.0, SeamCheck::Verified),
            (103, 105, 160.0, SeamCheck::Authored),
            (105, 107, 280.0, SeamCheck::Verified),
            (107, 109, 340.0, SeamCheck::Authored),
        ]
    );
    let Some(GeometryNode::CurveRelation(CurveRelation::Composite { segments })) =
        lowered.graph.get(lowered.root)
    else {
        panic!("a layout lowers to a composite");
    };
    let transitions: Vec<Transition> = segments.iter().map(|s| s.transition).collect();
    assert_eq!(
        transitions,
        vec![
            Transition::Discontinuous,
            Transition::Continuous,
            Transition::Discontinuous,
            Transition::Continuous,
            Transition::Discontinuous,
        ]
    );
}

#[test]
fn partial_lowering_keeps_the_exact_segments_and_names_the_refused_ones() {
    let model = model();
    let result = lower_horizontal_layout_partial(&model, HORIZONTAL, units(), None)
        .expect("layout is readable");

    assert_eq!(result.segment_count, 5);
    assert_eq!(
        result.lowered_count(),
        5,
        "every segment lowers exactly: clothoids are intrinsic curves"
    );
    assert!(result.is_complete());

    // Clothoids now lower exactly, so nothing is refused.
    assert!(
        result.refused.is_empty(),
        "clothoids are no longer refused: {:?}",
        result.refused
    );
    assert!(result.is_complete());

    // Every refusal states the capability that is missing, not a generic failure.
    for refusal in &result.refused {
        assert!(matches!(refusal.reason, AlignmentError::Unsupported { .. }));
    }
}

#[test]
fn a_spiral_seam_does_not_split_the_layout_into_runs() {
    let model = model();
    let result = lower_horizontal_layout_partial(&model, HORIZONTAL, units(), None)
        .expect("layout is readable");

    // line | CLOTHOID | arc | CLOTHOID | line, all exact: one run, as the
    // `is_complete` contract states. Runs split only at refused segments.
    assert_eq!(result.runs.len(), 1);
    assert_eq!(
        result.runs[0]
            .sources
            .iter()
            .map(|id| id.0)
            .collect::<Vec<_>>(),
        vec![101, 103, 105, 107, 109]
    );
    let authored: Vec<u64> = result.runs[0]
        .seams
        .iter()
        .filter(|seam| seam.position == SeamCheck::Authored)
        .map(|seam| seam.next.0)
        .collect();
    assert_eq!(authored, vec![105, 109], "the seams after each clothoid");

    // Each fixture spiral must appear in a run AND be an exact intrinsic
    // curve. Asserting against the fixture's known CLOTHOID ids, rather than
    // against the crate's own output, keeps this honest: a regression that
    // approximated spirals would still put them in runs, so only checking
    // membership would pass vacuously.
    const FIXTURE_SPIRALS: [u64; 2] = [103, 107];
    for spiral in FIXTURE_SPIRALS {
        assert!(
            result
                .runs
                .iter()
                .any(|run| run.sources.iter().any(|id| id.0 == spiral)),
            "spiral {spiral} must lower into a run, not be dropped"
        );
    }
    let intrinsics: usize = result
        .runs
        .iter()
        .flat_map(|run| run.graph.iter())
        .filter(|(_, node)| matches!(node, GeometryNode::Curve2(Curve2::Intrinsic(_))))
        .count();
    assert_eq!(
        intrinsics,
        FIXTURE_SPIRALS.len(),
        "each clothoid must lower to exactly one intrinsic curve"
    );
    let approximated = result
        .runs
        .iter()
        .flat_map(|run| run.graph.iter())
        .any(|(_, node)| {
            matches!(
                node,
                GeometryNode::Curve2(Curve2::Polyline(_))
                    | GeometryNode::Curve2(Curve2::BSpline(_))
            )
        });
    assert!(
        !approximated,
        "a spiral must never be discretised or fitted"
    );
    let lowered_total: usize = result.runs.iter().map(|run| run.sources.len()).sum();
    assert_eq!(lowered_total, result.lowered_count());
    assert_eq!(lowered_total + result.refused.len(), result.segment_count);
}

#[test]
fn a_fully_lowerable_layout_reports_complete_with_one_run() {
    // The pre-existing line->arc fixture has no spiral, so partial lowering
    // must agree with the strict entry point.
    let model = load("synthetic_alignment_layout.ifc");

    let result = lower_horizontal_layout_partial(&model, EntityId(121), units(), None)
        .expect("layout is readable");
    assert!(result.is_complete());
    assert_eq!(result.refused, vec![]);
    assert_eq!(result.runs.len(), 1);
    assert_eq!(result.lowered_count(), 2);

    // Same content the strict entry point produces. Graph identity differs by
    // construction (each graph carries its own id), so compare the parts that
    // describe the geometry rather than the container's identity.
    let strict = lower_horizontal_layout(&model, EntityId(121), units(), None).expect("lowers");
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
