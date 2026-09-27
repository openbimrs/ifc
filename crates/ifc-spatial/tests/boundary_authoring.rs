//! Authoring space boundaries and path connections.
//!
//! `CorrectPhysOrVirt` is the rule with teeth here: the
//! physical/virtual flag must agree with the type of the element
//! doing the bounding, so the two cannot be set independently.

use ifc_model::{Entity, Model, Transaction, Value};
use ifc_spatial::{connect_path_elements, create_space_boundary, BoundaryDraft, BoundaryLevel};

const GUID: &str = "1jQ2A$rnvCJhUvFV5RxFtz";
const GUID2: &str = "3rT4B$mkwDKiVwGW6SyGua";

fn element(tx: &mut Transaction, type_name: &str) -> ifc_model::EntityId {
    tx.create(Entity::new(type_name, vec![Value::Null; 8]))
}

fn draft(space: ifc_model::EntityId, bounded: ifc_model::EntityId) -> BoundaryDraft<'static> {
    BoundaryDraft {
        name: None,
        description: None,
        space,
        element: bounded,
        connection_geometry: None,
        physical_or_virtual: "PHYSICAL",
        internal_or_external: "INTERNAL",
        parent: None,
        corresponding: None,
    }
}

/// Each level stages its own type with its own arity.
#[test]
fn every_boundary_level_stages() {
    let cases = [
        (BoundaryLevel::Base, "IFCRELSPACEBOUNDARY", 9),
        (BoundaryLevel::First, "IFCRELSPACEBOUNDARY1STLEVEL", 10),
        (BoundaryLevel::Second, "IFCRELSPACEBOUNDARY2NDLEVEL", 11),
    ];

    for (level, expected, arity) in cases {
        let mut model = Model::default();
        let mut tx = Transaction::new(&model);
        let space = element(&mut tx, "IFCSPACE");
        let wall = element(&mut tx, "IFCWALL");
        let id = create_space_boundary(&mut tx, &model, level, GUID, draft(space, wall))
            .unwrap_or_else(|error| panic!("{expected} refused: {error:?}"));
        tx.commit(&mut model).expect("commit");

        let staged = model.get(id).expect("staged");
        assert_eq!(staged.type_name.as_ref(), expected);
        assert_eq!(staged.attributes.len(), arity, "{expected} arity");
        assert_eq!(staged.attributes[4], Value::Ref(space));
        assert_eq!(staged.attributes[5], Value::Ref(wall));
    }
}

/// CorrectPhysOrVirt: a physical boundary rejects a virtual element.
#[test]
fn a_physical_boundary_refuses_a_virtual_element() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let space = element(&mut tx, "IFCSPACE");
    let ghost = element(&mut tx, "IFCVIRTUALELEMENT");

    assert!(
        create_space_boundary(
            &mut tx,
            &model,
            BoundaryLevel::Base,
            GUID,
            draft(space, ghost),
        )
        .is_err(),
        "a PHYSICAL boundary accepted a virtual element",
    );
}

/// CorrectPhysOrVirt: a virtual boundary rejects a solid element.
#[test]
fn a_virtual_boundary_refuses_a_solid_element() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let space = element(&mut tx, "IFCSPACE");
    let wall = element(&mut tx, "IFCWALL");
    let mut virtual_draft = draft(space, wall);
    virtual_draft.physical_or_virtual = "VIRTUAL";

    assert!(
        create_space_boundary(&mut tx, &model, BoundaryLevel::Base, GUID, virtual_draft).is_err(),
        "a VIRTUAL boundary accepted a wall",
    );
}

/// An opening is a legal virtual boundary, and NOTDEFINED is free.
#[test]
fn the_permitted_virtual_combinations_stage() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let space = element(&mut tx, "IFCSPACE");
    let opening = element(&mut tx, "IFCOPENINGELEMENT");
    let wall = element(&mut tx, "IFCWALL");

    let mut virtual_opening = draft(space, opening);
    virtual_opening.physical_or_virtual = "VIRTUAL";
    create_space_boundary(&mut tx, &model, BoundaryLevel::Base, GUID, virtual_opening)
        .expect("an opening is a legal virtual boundary");

    let mut undefined = draft(space, wall);
    undefined.physical_or_virtual = "NOTDEFINED";
    create_space_boundary(&mut tx, &model, BoundaryLevel::Base, GUID2, undefined)
        .expect("NOTDEFINED is unconstrained");
    tx.commit(&mut model).expect("commit");
}

/// A slot the level does not declare cannot be filled.
#[test]
fn a_slot_the_level_lacks_is_refused() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let space = element(&mut tx, "IFCSPACE");
    let wall = element(&mut tx, "IFCWALL");
    let parent = create_space_boundary(
        &mut tx,
        &model,
        BoundaryLevel::First,
        GUID,
        draft(space, wall),
    )
    .expect("first level");
    tx.commit(&mut model).expect("commit");

    let mut tx = Transaction::new(&model);
    let mut with_parent = draft(space, wall);
    with_parent.parent = Some(parent);
    assert!(
        create_space_boundary(&mut tx, &model, BoundaryLevel::Base, GUID2, with_parent).is_err(),
        "the base level accepted a ParentBoundary it does not declare",
    );

    let mut with_corresponding = draft(space, wall);
    with_corresponding.corresponding = Some(parent);
    assert!(
        create_space_boundary(
            &mut tx,
            &model,
            BoundaryLevel::First,
            GUID2,
            with_corresponding
        )
        .is_err(),
        "the first level accepted a CorrespondingBoundary it does not declare",
    );
}

/// Path connections stage, and priorities are bounded 0..=100.
#[test]
fn path_connections_stage_with_bounded_priorities() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let first = element(&mut tx, "IFCWALL");
    let second = element(&mut tx, "IFCWALL");

    let id = connect_path_elements(
        &mut tx,
        GUID,
        first,
        second,
        (&[0, 50, 100], &[25]),
        ("ATSTART", "ATEND"),
    )
    .expect("path connection");
    tx.commit(&mut model).expect("commit");

    let staged = model.get(id).expect("staged");
    assert_eq!(staged.type_name.as_ref(), "IFCRELCONNECTSPATHELEMENTS");
    assert_eq!(staged.attributes.len(), 11);
    // RelatedConnectionType is slot 9, RelatingConnectionType slot 10:
    // the schema lists them in that order, which is not the argument
    // order a caller would assume.
    assert_eq!(staged.attributes[9], Value::Enum("ATEND".into()));
    assert_eq!(staged.attributes[10], Value::Enum("ATSTART".into()));
}

/// A priority outside 0..=100 is refused, not clamped.
#[test]
fn an_out_of_range_priority_is_refused() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let first = element(&mut tx, "IFCWALL");
    let second = element(&mut tx, "IFCWALL");

    for priorities in [(&[101][..], &[0][..]), (&[0][..], &[-1][..])] {
        assert!(
            connect_path_elements(
                &mut tx,
                GUID,
                first,
                second,
                priorities,
                ("ATPATH", "ATPATH"),
            )
            .is_err(),
            "an out-of-range priority was accepted",
        );
    }
}
