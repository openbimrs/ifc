//! Entity-graph baselines (#14): construction, lookup by id, iteration,
//! by-type queries and reference-heavy traversal, on a fully decoded model.
//!
//! The model is the eager read of the workload, so these numbers measure
//! the graph and its indices, never decoding. Each operation folds what it
//! touches into a checksum; the checksum computed once before timing is
//! asserted on every sample.
//!
//! # The extra-work probe
//!
//! `--probe` walks every traversal twice. The traversal's expected checksum
//! is computed independently of the probe, so the probe must change both
//! the asserted result (by exactly a factor of two) and the measured time
//! (by roughly that factor). A harness in which it changed neither would be
//! measuring something other than the walk.

use crate::alloc;
use crate::checksum::Fnv;
use crate::report::{Record, Recorder};
use crate::stats::time;
use crate::Workload;
use ifc_model::{breadth_first, Budget, Entity, EntityId, Model, ReverseIndex};
use std::hint::black_box;

/// Ids in a fixed pseudo-random order (xorshift64, fixed seed), so lookup
/// is measured without the help of file order and identically every run.
fn shuffled(model: &Model) -> Vec<EntityId> {
    let mut ids: Vec<EntityId> = model.ids().collect();
    let mut state: u64 = 0x9e37_79b9_7f4a_7c15;
    for i in (1..ids.len()).rev() {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        ids.swap(i, (state % (i as u64 + 1)) as usize);
    }
    ids
}

fn construct(header: &ifc_model::Header, entities: Vec<(EntityId, Entity)>) -> Model {
    let mut model = Model::new();
    *model.header_mut() = header.clone();
    // A codec knows its record count; so does this.
    model.reserve(entities.len());
    for (id, entity) in entities {
        model.insert(id, entity);
    }
    model
}

fn lookup(model: &Model, ids: &[EntityId]) -> u64 {
    let mut hash = Fnv::new();
    for &id in ids {
        let entity = model.get(black_box(id)).expect("listed id resolves");
        hash.u64(id.0 ^ entity.attributes.len() as u64);
    }
    hash.finish()
}

fn iterate(model: &Model) -> u64 {
    let mut hash = Fnv::new();
    for (id, entity) in model.iter() {
        hash.u64(id.0 ^ entity.attributes.len() as u64 ^ entity.type_name.len() as u64);
    }
    hash.finish()
}

/// Every type present asked for by name -- the type index alone -- and
/// every instance of the most common type resolved through it.
fn by_type(model: &Model, types: &[String]) -> u64 {
    let mut hash = Fnv::new();
    for name in types {
        hash.u64(model.ids_of_type(black_box(name)).len() as u64);
    }
    for (id, entity) in model.of_type(&types[0]) {
        hash.u64(id.0 ^ entity.attributes.len() as u64);
    }
    hash.finish()
}

/// Entities no other entity references: the starting points of the walk.
fn roots(model: &Model) -> Vec<EntityId> {
    let index = ReverseIndex::build(model);
    model.ids().filter(|id| !index.is_referenced(*id)).collect()
}

/// A breadth-first walk along forward references from every root, each
/// with its own visited set: the reference-chasing a consumer does to
/// resolve a relationship to its placement, geometry and owner history.
/// Returns the total entities visited; `passes` > 1 is the probe.
fn traverse(model: &Model, roots: &[EntityId], passes: usize) -> u64 {
    let successors = |id: EntityId| model.get(id).map(Entity::references).unwrap_or_default();
    let mut visited = 0u64;
    for _ in 0..passes {
        for &root in roots {
            let walk = breadth_first(black_box(root), Budget::DEFAULT, successors);
            assert!(walk.stop.is_complete(), "budget truncated a walk");
            visited += walk.visited.len() as u64;
        }
    }
    visited
}

/// Every id's referrers, in file order, folded into a checksum.
fn referrers(model: &Model, index: &ReverseIndex) -> u64 {
    let mut hash = Fnv::new();
    for id in model.ids() {
        for referrer in index.referrers(id) {
            hash.u64(referrer.from.0 ^ (referrer.slot as u64) << 48);
        }
    }
    hash.finish()
}

/// Times and measures every graph operation on `workload`'s decoded model.
pub(crate) fn run(workload: &Workload, model: &Model, probe: bool, out: &mut Recorder) {
    let n = model.len();
    let plan = out.plan;
    let record = |bench, samples, memory, checksum| Record {
        workload: workload.name.clone(),
        bench,
        entities: n,
        bytes: workload.bytes.len(),
        samples,
        memory,
        checksum,
    };

    // Construction from owned entities; cloning them is setup.
    let owned =
        || -> Vec<(EntityId, Entity)> { model.iter().map(|(id, e)| (id, e.clone())).collect() };
    let header = model.header();
    let expected = crate::checksum::model(model);
    let samples = time(
        plan,
        owned,
        |entities| construct(header, entities),
        |built| assert_eq!(built.len(), n),
    );
    let (memory, built) = alloc::measure(owned(), |entities| construct(header, entities));
    assert_eq!(
        crate::checksum::model(&built),
        expected,
        "construction lost content"
    );
    drop(built);
    out.push(record("model.construct", samples, Some(memory), expected));

    let ids = shuffled(model);
    let expected = lookup(model, &ids);
    let samples = time(
        plan,
        || (),
        |()| lookup(model, &ids),
        |sum| assert_eq!(*sum, expected),
    );
    out.push(record("model.lookup", samples, None, expected));

    let expected = iterate(model);
    let samples = time(
        plan,
        || (),
        |()| iterate(model),
        |sum| assert_eq!(*sum, expected),
    );
    out.push(record("model.iterate", samples, None, expected));

    let types: Vec<String> = model
        .type_histogram()
        .into_iter()
        .map(|(name, _)| name.to_owned())
        .collect();
    let expected = by_type(model, &types);
    let samples = time(
        plan,
        || (),
        |()| by_type(model, &types),
        |sum| assert_eq!(*sum, expected),
    );
    out.push(record("model.by_type", samples, None, expected));

    let roots = roots(model);
    let once = traverse(model, &roots, 1);
    let passes = if probe { 2 } else { 1 };
    let expected = once * passes as u64;
    let samples = time(
        plan,
        || (),
        |()| traverse(model, &roots, passes),
        |visited| assert_eq!(*visited, expected),
    );
    let (memory, _) = alloc::measure(passes, |passes| traverse(model, &roots, passes));
    out.push(record("model.traverse", samples, Some(memory), expected));

    let expected = referrers(model, &ReverseIndex::build(model));
    let samples = time(
        plan,
        || (),
        |()| ReverseIndex::build(model),
        |index| assert_eq!(referrers(model, index), expected),
    );
    let (memory, index) = alloc::measure(model, ReverseIndex::build);
    drop(index);
    out.push(record(
        "model.reverse_index",
        samples,
        Some(memory),
        expected,
    ));
}

/// The probe's own assertion, for the smoke run: two passes visit exactly
/// twice what one does.
pub(crate) fn probe_doubles(model: &Model) {
    let roots = roots(model);
    let once = traverse(model, &roots, 1);
    assert!(once > 0, "the traversal visited nothing");
    assert_eq!(traverse(model, &roots, 2), 2 * once);
}
