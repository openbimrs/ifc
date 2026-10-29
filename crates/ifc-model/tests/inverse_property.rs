//! Seeded random edit sequences round-trip through `Inverse` and `History`
//! (#465).
//!
//! Each round records a short random run of every public write, then checks
//! that applying the inverse restores the exact state (file order, unsorted
//! type buckets, header, next id), that `Restored` equals the id-set
//! differences, that the redo returns to the post-change state and that a
//! second application is refused. A history walk compares undo and redo
//! against an oracle of past states.
//!
//! Deterministic: a fixed list of seeds drives a local SplitMix64, copied
//! from `index_coherence_property.rs` so a failure names its seed and
//! replays identically. No dependency is added.

use std::collections::{BTreeSet, VecDeque};

#[path = "support/maintained.rs"]
mod maintained;

use ifc_model::{
    Edit, Entity, EntityId, Header, History, Inverse, InverseError, Model, Transaction, Value,
};

/// SplitMix64: tiny, fast, and good enough to explore edit orders.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn below(&mut self, bound: u64) -> u64 {
        self.next() % bound
    }

    fn chance(&mut self, percent: u64) -> bool {
        self.below(100) < percent
    }
}

const SEEDS: [u64; 24] = [
    0,
    1,
    2,
    3,
    5,
    8,
    13,
    21,
    34,
    55,
    89,
    144,
    233,
    377,
    610,
    987,
    1597,
    2584,
    4181,
    6765,
    0xC0FFEE,
    0xDEAD_BEEF,
    0x1F2E_3D4C,
    u64::MAX,
];
const ROUNDS: usize = 40;
const WARMUP: usize = 30;
/// Ids are drawn from a small range so edits collide with existing entities.
const ID_RANGE: u64 = 24;
const TYPES: [&str; 5] = ["IFCTESTA", "IFCTESTB", "IfcTestC", "IFCTESTD", "IFCTESTE"];

#[derive(Debug, Clone, PartialEq)]
struct State {
    header: Header,
    ids: Vec<EntityId>,
    entities: Vec<(EntityId, Entity)>,
    buckets: Vec<(String, Vec<EntityId>)>,
    histogram: Vec<(String, usize)>,
    next_id: EntityId,
}

fn state(model: &Model) -> State {
    maintained::assert_maintained(model, "state");
    let histogram: Vec<_> = model
        .type_histogram()
        .into_iter()
        .map(|(name, n)| (name.to_owned(), n))
        .collect();
    let mut buckets: Vec<_> = histogram
        .iter()
        .map(|(name, _)| (name.clone(), model.ids_of_type(name).to_vec()))
        .collect();
    buckets.sort();
    State {
        header: model.header().clone(),
        ids: model.ids().collect(),
        entities: model.iter().map(|(id, e)| (id, e.clone())).collect(),
        buckets,
        histogram,
        next_id: model.next_id(),
    }
}

fn id(rng: &mut Rng) -> EntityId {
    EntityId(1 + rng.below(ID_RANGE))
}

fn existing(rng: &mut Rng, model: &Model) -> Option<EntityId> {
    let ids: Vec<_> = model.ids().collect();
    (!ids.is_empty()).then(|| ids[rng.below(ids.len() as u64) as usize])
}

/// An integer, a GlobalId-like text, a reference or a list of references.
fn value(rng: &mut Rng) -> Value {
    match rng.below(5) {
        0 => Value::Integer(rng.below(100) as i64),
        1 | 2 => maintained::pool_text(rng.below(100)),
        3 => Value::Ref(id(rng)),
        _ => Value::List((0..rng.below(3)).map(|_| Value::Ref(id(rng))).collect()),
    }
}

fn entity(rng: &mut Rng) -> Entity {
    let name = TYPES[rng.below(TYPES.len() as u64) as usize];
    let len = rng.below(4) as usize;
    Entity::new(name, (0..len).map(|_| value(rng)).collect())
}

fn header(rng: &mut Rng) -> Header {
    Header {
        name: format!("file{}", rng.below(10)),
        schema: vec![format!("IFC{}", rng.below(4))],
        ..Header::default()
    }
}

