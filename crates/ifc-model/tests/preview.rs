//! `Model::preview` puts the model back exactly, revision included (#595).

use std::ops::Range;
use std::sync::Arc;

use ifc_model::{
    Diagnostic, Edit, Entity, EntityId, EntitySource, Header, History, Inverse, InverseError,
    Model, Transaction, Value,
};

/// Everything a preview must restore, diagnostics included.
#[derive(Debug, PartialEq)]
struct State {
    header: Header,
    entities: Vec<(EntityId, Entity)>,
    buckets: Vec<(String, Vec<EntityId>)>,
    next_id: EntityId,
    diagnostics: Vec<Diagnostic>,
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
        diagnostics: model.diagnostics().to_vec(),
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
fn redo_still_replays_after_a_preview() {
    let mut model = Model::new();
    let mut history = History::new(8);
    let w1 = history.track(&mut model, |m| m.push(wall(1)));
    let w2 = history.track(&mut model, |m| m.push(wall(2)));
    history.undo(&mut model).unwrap();
    let before = state(&model);
    let revision = model.revision();

    model.preview(|m| {
        let p = m.push(wall(3));
        m.set_attribute(p, 2, Value::Null);
        m.remove(w1);
        m.header_mut().name = "previewed".into();
    });
    assert_eq!(state(&model), before);
    assert_eq!(model.revision(), revision);

    let restored = history.redo(&mut model).unwrap();
    assert_eq!(restored.created, vec![w2]);
    assert!(model.contains(w1) && model.contains(w2));
}

#[test]
fn a_tracked_change_after_a_preview_keeps_the_undo_stack() {
    let mut model = Model::new();
    let mut history = History::new(8);
    history.track(&mut model, |m| m.push(wall(1)));
    model.preview(|m| m.push(wall(2)));
    history.track(&mut model, |m| m.push(wall(3)));
    assert_eq!(history.undo_len(), 2);
    history.undo(&mut model).unwrap();
    history.undo(&mut model).unwrap();
    assert!(model.is_empty());
}

#[test]
fn a_preview_restores_every_primitive() {
    let (mut model, [a, b, c, d]) = seeded();
    let before = state(&model);
    let revision = model.revision();
    model.preview(|m| {
        m.insert(EntityId(40), wall(40));
        m.insert(b, wall(41));
        m.push(wall(42));
        m.set_attribute(a, 5, Value::Integer(9));
        m.set_attributes(
            d,
            [
                (0, Value::Integer(7)),
                (3, Value::Integer(8)),
                (0, Value::Integer(9)),
            ],
        );
        m.retype(c, "IFCTESTONLY");
        m.remove(a);
        m.header_mut().name = "previewed".into();
        let mut tx = Transaction::new(m);
        tx.create(wall(50));
        tx.set_attribute(d, 0, Value::Null);
        tx.commit_reversible(m).unwrap();
    });
    assert_eq!(state(&model), before);
    assert_eq!(model.revision(), revision);
    assert_eq!(model.ids_of_type("IFCWALL"), &[a, c, d]);
}

#[test]
fn the_closure_sees_its_writes_and_ids_are_free_again() {
    let (mut model, _) = seeded();
    let next = model.next_id();
    let (len, id) = model.preview(|m| {
        let id = m.push(wall(9));
        (m.len(), id)
    });
    assert_eq!(len, 5);
    assert_eq!(id, next);
    assert!(!model.contains(id));
    assert_eq!(model.push(wall(10)), id);
}

#[test]
fn a_panicking_preview_rolls_back_and_resumes() {
    let mut model = Model::new();
    let mut history = History::new(8);
    history.track(&mut model, |m| m.push(wall(1)));
    history.undo(&mut model).unwrap();
    let before = state(&model);
    let revision = model.revision();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        model.preview(|m| {
            m.push(wall(2));
            panic!("boom")
        })
    }));
    let payload = result.unwrap_err();
    assert_eq!(payload.downcast_ref::<&str>(), Some(&"boom"));
    assert!(!model.is_recording());
    assert_eq!(state(&model), before);
    assert_eq!(model.revision(), revision);
    assert!(history.redo(&mut model).is_ok());
}

