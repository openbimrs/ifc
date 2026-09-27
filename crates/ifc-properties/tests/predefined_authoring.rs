//! Authoring the predefined property sets and the property enumeration.

use ifc_model::{Model, Transaction, Value};
use ifc_properties::{
    add_complex_property_template, add_door_lining_properties, add_door_panel_properties,
    add_permeable_covering_properties, add_property_dependency_relationship,
    add_property_enumeration, add_window_lining_properties, add_window_panel_properties,
    DoorLiningDraft, PropertyError, WindowLiningDraft,
};
use ifc_schema::{ifc4, ifc4x3};

const GUID: &str = "0EI0MSHbX9gg8Fxwar7lb8";

fn invalid(err: &PropertyError) -> bool {
    matches!(err, PropertyError::AuthoringInvalid { .. })
}

/// The hard-coded arities match the shipped schemas.
///
/// These writers fill positional slots from constants. That is only safe
/// while the constants agree with the normative schema, so it is asked
/// directly rather than trusted.
#[test]
fn written_arities_match_the_schema() {
    const EXPECTED: &[(&str, usize)] = &[
        ("IfcDoorLiningProperties", 17),
        ("IfcWindowLiningProperties", 16),
        ("IfcDoorPanelProperties", 9),
        ("IfcWindowPanelProperties", 9),
        ("IfcPermeableCoveringProperties", 9),
        ("IfcComplexPropertyTemplate", 7),
        ("IfcPropertyDependencyRelationship", 5),
        ("IfcPropertyEnumeration", 3),
    ];
    for schema in [ifc4(), ifc4x3()] {
        for (entity, arity) in EXPECTED {
            let declared = schema.attributes(entity);
            if declared.is_empty() {
                continue;
            }
            assert_eq!(declared.len(), *arity, "{entity} in {}", schema.name());
        }
    }
}

/// WR31/WR32: a depth without its thickness describes a lining that
/// cannot be built, and both parse fine.
#[test]
fn a_door_depth_without_its_thickness_is_refused() {
    let model = Model::new();
    let mut tx = Transaction::new(&model);

    let err = add_door_lining_properties(
        &mut tx,
        GUID,
        DoorLiningDraft {
            lining_depth: Some(0.1),
            ..DoorLiningDraft::default()
        },
    )
    .expect_err("WR31");
    assert!(invalid(&err), "{err}");

    let err = add_door_lining_properties(
        &mut tx,
        GUID,
        DoorLiningDraft {
            threshold_depth: Some(0.1),
            ..DoorLiningDraft::default()
        },
    )
    .expect_err("WR32");
    assert!(invalid(&err), "{err}");

    add_door_lining_properties(
        &mut tx,
        GUID,
        DoorLiningDraft {
            lining_depth: Some(0.1),
            lining_thickness: Some(0.02),
            ..DoorLiningDraft::default()
        },
    )
    .expect("a depth with its thickness is legal");
}

/// WR33/WR34: the door transom and casing pairs are XOR, both or neither.
#[test]
fn door_transom_and_casing_pairs_are_all_or_nothing() {
    let model = Model::new();
    let mut tx = Transaction::new(&model);

    for draft in [
        DoorLiningDraft {
            transom_offset: Some(0.5),
            ..DoorLiningDraft::default()
        },
        DoorLiningDraft {
            transom_thickness: Some(0.02),
            ..DoorLiningDraft::default()
        },
        DoorLiningDraft {
            casing_depth: Some(0.1),
            ..DoorLiningDraft::default()
        },
        DoorLiningDraft {
            casing_thickness: Some(0.02),
            ..DoorLiningDraft::default()
        },
    ] {
        let err = add_door_lining_properties(&mut tx, GUID, draft).expect_err("half a pair");
        assert!(invalid(&err), "{err}");
    }

    add_door_lining_properties(
        &mut tx,
        GUID,
        DoorLiningDraft {
            transom_offset: Some(0.5),
            transom_thickness: Some(0.02),
            casing_depth: Some(0.1),
            casing_thickness: Some(0.02),
            ..DoorLiningDraft::default()
        },
    )
    .expect("complete pairs are legal");
}

/// WR32/WR33 on windows are ORDERED, not XOR.
///
/// The contrast with the door rules is the point: a first transom offset
/// alone is perfectly legal on a window, while a door's transom offset
/// alone is not. A writer that shared one rule between the two would
/// wrongly refuse half the legal window files.
#[test]
fn window_offsets_are_ordered_not_paired() {
    let model = Model::new();
    let mut tx = Transaction::new(&model);

    add_window_lining_properties(
        &mut tx,
        GUID,
        WindowLiningDraft {
            first_transom_offset: Some(0.25),
            ..WindowLiningDraft::default()
        },
    )
    .expect("a first offset alone is legal on a window");

    for draft in [
        WindowLiningDraft {
            second_transom_offset: Some(0.75),
            ..WindowLiningDraft::default()
        },
        WindowLiningDraft {
            second_mullion_offset: Some(0.75),
            ..WindowLiningDraft::default()
        },
    ] {
        let err = add_window_lining_properties(&mut tx, GUID, draft)
            .expect_err("a second offset without the first");
        assert!(invalid(&err), "{err}");
    }

    add_window_lining_properties(
        &mut tx,
        GUID,
        WindowLiningDraft {
            first_transom_offset: Some(0.25),
            second_transom_offset: Some(0.75),
            first_mullion_offset: Some(0.3),
            second_mullion_offset: Some(0.6),
            ..WindowLiningDraft::default()
        },
    )
    .expect("both offsets in order are legal");
}

