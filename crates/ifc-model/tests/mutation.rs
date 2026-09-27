//! Transactional authoring: staged edits, preflight, atomic commit.

use ifc_model::{Conflict, Edit, Entity, EntityId, Model, Transaction, Value};

/// A model with a storey and a wall that references it.
fn model() -> (Model, EntityId, EntityId) {
    let mut model = Model::new();
    let storey = model.push(Entity::new(
        "IFCBUILDINGSTOREY",
        vec![Value::Text("L1".into())],
    ));
    let wall = model.push(Entity::new(
        "IFCWALL",
        vec![Value::Text("W1".into()), Value::Ref(storey)],
    ));
    (model, storey, wall)
}

/// A clean batch applies and reports what it did.
#[test]
fn a_valid_transaction_commits() {
    let (mut model, storey, _wall) = model();
    let before = model.len();

    let mut tx = Transaction::new(&model);
    let slab = tx.create(Entity::new("IFCSLAB", vec![Value::Ref(storey)]));
    tx.set_attribute(storey, 0, Value::Text("Level 1".into()));

    let applied = tx.commit(&mut model).expect("no conflicts");
    assert_eq!(applied.created, vec![slab], "the new id is reported");
    assert_eq!(model.len(), before + 1);
    assert_eq!(
        model.get(storey).unwrap().attribute(0).unwrap().as_text(),
        Some("Level 1"),
        "the attribute write landed"
    );
}

/// An entity created in a batch can be referenced by the same batch.
///
/// This is the reason preflight projects the end state instead of checking
/// each edit against the current model: the reference is valid at commit,
/// even though the target does not exist when the edit is staged.
#[test]
fn a_created_entity_is_referenceable_within_the_same_transaction() {
    let (mut model, _storey, _wall) = model();

    let mut tx = Transaction::new(&model);
    let point = tx.create(Entity::new("IFCCARTESIANPOINT", vec![]));
    let placement = tx.create(Entity::new("IFCAXIS2PLACEMENT3D", vec![Value::Ref(point)]));

    let applied = tx.commit(&mut model).expect("forward reference resolves");
    assert_eq!(applied.created, vec![point, placement]);
    assert_eq!(
        model
            .get(placement)
            .unwrap()
            .attribute(0)
            .unwrap()
            .as_ref_id(),
        Some(point)
    );
}

/// Removing an entity that something still points at is refused.
#[test]
fn a_removal_that_would_dangle_is_refused() {
    let (mut model, storey, wall) = model();
    let before = model.len();

    let mut tx = Transaction::new(&model);
    tx.remove(storey);

    let conflicts = tx.commit(&mut model).expect_err("the wall still refers");
    assert_eq!(
        conflicts,
        vec![Conflict::RemovalWouldDangle {
            edit: 0,
            removed: storey,
            referrer: wall,
            slot: 1,
        }],
        "the surviving referrer and its slot are both named"
    );
    assert_eq!(model.len(), before, "a refused commit changes nothing");
}

/// Removing the referrer in the same batch makes the removal legal.
#[test]
fn removing_referrer_and_target_together_is_allowed() {
    let (mut model, storey, wall) = model();

    let mut tx = Transaction::new(&model);
    tx.remove(storey);
    tx.remove(wall);

    let applied = tx.commit(&mut model).expect("nothing survives to dangle");
    assert_eq!(applied.removed.len(), 2);
    assert!(model.is_empty());
}

/// Re-pointing the referrer in the same batch also makes it legal.
#[test]
fn repointing_a_reference_permits_the_removal() {
    let (mut model, storey, wall) = model();
    let other = model.push(Entity::new(
        "IFCBUILDINGSTOREY",
        vec![Value::Text("L2".into())],
    ));

    let mut tx = Transaction::new(&model);
    tx.set_attribute(wall, 1, Value::Ref(other));
    tx.remove(storey);

    tx.commit(&mut model)
        .expect("the wall now points elsewhere");
    assert_eq!(
        model.get(wall).unwrap().attribute(1).unwrap().as_ref_id(),
        Some(other)
    );
    assert!(model.get(storey).is_none());
}

