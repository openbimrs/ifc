//! Seeded random edit sequences keep every index coherent with storage (#106).
//!
//! `Model` maintains its type index (`by_type`) and file order (`order`)
//! incrementally in `insert`, `retype`, `remove` and `Transaction::commit`.
//! The hand-picked cases in `type_index_consistency.rs` and `mutation.rs`
//! cover the sequences someone thought of; this test drives long random
//! sequences of every public mutation -- including refused, stale and
//! partly self-cancelling transactions -- and after each step compares the
//! model against:
//!
//! - a shadow store that applies the same operations naively (a `Vec` in
//!   file order), so storage itself is checked, not only its indices;
//! - a model rebuilt from scratch by inserting the stored entities in file
//!   order, so the incremental index must equal the one a fresh load builds;
//! - a scan: `ids_of_type` against the entities' own type names,
//!   `type_histogram` against counted types, `ids()` against the entity map
//!   with no duplicates, and `ReverseIndex` against references walked
//!   independently of `Value::for_each_ref`.
//!
//! A refused transaction must leave the model exactly as it was, revision
//! included.
//!
//! Deterministic: a fixed list of seeds drives a local SplitMix64, so a
//! failure names its seed and step and replays identically. No dependency
//! is added; the generator is twelve lines. Lazily registered entities
//! (`insert_lazy`) need a codec source and are out of scope here.

use std::collections::BTreeMap;

use ifc_model::{Edit, Entity, EntityId, Model, Referrer, ReverseIndex, Transaction, Value};

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
const STEPS: usize = 250;
/// Ids are drawn from a small range so edits collide with existing entities.
const ID_RANGE: u64 = 24;
/// Mixed case on purpose: the index keys on the upper-cased name. Not
/// schema names: the model is schema-agnostic, and real IFC names here would
/// count as retype coverage in the authored-coverage measurement
/// (`scripts/authored-coverage.py`) that this test does not prove.
const TYPES: [&str; 6] = [
    "XTESTALPHA",
    "XTestAlpha",
    "XTESTBETA",
    "xtestgamma",
    "XTESTDELTA",
    "XTestDelta",
];

/// The naive model: entities in file order, plus the highest id ever seen.
#[derive(Debug, Clone, PartialEq)]
struct Shadow {
    order: Vec<(EntityId, Entity)>,
    max_id: u64,
}

impl Shadow {
    fn get(&self, id: EntityId) -> Option<&Entity> {
        self.order.iter().find(|(i, _)| *i == id).map(|(_, e)| e)
    }

    fn get_mut(&mut self, id: EntityId) -> Option<&mut Entity> {
        self.order
            .iter_mut()
            .find(|(i, _)| *i == id)
            .map(|(_, e)| e)
    }

    fn insert(&mut self, id: EntityId, entity: Entity) {
        match self.get_mut(id) {
            Some(slot) => *slot = entity,
            None => self.order.push((id, entity)),
        }
        self.max_id = self.max_id.max(id.0);
    }

    fn remove(&mut self, id: EntityId) -> Option<Entity> {
        let position = self.order.iter().position(|(i, _)| *i == id)?;
        Some(self.order.remove(position).1)
    }

    fn retype(&mut self, id: EntityId, type_name: &str) -> Option<String> {
        let entity = self.get_mut(id)?;
        let previous = entity.type_name.to_string();
        entity.type_name = type_name.into();
        Some(previous)
    }

    fn set_attribute(&mut self, id: EntityId, slot: usize, value: Value) -> Option<Value> {
        let entity = self.get_mut(id)?;
        if slot >= entity.attributes.len() {
            entity.attributes.resize(slot + 1, Value::Null);
        }
        Some(std::mem::replace(&mut entity.attributes[slot], value))
    }

    fn apply(&mut self, edit: &Edit) {
        match edit {
            Edit::Create { id, entity } => self.insert(*id, entity.clone()),
            Edit::SetAttribute { id, slot, value } => {
                self.set_attribute(*id, *slot, value.clone());
            }
            Edit::Retype { id, type_name } => {
                self.retype(*id, type_name);
            }
            Edit::Remove { id } => {
                self.remove(*id);
            }
        }
    }
}

