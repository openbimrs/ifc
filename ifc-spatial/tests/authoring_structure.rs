//! An authored spatial structure, read back by SpatialTree.
//!
//! The tree is what every consumer of this crate actually uses, so
//! asserting through it proves the authored slots are the ones it
//! reads -- including the inversion between the two relationships.

use ifc_model::{Entity, EntityId, Model, Transaction, Value};
use ifc_spatial::{
    aggregate, contain, create_project, create_spatial_element, SpatialDraft, SpatialKind,
    SpatialTree,
};

/// A wall to place in the structure.
fn wall(tx: &mut Transaction, guid: &str) -> EntityId {
    let mut attributes = vec![Value::Null; 8];
    attributes[0] = Value::Text(guid.into());
    tx.create(Entity::new("IFCWALL", attributes))
}

/// A whole structure authored, then walked by the tree reader.
#[test]
fn an_authored_structure_walks_as_authored() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let project =
        create_project(&mut tx, "0aBcDeFgHiJkLmNoPqRsTu", Some("Tower"), None).expect("project");
    let named = |n| SpatialDraft {
        name: Some(n),
        ..SpatialDraft::default()
    };
    let site = create_spatial_element(
        &mut tx,
        SpatialKind::Site,
        "1aBcDeFgHiJkLmNoPqRsTu",
        named("Site"),
    )
    .expect("site");
    let building = create_spatial_element(
        &mut tx,
        SpatialKind::Building,
        "2aBcDeFgHiJkLmNoPqRsTu",
        named("Block A"),
    )
    .expect("building");
    let storey = create_spatial_element(
        &mut tx,
        SpatialKind::Storey,
        "3aBcDeFgHiJkLmNoPqRsTu",
        named("Level 1"),
    )
    .expect("storey");
    let w = wall(&mut tx, "4aBcDeFgHiJkLmNoPqRsTu");
    aggregate(&mut tx, "5aBcDeFgHiJkLmNoPqRsTu", project, &[site]).expect("project/site");
    aggregate(&mut tx, "6aBcDeFgHiJkLmNoPqRsTu", site, &[building]).expect("site/building");
    aggregate(&mut tx, "7aBcDeFgHiJkLmNoPqRsTu", building, &[storey]).expect("building/storey");
    contain(&mut tx, "8aBcDeFgHiJkLmNoPqRsTu", storey, &[w]).expect("storey contains wall");
    tx.commit(&mut model).expect("commit");

    let tree = SpatialTree::build(&model);
    // The wall hangs off the storey, not the other way round: this is
    // the assertion that fails if the two slot layouts get conflated.
    assert_eq!(tree.container_of(w), Some(storey));
    assert_eq!(tree.elements_of(storey), &[w]);
    // ancestors() walks containers, which a wall is not; the chain is
    // asserted from the storey up.
    assert_eq!(tree.ancestors(storey), vec![building, site, project]);
    assert_eq!(tree.roots(), &[project]);
    assert!(tree.dangling().is_empty(), "authored refs all resolve");
    assert!(tree.orphans().is_empty(), "every container has a parent");
}

/// The two relationships write their parent to different slots.
///
/// IfcRelAggregates puts the parent at 4; IfcRelContainedIn puts it
/// at 5. Asserting the raw slots proves the authoring respects the
/// inversion rather than passing by luck through a reader that
/// happens to look in both places.
#[test]
fn the_two_relationships_keep_their_opposite_slots() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let named = |n| SpatialDraft {
        name: Some(n),
        ..SpatialDraft::default()
    };
    let storey = create_spatial_element(
        &mut tx,
        SpatialKind::Storey,
        "1aBcDeFgHiJkLmNoPqRsTu",
        named("L1"),
    )
    .expect("storey");
    let w = wall(&mut tx, "2aBcDeFgHiJkLmNoPqRsTu");
    let agg = aggregate(&mut tx, "3aBcDeFgHiJkLmNoPqRsTu", storey, &[w]).expect("agg");
    let con = contain(&mut tx, "4aBcDeFgHiJkLmNoPqRsTu", storey, &[w]).expect("con");
    tx.commit(&mut model).expect("commit");

    let slot = |id, n: usize| model.get(id).unwrap().attributes[n].clone();
    assert_eq!(slot(agg, 4), Value::Ref(storey), "aggregates: parent at 4");
    assert_eq!(
        slot(con, 5),
        Value::Ref(storey),
        "contained-in: parent at 5"
    );
    assert_eq!(slot(agg, 5), Value::List(vec![Value::Ref(w)]));
    assert_eq!(slot(con, 4), Value::List(vec![Value::Ref(w)]));
}

