//! Value forms of predefined property set attributes (#149): enumerations,
//! entities, selects, EXPRESS simple types and aggregates, each checked
//! against the declared release; window sets; and the entity names
//! `exact_predefined_sets` accepts.

mod predefined_support;

use std::sync::Arc;

use ifc_model::EntityId;
use ifc_properties::{
    exact_predefined_sets, exact_property, ExactEntityRef, ExactPropertyError, ExactSource,
    ExactValue, SchemaVersion,
};
use predefined_support::*;

#[test]
fn window_lining_and_panel_sets_resolve_on_occurrence_and_type() {
    for (_, version) in RELEASES {
        let schema = table(version);
        let window_type = if version == SchemaVersion::Ifc2x3 {
            rooted(
                schema,
                2,
                "IfcWindowStyle",
                &[
                    ("HasPropertySets", "(#21)"),
                    ("ConstructionType", ".WOOD."),
                    ("OperationType", ".SINGLE_PANEL."),
                    ("ParameterTakesPrecedence", ".F."),
                    ("Sizeable", ".F."),
                ],
            )
        } else {
            rooted(
                schema,
                2,
                "IfcWindowType",
                &[
                    ("HasPropertySets", "(#21)"),
                    ("PredefinedType", ".WINDOW."),
                    ("PartitioningType", ".SINGLE_PANEL."),
                ],
            )
        };
        let records = vec![
            rooted(schema, 1, "IfcWindow", &[]),
            window_type,
            rooted(
                schema,
                3,
                "IfcRelDefinesByType",
                &[("RelatedObjects", "(#1)"), ("RelatingType", "#2")],
            ),
            rooted(
                schema,
                30,
                "IfcRelDefinesByProperties",
                &[
                    ("RelatedObjects", "(#1)"),
                    ("RelatingPropertyDefinition", "#20"),
                ],
            ),
            rooted(
                schema,
                20,
                "IfcWindowLiningProperties",
                &[("LiningDepth", "80."), ("FirstTransomOffset", "0.25")],
            ),
            rooted(
                schema,
                21,
                "IfcWindowPanelProperties",
                &[
                    ("OperationType", ".SIDEHUNGLEFTHAND."),
                    ("PanelPosition", ".LEFT."),
                    ("FrameDepth", "60."),
                ],
            ),
        ];
        let m = parse(version, &records);
        let window = EntityId(1);
        let depth = present(exact_property(&m, window, None, "LiningDepth"));
        assert_eq!(
            (depth.source, depth.value),
            (ExactSource::Occurrence, ExactValue::Real(80.0))
        );
        let transom = present(exact_property(&m, window, None, "FirstTransomOffset"));
        assert_eq!(
            transom.value_type.as_deref(),
            Some("IFCNORMALISEDRATIOMEASURE")
        );
        assert_eq!(transom.value, ExactValue::Real(0.25));
        let panels = exact_predefined_sets(&m, window, "IfcWindowPanelProperties").expect("lists");
        assert_eq!(panels.len(), 1);
        assert_eq!(panels[0].source, ExactSource::Type(EntityId(2)));
        let operation = panels[0].attribute("OperationType").expect("declared");
        assert_eq!(operation.value, enum_value("SIDEHUNGLEFTHAND"));
        assert_eq!(
            operation.value_type.as_deref(),
            Some("IFCWINDOWPANELOPERATIONENUM")
        );
        assert_eq!(
            panels[0].attribute("FrameDepth").map(|a| a.value.clone()),
            Some(ExactValue::Real(60.0))
        );
    }
}

