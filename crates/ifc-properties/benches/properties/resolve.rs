//! Property-resolution benchmarks (#352).
//!
//! | Benchmark | Measures |
//! | --- | --- |
//! | `properties.index.build` | `PropertyIndex::build`: every relationship validated once |
//! | `properties.exact_property.every_object` | one property (`Pset_WallCommon.IsExternal`, inherited or overridden) for every wall, through one index |
//! | `properties.exact_properties.every_object` | every property of every wall, through one index |
//! | `properties.exact_property.per_call.100` | the free function `exact_property` for 100 walls spread over the model, each call on its own |
//!
//! The `every_object` benchmarks include building the index: that is what
//! a caller resolving every object pays. Their outputs are collected and
//! checksummed outside the clock; the checksum folds in the `Debug` form of
//! every result, so a run that answered differently cannot be pooled with
//! or compared against one that did not.

use std::hint::black_box;

use ifc_model::{EntityId, Model};
use ifc_properties::{
    exact_properties, exact_property, ExactPropertyEntry, ExactPropertyError, ExactResolution,
    PropertyIndex,
};

type One = Result<ExactResolution, ExactPropertyError>;
type Every = Result<Vec<ExactPropertyEntry>, ExactPropertyError>;

use crate::alloc;
use crate::checksum::Fnv;
use crate::report::{Record, Recorder};
use crate::stats::time;
use crate::Workload;

const SET: &str = "Pset_WallCommon";
const PROPERTY: &str = "IsExternal";

/// Folds the `Debug` form of every answer into one checksum.
fn checksum<T: std::fmt::Debug>(answers: &[Result<T, ExactPropertyError>]) -> u64 {
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

fn one_property_each(model: &Model, walls: &[EntityId]) -> Vec<One> {
    let index = PropertyIndex::build(model);
    walls
        .iter()
        .map(|&wall| index.exact_property(black_box(wall), Some(SET), PROPERTY))
        .collect()
}

fn every_property_each(model: &Model, walls: &[EntityId]) -> Vec<Every> {
    let index = PropertyIndex::build(model);
    walls
        .iter()
        .map(|&wall| index.exact_properties(black_box(wall)))
        .collect()
}

fn per_call(model: &Model, walls: &[EntityId]) -> Vec<One> {
    walls
        .iter()
        .map(|&wall| exact_property(model, black_box(wall), Some(SET), PROPERTY))
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
    let walls: Vec<EntityId> = model.ids_of_type("IFCWALL").to_vec();
    assert_eq!(walls.len(), workload.walls, "every wall was read");
    // The per-call cost is the same for every wall; a hundred spread over
    // the model keep the quadratic path measurable at every scale.
    let stride = walls.len().div_ceil(100).max(1);
    let sample: Vec<EntityId> = walls.iter().step_by(stride).copied().collect();

    // The reference answers, through the free functions, wall by wall: the
    // index must give exactly these.
    let reference_one: Vec<_> = sample
        .iter()
        .map(|&wall| exact_property(model, wall, Some(SET), PROPERTY))
        .collect();
    let index = PropertyIndex::build(model);
    for (&wall, answer) in sample.iter().zip(&reference_one) {
        assert_eq!(&index.exact_property(wall, Some(SET), PROPERTY), answer);
        assert_eq!(index.exact_properties(wall), exact_properties(model, wall));
        assert!(answer.is_ok(), "the workload resolves: {answer:?}");
    }
    drop(index);

    let bench = "properties.index.build";
    if wanted(selection, bench) {
        let samples = time(
            plan,
            || (),
            |()| PropertyIndex::build(model),
            |index| assert!(index.schema().is_ok()),
        );
        let (memory, index) = alloc::measure(model, PropertyIndex::build);
        // Its `Debug` form names the release and how many objects it holds.
        let mut hash = Fnv::new();
        hash.bytes(format!("{index:?}").as_bytes());
        let expected = hash.finish();
        drop(index);
        out.push(record(bench, samples, Some(memory), expected));
    }

    let bench = "properties.exact_property.every_object";
    if wanted(selection, bench) {
        let expected = checksum(&one_property_each(model, &walls));
        let samples = time(
            plan,
            || (),
            |()| one_property_each(model, &walls),
            |answers| assert_eq!(answers.len(), walls.len()),
        );
        let (memory, answers) = alloc::measure((), |()| one_property_each(model, &walls));
        drop(answers);
        out.push(record(bench, samples, Some(memory), expected));
    }

    let bench = "properties.exact_properties.every_object";
    if wanted(selection, bench) {
        let expected = checksum(&every_property_each(model, &walls));
        let samples = time(
            plan,
            || (),
            |()| every_property_each(model, &walls),
            |answers| assert_eq!(answers.len(), walls.len()),
        );
        let (memory, answers) = alloc::measure((), |()| every_property_each(model, &walls));
        drop(answers);
        out.push(record(bench, samples, Some(memory), expected));
    }

    let bench = "properties.exact_property.per_call.100";
    if wanted(selection, bench) {
        let expected = checksum(&reference_one);
        let samples = time(
            plan,
            || (),
            |()| per_call(model, &sample),
            |answers| assert_eq!(answers.len(), sample.len()),
        );
        out.push(record(bench, samples, None, expected));
    }
}