fn random_id(rng: &mut Rng) -> EntityId {
    EntityId(1 + rng.below(ID_RANGE))
}

fn random_type(rng: &mut Rng) -> &'static str {
    TYPES[rng.below(TYPES.len() as u64) as usize]
}

/// A value that may reference an existing id, an absent one, or nothing.
fn random_value(rng: &mut Rng, shadow: &Shadow, depth: u32) -> Value {
    let existing = |rng: &mut Rng| {
        if shadow.order.is_empty() {
            random_id(rng)
        } else {
            shadow.order[rng.below(shadow.order.len() as u64) as usize].0
        }
    };
    match rng.below(if depth == 0 { 7 } else { 5 }) {
        0 => Value::Null,
        1 => Value::Text(format!("t{}", rng.below(100)).into()),
        // Mostly references that resolve, so transactions often commit.
        2 | 3 => Value::Ref(existing(rng)),
        4 => Value::Ref(random_id(rng)),
        5 => Value::List(
            (0..rng.below(4))
                .map(|_| random_value(rng, shadow, depth + 1))
                .collect(),
        ),
        _ => Value::Typed {
            type_name: "IFCLABEL".into(),
            value: Box::new(random_value(rng, shadow, depth + 1)),
        },
    }
}

fn random_entity(rng: &mut Rng, shadow: &Shadow) -> Entity {
    let arity = rng.below(4) as usize;
    Entity::new(
        random_type(rng),
        (0..arity).map(|_| random_value(rng, shadow, 0)).collect(),
    )
}

/// Every reference in `value`, walked here rather than through
/// `Value::for_each_ref` so the oracle does not share the code under test.
fn refs(value: &Value, out: &mut Vec<EntityId>) {
    match value {
        Value::Ref(id) => out.push(*id),
        Value::List(items) => items.iter().for_each(|item| refs(item, out)),
        Value::Typed { value, .. } => refs(value, out),
        _ => {}
    }
}

/// Everything observable about the model's storage and indices.
#[derive(Debug, PartialEq)]
struct Snapshot {
    revision: u64,
    ids: Vec<EntityId>,
    entities: Vec<(EntityId, Entity)>,
    histogram: Vec<(String, usize)>,
    by_type: Vec<(String, Vec<EntityId>)>,
    next_id: EntityId,
}

fn snapshot(model: &Model) -> Snapshot {
    Snapshot {
        revision: model.revision(),
        ids: model.ids().collect(),
        entities: model.iter().map(|(id, e)| (id, e.clone())).collect(),
        histogram: model
            .type_histogram()
            .into_iter()
            .map(|(name, count)| (name.to_owned(), count))
            .collect(),
        by_type: TYPES
            .iter()
            .map(|name| ((*name).to_owned(), model.ids_of_type(name).to_vec()))
            .collect(),
        next_id: model.next_id(),
    }
}