/// Writing a reference to an entity that will not exist is refused.
#[test]
fn a_dangling_reference_is_refused() {
    let (mut model, _storey, wall) = model();

    let mut tx = Transaction::new(&model);
    tx.set_attribute(wall, 1, Value::Ref(EntityId(9999)));

    let conflicts = tx.commit(&mut model).expect_err("target does not exist");
    assert_eq!(
        conflicts,
        vec![Conflict::DanglingReference {
            edit: 0,
            from: wall,
            slot: 1,
            target: EntityId(9999),
        }]
    );
}

/// References nested inside lists are checked too.
///
/// An IFC relationship holds its members in an aggregate, so a check that
/// only looked at top-level `Value::Ref` would miss almost every real edit.
#[test]
fn references_nested_in_a_list_are_checked() {
    let (mut model, storey, _wall) = model();

    let mut tx = Transaction::new(&model);
    tx.create(Entity::new(
        "IFCRELAGGREGATES",
        vec![Value::List(vec![
            Value::Ref(storey),
            Value::Ref(EntityId(4242)),
        ])],
    ));

    let conflicts = tx.commit(&mut model).expect_err("one member is missing");
    assert_eq!(conflicts.len(), 1, "only the bad member is reported");
    assert!(matches!(
        conflicts[0],
        Conflict::DanglingReference {
            target: EntityId(4242),
            ..
        }
    ));
}

/// Editing an entity that does not exist is refused.
#[test]
fn editing_a_missing_entity_is_refused() {
    let (mut model, _storey, _wall) = model();

    let mut tx = Transaction::new(&model);
    tx.set_attribute(EntityId(777), 0, Value::Integer(1));

    let conflicts = tx.commit(&mut model).expect_err("no such entity");
    assert_eq!(
        conflicts,
        vec![Conflict::MissingTarget {
            edit: 0,
            id: EntityId(777)
        }]
    );
}

/// A transaction opened against an older model refuses to commit.
#[test]
fn a_stale_transaction_is_refused() {
    let (mut model, storey, _wall) = model();

    let mut tx = Transaction::new(&model);
    tx.set_attribute(storey, 0, Value::Text("Renamed".into()));

    // Someone else edits in between.
    model.push(Entity::new("IFCSLAB", vec![]));

    let conflicts = tx.commit(&mut model).expect_err("the model moved");
    assert!(
        matches!(conflicts[0], Conflict::StaleRevision { .. }),
        "got {:?}",
        conflicts[0]
    );
    assert_eq!(
        model.get(storey).unwrap().attribute(0).unwrap().as_text(),
        Some("L1"),
        "the stale edit did not land"
    );
}

/// Every direct edit method moves the revision.
///
/// If one did not, a transaction opened before it would commit against a
/// model that had silently changed underneath.
#[test]
fn every_mutation_moves_the_revision() {
    let (mut model, storey, wall) = model();

    let start = model.revision();
    model.set_attribute(storey, 0, Value::Text("a".into()));
    let after_set = model.revision();
    assert!(after_set > start, "set_attribute must bump");

    model.retype(storey, "IFCSPACE");
    let after_retype = model.revision();
    assert!(after_retype > after_set, "retype must bump");

    model.set_attributes(wall, [(0, Value::Text("b".into()))]);
    let after_batch = model.revision();
    assert!(after_batch > after_retype, "set_attributes must bump");

    model.remove(wall);
    assert!(model.revision() > after_batch, "remove must bump");
}

/// A rejected batch reports every conflict, not just the first.
#[test]
fn preflight_reports_all_conflicts() {
    let (model, _storey, wall) = model();

    let mut tx = Transaction::new(&model);
    tx.set_attribute(EntityId(500), 0, Value::Integer(1));
    tx.set_attribute(wall, 1, Value::Ref(EntityId(501)));
    tx.retype(EntityId(502), "IFCWALL");

    let conflicts = tx.preflight(&model);
    assert_eq!(conflicts.len(), 3, "got {conflicts:?}");
    assert!(matches!(
        conflicts[0],
        Conflict::MissingTarget { edit: 0, .. }
    ));
    assert!(matches!(
        conflicts[1],
        Conflict::DanglingReference { edit: 1, .. }
    ));
    assert!(matches!(
        conflicts[2],
        Conflict::MissingTarget { edit: 2, .. }
    ));
}

