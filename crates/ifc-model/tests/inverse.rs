//! `Inverse` restores the model exactly: file order, type-index order,
//! attribute counts, the header and the next free id (#465).

use std::ops::Range;
use std::sync::Arc;

use ifc_model::{
    Edit, Entity, EntityId, EntitySource, Header, Inverse, InverseError, Model, Restored,
    Transaction, Value,
};

/// Everything an undo must restore; the revision is deliberately absent.
#[derive(Debug, PartialEq)]
struct State {
    header: Header,
    entities: Vec<(EntityId, Entity)>,
    buckets: Vec<(String, Vec<EntityId>)>,
    next_id: EntityId,
}

fn state(model: &Model) -> State {
    let mut buckets: Vec<_> = model
        .type_histogram()
        .into_iter()
        .map(|(name, _)| (name.to_owned(), model.ids_of_type(name).to_vec()))
        .collect();
    buckets.sort();
    State {
        header: model.header().clone(),
        entities: model.iter().map(|(id, e)| (id, e.clone())).collect(),
        buckets,
        next_id: model.next_id(),
    }
}

fn wall(n: i64) -> Entity {
    Entity::new("IFCWALL", vec![Value::Integer(n)])
}

fn seeded() -> (Model, [EntityId; 4]) {
    let mut model = Model::new();
    let a = model.push(wall(1));
    let b = model.push(Entity::new("IFCSLAB", vec![Value::Integer(2)]));
    let c = model.push(wall(3));
    let d = model.push(wall(4));
    (model, [a, b, c, d])
}

#[test]
fn undoing_a_create_empties_the_model_and_frees_the_id() {
    let mut model = Model::new();
    let before = state(&model);
    let (id, inverse) = Inverse::record(&mut model, |m| m.push(wall(1)));
    assert_eq!(id, EntityId(1));
    assert_eq!(inverse.len(), 1);
    let (restored, redo) = inverse.apply(&mut model).unwrap();
    assert_eq!(state(&model), before);
    assert_eq!(model.next_id(), EntityId(1));
    assert_eq!(restored.removed, vec![id]);
    assert!(restored.created.is_empty() && restored.modified.is_empty());
    let (restored, _) = redo.apply(&mut model).unwrap();
    assert!(model.contains(id));
    assert_eq!(restored.created, vec![id]);
}

#[test]
fn undoing_a_middle_removal_restores_both_orders() {
    let (mut model, [a, _b, c, d]) = seeded();
    let before = state(&model);
    let (removed, inverse) = Inverse::record(&mut model, |m| m.remove(c));
    assert!(removed.is_some());
    let (restored, redo) = inverse.apply(&mut model).unwrap();
    assert_eq!(state(&model), before);
    assert_eq!(model.ids_of_type("IFCWALL"), &[a, c, d]);
    assert_eq!(restored.created, vec![c]);
    let (_, _) = redo.apply(&mut model).unwrap();
    assert!(!model.contains(c));
}

#[test]
fn set_attribute_past_the_end_restores_the_attribute_count() {
    let (mut model, [a, ..]) = seeded();
    let before = state(&model);
    let (_, inverse) = Inverse::record(&mut model, |m| m.set_attribute(a, 5, Value::Integer(9)));
    assert_eq!(model.get(a).unwrap().attributes.len(), 6);
    let (restored, redo) = inverse.apply(&mut model).unwrap();
    assert_eq!(state(&model), before);
    assert_eq!(restored.modified, vec![a]);
    redo.apply(&mut model).unwrap();
    assert_eq!(model.get(a).unwrap().attributes.len(), 6);
    assert_eq!(model.get(a).unwrap().attributes[5], Value::Integer(9));
}

#[test]
fn set_attributes_with_a_repeated_slot_restores_the_first_value() {
    let (mut model, [a, ..]) = seeded();
    let before = state(&model);
    let (_, inverse) = Inverse::record(&mut model, |m| {
        m.set_attributes(
            a,
            [
                (0, Value::Integer(7)),
                (3, Value::Integer(8)),
                (0, Value::Integer(9)),
            ],
        )
    });
    inverse.apply(&mut model).unwrap();
    assert_eq!(state(&model), before);
}

#[test]
fn retype_across_buckets_restores_bucket_order() {
    let (mut model, [a, _b, c, d]) = seeded();
    let before = state(&model);
    let (_, inverse) = Inverse::record(&mut model, |m| m.retype(c, "IFCTESTONLY"));
    assert_eq!(model.ids_of_type("IFCWALL"), &[a, d]);
    let (restored, redo) = inverse.apply(&mut model).unwrap();
    assert_eq!(state(&model), before);
    assert_eq!(restored.modified, vec![c]);
    redo.apply(&mut model).unwrap();
    assert_eq!(model.ids_of_type("IFCWALL"), &[a, d]);
}