/// Window offsets are normalised ratios, not lengths.
#[test]
fn window_offsets_are_bounded_ratios() {
    let model = Model::new();
    let mut tx = Transaction::new(&model);
    let err = add_window_lining_properties(
        &mut tx,
        GUID,
        WindowLiningDraft {
            first_transom_offset: Some(1.5),
            ..WindowLiningDraft::default()
        },
    )
    .expect_err("IfcNormalisedRatioMeasure is bounded to [0, 1]");
    assert!(invalid(&err), "{err}");
}

/// Offsets are plain lengths and may be negative; thicknesses may not.
///
/// Collapsing the three length measure kinds into one non-negative
/// check would refuse a legal negative lining offset.
#[test]
fn length_measures_keep_their_own_sign_rules() {
    let model = Model::new();
    let mut tx = Transaction::new(&model);

    add_door_lining_properties(
        &mut tx,
        GUID,
        DoorLiningDraft {
            lining_offset: Some(-0.01),
            threshold_offset: Some(-0.02),
            ..DoorLiningDraft::default()
        },
    )
    .expect("IfcLengthMeasure admits a negative offset");

    for draft in [
        DoorLiningDraft {
            lining_depth: Some(0.0),
            lining_thickness: Some(0.01),
            ..DoorLiningDraft::default()
        },
        DoorLiningDraft {
            lining_depth: Some(0.1),
            lining_thickness: Some(-0.01),
            ..DoorLiningDraft::default()
        },
    ] {
        let err = add_door_lining_properties(&mut tx, GUID, draft)
            .expect_err("a non-positive depth or negative thickness");
        assert!(invalid(&err), "{err}");
    }
}

/// Each panel enumeration is its own closed list.
///
/// The window operation enum has no USERDEFINED member while the door
/// and permeable-covering ones do, so a shared token list would accept
/// a window file the schema rejects.
#[test]
fn panel_enumerations_are_per_entity() {
    let mut model = Model::new();
    let mut tx = Transaction::new(&model);

    let id = add_door_panel_properties(
        &mut tx,
        GUID,
        Some("Leaf"),
        "SWINGING",
        "LEFT",
        (Some(0.04), Some(0.5)),
    )
    .expect("a door panel with legal tokens");
    tx.commit(&mut model).expect("commits");
    let entity = model.get(id).expect("staged");
    assert_eq!(entity.attributes[5], Value::Enum("SWINGING".into()));
    assert_eq!(entity.attributes[7], Value::Enum("LEFT".into()));

    let mut tx = Transaction::new(&model);
    let err = add_window_panel_properties(&mut tx, GUID, None, "USERDEFINED", "TOP", (None, None))
        .expect_err("IfcWindowPanelOperationEnum has no USERDEFINED");
    assert!(invalid(&err), "{err}");

    add_permeable_covering_properties(&mut tx, GUID, None, "USERDEFINED", "TOP", (None, None))
        .expect("the permeable covering enum does have USERDEFINED");

    let err = add_door_panel_properties(&mut tx, GUID, None, "SWINGING", "TOP", (None, None))
        .expect_err("TOP is a window position, not a door one");
    assert!(invalid(&err), "{err}");
}

/// PanelWidth is a fraction of the opening, not a length.
#[test]
fn panel_width_is_a_normalised_ratio() {
    let model = Model::new();
    let mut tx = Transaction::new(&model);
    let err =
        add_door_panel_properties(&mut tx, GUID, None, "SLIDING", "LEFT", (None, Some(900.0)))
            .expect_err("900 is a millimetre reading, not a ratio");
    assert!(invalid(&err), "{err}");
    add_door_panel_properties(&mut tx, GUID, None, "SLIDING", "LEFT", (None, Some(0.9)))
        .expect("a fraction is legal");
}

/// A malformed GlobalId is refused before anything is staged.
#[test]
fn a_malformed_guid_is_refused() {
    let model = Model::new();
    let mut tx = Transaction::new(&model);
    let err = add_door_lining_properties(&mut tx, "nope", DoorLiningDraft::default())
        .expect_err("four characters is not a GUID");
    assert!(invalid(&err), "{err}");
    assert!(tx.is_empty(), "nothing is staged when the GUID fails");
}

