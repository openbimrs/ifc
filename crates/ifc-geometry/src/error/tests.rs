use super::*;

#[test]
fn errors_name_the_entity_so_failures_are_locatable() {
    let e = GeometryError::MissingAttribute {
        entity: EntityId(42),
        type_name: "IFCEXTRUDEDAREASOLID".into(),
        attribute: "SweptArea",
    };
    assert_eq!(e.entity(), Some(EntityId(42)));
    assert!(e.to_string().contains("#42"));
    assert!(e.to_string().contains("SweptArea"));
}

#[test]
fn unsupported_is_distinguishable_from_corruption() {
    let unsupported = GeometryError::Unsupported {
        entity: EntityId(1),
        type_name: "IFCSECTIONEDSPINE".into(),
        detail: "spine interpolation",
    };
    let broken = GeometryError::Degenerate {
        entity: EntityId(1),
        type_name: "IFCCIRCLE".into(),
        detail: "zero radius".into(),
    };
    assert!(unsupported.is_unsupported());
    assert!(!broken.is_unsupported());
}

#[test]
fn unit_refusals_name_the_entity_and_are_not_unsupported() {
    let undeclared = GeometryError::LengthUnitUndeclared {
        project: Some(EntityId(1)),
        assignment: Some(EntityId(2)),
    };
    assert_eq!(undeclared.entity(), Some(EntityId(2)));
    let no_assignment = GeometryError::LengthUnitUndeclared {
        project: Some(EntityId(1)),
        assignment: None,
    };
    assert_eq!(no_assignment.entity(), Some(EntityId(1)));
    let no_project = GeometryError::LengthUnitUndeclared {
        project: None,
        assignment: None,
    };
    assert_eq!(no_project.entity(), None);
    let angle = GeometryError::AngleUnitUndeclared {
        entity: EntityId(3),
    };
    assert_eq!(angle.entity(), Some(EntityId(3)));
    let unreadable = GeometryError::UnitUnreadable {
        unit: EntityId(4),
        unit_type: "LENGTHUNIT",
        detail: "x".into(),
    };
    assert_eq!(unreadable.entity(), Some(EntityId(4)));
    let ambiguous = GeometryError::UnitsAmbiguous {
        first: EntityId(5),
        second: EntityId(6),
        kind: "LENGTHUNIT",
    };
    assert_eq!(ambiguous.entity(), Some(EntityId(6)));
    for e in [undeclared, angle, unreadable, ambiguous] {
        assert!(!e.is_unsupported());
    }
}
