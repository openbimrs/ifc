//! Door operation geometry (#148): every refused operation type, and every
//! input that is missing or contradicts the operation, is a typed error,
//! never a default.

#![cfg(all(feature = "step", feature = "properties", feature = "geometry-select"))]

mod door_support;

use door_support::*;
use ifc::{door_operation, DoorOperationError, RefusedOperation};
use ifc_properties::SchemaVersion;

const IFC4: SchemaVersion = SchemaVersion::Ifc4;

fn swinging(operation: &'static str) -> Door {
    Door::new(
        IFC4,
        operation,
        vec![(20, panel(IFC4, 20, "SWINGING", "LEFT", Some("1.")))],
    )
}

fn refused(door: &Door) -> DoorOperationError {
    door_operation(&door.model(), DOOR).expect_err("refused")
}

#[test]
fn each_refused_operation_type_names_its_reason() {
    use RefusedOperation as R;
    let cases = [
        (IFC4, "NOTDEFINED", R::NotDefined),
        (IFC4, "USERDEFINED", R::UserDefined),
        (IFC4, "REVOLVING", R::Revolving),
        (IFC4, "FOLDING_TO_LEFT", R::Folding),
        (IFC4, "FOLDING_TO_RIGHT", R::Folding),
        (IFC4, "DOUBLE_DOOR_FOLDING", R::Folding),
        (
            IFC4,
            "DOUBLE_DOOR_SINGLE_SWING_OPPOSITE_LEFT",
            R::AmbiguousSwingDirection,
        ),
        (
            IFC4,
            "DOUBLE_DOOR_SINGLE_SWING_OPPOSITE_RIGHT",
            R::AmbiguousSwingDirection,
        ),
        (SchemaVersion::Ifc4x3, "REVOLVING_VERTICAL", R::Revolving),
        (SchemaVersion::Ifc4x3, "LIFTING_HORIZONTAL", R::Lifting),
        (SchemaVersion::Ifc4x3, "LIFTING_VERTICAL_LEFT", R::Lifting),
        (SchemaVersion::Ifc4x3, "LIFTING_VERTICAL_RIGHT", R::Lifting),
        (
            SchemaVersion::Ifc4x3,
            "DOUBLE_DOOR_LIFTING_VERTICAL",
            R::Lifting,
        ),
        (SchemaVersion::Ifc2x3, "NOTDEFINED", R::NotDefined),
        (SchemaVersion::Ifc2x3, "REVOLVING", R::Revolving),
    ];
    for (version, operation, reason) in cases {
        let door = Door::new(
            version,
            operation,
            vec![(20, panel(version, 20, "SWINGING", "LEFT", Some("1.")))],
        );
        assert_eq!(
            refused(&door),
            DoorOperationError::RefusedOperation {
                operation: operation.into(),
                reason,
            },
            "{operation}"
        );
    }
}

#[test]
fn an_operation_type_foreign_to_the_release_is_malformed() {
    // `SWING_FIXED_LEFT` joined the enumeration in IFC4.
    let v = SchemaVersion::Ifc2x3;
    let door = Door::new(
        v,
        "SWING_FIXED_LEFT",
        vec![(20, panel(v, 20, "SWINGING", "LEFT", Some("1.")))],
    );
    assert_eq!(
        refused(&door),
        DoorOperationError::MalformedAttribute {
            entity: DOOR_TYPE,
            attribute: "OperationType",
        }
    );
}

#[test]
fn a_door_without_panel_properties_is_refused() {
    let door = Door::new(IFC4, "SINGLE_SWING_LEFT", Vec::new());
    assert_eq!(
        refused(&door),
        DoorOperationError::NoPanelProperties { door: DOOR }
    );
}

#[test]
fn a_missing_overall_width_is_not_taken_from_geometry() {
    let mut door = swinging("SINGLE_SWING_LEFT");
    door.width = None;
    assert_eq!(
        refused(&door),
        DoorOperationError::MissingOverallWidth { door: DOOR }
    );
}

#[test]
fn conflicting_occurrence_and_type_operations_are_refused() {
    let mut door = swinging("SINGLE_SWING_LEFT");
    door.occurrence_operation = Some("SINGLE_SWING_RIGHT");
    assert_eq!(
        refused(&door),
        DoorOperationError::ConflictingOperationType {
            occurrence: "SINGLE_SWING_RIGHT".into(),
            type_object: DOOR_TYPE,
            type_value: "SINGLE_SWING_LEFT".into(),
        }
    );
}

#[test]
fn an_untyped_door_without_an_operation_type_is_refused() {
    let mut door = swinging("SINGLE_SWING_LEFT");
    door.type_operation = None;
    door.occurrence_panels = std::mem::take(&mut door.type_panels);
    assert_eq!(
        refused(&door),
        DoorOperationError::MissingOperationType { door: DOOR }
    );
}

#[test]
fn an_ifc4_door_typed_by_a_door_style_is_refused() {
    // IFC4 `CorrectStyleAssigned`: the type object has to be an IfcDoorType.
    let mut door = swinging("SINGLE_SWING_LEFT");
    door.type_entity = Some("IfcDoorStyle");
    assert!(matches!(
        refused(&door),
        DoorOperationError::UnsupportedTypeObject {
            type_object: DOOR_TYPE,
            ..
        }
    ));
}

#[test]
fn a_non_door_is_refused() {
    let mut door = swinging("SINGLE_SWING_LEFT");
    door.type_operation = None;
    door.type_panels.clear();
    // A window has an `OverallWidth` too, and no door operation.
    door.door_entity = "IfcWindow";
    assert!(matches!(
        refused(&door),
        DoorOperationError::NotADoor { entity: DOOR, .. }
    ));
}

