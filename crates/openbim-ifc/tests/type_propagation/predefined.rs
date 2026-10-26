//! Reconciling `PredefinedType` with the type's.

use super::*;

// --- predefined type ---

#[test]
fn sync_unsets_the_wall_predefined_type_the_type_provides() {
    each_release(|mut f| {
        if !f.has_predefined("IfcWall") {
            // IFC2X3: `IfcWall` has no `PredefinedType`; sync does
            // nothing and the assignment still succeeds.
            let wall = f.wall(vec![]);
            let op = f.assign(
                f.base.wall_type,
                &[wall],
                AssignTypeOptions::new().predefined_type(PredefinedTypeMode::Sync),
            );
            f.apply(vec![op]);
            return;
        }
        let wall = f.wall(vec![("PredefinedType", enumeration("PARTITIONING"))]);
        let sync = AssignTypeOptions::new().predefined_type(PredefinedTypeMode::Sync);
        let op = f.assign(f.base.wall_type, &[wall], sync);
        f.apply(vec![op]);
        assert_eq!(f.attr(wall, "PredefinedType"), Value::Null, "{}", f.name());
    });
}

#[test]
fn validate_refuses_a_conflicting_predefined_type_and_accepts_an_equal_one() {
    for (release, token) in RELEASES.into_iter().skip(1) {
        let mut f = fixture(release, token);
        let validate = AssignTypeOptions::new().predefined_type(PredefinedTypeMode::Validate);
        let conflicting = f.wall(vec![("PredefinedType", enumeration("PARTITIONING"))]);
        let op = f.assign(f.base.wall_type, &[conflicting], validate);
        let failure = f.refuse(vec![op]);
        assert!(
            matches!(&failure, AuthoringFailure::PredefinedTypeConflict { object, object_value, type_value }
                if *object == conflicting && object_value == "PARTITIONING" && type_value == "SOLIDWALL"),
            "{failure:?}"
        );
        let equal = f.wall(vec![("PredefinedType", enumeration("SOLIDWALL"))]);
        let undefined = f.wall(vec![("PredefinedType", enumeration("NOTDEFINED"))]);
        let op = f.assign(f.base.wall_type, &[equal, undefined], validate);
        f.apply(vec![op]);
        assert_eq!(f.attr(equal, "PredefinedType"), enumeration("SOLIDWALL"));
    }
}

#[test]
fn userdefined_compares_object_type_with_element_type() {
    for (release, token) in RELEASES.into_iter().skip(1) {
        let mut f = fixture(release, token);
        let mut b = Batch::default();
        let custom = b.push(AuthorOp::TypeObject {
            type_name: "IfcDoorType".into(),
            attributes: named(vec![
                ("Name", text("Custom")),
                ("ElementType", text("Revolving")),
                ("PredefinedType", enumeration("USERDEFINED")),
                ("OperationType", enumeration("REVOLVING")),
            ]),
            owner_history: Some(f.base.owner),
        });
        let mut door = |b: &mut Batch, name: &str| {
            b.push(f.product(
                "IfcDoor",
                vec![
                    ("Name", text("Door")),
                    ("ObjectType", text(name)),
                    ("PredefinedType", enumeration("USERDEFINED")),
                ],
            ))
        };
        let same = door(&mut b, "Revolving");
        let other = door(&mut b, "Sliding");
        let ids = f.apply(b.ops);
        let real =
            |handle: EntityId| ids[usize::try_from(handle.0 - HANDLE_BASE).unwrap()].unwrap();
        let (custom, same, other) = (real(custom), real(same), real(other));

        let validate = AssignTypeOptions::new().predefined_type(PredefinedTypeMode::Validate);
        let op = f.assign(custom, &[other], validate);
        let failure = f.refuse(vec![op]);
        assert!(
            matches!(&failure, AuthoringFailure::PredefinedTypeConflict { type_value, .. } if type_value == "USERDEFINED ('Revolving')"),
            "{failure:?}"
        );
        let op = f.assign(custom, &[same], validate);
        f.apply(vec![op]);

        let sync = AssignTypeOptions::new()
            .predefined_type(PredefinedTypeMode::Sync)
            .replace(true);
        let op = f.assign(custom, &[other], sync);
        f.apply(vec![op]);
        assert_eq!(f.attr(other, "PredefinedType"), Value::Null);
        assert_eq!(f.attr(other, "ObjectType"), Value::Null);
    }
}

#[test]
fn a_notdefined_type_leaves_the_object_predefined_type() {
    for (release, token) in RELEASES.into_iter().skip(1) {
        let mut f = fixture(release, token);
        let mut b = Batch::default();
        let open = wall_type(
            &mut b,
            f.base.owner,
            "Open",
            f.base.other_wall_map,
            "NOTDEFINED",
        );
        let ids = f.apply(b.ops);
        let open = ids[usize::try_from(open.0 - HANDLE_BASE).unwrap()].unwrap();
        let wall = f.wall(vec![("PredefinedType", enumeration("PARTITIONING"))]);
        let op = f.assign(
            open,
            &[wall],
            AssignTypeOptions::new().predefined_type(PredefinedTypeMode::Sync),
        );
        f.apply(vec![op]);
        assert_eq!(f.attr(wall, "PredefinedType"), enumeration("PARTITIONING"));
    }
}

#[test]
fn ifc2x3_slab_predefined_type_is_validated() {
    let mut f = fixture(SchemaVersion::Ifc2x3, "IFC2X3");
    let mut b = Batch::default();
    let slab_type = b.push(AuthorOp::TypeObject {
        type_name: "IfcSlabType".into(),
        attributes: named(vec![
            ("Name", text("S")),
            ("PredefinedType", enumeration("ROOF")),
        ]),
        owner_history: Some(f.base.owner),
    });
    let slab = b.push(f.product(
        "IfcSlab",
        vec![
            ("Name", text("Slab")),
            ("PredefinedType", enumeration("FLOOR")),
        ],
    ));
    let ids = f.apply(b.ops);
    let real = |handle: EntityId| ids[usize::try_from(handle.0 - HANDLE_BASE).unwrap()].unwrap();
    let (slab_type, slab) = (real(slab_type), real(slab));
    let op = f.assign(
        slab_type,
        &[slab],
        AssignTypeOptions::new().predefined_type(PredefinedTypeMode::Validate),
    );
    assert!(matches!(
        f.refuse(vec![op]),
        AuthoringFailure::PredefinedTypeConflict { .. }
    ));
    let op = f.assign(
        slab_type,
        &[slab],
        AssignTypeOptions::new().predefined_type(PredefinedTypeMode::Sync),
    );
    f.apply(vec![op]);
    assert_eq!(f.attr(slab, "PredefinedType"), Value::Null);
}