/// Preflight does not mutate, so it can be called before committing.
#[test]
fn preflight_leaves_the_model_untouched() {
    let (model, storey, _wall) = model();

    let mut tx = Transaction::new(&model);
    tx.set_attribute(storey, 0, Value::Text("changed".into()));
    let revision = model.revision();

    assert!(tx.preflight(&model).is_empty());
    assert_eq!(model.revision(), revision, "preflight is read-only");
    assert_eq!(
        model.get(storey).unwrap().attribute(0).unwrap().as_text(),
        Some("L1")
    );
}

/// Staging the same removal twice is idempotent, not an error.
///
/// The end state is identical either way, so refusing would punish a caller
/// that built the batch from two overlapping rules.
#[test]
fn removing_the_same_entity_twice_is_idempotent() {
    let (mut model, storey, wall) = model();

    let mut tx = Transaction::new(&model);
    tx.remove(wall);
    tx.remove(wall);
    tx.remove(storey);

    let applied = tx.commit(&mut model).expect("got {applied:?}");
    assert_eq!(
        applied.removed.len(),
        2,
        "the second removal is a no-op, not a second entry"
    );
    assert!(model.is_empty());
}

/// A duplicated bad removal is reported once, against its first occurrence.
#[test]
fn a_repeated_conflict_is_reported_once() {
    let (model, storey, wall) = model();

    let mut tx = Transaction::new(&model);
    tx.remove(storey);
    tx.remove(storey);

    let conflicts = tx.preflight(&model);
    assert_eq!(
        conflicts,
        vec![Conflict::RemovalWouldDangle {
            edit: 0,
            removed: storey,
            referrer: wall,
            slot: 1,
        }],
        "one problem, one conflict"
    );
}
/// Removed entities come back with their contents.
#[test]
fn a_removal_returns_the_entity() {
    let (mut model, storey, wall) = model();

    let mut tx = Transaction::new(&model);
    tx.remove(wall);
    tx.remove(storey);

    let applied = tx.commit(&mut model).expect("both go");
    let names: Vec<_> = applied
        .removed
        .iter()
        .map(|(_, e)| e.type_name.to_string())
        .collect();
    assert_eq!(names, vec!["IFCWALL", "IFCBUILDINGSTOREY"]);
}

/// Referencing an entity the same batch removes is refused.
///
/// This is the mirror of the forward-reference case: projection must subtract
/// removals as well as add creates, or a batch can point at a hole it dug.
#[test]
fn referencing_a_removed_entity_is_refused() {
    let (mut model, storey, wall) = model();
    let other = model.push(Entity::new(
        "IFCBUILDINGSTOREY",
        vec![Value::Text("L2".into())],
    ));

    let mut tx = Transaction::new(&model);
    // Re-point the wall at a storey that this same batch deletes.
    tx.set_attribute(wall, 1, Value::Ref(other));
    tx.remove(other);

    let conflicts = tx.commit(&mut model).expect_err("the target is going away");
    assert!(
        conflicts.iter().any(|c| matches!(
            c,
            Conflict::DanglingReference {
                target,
                ..
            } if *target == other
        )),
        "got {conflicts:?}"
    );
    assert!(model.get(other).is_some(), "nothing was applied");
    assert_eq!(
        model.get(wall).unwrap().attribute(1).unwrap().as_ref_id(),
        Some(storey),
        "the wall still points at its original storey"
    );
}

/// A create staged onto an id that already exists is refused.
///
/// `Transaction::create` allocates fresh ids, so this cannot happen by
/// accident -- but an `Edit::Create` can also arrive from a deserialized or
/// replayed batch, and silently overwriting an entity is exactly the
/// behaviour a transaction exists to prevent.
#[test]
fn a_create_over_an_existing_id_is_refused() {
    let (mut model, storey, _wall) = model();
    let before = model.get(storey).unwrap().clone();

    let mut tx = Transaction::new(&model);
    // Replay a create at an id the model already holds.
    tx.stage(Edit::Create {
        id: storey,
        entity: Entity::new("IFCSLAB", vec![]),
    });

    let conflicts = tx.commit(&mut model).expect_err("that id is taken");
    assert_eq!(
        conflicts,
        vec![Conflict::IdAlreadyExists {
            edit: 0,
            id: storey
        }]
    );
    assert_eq!(
        model.get(storey).unwrap(),
        &before,
        "the existing entity is untouched"
    );
}