#[test]
fn replacing_an_occupant_keeps_its_file_position() {
    let (mut model, [a, _b, c, _d]) = seeded();
    let before = state(&model);
    let (_, inverse) = Inverse::record(&mut model, |m| {
        m.insert(c, Entity::new("IFCSLAB", vec![]));
        m.insert(a, wall(100));
    });
    let (_, redo) = inverse.apply(&mut model).unwrap();
    assert_eq!(state(&model), before);
    redo.apply(&mut model).unwrap();
    assert_eq!(model.get(a).unwrap().attributes[0], Value::Integer(100));
}

#[test]
fn staging_a_create_twice_on_one_id_round_trips() {
    let (mut model, [_, _, _, d]) = seeded();
    let before = state(&model);
    let id = EntityId(d.0 + 1);
    let mut tx = Transaction::new(&model);
    tx.stage(Edit::Create {
        id,
        entity: wall(1),
    });
    tx.stage(Edit::Create {
        id,
        entity: Entity::new("IFCSLAB", vec![]),
    });
    // Preflight refuses nothing here: the id is free in the model.
    let (_, inverse) = tx.commit_reversible(&mut model).unwrap();
    assert_eq!(model.get(id).unwrap().type_name.as_ref(), "IFCSLAB");
    inverse.apply(&mut model).unwrap();
    assert_eq!(state(&model), before);
}

#[test]
fn header_through_a_transaction_and_header_mut() {
    let (mut model, _) = seeded();
    let before = state(&model);
    let header = Header {
        name: "next.ifc".into(),
        ..Header::default()
    };
    let mut tx = Transaction::new(&model);
    tx.set_header(header.clone());
    assert!(!tx.is_empty());
    assert_eq!(tx.header(), Some(&header));
    let (_, inverse) = tx.commit_reversible(&mut model).unwrap();
    assert_eq!(model.header(), &header);
    let (restored, redo) = inverse.apply(&mut model).unwrap();
    assert!(restored.header);
    assert_eq!(state(&model), before);
    redo.apply(&mut model).unwrap();
    assert_eq!(model.header(), &header);

    let before = state(&model);
    let (_, inverse) = Inverse::record(&mut model, |m| m.header_mut().schema = vec!["IFC4".into()]);
    inverse.apply(&mut model).unwrap();
    assert_eq!(state(&model), before);
}

#[test]
fn header_mut_bumps_the_revision() {
    let mut model = Model::new();
    let revision = model.revision();
    model.header_mut().name = "x".into();
    assert!(model.revision() > revision);
}

#[test]
fn a_stale_inverse_is_refused_and_changes_nothing() {
    let (mut model, [a, ..]) = seeded();
    let (_, inverse) = Inverse::record(&mut model, |m| m.set_attribute(a, 0, Value::Integer(5)));
    model.push(wall(99));
    let before = state(&model);
    let revision = model.revision();
    let err = inverse.clone().apply(&mut model).unwrap_err();
    assert_eq!(
        err,
        InverseError::StaleRevision {
            expected: inverse.revision(),
            found: revision
        }
    );
    assert_eq!(state(&model), before);
    assert_eq!(model.revision(), revision);
}

#[test]
fn undo_bumps_the_revision_instead_of_restoring_it() {
    let (mut model, [a, ..]) = seeded();
    let (_, inverse) = Inverse::record(&mut model, |m| m.set_attribute(a, 0, Value::Integer(5)));
    let after = model.revision();
    inverse.apply(&mut model).unwrap();
    assert!(model.revision() > after);
}

#[test]
fn a_refused_commit_leaves_the_model_unchanged() {
    let (mut model, [a, ..]) = seeded();
    let before = state(&model);
    let revision = model.revision();
    let mut tx = Transaction::new(&model);
    tx.remove(a);
    // The reference to `a` was never written, so removal alone is fine; make
    // it conflict with a missing target instead.
    tx.set_attribute(EntityId(999), 0, Value::Null);
    let conflicts = tx.commit_reversible(&mut model).unwrap_err();
    assert!(!conflicts.is_empty());
    assert_eq!(state(&model), before);
    assert_eq!(model.revision(), revision);
    assert!(!model.is_recording());
}

#[test]
fn nested_recordings_give_the_outer_inverse_everything() {
    let (mut model, [a, _, c, _]) = seeded();
    let before = state(&model);
    let ((inner_len, ()), outer) = Inverse::record(&mut model, |m| {
        m.set_attribute(a, 0, Value::Integer(5));
        let ((), inner) = Inverse::record(m, |m| {
            m.remove(c);
        });
        (inner.len(), m.header_mut().name = "n".into())
    });
    assert_eq!(inner_len, 1);
    assert_eq!(outer.len(), 3);
    outer.apply(&mut model).unwrap();
    assert_eq!(state(&model), before);
}

#[test]
fn applying_while_recording_is_itself_recorded() {
    let (mut model, [a, ..]) = seeded();
    let (_, inverse) = Inverse::record(&mut model, |m| m.set_attribute(a, 0, Value::Integer(5)));
    let after = state(&model);
    let before = state(&{
        let mut copy = model.clone();
        inverse.clone().apply(&mut copy).unwrap();
        copy
    });
    let (_, outer) = Inverse::record(&mut model, |m| inverse.apply(m).unwrap());
    assert_eq!(state(&model), before);
    outer.apply(&mut model).unwrap();
    assert_eq!(state(&model), after);
}

