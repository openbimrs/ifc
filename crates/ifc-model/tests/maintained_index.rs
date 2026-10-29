//! The reverse and GlobalId indexes a model keeps once built (#587).
//!
//! After the first query every public write keeps both current; each test
//! builds them, writes, and compares with a fresh build and a scan
//! (`support/maintained.rs`). The seeded sequences are in
//! `index_coherence_property.rs` and `inverse_property.rs`.

use std::ops::Range;
use std::sync::Arc;

#[path = "support/maintained.rs"]
mod maintained;

use ifc_model::guid::Guid;
use ifc_model::{
    Edit, Entity, EntityId, EntitySource, History, Inverse, Model, Referrer, ReverseIndex,
    Transaction, Value,
};
use maintained::{assert_maintained, POOL};

const G1: usize = 0;
const G2: usize = 1;

fn guid(n: usize) -> Guid {
    Guid::parse(POOL[n]).expect("a valid GlobalId")
}

fn text(n: usize) -> Value {
    Value::Text(POOL[n].into())
}

fn entity(type_name: &str, attributes: Vec<Value>) -> Entity {
    Entity::new(type_name, attributes)
}

fn r(id: u64) -> Value {
    Value::Ref(EntityId(id))
}

/// Three entities: a root holding G1 and a reference, a target, a list.
fn base() -> Model {
    let mut model = Model::new();
    model.insert(EntityId(1), entity("XROOT", vec![text(G1), r(2)]));
    model.insert(EntityId(2), entity("XTARGET", vec![]));
    model.insert(
        EntityId(3),
        entity("XLIST", vec![Value::Null, Value::List(vec![r(1), r(2)])]),
    );
    model
}

/// `base` with both indexes built.
fn built() -> Model {
    let model = base();
    assert_maintained(&model, "base");
    model
}

#[test]
fn a_fresh_model_builds_equal_indexes_without_a_revision_bump() {
    let model = base();
    let revision = model.revision();
    assert_maintained(&model, "fresh");
    assert_eq!(model.revision(), revision);
    assert_eq!(
        model.reverse_index().referrers(EntityId(2)),
        [
            Referrer {
                from: EntityId(1),
                slot: 1
            },
            Referrer {
                from: EntityId(3),
                slot: 1
            }
        ]
    );
}

#[test]
fn indexes_are_not_built_unless_asked() {
    let mut model = base();
    model.set_attribute(EntityId(2), 0, r(1));
    model.remove(EntityId(3));
    model.push(entity("XNEW", vec![r(1)]));
    let text = format!("{model:?}");
    assert!(text.contains("reverse: unbuilt"), "{text}");
    assert!(text.contains("global_ids: unbuilt"), "{text}");
    model.reverse_index();
    let text = format!("{model:?}");
    assert!(text.contains("reverse: built"), "{text}");
    assert!(text.contains("global_ids: unbuilt"), "{text}");
}

#[test]
fn insert_and_push_are_indexed() {
    let mut model = built();
    model.insert(EntityId(10), entity("XROOT", vec![text(G2), r(1), r(99)]));
    assert_maintained(&model, "insert");
    let id = model.push(entity("XROOT", vec![text(G1), r(10)]));
    assert_maintained(&model, "push");
    assert_eq!(model.global_id_candidates(&guid(G1)), [EntityId(1), id]);
}

#[test]
fn insert_over_an_occupant_moves_only_the_difference() {
    let mut model = built();
    // Same type, other references and GlobalId.
    model.insert(EntityId(1), entity("XROOT", vec![text(G2), r(3)]));
    assert_maintained(&model, "same type");
    // Other type.
    model.insert(EntityId(1), entity("XOTHER", vec![r(2), r(2)]));
    assert_maintained(&model, "other type");
    // Identical content.
    model.insert(EntityId(1), entity("XOTHER", vec![r(2), r(2)]));
    assert_maintained(&model, "identical");
}

#[test]
fn set_attribute_adds_removes_and_extends() {
    let mut model = built();
    model.set_attribute(EntityId(2), 0, r(1));
    assert_maintained(&model, "add a reference");
    model.set_attribute(EntityId(2), 0, Value::Null);
    assert_maintained(&model, "remove it");
    model.set_attribute(EntityId(2), 4, r(3));
    assert_maintained(&model, "write past the end");
    model.set_attribute(EntityId(1), 1, Value::List(vec![r(2), r(2), r(3)]));
    assert_maintained(&model, "a list naming a target twice");
    assert!(model.set_attribute(EntityId(77), 0, r(1)).is_none());
    assert_maintained(&model, "missing entity");
}

