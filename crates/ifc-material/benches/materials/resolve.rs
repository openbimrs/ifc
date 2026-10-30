//! Material-resolution benchmarks (#609).
//!
//! | Benchmark | Measures |
//! | --- | --- |
//! | `material.index.build` | `MaterialIndex::new`: every material and typing relationship read once, on a model with no reverse index |
//! | `material.index.every_wall` | `MaterialIndex::new` and `assigned_material` of every wall, on a model with no reverse index |
//! | `material.index.every_wall.indexed` | the same on a model whose reverse index was built before timing |
//! | `material.per_call.100` | `MaterialView::assigned_material` for 100 walls spread over the model, each call on its own, no reverse index |
//! | `material.per_call.100.indexed` | the same on a model whose reverse index was built before timing |
//!
//! The `every_wall` benchmarks include building the index: that is what a
//! caller resolving every wall pays. Their outputs are collected and
//! checksummed outside the clock; the checksum folds in the `Debug` form of
//! every result, so a run that answered differently cannot be pooled with
//! or compared against one that did not.

use std::hint::black_box;

use ifc_material::{
    AssignmentSource, MaterialIndex, MaterialResult, MaterialView, ResolvedAssignment,
};
use ifc_model::{EntityId, Model};

use crate::alloc;
use crate::checksum::Fnv;
use crate::report::{Record, Recorder};
use crate::stats::time;
use crate::Workload;

type Answer<'m> = MaterialResult<Option<ResolvedAssignment<'m>>>;

/// Folds the `Debug` form of every answer into one checksum.
fn checksum(answers: &[Answer<'_>]) -> u64 {
    let mut hash = Fnv::new();
    let mut text = String::new();
    for answer in answers {
        use std::fmt::Write;
        text.clear();
        let _ = write!(text, "{answer:?}");
        hash.bytes(text.as_bytes());
    }
    hash.finish()
}

fn every_wall<'m>(model: &'m Model, walls: &[EntityId]) -> Vec<Answer<'m>> {
    let index = MaterialIndex::new(MaterialView::new(model));
    walls
        .iter()
        .map(|&wall| index.assigned_material(black_box(wall)))
        .collect()
}

fn per_call<'m>(model: &'m Model, walls: &[EntityId]) -> Vec<Answer<'m>> {
    let view = MaterialView::new(model);
    walls
        .iter()
        .map(|&wall| view.assigned_material(black_box(wall)))
        .collect()
}

/// Which benchmarks to run; all by default.
pub(crate) fn wanted(selection: &[String], bench: &str) -> bool {
    selection.is_empty() || selection.iter().any(|name| name == bench)
}

/// Times every selected benchmark on `workload`'s model.
pub(crate) fn run(workload: &Workload, model: &Model, selection: &[String], out: &mut Recorder) {
    let plan = out.plan;
    let entities = model.len();
    let record = |bench, samples, memory, checksum| Record {
        workload: workload.name.clone(),
        bench,
        entities,
        bytes: workload.bytes.len(),
        samples,
        memory,
        checksum,
    };
    assert!(
        model.reverse_index_if_built().is_none(),
        "the model is bare"
    );
    let indexed = model.clone();
    indexed.reverse_index();

    let mut walls: Vec<EntityId> = model.ids_of_type("IFCWALL").to_vec();
    walls.sort_unstable();
    assert_eq!(walls.len(), workload.walls, "every wall was read");
    // The per-call cost is the same for every wall; a hundred spread over
    // the model keep the quadratic path measurable at every scale.
    let stride = walls.len().div_ceil(100).max(1);
    let sample: Vec<EntityId> = walls.iter().step_by(stride).copied().collect();

    // The view's answers, wall by wall: the index must give exactly these,
    // for every wall at the small scales and for the sample at every scale.
    let view = MaterialView::new(model);
    let checked: Vec<EntityId> = if walls.len() <= 1_000 {
        walls.clone()
    } else {
        sample.clone()
    };
    let index = MaterialIndex::new(view);
    let on_indexed = MaterialView::new(&indexed);
    for &wall in &checked {
        let position = walls.binary_search(&wall).expect("a wall");
        let reference = view.assigned_material(wall);
        let resolved = reference
            .as_ref()
            .unwrap_or_else(|e| panic!("the workload resolves: {e:?}"))
            .as_ref()
            .expect("every wall has a material");
        let expected = if position % 2 == 0 {
            AssignmentSource::Occurrence
        } else {
            let AssignmentSource::Type(_) = resolved.source else {
                panic!("wall {position} inherits from its type");
            };
            resolved.source
        };
        assert_eq!(resolved.source, expected);
        assert_eq!(
            format!("{:?}", index.assigned_material(wall)),
            format!("{reference:?}")
        );
        assert_eq!(
            format!("{:?}", on_indexed.assigned_material(wall)),
            format!("{reference:?}")
        );
    }
    drop(index);

    let reference = checksum(&per_call(model, &sample));

    let bench = "material.index.build";
    if wanted(selection, bench) {
        let samples = time(
            plan,
            || (),
            |()| MaterialIndex::new(MaterialView::new(model)),
            |index| assert!(index.view().schema().is_ok()),
        );
        let (memory, index) =
            alloc::measure(model, |model| MaterialIndex::new(MaterialView::new(model)));
        // Its `Debug` form names the release and how many relationships it refuses.
        let mut hash = Fnv::new();
        hash.bytes(format!("{index:?}").as_bytes());
        let expected = hash.finish();
        drop(index);
        out.push(record(bench, samples, Some(memory), expected));
    }

    for (bench, target) in [
        ("material.index.every_wall", model),
        ("material.index.every_wall.indexed", &indexed),
    ] {
        if wanted(selection, bench) {
            let expected = checksum(&every_wall(target, &walls));
            let samples = time(
                plan,
                || (),
                |()| every_wall(target, &walls),
                |answers| assert_eq!(answers.len(), walls.len()),
            );
            let (memory, answers) = alloc::measure((), |()| every_wall(target, &walls));
            drop(answers);
            out.push(record(bench, samples, Some(memory), expected));
        }
    }

    for (bench, target) in [
        ("material.per_call.100", model),
        ("material.per_call.100.indexed", &indexed),
    ] {
        if wanted(selection, bench) {
            let samples = time(
                plan,
                || (),
                |()| per_call(target, &sample),
                |answers| assert_eq!(answers.len(), sample.len()),
            );
            out.push(record(bench, samples, None, reference));
        }
    }
}