/// WR01: every enumeration value must share one measure type.
///
/// TYPEOF compares the declared measure, so a bare 2.0 and an
/// IFCLENGTHMEASURE(2.0) are different types even though both print as
/// a real. A mixed list makes the enumeration uninterpretable.
#[test]
fn enumeration_values_must_share_one_type() {
    let model = Model::new();
    let mut tx = Transaction::new(&model);

    let length = |v: f64| Value::Typed {
        type_name: "IFCLENGTHMEASURE".into(),
        value: Box::new(Value::Real(v)),
    };

    add_property_enumeration(&mut tx, "Widths", vec![length(0.9), length(1.2)], None)
        .expect("one measure throughout is legal");

    let err = add_property_enumeration(&mut tx, "Mixed", vec![length(0.9), Value::Real(1.2)], None)
        .expect_err("a wrapped and a bare value are different types");
    assert!(invalid(&err), "{err}");

    let err = add_property_enumeration(
        &mut tx,
        "Mixed",
        vec![Value::Text("a".into()), Value::Integer(1)],
        None,
    )
    .expect_err("text among integers");
    assert!(invalid(&err), "{err}");
}

/// The value list is UNIQUE and non-empty, and the name is the key.
#[test]
fn enumeration_values_are_unique_and_named() {
    let model = Model::new();
    let mut tx = Transaction::new(&model);

    let err = add_property_enumeration(&mut tx, "Sizes", Vec::new(), None)
        .expect_err("EnumerationValues is LIST [1:?]");
    assert!(invalid(&err), "{err}");

    let err = add_property_enumeration(&mut tx, "  ", vec![Value::Integer(1)], None)
        .expect_err("UR1 makes Name the key, so it cannot be blank");
    assert!(invalid(&err), "{err}");

    let err = add_property_enumeration(
        &mut tx,
        "Sizes",
        vec![Value::Integer(1), Value::Integer(1)],
        None,
    )
    .expect_err("the list is UNIQUE");
    assert!(invalid(&err), "{err}");

    add_property_enumeration(
        &mut tx,
        "Sizes",
        vec![Value::Integer(1), Value::Integer(2)],
        None,
    )
    .expect("distinct values of one type are legal");
}

/// NoSelfReference: a property cannot depend on itself.
#[test]
fn a_property_cannot_depend_on_itself() {
    let mut model = Model::new();
    let a = model.push(ifc_model::Entity::new("IFCPROPERTYSINGLEVALUE", vec![]));
    let b = model.push(ifc_model::Entity::new("IFCPROPERTYSINGLEVALUE", vec![]));
    let mut tx = Transaction::new(&model);

    let err = add_property_dependency_relationship(&mut tx, None, None, (a, a), None)
        .expect_err("a self-dependency loops any dependency walk");
    assert!(invalid(&err), "{err}");

    add_property_dependency_relationship(&mut tx, Some("derives"), None, (a, b), Some("x*2"))
        .expect("two distinct properties are legal");
}

/// A complex template refuses a repeated child template name.
#[test]
fn complex_template_children_are_uniquely_named() {
    let mut model = Model::new();
    let a = model.push(ifc_model::Entity::new("IFCSIMPLEPROPERTYTEMPLATE", vec![]));
    let b = model.push(ifc_model::Entity::new("IFCSIMPLEPROPERTYTEMPLATE", vec![]));
    let mut tx = Transaction::new(&model);

    let err = add_complex_property_template(
        &mut tx,
        GUID,
        Some("Assembly"),
        (None, None),
        &[("Width", a), ("Width", b)],
    )
    .expect_err("UniquePropertyTemplateNames");
    assert!(invalid(&err), "{err}");

    let err = add_complex_property_template(
        &mut tx,
        GUID,
        None,
        (None, Some("X_COMPLEX")),
        &[("Width", a)],
    )
    .expect_err("X_COMPLEX is not a member of the template type enum");
    assert!(invalid(&err), "{err}");

    add_complex_property_template(
        &mut tx,
        GUID,
        Some("Assembly"),
        (Some("Leaf"), Some("P_COMPLEX")),
        &[("Width", a), ("Height", b)],
    )
    .expect("distinct child names are legal");
}

/// The window has its own WR31, distinct from the door's.
///
/// Both entities state the rule separately, so each writer needs its
/// own guard; testing only the door leaves the window unprotected.
#[test]
fn a_window_lining_depth_needs_its_thickness() {
    let model = Model::new();
    let mut tx = Transaction::new(&model);
    let err = add_window_lining_properties(
        &mut tx,
        GUID,
        WindowLiningDraft {
            lining_depth: Some(0.1),
            ..WindowLiningDraft::default()
        },
    )
    .expect_err("WR31 on the window");
    assert!(invalid(&err), "{err}");

    add_window_lining_properties(
        &mut tx,
        GUID,
        WindowLiningDraft {
            lining_depth: Some(0.1),
            lining_thickness: Some(0.02),
            ..WindowLiningDraft::default()
        },
    )
    .expect("a depth with its thickness is legal");
}