#[test]
fn set_attributes_applies_several_slots_and_the_last_edit_wins() {
    let mut model = built();
    model.set_attributes(
        EntityId(3),
        [
            (0, text(G2)),
            (1, r(1)),
            (1, Value::List(vec![r(2)])),
            (3, r(1)),
        ],
    );
    assert_maintained(&model, "several slots");
    model.set_attributes(EntityId(3), []);
    assert_maintained(&model, "no edits");
}

#[test]
fn retype_leaves_the_indexes_alone() {
    let mut model = built();
    model.retype(EntityId(1), "XRETYPED");
    assert_maintained(&model, "retype");
}

#[test]
fn remove_drops_references_and_leaves_dangling_ones() {
    let mut model = built();
    model.remove(EntityId(3));
    assert_maintained(&model, "remove a referrer");
    // Removing a target keeps its referrers: the reference now dangles.
    model.remove(EntityId(2));
    assert_maintained(&model, "remove a target");
    assert!(model.reverse_index().is_referenced(EntityId(2)));
    model.remove(EntityId(1));
    assert_maintained(&model, "remove the last");
    assert!(model.reverse_index().is_empty());
    assert!(model.remove(EntityId(1)).is_none());
}

#[test]
fn a_committed_transaction_updates_and_a_refused_one_changes_nothing() {
    let mut model = built();
    let mut tx = Transaction::new(&model);
    let created = tx.create(entity("XROOT", vec![text(G2), r(1)]));
    tx.set_attribute(EntityId(2), 0, r(created.0));
    tx.remove(EntityId(3));
    tx.commit(&mut model).expect("the transaction commits");
    assert_maintained(&model, "committed");

    let revision = model.revision();
    let index = model.reverse_index().clone();
    // Removing a referenced entity is refused.
    let mut tx = Transaction::new(&model);
    tx.remove(EntityId(2));
    tx.stage(Edit::Create {
        id: EntityId(50),
        entity: entity("XROOT", vec![text(G1)]),
    });
    assert!(tx.commit(&mut model).is_err());
    assert_eq!(model.revision(), revision);
    assert_eq!(model.reverse_index(), &index);
    assert_maintained(&model, "refused");
    assert!(model.get(EntityId(50)).is_none());
}

#[test]
fn undo_and_redo_keep_the_indexes() {
    let mut model = built();
    let (_, inverse) = Inverse::record(&mut model, |m| {
        m.insert(EntityId(5), entity("XROOT", vec![text(G1), r(2)]));
        m.insert(EntityId(1), entity("XROOT", vec![text(G2)]));
        m.set_attribute(EntityId(2), 6, r(3));
        m.set_attributes(EntityId(3), [(0, r(1)), (1, Value::Null)]);
        m.remove(EntityId(3));
        m.retype(EntityId(2), "XOTHER");
    });
    assert_maintained(&model, "after");
    let (_, redo) = inverse.apply(&mut model).expect("undo");
    assert_maintained(&model, "undone");
    redo.apply(&mut model).expect("redo");
    assert_maintained(&model, "redone");
}

/// Undoing a write past the end truncates the slot it created, with its
/// reference.
#[test]
fn undoing_a_slot_written_past_the_end_drops_its_reference() {
    let mut model = built();
    let before = model.reverse_index().clone();
    let (_, inverse) = Inverse::record(&mut model, |m| {
        m.set_attribute(EntityId(2), 5, r(3));
    });
    assert_maintained(&model, "written");
    assert!(model
        .reverse_index()
        .referrers(EntityId(3))
        .contains(&Referrer {
            from: EntityId(2),
            slot: 5
        }));
    inverse.apply(&mut model).expect("undo");
    assert_maintained(&model, "undone");
    assert_eq!(model.reverse_index(), &before);
}

#[test]
fn history_walks_keep_the_indexes() {
    let mut model = built();
    let mut history = History::new(8);
    for n in 0..5_u64 {
        history.track(&mut model, |m| {
            m.push(entity("XROOT", vec![text(n as usize % 3), r(1 + n % 3)]));
            m.set_attribute(EntityId(1 + n % 3), 2, r(1));
        });
        assert_maintained(&model, "tracked");
    }
    while history.can_undo() {
        history.undo(&mut model).expect("undo");
        assert_maintained(&model, "undo");
    }
    while history.can_redo() {
        history.redo(&mut model).expect("redo");
        assert_maintained(&model, "redo");
    }
}