#[test]
fn enumeration_constants_are_bound_to_the_release() {
    // `FIXEDPANEL` joined `IfcDoorPanelOperationEnum` in IFC4.
    for (_, version) in RELEASES {
        let sets = [panel(
            version,
            20,
            &[
                ("PanelOperation", ".FIXEDPANEL."),
                ("PanelPosition", ".LEFT."),
            ],
        )];
        let m = door_model(version, &[20], &[], &sets);
        let result = exact_property(&m, DOOR, None, "PanelOperation");
        match version {
            SchemaVersion::Ifc2x3 => assert_eq!(
                result,
                Err(ExactPropertyError::UnsupportedValue {
                    property: EntityId(20)
                })
            ),
            _ => assert_eq!(present(result).value, enum_value("FIXEDPANEL")),
        }
    }
    let version = SchemaVersion::Ifc4;
    let sets = [panel(
        version,
        20,
        &[("PanelOperation", ".HINGED."), ("PanelPosition", ".LEFT.")],
    )];
    let m = door_model(version, &[20], &[], &sets);
    assert_eq!(
        exact_property(&m, DOOR, None, "PanelOperation"),
        Err(ExactPropertyError::UnsupportedValue {
            property: EntityId(20)
        })
    );
}

#[test]
fn values_the_declared_type_does_not_accept_are_refused() {
    let version = SchemaVersion::Ifc4;
    let schema = table(version);
    let unsupported = Err(ExactPropertyError::UnsupportedValue {
        property: EntityId(10),
    });
    // A defined-type attribute is written bare, never typed; a length is a
    // real, not text or an integer.
    for depth in ["IFCPOSITIVELENGTHMEASURE(50.)", "'50'", "50", ".FIFTY."] {
        let m = door_model(
            version,
            &[10],
            &[],
            &[lining(version, 10, &[("LiningDepth", depth)])],
        );
        assert_eq!(
            exact_property(&m, DOOR, None, "LiningDepth"),
            unsupported,
            "{depth}"
        );
    }
    // An entity attribute resolves to a checked, unfollowed reference.
    let aspect = record(
        schema,
        50,
        "IfcShapeAspect",
        &[
            ("ShapeRepresentations", "(#1)"),
            ("ProductDefinitional", ".U."),
        ],
    );
    let with_aspect = |target: &str| {
        let set = lining(version, 10, &[("ShapeAspectStyle", target)]);
        door_model(version, &[10], &[], &[set, aspect.clone()])
    };
    let style = present(exact_property(
        &with_aspect("#50"),
        DOOR,
        None,
        "ShapeAspectStyle",
    ));
    assert_eq!(
        style.value,
        ExactValue::Entity(ExactEntityRef {
            id: EntityId(50),
            type_name: Arc::from("IFCSHAPEASPECT"),
        })
    );
    assert_eq!(style.value_type.as_deref(), Some("IFCSHAPEASPECT"));
    assert_eq!(
        exact_property(&with_aspect("#1"), DOOR, None, "ShapeAspectStyle"),
        unsupported
    );
    assert_eq!(
        exact_property(&with_aspect("#99"), DOOR, None, "ShapeAspectStyle"),
        Err(ExactPropertyError::MissingReference {
            from: EntityId(10),
            to: EntityId(99),
        })
    );
    assert_eq!(
        exact_property(&with_aspect("'x'"), DOOR, None, "ShapeAspectStyle"),
        unsupported
    );
    // A Name that is neither text nor `$` is malformed, not unnamed.
    let m = door_model(
        version,
        &[10],
        &[],
        &[lining(version, 10, &[("Name", "7")])],
    );
    assert_eq!(
        exact_property(&m, DOOR, None, "LiningDepth"),
        Err(ExactPropertyError::MalformedName {
            entity: EntityId(10),
            attribute: "Name",
        })
    );
}