/// One random write through the public API.
fn op(rng: &mut Rng, model: &mut Model) {
    match rng.below(9) {
        0 => model.insert(id(rng), entity(rng)),
        1 => {
            model.push(entity(rng));
        }
        2 => {
            if let Some(target) = existing(rng, model) {
                model.remove(target);
            }
        }
        3 => {
            if let Some(target) = existing(rng, model) {
                let name = TYPES[rng.below(TYPES.len() as u64) as usize];
                model.retype(target, name);
            }
        }
        4 => {
            if let Some(target) = existing(rng, model) {
                let slot = rng.below(5) as usize;
                model.set_attribute(target, slot, value(rng));
            }
        }
        5 => {
            if let Some(target) = existing(rng, model) {
                let edits: Vec<_> = (0..1 + rng.below(3))
                    .map(|_| (rng.below(5) as usize, value(rng)))
                    .collect();
                model.set_attributes(target, edits);
            }
        }
        6 => *model.header_mut() = header(rng),
        7 => transaction(rng, model),
        _ => {
            // A nested recording whose own inverse is discarded.
            Inverse::record(model, |m| {
                for _ in 0..1 + rng.below(3) {
                    op_flat(rng, m);
                }
            });
        }
    }
}

/// Writes without nesting, so recursion terminates.
fn op_flat(rng: &mut Rng, model: &mut Model) {
    if rng.chance(50) {
        model.push(entity(rng));
    } else if let Some(target) = existing(rng, model) {
        model.set_attribute(target, rng.below(5) as usize, Value::Null);
    }
}

/// A random transaction, committed reversibly; it may be refused.
fn transaction(rng: &mut Rng, model: &mut Model) {
    let mut tx = Transaction::new(model);
    for _ in 0..1 + rng.below(4) {
        match rng.below(4) {
            0 => {
                tx.create(entity(rng));
            }
            1 => {
                tx.stage(Edit::Create {
                    id: id(rng),
                    entity: entity(rng),
                });
            }
            2 => {
                tx.remove(id(rng));
            }
            _ => {
                tx.set_attribute(id(rng), rng.below(5) as usize, Value::Integer(1));
            }
        }
    }
    if rng.chance(40) {
        tx.set_header(header(rng));
    }
    let before = state(model);
    match tx.commit_reversible(model) {
        Ok((_, inverse)) => {
            if rng.chance(30) {
                // Undo it again: the nested inverse is a real inverse.
                inverse.apply(model).expect("fresh inverse applies");
            }
        }
        Err(_) => assert_eq!(state(model), before, "a refused commit changed the model"),
    }
}

/// Half the seeds build the indexes before the first write, the rest at
/// the first `state`, so both histories of an index are covered.
fn warm(rng: &mut Rng) -> Model {
    let mut model = Model::new();
    if rng.chance(50) {
        model.reverse_index();
        model.global_id_candidates(&ifc_model::guid::Guid::parse(maintained::POOL[0]).unwrap());
    }
    for _ in 0..WARMUP {
        op(rng, &mut model);
    }
    model
}

fn id_set(state: &State) -> BTreeSet<EntityId> {
    state.ids.iter().copied().collect()
}

#[test]
fn applying_an_inverse_restores_the_exact_state() {
    for seed in SEEDS {
        let mut rng = Rng(seed);
        let mut model = warm(&mut rng);
        for round in 0..ROUNDS {
            let ctx = format!("seed {seed} round {round}");
            let s0 = state(&model);
            let ((), inverse) = Inverse::record(&mut model, |m| {
                for _ in 0..1 + rng.below(6) {
                    op(&mut rng, m);
                }
            });
            let s1 = state(&model);

            let stale = inverse.clone();
            let (restored, redo) = inverse
                .apply(&mut model)
                .unwrap_or_else(|e| panic!("{ctx}: {e}"));
            assert_eq!(state(&model), s0, "{ctx}: undo");

            let (before, after) = (id_set(&s1), id_set(&s0));
            let set = |ids: &[EntityId]| ids.iter().copied().collect::<BTreeSet<_>>();
            assert_eq!(
                set(&restored.created),
                after.difference(&before).copied().collect(),
                "{ctx}: created"
            );
            assert_eq!(
                set(&restored.removed),
                before.difference(&after).copied().collect(),
                "{ctx}: removed"
            );
            for id in &restored.modified {
                assert!(before.contains(id) && after.contains(id), "{ctx}: modified");
            }
            if s0.header != s1.header {
                assert!(restored.header, "{ctx}: header flag");
            }
            // Every id whose entity differs must be listed.
            for (id, entity) in &s0.entities {
                let was = s1.entities.iter().find(|(i, _)| i == id).map(|(_, e)| e);
                if was.is_some_and(|e| e != entity) {
                    assert!(
                        restored.modified.contains(id),
                        "{ctx}: unlisted edit of {id:?}"
                    );
                }
            }

            // An empty inverse never moved the revision, so it stays valid.
            if !stale.is_empty() {
                assert!(matches!(
                    stale.apply(&mut model),
                    Err(InverseError::StaleRevision { .. })
                ));
            }
            assert_eq!(state(&model), s0, "{ctx}: stale apply changed the model");

            redo.apply(&mut model)
                .unwrap_or_else(|e| panic!("{ctx}: redo {e}"));
            assert_eq!(state(&model), s1, "{ctx}: redo");
        }
    }
}

