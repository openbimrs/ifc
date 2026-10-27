//! `History`: a bounded undo and redo stack over recorded inverses (#465).

use ifc_model::{Entity, EntityId, History, HistoryError, Model, Value};

fn wall(n: i64) -> Entity {
    Entity::new("IFCWALL", vec![Value::Integer(n)])
}

fn ids(model: &Model) -> Vec<EntityId> {
    model.ids().collect()
}

#[test]
fn undo_redo_and_a_new_change_clearing_redo() {
    let mut model = Model::new();
    let mut history = History::new(8);
    let a = history.track(&mut model, |m| m.push(wall(1)));
    let b = history.track(&mut model, |m| m.push(wall(2)));
    assert_eq!((history.undo_len(), history.redo_len()), (2, 0));

    let restored = history.undo(&mut model).unwrap();
    assert_eq!(restored.removed, vec![b]);
    assert_eq!(ids(&model), vec![a]);
    assert!(history.can_redo());

    history.redo(&mut model).unwrap();
    assert_eq!(ids(&model), vec![a, b]);

    history.undo(&mut model).unwrap();
    history.track(&mut model, |m| m.push(wall(3)));
    assert!(!history.can_redo());
    assert_eq!(history.redo(&mut model), Err(HistoryError::NothingToRedo));
}

#[test]
fn nothing_to_undo_or_redo() {
    let mut model = Model::new();
    let mut history = History::new(2);
    assert_eq!(history.undo(&mut model), Err(HistoryError::NothingToUndo));
    assert_eq!(history.redo(&mut model), Err(HistoryError::NothingToRedo));
    assert_eq!(history.peek_undo(&model), Err(HistoryError::NothingToUndo));
    assert_eq!(history.peek_redo(&model), Err(HistoryError::NothingToRedo));
}

#[test]
fn the_limit_drops_the_oldest_entry() {
    let mut model = Model::new();
    let mut history = History::new(2);
    for n in 0..3 {
        history.track(&mut model, |m| m.push(wall(n)));
    }
    assert_eq!(history.undo_len(), 2);
    history.undo(&mut model).unwrap();
    history.undo(&mut model).unwrap();
    assert_eq!(history.undo(&mut model), Err(HistoryError::NothingToUndo));
    // The oldest push can no longer be undone.
    assert_eq!(model.len(), 1);
}

#[test]
fn limit_zero_records_nothing() {
    let mut model = Model::new();
    let mut history = History::default();
    history.track(&mut model, |m| m.push(wall(1)));
    assert!(!model.is_recording());
    assert_eq!(history.undo_len(), 0);
    assert_eq!(model.len(), 1);
}

#[test]
fn empty_changes_are_not_pushed() {
    let mut model = Model::new();
    let mut history = History::new(4);
    history.track(&mut model, |_| ());
    assert_eq!(history.undo_len(), 0);
}

#[test]
fn an_untracked_write_makes_undo_stale_until_the_next_track() {
    let mut model = Model::new();
    let mut history = History::new(4);
    history.track(&mut model, |m| m.push(wall(1)));
    model.push(wall(2));

    let err = history.undo(&mut model).unwrap_err();
    assert!(matches!(err, HistoryError::StaleRevision { .. }));
    assert!(matches!(
        history.peek_undo(&model),
        Err(HistoryError::StaleRevision { .. })
    ));
    assert_eq!(history.undo_len(), 1);
    assert_eq!(model.len(), 2);

    history.track(&mut model, |m| m.push(wall(3)));
    assert_eq!(history.undo_len(), 1);
    history.undo(&mut model).unwrap();
    assert_eq!(model.len(), 2);
}

#[test]
fn set_limit_trims_both_stacks() {
    let mut model = Model::new();
    let mut history = History::new(6);
    for n in 0..4 {
        history.track(&mut model, |m| m.push(wall(n)));
    }
    history.undo(&mut model).unwrap();
    history.undo(&mut model).unwrap();
    assert_eq!((history.undo_len(), history.redo_len()), (2, 2));
    history.set_limit(1);
    assert_eq!((history.undo_len(), history.redo_len()), (1, 1));
    history.set_limit(0);
    assert!(!history.can_undo() && !history.can_redo());
    assert_eq!(history.limit(), 0);
}

#[test]
fn peek_equals_the_result() {
    let mut model = Model::new();
    let mut history = History::new(4);
    history.track(&mut model, |m| {
        m.push(wall(1));
        m.push(wall(2));
    });
    let peeked = history.peek_undo(&model).unwrap();
    assert_eq!(history.undo(&mut model).unwrap(), peeked);
    let peeked = history.peek_redo(&model).unwrap();
    assert_eq!(history.redo(&mut model).unwrap(), peeked);
}

#[test]
fn clear_forgets_everything() {
    let mut model = Model::new();
    let mut history = History::new(4);
    history.track(&mut model, |m| m.push(wall(1)));
    history.clear();
    assert_eq!(history.undo(&mut model), Err(HistoryError::NothingToUndo));
}

#[test]
fn replaying_after_undo_reuses_the_ids() {
    let mut model = Model::new();
    let mut history = History::new(4);
    let first = history.track(&mut model, |m| m.push(wall(1)));
    history.undo(&mut model).unwrap();
    let again = history.track(&mut model, |m| m.push(wall(1)));
    assert_eq!(first, again);
}