#[test]
fn global_id_candidates_follow_the_first_attribute() {
    let mut model = Model::new();
    let g = guid(G1);
    assert!(model.global_id_candidates(&g).is_empty());
    // Two holders, the higher id inserted first: candidates ascend.
    model.insert(EntityId(7), entity("XROOT", vec![text(G1)]));
    model.insert(EntityId(3), entity("XROOT", vec![text(G1)]));
    assert_eq!(model.global_id_candidates(&g), [EntityId(3), EntityId(7)]);
    // Text that is not a GlobalId is not indexed, valid or not.
    for invalid in 3..POOL.len() {
        model.insert(
            EntityId(20 + invalid as u64),
            entity("XROOT", vec![text(invalid)]),
        );
    }
    assert_maintained(&model, "invalid texts");
    // Slot 0 changed: the entity leaves one list and joins another.
    model.set_attribute(EntityId(3), 0, text(G2));
    assert_eq!(model.global_id_candidates(&g), [EntityId(7)]);
    assert_eq!(model.global_id_candidates(&guid(G2)), [EntityId(3)]);
    // A later slot does not matter.
    model.set_attribute(EntityId(7), 1, text(G2));
    assert_eq!(model.global_id_candidates(&g), [EntityId(7)]);
    // Removed, then restored by undo.
    let (_, inverse) = Inverse::record(&mut model, |m| {
        m.remove(EntityId(7));
    });
    assert!(model.global_id_candidates(&g).is_empty());
    inverse.apply(&mut model).expect("undo");
    assert_eq!(model.global_id_candidates(&g), [EntityId(7)]);
    assert_maintained(&model, "end");
}

/// Decodes `span.start` as an index into a list of entities.
#[derive(Debug)]
struct Table(Vec<Entity>);

impl EntitySource for Table {
    fn decode(&self, span: Range<usize>) -> Entity {
        self.0[span.start].clone()
    }
}

#[test]
fn lazy_registration_drops_the_indexes_and_they_rebuild_equal() {
    let table = Table(vec![
        entity("XROOT", vec![text(G1), r(2)]),
        entity("XTARGET", vec![]),
        entity("XROOT", vec![text(G1), r(1)]),
    ]);
    let mut model = Model::with_source(Arc::new(table));
    model.insert_lazy(EntityId(1), "XROOT", 0..1);
    model.insert_lazy(EntityId(2), "XTARGET", 1..2);
    assert_maintained(&model, "two lazy entities");
    model.insert_lazy(EntityId(3), "XROOT", 2..3);
    let text = format!("{model:?}");
    assert!(text.contains("reverse: unbuilt"), "{text}");
    assert_maintained(&model, "a third, registered after the build");
    assert_eq!(
        model.global_id_candidates(&guid(G1)),
        [EntityId(1), EntityId(3)]
    );
}

#[test]
fn a_clone_keeps_the_indexes_and_the_two_diverge() {
    let mut model = built();
    let mut copy = model.clone();
    assert_eq!(copy.reverse_index(), model.reverse_index());
    copy.set_attribute(EntityId(2), 0, r(3));
    model.remove(EntityId(1));
    assert_maintained(&copy, "clone");
    assert_maintained(&model, "original");
    assert!(copy.reverse_index().is_referenced(EntityId(3)));
    assert!(!model.reverse_index().is_referenced(EntityId(3)));
}

#[test]
fn dropped_indexes_rebuild_equal_and_are_not_kept_until_asked() {
    let mut model = built();
    model.drop_derived_indexes();
    let text = format!("{model:?}");
    assert!(text.contains("reverse: unbuilt"), "{text}");
    assert!(text.contains("global_ids: unbuilt"), "{text}");
    model.set_attribute(EntityId(2), 0, r(1));
    assert_maintained(&model, "rebuilt");
    assert_eq!(model.reverse_index(), &ReverseIndex::build(&model));
}

#[test]
fn a_preview_keeps_built_indexes() {
    let mut model = built();
    let revision = model.revision();
    model.preview(|m| {
        m.set_attribute(EntityId(1), 0, text(G2));
        m.push(entity("XNEW", vec![text(G1), r(1)]));
        m.remove(EntityId(2));
    });
    assert_eq!(model.revision(), revision);
    assert_maintained(&model, "after preview");
}

#[test]
fn indexes_built_inside_a_preview_stay_coherent() {
    let mut model = base();
    model.preview(|m| {
        m.push(entity("XNEW", vec![text(G2), r(1)]));
        assert_maintained(m, "inside");
        m.remove(EntityId(1));
        assert_maintained(m, "inside, after remove");
    });
    assert_maintained(&model, "after preview");
    assert_eq!(model.global_id_candidates(&guid(G1)), &[EntityId(1)]);
}
