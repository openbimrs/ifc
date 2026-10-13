//! Unit tests for the `GlobalId` index, over models built in memory.

use ifc_model::{Entity, EntityId, Model, Value};
use ifc_schema::Schema;

use super::*;

const WALL: &str = "2O2Fr$t4X7Zf8NOew3FLOH";
const SHARED: &str = "1kTvXnbbzCWw8lcMd1dR4o";

fn schema() -> &'static Schema {
    crate::schema::for_version(crate::SchemaVersion::Ifc4)
        .or_else(|_| crate::schema::for_version(crate::SchemaVersion::Ifc4x3))
        .or_else(|_| crate::schema::for_version(crate::SchemaVersion::Ifc2x3))
        .expect("the facade's tests bundle a release")
}

fn root(type_name: &str, global_id: Value) -> Entity {
    Entity::new(type_name, vec![global_id, Value::Null, Value::Null])
}

fn text(global_id: &str) -> Value {
    Value::Text(global_id.into())
}

/// #1 wall; #2 and #4 share a GlobalId; #3 is a point whose first slot
/// would parse; #5 has none; #6 a malformed one; #7 shares #2's too; #8 a
/// typed wrapper around a valid one.
fn model() -> Model {
    let mut model = Model::new();
    model.push(root("IFCWALL", text(WALL)));
    model.push(root("IFCSLAB", text(SHARED)));
    model.push(Entity::new(
        "IFCCARTESIANPOINT",
        vec![text("0dGA15OQf9_P5UdNj5qcto")],
    ));
    model.push(root("IFCBEAM", text(SHARED)));
    model.push(root("IFCCOLUMN", Value::Null));
    model.push(root("IFCDOOR", text("not a GlobalId")));
    model.push(root("IFCPROPERTYSET", text(SHARED)));
    model.push(root(
        "IFCWINDOW",
        Value::Typed {
            type_name: "IFCGLOBALLYUNIQUEID".into(),
            value: Box::new(text("31t64Uzj98DhdKplMxBpF5")),
        },
    ));
    model
}

fn guid(text: &str) -> Guid {
    Guid::parse(text).expect("a valid GlobalId")
}

#[test]
fn a_unique_global_id_finds_its_entity() {
    let index = GlobalIdIndex::build(&model(), schema());
    assert_eq!(index.get(&guid(WALL)), Ok(Some(EntityId(1))));
    assert_eq!(
        index.get(&guid("31t64Uzj98DhdKplMxBpF5")),
        Ok(Some(EntityId(8))),
        "a typed wrapper is read through"
    );
}

#[test]
fn an_absent_global_id_is_none_and_a_non_root_is_not_indexed() {
    let index = GlobalIdIndex::build(&model(), schema());
    assert_eq!(index.get(&guid("3MtNd0Uaz8Ix2OPtRq_3Kk")), Ok(None));
    assert_eq!(
        index.get(&guid("0dGA15OQf9_P5UdNj5qcto")),
        Ok(None),
        "a point's first slot is not a GlobalId"
    );
}

/// Neither the first nor the last holder wins: every one is named.
#[test]
fn a_duplicated_global_id_is_refused_with_every_holder() {
    let index = GlobalIdIndex::build(&model(), schema());
    let error = index.get(&guid(SHARED)).unwrap_err();
    assert_eq!(error.global_id, guid(SHARED));
    assert_eq!(error.entities, [EntityId(2), EntityId(4), EntityId(7)]);
    assert_eq!(
        error.to_string(),
        format!("GlobalId {SHARED} is carried by #2, #4, #7")
    );
    let duplicates: Vec<_> = index.duplicates().collect();
    assert_eq!(
        duplicates,
        [(&guid(SHARED), &[EntityId(2), EntityId(4), EntityId(7)][..])]
    );
    assert_eq!(index.len(), 3, "the wall, the window and the shared one");
}

#[test]
fn unset_and_malformed_global_ids_are_listed_not_indexed() {
    let index = GlobalIdIndex::build(&model(), schema());
    assert_eq!(index.invalid(), [EntityId(5), EntityId(6)]);
}

#[test]
fn the_one_shot_scan_agrees_with_the_index() {
    let model = model();
    let schema = schema();
    let index = GlobalIdIndex::build(&model, schema);
    for text in [
        WALL,
        SHARED,
        "31t64Uzj98DhdKplMxBpF5",
        "3MtNd0Uaz8Ix2OPtRq_3Kk",
    ] {
        let guid = guid(text);
        assert_eq!(by_guid(&model, schema, &guid), index.get(&guid), "{text}");
    }
}

#[test]
fn an_empty_model_has_an_empty_index() {
    let index = GlobalIdIndex::build(&Model::new(), schema());
    assert!(index.is_empty());
    assert_eq!(index.duplicates().count(), 0);
}