/// Assert every index agrees with the shadow, a scan and a fresh rebuild.
fn check(model: &Model, shadow: &Shadow, at: &str) {
    // Storage and file order.
    let ids: Vec<EntityId> = model.ids().collect();
    let expected_ids: Vec<EntityId> = shadow.order.iter().map(|(id, _)| *id).collect();
    assert_eq!(ids, expected_ids, "{at}: file order");
    let mut unique = ids.clone();
    unique.sort_unstable();
    unique.dedup();
    assert_eq!(unique.len(), ids.len(), "{at}: ids() has duplicates");
    assert_eq!(model.len(), ids.len(), "{at}: len() vs ids()");
    let stored: Vec<(EntityId, Entity)> = model.iter().map(|(i, e)| (i, e.clone())).collect();
    assert_eq!(stored, shadow.order, "{at}: stored entities");
    for raw in 0..=ID_RANGE + 2 {
        let id = EntityId(raw);
        assert_eq!(
            model.contains(id),
            shadow.get(id).is_some(),
            "{at}: contains({id})"
        );
    }
    assert_eq!(
        model.next_id(),
        EntityId(shadow.max_id + 1),
        "{at}: next_id"
    );

    // Type index and histogram against a scan.
    let mut scan: BTreeMap<String, Vec<EntityId>> = BTreeMap::new();
    for (id, entity) in &shadow.order {
        scan.entry(entity.type_name.to_ascii_uppercase())
            .or_default()
            .push(*id);
    }
    for name in TYPES {
        let mut indexed = model.ids_of_type(name).to_vec();
        let count = indexed.len();
        indexed.sort_unstable();
        indexed.dedup();
        assert_eq!(indexed.len(), count, "{at}: ids_of_type({name}) duplicates");
        let expected = scan
            .get(&name.to_ascii_uppercase())
            .cloned()
            .unwrap_or_default();
        let mut expected_sorted = expected.clone();
        expected_sorted.sort_unstable();
        assert_eq!(indexed, expected_sorted, "{at}: ids_of_type({name})");
    }
    let mut histogram: Vec<(String, usize)> = scan
        .iter()
        .map(|(name, ids)| (name.clone(), ids.len()))
        .collect();
    histogram.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let actual: Vec<(String, usize)> = model
        .type_histogram()
        .into_iter()
        .map(|(name, count)| (name.to_owned(), count))
        .collect();
    assert_eq!(actual, histogram, "{at}: type_histogram");

    // A model rebuilt from the stored entities has the same indices.
    let mut rebuilt = Model::new();
    for (id, entity) in model.iter() {
        rebuilt.insert(id, entity.clone());
    }
    assert_eq!(
        rebuilt.type_histogram(),
        model.type_histogram(),
        "{at}: histogram vs rebuild"
    );
    for name in TYPES {
        let mut a = model.ids_of_type(name).to_vec();
        let mut b = rebuilt.ids_of_type(name).to_vec();
        a.sort_unstable();
        b.sort_unstable();
        assert_eq!(a, b, "{at}: ids_of_type({name}) vs rebuild");
    }

    // ReverseIndex against an independent reference walk.
    let mut incoming: BTreeMap<EntityId, Vec<Referrer>> = BTreeMap::new();
    for (from, entity) in &shadow.order {
        for (slot, attribute) in entity.attributes.iter().enumerate() {
            let mut targets = Vec::new();
            refs(attribute, &mut targets);
            for target in targets {
                incoming
                    .entry(target)
                    .or_default()
                    .push(Referrer { from: *from, slot });
            }
        }
    }
    for referrers in incoming.values_mut() {
        referrers.sort_unstable();
        referrers.dedup();
    }
    let index = ReverseIndex::build(model);
    let fresh = ReverseIndex::build(&rebuilt);
    assert_eq!(index.len(), incoming.len(), "{at}: ReverseIndex::len");
    for raw in 0..=ID_RANGE + 2 {
        let target = EntityId(raw);
        let expected = incoming.get(&target).map_or(&[][..], Vec::as_slice);
        assert_eq!(
            index.referrers(target),
            expected,
            "{at}: referrers({target})"
        );
        assert_eq!(
            fresh.referrers(target),
            expected,
            "{at}: rebuilt referrers({target})"
        );
    }
}

/// Stage 1..=5 random edits on `tx`.
fn stage_random(rng: &mut Rng, tx: &mut Transaction, shadow: &Shadow) {
    for _ in 0..1 + rng.below(5) {
        match rng.below(6) {
            0 => {
                tx.create(random_entity(rng, shadow));
            }
            // An explicit id: may reuse a removed id or collide with a live one.
            1 => {
                tx.stage(Edit::Create {
                    id: random_id(rng),
                    entity: random_entity(rng, shadow),
                });
            }
            2 => {
                let value = random_value(rng, shadow, 0);
                tx.set_attribute(random_id(rng), rng.below(4) as usize, value);
            }
            3 => {
                tx.retype(random_id(rng), random_type(rng));
            }
            _ => {
                tx.remove(random_id(rng));
            }
        }
    }
}

#[derive(Debug, Default)]
struct Tally {
    committed: usize,
    refused: usize,
    stale: usize,
}