/// Structures that parse but describe nothing are refused.
#[test]
fn meaningless_structures_are_refused() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let g = "1aBcDeFgHiJkLmNoPqRsTu";
    let named = |n| SpatialDraft {
        name: Some(n),
        ..SpatialDraft::default()
    };
    let s = create_spatial_element(&mut tx, SpatialKind::Storey, g, named("L1")).expect("storey");

    // A malformed GlobalId: nothing can reference this container.
    assert!(create_project(&mut tx, "not-a-guid", None, None).is_err());
    assert!(create_spatial_element(&mut tx, SpatialKind::Site, "short", named("S")).is_err());

    // An empty relationship relates nothing.
    assert!(aggregate(&mut tx, g, s, &[]).is_err());
    assert!(contain(&mut tx, g, s, &[]).is_err());

    // A container inside itself is a cycle the tree walk must never see.
    assert!(aggregate(&mut tx, g, s, &[s]).is_err());
    assert!(contain(&mut tx, g, s, &[s]).is_err());

    // Project is not an IfcSpatialStructureElement.
    assert!(create_spatial_element(&mut tx, SpatialKind::Project, g, named("P")).is_err());
}

/// Each container is written at its own schema width, with LongName
/// in its own slot.
///
/// Width matters because a trailing slot the schema does not define
/// for this type is not a harmless blank: a reader indexing by
/// position past the end of the entity gets a different answer than
/// the schema describes.
#[test]
fn each_container_is_written_at_its_own_width() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let draft = SpatialDraft {
        name: Some("Name"),
        long_name: Some("Long"),
        ..SpatialDraft::default()
    };
    let site = create_spatial_element(&mut tx, SpatialKind::Site, "1aBcDeFgHiJkLmNoPqRsTu", draft)
        .expect("site");
    let building = create_spatial_element(
        &mut tx,
        SpatialKind::Building,
        "2aBcDeFgHiJkLmNoPqRsTu",
        draft,
    )
    .expect("building");
    let storey = create_spatial_element(
        &mut tx,
        SpatialKind::Storey,
        "3aBcDeFgHiJkLmNoPqRsTu",
        draft,
    )
    .expect("storey");
    let space =
        create_spatial_element(&mut tx, SpatialKind::Space, "4aBcDeFgHiJkLmNoPqRsTu", draft)
            .expect("space");
    let project =
        create_project(&mut tx, "5aBcDeFgHiJkLmNoPqRsTu", Some("P"), None).expect("project");
    tx.commit(&mut model).expect("commit");

    let width = |id| model.get(id).unwrap().attributes.len();
    assert_eq!(width(site), 14, "IfcSite");
    assert_eq!(width(building), 12, "IfcBuilding");
    assert_eq!(width(storey), 10, "IfcBuildingStorey");
    assert_eq!(width(space), 11, "IfcSpace");
    assert_eq!(width(project), 9, "IfcProject");

    // Name at 2, LongName at 7: distinct fields, distinct slots.
    let slot = |id, n: usize| model.get(id).unwrap().attributes[n].clone();
    assert_eq!(slot(storey, 2), Value::Text("Name".into()));
    assert_eq!(slot(storey, 7), Value::Text("Long".into()));
}