#[test]
fn a_door_type_is_refused_as_no_door() {
    // The exact property reader accepts a type object since
    // ifc-properties#193; the type still has no placement or operation of
    // its own, so it is refused here, in every release.
    for version in [IFC4, SchemaVersion::Ifc2x3, SchemaVersion::Ifc4x3] {
        let door = Door::new(
            version,
            "SINGLE_SWING_LEFT",
            vec![(20, panel(version, 20, "SWINGING", "LEFT", Some("1.")))],
        );
        assert!(
            matches!(
                door_operation(&door.model(), DOOR_TYPE),
                Err(DoorOperationError::NotADoor {
                    entity: DOOR_TYPE,
                    ..
                })
            ),
            "{version:?}"
        );
    }
}

#[test]
fn panels_must_match_the_operation() {
    // Wrong count.
    let mut door = swinging("DOUBLE_DOOR_SINGLE_SWING");
    assert_eq!(
        refused(&door),
        DoorOperationError::PanelCount {
            expected: 2,
            found: 1
        }
    );
    // Wrong panel operation.
    door = Door::new(
        IFC4,
        "SINGLE_SWING_LEFT",
        vec![(20, panel(IFC4, 20, "SLIDING", "LEFT", None))],
    );
    assert_eq!(
        refused(&door),
        DoorOperationError::PanelMismatch {
            set: EntityId(20),
            attribute: "PanelOperation",
            found: "SLIDING".into(),
        }
    );
    // Two LEFT panels: no RIGHT leaf.
    door = Door::new(
        IFC4,
        "DOUBLE_DOOR_SINGLE_SWING",
        vec![
            (20, panel(IFC4, 20, "SWINGING", "LEFT", Some("0.5"))),
            (21, panel(IFC4, 21, "SWINGING", "LEFT", Some("0.5"))),
        ],
    );
    assert_eq!(
        refused(&door),
        DoorOperationError::PanelMismatch {
            set: EntityId(21),
            attribute: "PanelPosition",
            found: "LEFT".into(),
        }
    );
    // SWING_FIXED with two swinging panels.
    door = Door::new(
        IFC4,
        "SWING_FIXED_LEFT",
        vec![
            (20, panel(IFC4, 20, "SWINGING", "LEFT", Some("0.5"))),
            (21, panel(IFC4, 21, "SWINGING", "RIGHT", Some("0.5"))),
        ],
    );
    assert!(matches!(
        refused(&door),
        DoorOperationError::PanelMismatch {
            attribute: "PanelOperation",
            ..
        }
    ));
}

#[test]
fn panel_widths_must_be_stated_valid_and_partition_the_opening() {
    let pair = |left: Option<&str>, right: Option<&str>| {
        Door::new(
            IFC4,
            "DOUBLE_DOOR_SINGLE_SWING",
            vec![
                (20, panel(IFC4, 20, "SWINGING", "LEFT", left)),
                (21, panel(IFC4, 21, "SWINGING", "RIGHT", right)),
            ],
        )
    };
    assert_eq!(
        refused(&pair(Some("0.5"), None)),
        DoorOperationError::MissingPanelWidth { set: EntityId(21) }
    );
    assert_eq!(
        refused(&pair(Some("0.5"), Some("0.4"))),
        DoorOperationError::PanelWidthsDoNotPartition { sum: 0.9 }
    );
    assert_eq!(
        refused(&pair(Some("0."), Some("1."))),
        DoorOperationError::InvalidPanelWidth {
            set: EntityId(20),
            width: 0.0
        }
    );
    // A ratio within the partition tolerance is accepted, and each leaf
    // stays anchored at its own jamb: the RIGHT one ends at OverallWidth.
    assert!(door_operation(&pair(Some("0.6667"), Some("0.3333")).model(), DOOR).is_ok());
    let op = door_operation(&pair(Some("0.5"), Some("0.5000004")).model(), DOOR).expect("ok");
    let right = &op.leaves[1];
    assert_near(right.frame_world.origin[0], 0.9 * (1.0 - 0.5000004));
    assert_near(right.frame_world.origin[0] + right.width, 0.9);
}

#[test]
fn a_narrow_single_panel_must_say_which_side_it_stands_on() {
    let narrow = |position| {
        Door::new(
            IFC4,
            "SINGLE_SWING_LEFT",
            vec![(20, panel(IFC4, 20, "SWINGING", position, Some("0.5")))],
        )
    };
    for position in ["MIDDLE", "NOTDEFINED"] {
        assert_eq!(
            refused(&narrow(position)),
            DoorOperationError::UnplacedPanel { set: EntityId(20) },
            "{position}"
        );
    }
    let op = door_operation(&narrow("RIGHT").model(), DOOR).expect("placed");
    let leaf = &op.leaves[0];
    assert_close(leaf.frame_world.origin, [0.45, 0.0, 0.0]);
    assert_close(leaf.swing.expect("swing").center, [0.45, 0.0, 0.0]);
    // A full-width panel needs no position.
    let op = door_operation(
        &Door::new(
            IFC4,
            "SINGLE_SWING_LEFT",
            vec![(20, panel(IFC4, 20, "SWINGING", "MIDDLE", None))],
        )
        .model(),
        DOOR,
    )
    .expect("full width");
    assert_close(op.leaves[0].frame_world.origin, [0.0, 0.0, 0.0]);
}