#[test]
fn an_aggregate_attribute_refuses_only_when_selected() {
    let version = SchemaVersion::Ifc4;
    let set = rooted(
        table(version),
        10,
        "IfcReinforcementDefinitionProperties",
        &[
            ("DefinitionType", "'Main'"),
            ("ReinforcementSectionDefinitions", "(#51)"),
        ],
    );
    let m = door_model(version, &[10], &[], &[set]);
    let kind = present(exact_property(&m, DOOR, None, "DefinitionType"));
    assert_eq!(kind.value, ExactValue::Text("Main".into()));
    assert_eq!(kind.value_type.as_deref(), Some("IFCLABEL"));
    let refused = Err(ExactPropertyError::UnsupportedDefinition {
        entity: EntityId(10),
        type_name: "IFCREINFORCEMENTDEFINITIONPROPERTIES".into(),
    });
    assert_eq!(
        exact_property(&m, DOOR, None, "ReinforcementSectionDefinitions").map(|_| ()),
        refused.clone()
    );
    assert_eq!(
        exact_predefined_sets(&m, DOOR, "IfcReinforcementDefinitionProperties").map(|_| ()),
        refused
    );
}

#[test]
fn select_and_simple_typed_attributes_resolve_in_ifc2x3() {
    let version = SchemaVersion::Ifc2x3;
    let schema = table(version);
    let factor = |value: &str| {
        rooted(
            schema,
            10,
            "IfcServiceLifeFactor",
            &[
                ("PredefinedType", ".A_QUALITYOFCOMPONENTS."),
                ("MostUsedValue", value),
            ],
        )
    };
    // `MostUsedValue : IfcMeasureValue` keeps the member type written.
    let m = door_model(version, &[10], &[], &[factor("IFCRATIOMEASURE(0.9)")]);
    let used = present(exact_property(&m, DOOR, None, "MostUsedValue"));
    assert_eq!(used.value_type.as_deref(), Some("IFCRATIOMEASURE"));
    assert_eq!(used.value, ExactValue::Real(0.9));
    // `IfcLabel` is no `IfcMeasureValue` member.
    let m = door_model(version, &[10], &[], &[factor("IFCLABEL('x')")]);
    assert_eq!(
        exact_property(&m, DOOR, None, "MostUsedValue"),
        Err(ExactPropertyError::UnsupportedValue {
            property: EntityId(10)
        })
    );
    // `InputPhase : INTEGER` is an EXPRESS simple type.
    let electrical = rooted(
        schema,
        10,
        "IfcElectricalBaseProperties",
        &[
            ("InputVoltage", "230."),
            ("InputFrequency", "50."),
            ("InputPhase", "3"),
        ],
    );
    let m = door_model(version, &[10], &[], &[electrical]);
    let phase = present(exact_property(&m, DOOR, None, "InputPhase"));
    assert_eq!(phase.value_type.as_deref(), Some("INTEGER"));
    assert_eq!(phase.value, ExactValue::Integer(3));
}

#[test]
fn exact_predefined_sets_refuses_what_is_not_a_predefined_set() {
    let version = SchemaVersion::Ifc4;
    let m = door_model(version, &[], &[], &[]);
    for name in [
        "IfcPropertySet",
        "IfcElementQuantity",
        "IfcQuantitySet",
        "IfcPropertySetDefinition",
        "IfcDoor",
        "IfcNoSuchEntity",
        // An IFC2X3 predefined set IFC4 no longer declares.
        "IfcServiceLifeFactor",
    ] {
        assert_eq!(
            exact_predefined_sets(&m, DOOR, name),
            Err(ExactPropertyError::NotAPredefinedSet {
                name: name.into(),
                schema: version,
            }),
            "{name}"
        );
    }
    // A supertype lists every predefined set; spelling is case-insensitive.
    let sets = [
        lining(version, 10, &[("LiningDepth", "50.")]),
        leaf(version, 20, ".LEFT."),
    ];
    let m = door_model(version, &[10, 20], &[], &sets);
    let all = exact_predefined_sets(&m, DOOR, "IfcPreDefinedPropertySet").expect("lists");
    assert_eq!(all.len(), 2);
    assert_eq!(
        exact_predefined_sets(&m, DOOR, "IFCDOORPANELPROPERTIES").map(|sets| sets.len()),
        Ok(1)
    );
}
