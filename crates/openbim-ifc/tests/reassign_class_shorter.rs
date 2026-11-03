//! A class with fewer attributes than the stored record shortens it (#634):
//! the transaction truncates, and undo, redo and replay stay byte-exact.

#![cfg(all(
    feature = "step",
    feature = "author",
    feature = "ifc2x3",
    feature = "ifc4",
    feature = "ifc4x3"
))]

mod reassign_support;

use ifc::{apply_authoring, AuthorOp, EntityId, History, ReassignClassOptions, SchemaVersion};
use reassign_support::*;

#[test]
fn an_ifc2x3_door_becomes_a_wall_with_a_shorter_record() {
    let schema = ifc::schema::for_version(SchemaVersion::Ifc2x3).unwrap();
    let mut model = read("cascade_ifc2x3.ifc");
    let original = bytes(&model);
    let held = model.get(EntityId(100)).unwrap().attributes.len();
    let ops = [AuthorOp::ReassignClass {
        entity: EntityId(100),
        type_name: "IfcWall".into(),
        attributes: vec![],
        options: ReassignClassOptions::new().drop_unmapped(true),
    }];

    let mut history = History::new(8);
    let outcome = history
        .track(&mut model, |m| apply_authoring(m, schema, &ops, 5))
        .expect("reassigns");
    let report = outcome.reassignments[0].as_ref().expect("a report");
    assert_eq!(report.dropped, ["OverallHeight", "OverallWidth"]);
    assert_eq!(class(&model, EntityId(100)), "IFCWALL");
    let shortened = model.get(EntityId(100)).unwrap().attributes.len();
    assert_eq!((held, shortened), (10, 8));
    let after = bytes(&model);

    history.undo(&mut model).unwrap();
    assert_eq!(bytes(&model), original, "undo");
    history.redo(&mut model).unwrap();
    assert_eq!(bytes(&model), after, "redo");
    history.undo(&mut model).unwrap();
    apply_authoring(&mut model, schema, &ops, 5).unwrap();
    assert_eq!(bytes(&model), after, "replay");
}

#[test]
fn an_ifc4_door_built_in_memory_becomes_a_wall() {
    let (mut model, _) = scene();
    let applied = apply(&mut model, ifc4(), &[product("IfcDoor", None, vec![])]).unwrap();
    let door = applied.ids[0].expect("produced");
    let held = model.get(door).unwrap().attributes.len();
    let original = bytes(&model);

    let mut history = History::new(8);
    history
        .track(&mut model, |m| {
            apply(m, ifc4(), &[reassign(door, "IfcWall", vec![])])
        })
        .expect("reassigns");

    assert_eq!(class(&model, door), "IFCWALL");
    let shortened = model.get(door).unwrap().attributes.len();
    assert_eq!((held, shortened), (13, 9));
    let after = bytes(&model);
    history.undo(&mut model).unwrap();
    assert_eq!(bytes(&model), original, "undo");
    history.redo(&mut model).unwrap();
    assert_eq!(bytes(&model), after, "redo");
}