#[test]
fn a_preview_inside_a_recording_adds_nothing_to_it() {
    let (mut model, [a, ..]) = seeded();
    let before = state(&model);
    let ((), inverse) = Inverse::record(&mut model, |m| {
        m.push(wall(10));
        m.preview(|m| {
            m.push(wall(11));
            m.remove(a);
        });
        m.push(wall(12));
    });
    assert_eq!(inverse.len(), 2);
    inverse.apply(&mut model).unwrap();
    assert_eq!(state(&model), before);
}

#[test]
fn nested_previews_restore_each_level() {
    let (mut model, _) = seeded();
    let before = state(&model);
    let revision = model.revision();
    model.preview(|m| {
        m.push(wall(10));
        let middle = state(m);
        let middle_revision = m.revision();
        m.preview(|m| {
            m.push(wall(11));
            m.header_mut().name = "inner".into();
        });
        assert_eq!(state(m), middle);
        assert_eq!(m.revision(), middle_revision);
    });
    assert_eq!(state(&model), before);
    assert_eq!(model.revision(), revision);
}

#[test]
fn a_transaction_opened_before_a_preview_still_commits() {
    let (mut model, [a, ..]) = seeded();
    let mut tx = Transaction::new(&model);
    tx.set_attribute(a, 0, Value::Integer(99));
    model.preview(|m| m.push(wall(10)));
    tx.commit(&mut model).unwrap();
    assert_eq!(model.get(a).unwrap().attributes[0], Value::Integer(99));
}

#[test]
fn an_inverse_recorded_inside_a_preview_is_stale_after_it() {
    let (mut model, _) = seeded();
    let inverse = model.preview(|m| Inverse::record(m, |m| m.push(wall(10))).1);
    assert!(matches!(
        inverse.apply(&mut model),
        Err(InverseError::StaleRevision { .. })
    ));
}

#[test]
fn revisions_drawn_inside_are_never_handed_out_again() {
    let (mut model, _) = seeded();
    let revision = model.revision();
    let drawn = model.preview(|m| {
        m.push(wall(10));
        m.revision()
    });
    assert_ne!(drawn, revision);
    assert_eq!(model.revision(), revision);
    model.push(wall(11));
    assert!(model.revision() > drawn);
}

#[test]
fn diagnostics_pushed_inside_are_dropped() {
    let (mut model, _) = seeded();
    model.push_diagnostic(Diagnostic::warning(0..1, "kept"));
    let before = state(&model);
    model.preview(|m| m.push_diagnostic(Diagnostic::warning(2..3, "dropped")));
    assert_eq!(state(&model), before);
    assert_eq!(model.diagnostics().len(), 1);
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
fn a_lazy_model_previews() {
    let table = Table(vec![wall(1), Entity::new("IFCSLAB", vec![]), wall(3)]);
    let mut model = Model::with_source(Arc::new(table));
    for (i, name) in ["IFCWALL", "IFCSLAB", "IFCWALL"].into_iter().enumerate() {
        model.insert_lazy(EntityId(i as u64 + 1), name, i..i + 1);
    }
    let before = state(&model);
    let revision = model.revision();
    model.preview(|m| {
        m.remove(EntityId(1));
        m.retype(EntityId(3), "IFCTESTONLY");
        m.set_attribute(EntityId(2), 1, Value::Integer(4));
    });
    assert_eq!(state(&model), before);
    assert_eq!(model.revision(), revision);
}

#[test]
fn staged_edits_roll_back_through_a_preview() {
    let (mut model, [a, ..]) = seeded();
    let before = state(&model);
    model.preview(|m| {
        let mut tx = Transaction::new(m);
        tx.stage(Edit::Remove { id: a });
        tx.commit(m).unwrap();
        assert!(!m.contains(a));
    });
    assert_eq!(state(&model), before);
}

#[test]
#[should_panic(expected = "ended a recording it did not begin")]
fn a_change_that_ends_the_preview_recording_panics() {
    let (mut model, _) = seeded();
    model.preview(|m| {
        m.end_recording();
    });
}