#[test]
fn a_preview_restores_the_exact_state_and_revision() {
    for seed in SEEDS {
        let mut rng = Rng(seed ^ 0x5EED);
        let mut model = warm(&mut rng);
        for round in 0..ROUNDS {
            let ctx = format!("seed {seed} round {round}");
            let s0 = state(&model);
            let r0 = model.revision();
            let wrapped = rng.chance(30);
            let mut run = |m: &mut Model| {
                m.preview(|m| {
                    for _ in 0..1 + rng.below(6) {
                        op(&mut rng, m);
                    }
                    m.revision()
                })
            };
            let drawn = if wrapped {
                let (drawn, inverse) = Inverse::record(&mut model, run);
                assert!(inverse.is_empty(), "{ctx}: the recording kept steps");
                drawn
            } else {
                run(&mut model)
            };
            assert_eq!(state(&model), s0, "{ctx}: preview");
            assert_eq!(model.revision(), r0, "{ctx}: revision");
            op(&mut rng, &mut model);
            assert!(
                model.revision() == r0 || model.revision() > drawn,
                "{ctx}: a drawn revision was handed out again"
            );
        }
    }
}

#[test]
fn history_walk_matches_a_state_oracle() {
    for seed in SEEDS {
        let mut rng = Rng(seed ^ 0xA5A5);
        let mut model = warm(&mut rng);
        let mut limit = 1 + rng.below(8) as usize;
        let mut history = History::new(limit);
        // States before each undo entry (the last is the next undo target),
        // and states after each undone entry (the last is the next redo).
        let mut past: VecDeque<State> = VecDeque::new();
        let mut future: Vec<State> = Vec::new();
        for step in 0..120 {
            let ctx = format!("seed {seed} step {step}");
            let current = state(&model);
            match rng.below(5) {
                0 | 1 => {
                    let ((), inverse) = Inverse::record(&mut model, |m| {
                        for _ in 0..1 + rng.below(4) {
                            op(&mut rng, m);
                        }
                    });
                    if !inverse.is_empty() {
                        past.push_back(current);
                        future.clear();
                        while past.len() > limit {
                            past.pop_front();
                        }
                    }
                    history.push(inverse);
                }
                2 => {
                    let result = history.undo(&mut model);
                    match past.pop_back() {
                        Some(previous) => {
                            result.unwrap_or_else(|e| panic!("{ctx}: {e}"));
                            assert_eq!(state(&model), previous, "{ctx}: undo");
                            future.push(current);
                            while future.len() > limit {
                                future.remove(0);
                            }
                        }
                        None => assert!(result.is_err(), "{ctx}"),
                    }
                }
                3 => {
                    let result = history.redo(&mut model);
                    match future.pop() {
                        Some(next) => {
                            result.unwrap_or_else(|e| panic!("{ctx}: {e}"));
                            assert_eq!(state(&model), next, "{ctx}: redo");
                            past.push_back(current);
                            while past.len() > limit {
                                past.pop_front();
                            }
                        }
                        None => assert!(result.is_err(), "{ctx}"),
                    }
                }
                _ => {
                    limit = 1 + rng.below(8) as usize;
                    history.set_limit(limit);
                    while past.len() > limit {
                        past.pop_front();
                    }
                    while future.len() > limit {
                        future.remove(0);
                    }
                }
            }
            assert_eq!(history.undo_len(), past.len(), "{ctx}: undo depth");
            assert_eq!(history.redo_len(), future.len(), "{ctx}: redo depth");
        }
    }
}
