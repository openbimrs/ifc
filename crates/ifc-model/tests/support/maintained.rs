//! The oracle for the indexes a model keeps once built (#587): the
//! maintained reverse index equals a fresh build, and the GlobalId
//! candidates equal a scan with the same rule.

#![allow(dead_code)] // each test binary uses a part

use std::collections::BTreeMap;

use ifc_model::guid::Guid;
use ifc_model::{EntityId, Model, ReverseIndex, Value};

/// Texts for the first attribute: three valid GlobalIds, so edits collide on
/// them, and four that are not (bad lead digit, bad alphabet, too short,
/// too long).
pub const POOL: [&str; 7] = [
    "0000000000000000000001",
    "0000000000000000000002",
    "3zzzzzzzzzzzzzzzzzzzzz",
    "4000000000000000000000",
    "000000000000000000000!",
    "0000",
    "00000000000000000000000",
];

pub fn pool_text(n: u64) -> Value {
    Value::Text(POOL[(n as usize) % POOL.len()].into())
}

/// Asserts both indexes agree with the model's entities. Builds them when
/// nothing did yet.
pub fn assert_maintained(model: &Model, at: &str) {
    assert_eq!(
        model.reverse_index(),
        &ReverseIndex::build(model),
        "{at}: the maintained reverse index differs from a fresh build"
    );
    let mut scan: BTreeMap<Guid, Vec<EntityId>> = BTreeMap::new();
    for (id, entity) in model.iter() {
        if let Some(Value::Text(text)) = entity.attributes.first() {
            if let Some(guid) = Guid::parse(text) {
                scan.entry(guid).or_default().push(id);
            }
        }
    }
    for (guid, ids) in &scan {
        let mut expected = ids.clone();
        expected.sort_unstable();
        assert_eq!(
            model.global_id_candidates(guid),
            expected,
            "{at}: candidates of {guid}"
        );
    }
    for text in POOL {
        if let Some(guid) = Guid::parse(text) {
            if !scan.contains_key(&guid) {
                assert!(
                    model.global_id_candidates(&guid).is_empty(),
                    "{at}: stale candidates of {guid}"
                );
            }
        }
    }
}
