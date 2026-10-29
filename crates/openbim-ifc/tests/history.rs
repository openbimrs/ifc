//! Undo and redo over authoring batches restore the exact STEP bytes (#465),
//! in IFC2X3, IFC4 and IFC4X3.
//!
//! Over `test/fixtures/synthetic-relations/cascade_*.ifc`; see
//! `cascade_delete.rs` for the ids.

#![cfg(all(
    feature = "step",
    feature = "author",
    feature = "ifc2x3",
    feature = "ifc4",
    feature = "ifc4x3"
))]

use ifc::{
    apply_authoring, AuthorOp, Codec, EntityId, History, HistoryError, Model, PurgeOptions,
    RemoveProductOptions, ReverseIndex, Schema, SchemaVersion, StepCodec, Value,
};

const FIXTURES: [(&str, SchemaVersion); 3] = [
    ("cascade_ifc2x3.ifc", SchemaVersion::Ifc2x3),
    ("cascade_ifc4.ifc", SchemaVersion::Ifc4),
    ("cascade_ifc4x3.ifc", SchemaVersion::Ifc4x3),
];

fn read(name: &str) -> Model {
    let path = format!(
        "{}/../../test/fixtures/synthetic-relations/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    StepCodec
        .read_path(std::path::Path::new(&path))
        .unwrap_or_else(|error| panic!("{name}: {error}"))
}

fn each_release(mut check: impl FnMut(&str, Model, &'static Schema)) {
    for (name, version) in FIXTURES {
        let schema = ifc::schema::for_version(version).expect("bundled");
        check(name, read(name), schema);
    }
}

fn bytes(model: &Model) -> Vec<u8> {
    StepCodec.write_bytes(model).expect("writes")
}

fn ids(model: &Model) -> Vec<EntityId> {
    model.ids().collect()
}

fn remove_product(product: u64) -> AuthorOp {
    AuthorOp::RemoveProduct {
        product: EntityId(product),
        options: RemoveProductOptions::new().rebase_placements(true),
    }
}

fn purge() -> AuthorOp {
    AuthorOp::Purge(PurgeOptions::new())
}

fn site(name: &str) -> AuthorOp {
    AuthorOp::Spatial {
        type_name: "IfcSite".to_owned(),
        parent: EntityId(12),
        attributes: vec![
            ("Name".to_owned(), Value::Text(name.into())),
            ("CompositionType".to_owned(), Value::Enum("ELEMENT".into())),
        ],
        placement: None,
        owner_history: Some(EntityId(5)),
    }
}

fn track(history: &mut History, model: &mut Model, schema: &Schema, ops: &[AuthorOp], seed: u128) {
    history
        .track(model, |m| apply_authoring(m, schema, ops, seed))
        .unwrap_or_else(|error| panic!("{error}"));
}

#[test]
fn removal_and_purge_undo_to_the_exact_bytes() {
    each_release(|name, mut model, schema| {
        let (b0, ids0) = (bytes(&model), ids(&model));
        let mut history = History::new(8);
        track(
            &mut history,
            &mut model,
            schema,
            &[remove_product(20), purge()],
            1,
        );
        let b1 = bytes(&model);
        assert_ne!(b1, b0, "{name}");

        history.undo(&mut model).unwrap();
        assert_eq!(bytes(&model), b0, "{name}: undo");
        assert_eq!(ids(&model), ids0, "{name}: ids");

        history.redo(&mut model).unwrap();
        assert_eq!(bytes(&model), b1, "{name}: redo");
    });
}

#[test]
fn a_seed_is_free_again_after_undoing_its_batch() {
    // #548: the seed names a batch; once the batch is undone its GlobalIds
    // are gone, so the same operations under the same seed are accepted and
    // derive the same ids and GlobalIds.
    each_release(|name, mut model, schema| {
        let b0 = bytes(&model);
        let mut history = History::new(8);
        track(&mut history, &mut model, schema, &[site("S")], 1);
        let b1 = bytes(&model);

        history.undo(&mut model).unwrap();
        assert_eq!(bytes(&model), b0, "{name}");

        track(&mut history, &mut model, schema, &[site("S")], 1);
        assert_eq!(bytes(&model), b1, "{name}: replay");
    });
}

#[test]
fn a_header_edit_undoes() {
    each_release(|name, mut model, _| {
        let b0 = bytes(&model);
        let mut history = History::new(8);
        history.track(&mut model, |m| m.header_mut().name = "renamed.ifc".into());
        assert_ne!(bytes(&model), b0, "{name}");
        history.undo(&mut model).unwrap();
        assert_eq!(bytes(&model), b0, "{name}");
    });
}

#[test]
fn three_changes_undo_and_redo_in_order() {
    each_release(|name, mut model, schema| {
        let mut history = History::new(8);
        let mut states = vec![bytes(&model)];
        for (n, ops) in [vec![remove_product(150)], vec![site("A")], vec![purge()]]
            .into_iter()
            .enumerate()
        {
            track(&mut history, &mut model, schema, &ops, 10 + n as u128);
            states.push(bytes(&model));
        }
        for expected in states.iter().rev().skip(1) {
            history.undo(&mut model).unwrap();
            assert_eq!(&bytes(&model), expected, "{name}: undo");
        }
        assert_eq!(history.undo(&mut model), Err(HistoryError::NothingToUndo));
        for expected in states.iter().skip(1) {
            history.redo(&mut model).unwrap();
            assert_eq!(&bytes(&model), expected, "{name}: redo");
        }
    });
}

#[test]
fn an_untracked_edit_makes_undo_stale() {
    each_release(|name, mut model, schema| {
        let mut history = History::new(8);
        track(&mut history, &mut model, schema, &[remove_product(150)], 1);
        model.set_attribute(EntityId(12), 2, Value::Text("Edited".into()));
        let before = bytes(&model);
        assert!(
            matches!(
                history.undo(&mut model),
                Err(HistoryError::StaleRevision { .. })
            ),
            "{name}"
        );
        assert_eq!(bytes(&model), before, "{name}: unchanged");
    });
}

#[test]
fn a_preview_leaves_undo_and_redo_valid() {
    // #595: looking at a change (here: what a site would create) must not
    // strand the redo stack or the revision a transaction targets.
    each_release(|name, mut model, schema| {
        let mut history = History::new(8);
        track(&mut history, &mut model, schema, &[remove_product(150)], 1);
        track(&mut history, &mut model, schema, &[site("A")], 2);
        let redone = bytes(&model);

        history.undo(&mut model).unwrap();
        let (before, revision) = (bytes(&model), model.revision());
        let created = model.preview(|m| {
            m.reverse_index();
            apply_authoring(m, schema, &[site("A")], 2)
                .expect("the preview authors")
                .created
        });
        assert!(!created.is_empty(), "{name}");
        assert_eq!(bytes(&model), before, "{name}: bytes");
        assert_eq!(model.revision(), revision, "{name}: revision");
        assert_eq!(
            model.reverse_index(),
            &ReverseIndex::build(&model),
            "{name}: reverse index"
        );

        history.redo(&mut model).unwrap();
        assert_eq!(bytes(&model), redone, "{name}: redo");
        for id in &created {
            assert!(model.contains(*id), "{name}: {id:?}");
        }
        history.undo(&mut model).unwrap();
        history.undo(&mut model).unwrap();
    });
}