#[test]
fn restoring_removals_and_creates_reports_the_net_effect() {
    let (mut model, [a, b, c, _]) = seeded();
    let (created, inverse) = Inverse::record(&mut model, |m| {
        m.remove(b);
        m.set_attribute(a, 0, Value::Integer(5));
        m.remove(c);
        m.push(wall(10))
    });
    let (restored, _) = inverse.apply(&mut model).unwrap();
    // `Restored` is non-exhaustive, so it is read field by field.
    let mut sorted = restored.clone();
    sorted.created.sort();
    assert_eq!(sorted.created, vec![b, c]);
    assert_eq!(sorted.removed, vec![created]);
    assert_eq!(sorted.modified, vec![a]);
    assert!(!sorted.header);
}

#[test]
fn an_empty_recording_applies_without_a_bump() {
    let (mut model, _) = seeded();
    let (_, inverse) = Inverse::record(&mut model, |_| ());
    assert!(inverse.is_empty());
    let revision = model.revision();
    let (restored, _) = inverse.apply(&mut model).unwrap();
    assert_eq!(model.revision(), revision);
    assert_eq!(restored, Restored::default());
}

#[test]
fn a_panicking_change_closes_the_recording() {
    let (mut model, _) = seeded();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        Inverse::record(&mut model, |_| panic!("boom"))
    }));
    assert!(result.is_err());
    assert!(!model.is_recording());
}

#[test]
fn end_recording_without_begin_is_none() {
    assert!(Model::new().end_recording().is_none());
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
fn a_lazy_model_round_trips() {
    let table = Table(vec![wall(1), Entity::new("IFCSLAB", vec![]), wall(3)]);
    let mut model = Model::with_source(Arc::new(table));
    for (i, name) in ["IFCWALL", "IFCSLAB", "IFCWALL"].into_iter().enumerate() {
        model.insert_lazy(EntityId(i as u64 + 1), name, i..i + 1);
    }
    assert_eq!(model.decoded_len(), 0);
    let before = state(&model);
    let (_, inverse) = Inverse::record(&mut model, |m| {
        m.remove(EntityId(1));
        m.retype(EntityId(3), "IFCTESTONLY");
        m.set_attribute(EntityId(2), 1, Value::Integer(4));
    });
    let (_, redo) = inverse.apply(&mut model).unwrap();
    assert_eq!(state(&model), before);
    redo.apply(&mut model).unwrap();
    assert!(!model.contains(EntityId(1)));
}

#[test]
fn an_inverse_is_refused_by_another_model() {
    let (mut a, [a0, ..]) = seeded();
    let (mut b, _) = seeded();
    let (_, inverse) = Inverse::record(&mut a, |m| m.remove(a0));
    let before = state(&b);
    // Both models took the same number of writes; per-model counters would
    // have matched here.
    let err = inverse.apply(&mut b).unwrap_err();
    assert!(matches!(err, InverseError::StaleRevision { .. }));
    assert_eq!(state(&b), before);
}

#[test]
fn a_clone_accepts_an_inverse_until_it_diverges() {
    let (mut a, [a0, ..]) = seeded();
    let (_, inverse) = Inverse::record(&mut a, |m| m.set_attribute(a0, 0, Value::Integer(9)));
    let mut same = a.clone();
    let mut diverged = a.clone();
    diverged.push(wall(50));
    assert!(inverse.clone().apply(&mut diverged).is_err());
    assert!(inverse.clone().apply(&mut same).is_ok());
    // The original diverges the other way.
    a.push(wall(60));
    assert!(inverse.apply(&mut a).is_err());
}

#[test]
fn an_empty_set_attributes_changes_nothing() {
    let (mut model, [a, ..]) = seeded();
    let revision = model.revision();
    assert_eq!(model.set_attributes(a, []), Some(vec![]));
    assert_eq!(model.revision(), revision);
    assert_eq!(model.set_attributes(EntityId(99), []), None);
}

#[test]
fn a_clone_taken_while_recording_is_not_recording() {
    let (mut model, _) = seeded();
    let (clone, inverse) = Inverse::record(&mut model, |m| {
        m.push(wall(5));
        m.clone()
    });
    assert!(!clone.is_recording());
    assert_eq!(inverse.len(), 1);
    assert!(model.end_recording().is_none());
}

#[test]
fn an_unbalanced_begin_inside_record_still_returns_its_own_inverse() {
    let (mut model, [a, ..]) = seeded();
    let before = state(&model);
    let (_, inverse) = Inverse::record(&mut model, |m| {
        m.set_attribute(a, 0, Value::Integer(5));
        m.begin_recording();
        m.push(wall(6));
    });
    assert!(!model.is_recording());
    assert_eq!(inverse.len(), 2);
    inverse.apply(&mut model).unwrap();
    assert_eq!(state(&model), before);
}