/// Run one transaction against `model`, checking refusal leaves it intact.
fn transaction(rng: &mut Rng, model: &mut Model, shadow: &mut Shadow, tally: &mut Tally, at: &str) {
    let mut tx = Transaction::new(model);
    stage_random(rng, &mut tx, shadow);

    // Sometimes the model moves underneath an open transaction.
    let stale = rng.chance(10);
    if stale {
        let id = model.push(Entity::new("XTESTSTALE", vec![]));
        shadow.insert(id, Entity::new("XTESTSTALE", vec![]));
    }

    let before = snapshot(model);
    let edits = tx.edits().to_vec();
    match tx.commit(model) {
        Ok(applied) => {
            assert!(!stale, "{at}: a stale transaction committed");
            tally.committed += 1;
            let mut created = Vec::new();
            for edit in &edits {
                if let Edit::Create { id, .. } = edit {
                    created.push(*id);
                }
                shadow.apply(edit);
            }
            assert_eq!(applied.created, created, "{at}: Applied::created");
            assert_eq!(
                applied.revision,
                model.revision(),
                "{at}: Applied::revision"
            );
        }
        Err(conflicts) => {
            assert!(!conflicts.is_empty(), "{at}: refused with no conflict");
            if stale {
                tally.stale += 1;
            } else {
                tally.refused += 1;
            }
            assert_eq!(
                snapshot(model),
                before,
                "{at}: a refused transaction changed the model: {conflicts:?}"
            );
        }
    }
}

fn run(seed: u64, tally: &mut Tally) {
    let mut rng = Rng(seed);
    let mut model = Model::new();
    let mut shadow = Shadow {
        order: Vec::new(),
        max_id: 0,
    };

    for step in 0..STEPS {
        let at = format!("seed {seed:#x} step {step}");
        match rng.below(10) {
            0 | 1 => {
                let id = random_id(&mut rng);
                let entity = random_entity(&mut rng, &shadow);
                model.insert(id, entity.clone());
                shadow.insert(id, entity);
            }
            2 => {
                let entity = random_entity(&mut rng, &shadow);
                let id = model.push(entity.clone());
                assert_eq!(id, EntityId(shadow.max_id + 1), "{at}: push id");
                shadow.insert(id, entity);
            }
            3 => {
                let id = random_id(&mut rng);
                assert_eq!(model.remove(id), shadow.remove(id), "{at}: remove({id})");
            }
            4 => {
                let id = random_id(&mut rng);
                let name = random_type(&mut rng);
                assert_eq!(
                    model.retype(id, name).map(|p| p.to_string()),
                    shadow.retype(id, name),
                    "{at}: retype({id}, {name})"
                );
            }
            5 => {
                let id = random_id(&mut rng);
                let slot = rng.below(5) as usize;
                let value = random_value(&mut rng, &shadow, 0);
                assert_eq!(
                    model.set_attribute(id, slot, value.clone()),
                    shadow.set_attribute(id, slot, value),
                    "{at}: set_attribute({id}, {slot})"
                );
            }
            6 => {
                let id = random_id(&mut rng);
                let edits: Vec<(usize, Value)> = (0..1 + rng.below(3))
                    .map(|_| (rng.below(5) as usize, random_value(&mut rng, &shadow, 0)))
                    .collect();
                let expected = shadow.get(id).is_some().then(|| {
                    edits
                        .iter()
                        .map(|(slot, value)| {
                            shadow
                                .set_attribute(id, *slot, value.clone())
                                .expect("present")
                        })
                        .collect::<Vec<_>>()
                });
                assert_eq!(
                    model.set_attributes(id, edits),
                    expected,
                    "{at}: set_attributes({id})"
                );
            }
            _ => transaction(&mut rng, &mut model, &mut shadow, tally, &at),
        }
        check(&model, &shadow, &at);
    }
}

#[test]
fn indices_stay_coherent_under_random_edit_sequences() {
    let mut tally = Tally::default();
    for seed in SEEDS {
        run(seed, &mut tally);
    }
    // Guard against a generator that never reaches a branch: each outcome
    // must occur often enough to have been exercised.
    assert!(tally.committed >= 100, "too few commits: {tally:?}");
    assert!(tally.refused >= 100, "too few refusals: {tally:?}");
    assert!(tally.stale >= 20, "too few stale commits: {tally:?}");
}
