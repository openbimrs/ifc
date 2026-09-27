//! Exact attributes of predefined property sets (#149).
//!
//! Door lining and panel sets on occurrences and on types, in
//! every release that declares them (all three). Records are built by
//! attribute name from each release's bundled table, so each carries that
//! release's exact layout: IFC2X3 types doors with `IfcDoorStyle`, IFC4 and
//! IFC4X3 with `IfcDoorType`; IFC4 adds `LiningToPanelOffsetX` and retypes
//! `LiningThickness` from `IfcPositiveLengthMeasure` to
//! `IfcNonNegativeLengthMeasure` (`references/ifc-spec/*/*.exp`).

mod predefined_support;

use ifc_model::EntityId;
use ifc_properties::{
    exact_predefined_sets, exact_properties, exact_properties_where, exact_property, exact_unit,
    ExactPropertyError, ExactResolution, ExactSource, ExactValue, SchemaVersion,
};
use predefined_support::*;

#[test]
fn door_lining_on_the_occurrence_and_panels_on_the_type_resolve() {
    for (_, version) in RELEASES {
        let sets = [
            lining(
                version,
                10,
                &[("LiningDepth", "50."), ("LiningThickness", "20.")],
            ),
            leaf(version, 20, ".LEFT."),
            leaf(version, 21, ".RIGHT."),
        ];
        let m = door_model(version, &[10], &[20, 21], &sets);

        let depth = present(exact_property(&m, DOOR, None, "LiningDepth"));
        assert_eq!(depth.source, ExactSource::Occurrence);
        assert_eq!(depth.property_set.as_ref(), "IfcDoorLiningProperties");
        assert_eq!(
            (depth.set_id, depth.property_id),
            (EntityId(10), EntityId(10))
        );
        assert_eq!(
            depth.value_type.as_deref(),
            Some("IFCPOSITIVELENGTHMEASURE")
        );
        assert_eq!((depth.unit_id, depth.value), (None, ExactValue::Real(50.0)));
        // The release's own declaration types each attribute.
        let thickness = present(exact_property(&m, DOOR, None, "LiningThickness"));
        let expected = match version {
            SchemaVersion::Ifc2x3 => "IFCPOSITIVELENGTHMEASURE",
            _ => "IFCNONNEGATIVELENGTHMEASURE",
        };
        assert_eq!(
            thickness.value_type.as_deref(),
            Some(expected),
            "{version:?}"
        );
        let offset = exact_property(&m, DOOR, None, "LiningToPanelOffsetX");
        match version {
            SchemaVersion::Ifc2x3 => assert_eq!(offset, Ok(ExactResolution::Absent)),
            _ => assert_eq!(present(offset).value, ExactValue::Null),
        }

        // One panel set per leaf: ambiguous by name, listed by entity.
        assert_eq!(
            exact_property(&m, DOOR, None, "PanelOperation"),
            Err(ExactPropertyError::DuplicateMatchingSets {
                source: ExactSource::Type(DOOR_TYPE),
                first: EntityId(20),
                second: EntityId(21),
            })
        );
        // Two unnamed panel sets are no duplicate set: a name neither holds
        // is still proven absent.
        assert_eq!(
            exact_property(&m, DOOR, None, "FireRating"),
            Ok(ExactResolution::Absent)
        );
        let panels = exact_predefined_sets(&m, DOOR, "IfcDoorPanelProperties").expect("lists");
        assert_eq!(panels.len(), 2, "{version:?}");
        for (panel, position) in panels.iter().zip(["LEFT", "RIGHT"]) {
            assert_eq!(panel.source, ExactSource::Type(DOOR_TYPE));
            assert_eq!(panel.entity.as_ref(), "IfcDoorPanelProperties");
            assert_eq!(panel.name, None);
            let names: Vec<&str> = panel.attributes.iter().map(|a| a.name.as_ref()).collect();
            assert_eq!(
                names,
                [
                    "PanelDepth",
                    "PanelOperation",
                    "PanelWidth",
                    "PanelPosition",
                    "ShapeAspectStyle"
                ]
            );
            let get = |name| panel.attribute(name).expect(name);
            assert_eq!(get("PanelOperation").value, enum_value("SWINGING"));
            assert_eq!(
                get("PanelOperation").value_type.as_deref(),
                Some("IFCDOORPANELOPERATIONENUM")
            );
            assert_eq!(get("PanelPosition").value, enum_value(position));
            assert_eq!(get("PanelWidth").value, ExactValue::Real(0.5));
            assert_eq!(
                get("PanelWidth").value_type.as_deref(),
                Some("IFCNORMALISEDRATIOMEASURE")
            );
            assert_eq!(get("ShapeAspectStyle").value, ExactValue::Null);
            assert!(panel.attribute("NoSuchAttribute").is_none());
        }
        let linings = exact_predefined_sets(&m, DOOR, "IfcDoorLiningProperties").expect("lists");
        assert_eq!(linings.len(), 1);
        assert_eq!(linings[0].source, ExactSource::Occurrence);
    }
}

#[test]
fn door_lining_on_the_type_and_a_panel_on_the_occurrence_resolve() {
    for (_, version) in RELEASES {
        let sets = [
            lining(version, 10, &[("Name", "'Lining'"), ("LiningDepth", "60.")]),
            leaf(version, 20, ".MIDDLE."),
        ];
        let m = door_model(version, &[20], &[10], &sets);
        let depth = present(exact_property(&m, DOOR, Some("Lining"), "LiningDepth"));
        assert_eq!(depth.source, ExactSource::Type(DOOR_TYPE));
        assert_eq!(depth.property_set.as_ref(), "Lining");
        assert_eq!(depth.value, ExactValue::Real(60.0));
        let operation = present(exact_property(
            &m,
            DOOR,
            Some("IfcDoorPanelProperties"),
            "PanelOperation",
        ));
        assert_eq!(operation.source, ExactSource::Occurrence);
        assert_eq!(operation.value, enum_value("SWINGING"));
        let linings = exact_predefined_sets(&m, DOOR, "IfcDoorLiningProperties").expect("lists");
        assert_eq!(linings[0].name.as_deref(), Some("Lining"));
        assert_eq!(linings[0].source, ExactSource::Type(DOOR_TYPE));
    }
}

#[test]
fn both_sources_are_listed_without_an_override() {
    let version = SchemaVersion::Ifc4;
    let sets = [
        lining(version, 10, &[("LiningDepth", "50.")]),
        lining(version, 11, &[("LiningDepth", "70.")]),
    ];
    let m = door_model(version, &[10], &[11], &sets);
    // By name, the occurrence value overrides the inherited one.
    let depth = present(exact_property(&m, DOOR, None, "LiningDepth"));
    assert_eq!(
        (depth.source, depth.value),
        (ExactSource::Occurrence, ExactValue::Real(50.0))
    );
    // Listed by entity, both are reported with their sources.
    let linings = exact_predefined_sets(&m, DOOR, "IfcDoorLiningProperties").expect("lists");
    let sources: Vec<_> = linings.iter().map(|set| (set.source, set.set_id)).collect();
    assert_eq!(
        sources,
        [
            (ExactSource::Occurrence, EntityId(10)),
            (ExactSource::Type(DOOR_TYPE), EntityId(11)),
        ]
    );
}

#[test]
fn a_missing_optional_attribute_is_an_exact_absence_of_its_value() {
    for (_, version) in RELEASES {
        let sets = [lining(version, 10, &[("LiningDepth", "50.")])];
        let m = door_model(version, &[10], &[], &sets);
        let threshold = present(exact_property(&m, DOOR, None, "ThresholdDepth"));
        assert_eq!(threshold.value, ExactValue::Null);
        assert_eq!(
            threshold.value_type.as_deref(),
            Some("IFCPOSITIVELENGTHMEASURE")
        );
        // An attribute no release declares for the set is absent, not `$`.
        assert_eq!(
            exact_property(&m, DOOR, None, "PanelWidth"),
            Ok(ExactResolution::Absent)
        );
        // No panel set at all: a proven empty list.
        assert_eq!(
            exact_predefined_sets(&m, DOOR, "IfcDoorPanelProperties"),
            Ok(Vec::new())
        );
    }
}

#[test]
fn a_required_attribute_left_unset_is_refused() {
    for (_, version) in RELEASES {
        let sets = [panel(version, 20, &[("PanelPosition", ".LEFT.")])];
        let m = door_model(version, &[20], &[], &sets);
        let missing = Err(ExactPropertyError::MissingValueSlot {
            property: EntityId(20),
        });
        assert_eq!(
            exact_property(&m, DOOR, None, "PanelOperation").map(|_| ()),
            missing
        );
        assert_eq!(
            exact_predefined_sets(&m, DOOR, "IfcDoorPanelProperties").map(|_| ()),
            missing
        );
        // Another, well-formed attribute of the set still resolves by name.
        let position = present(exact_property(&m, DOOR, None, "PanelPosition"));
        assert_eq!(position.value, enum_value("LEFT"));
    }
}

#[test]
fn a_length_is_scaled_by_the_project_unit_and_a_ratio_is_not() {
    for (_, version) in RELEASES {
        let sets = [
            lining(version, 10, &[("LiningDepth", "50.")]),
            leaf(version, 20, ".LEFT."),
        ];
        let m = door_model(version, &[10, 20], &[], &sets);
        let depth = present(exact_property(&m, DOOR, None, "LiningDepth"));
        let unit = exact_unit(&m, depth.value_type.as_deref().unwrap(), depth.unit_id)
            .expect("the project length unit");
        assert!(unit.from_project);
        assert_eq!(unit.unit, Some(EntityId(7)));
        let ExactValue::Real(value) = depth.value else {
            panic!("{:?}", depth.value)
        };
        assert!((value * unit.scale - 0.05).abs() < 1e-12, "{version:?}");
        let width = present(exact_property(&m, DOOR, None, "PanelWidth"));
        let ratio = exact_unit(&m, width.value_type.as_deref().unwrap(), None).expect("ratio");
        assert_eq!((ratio.unit, ratio.scale), (None, 1.0));
    }
}

#[test]
fn enumeration_and_lookup_agree_on_predefined_attributes() {
    let mut compared = 0;
    for (_, version) in RELEASES {
        let named = [
            lining(version, 10, &[("Name", "'Lining'"), ("LiningDepth", "50.")]),
            leaf(version, 20, ".LEFT."),
        ];
        let two_leaves = [leaf(version, 20, ".LEFT."), leaf(version, 21, ".RIGHT.")];
        let unnamed = [lining(version, 10, &[("LiningDepth", "50.")])];
        let models = [
            door_model(version, &[10], &[20], &named),
            door_model(version, &[20, 21], &[], &two_leaves),
            door_model(version, &[], &[10], &unnamed),
        ];
        let schema = table(version);
        let mut names: Vec<&str> = schema.attribute_names("IfcDoorLiningProperties");
        names.extend(schema.attribute_names("IfcDoorPanelProperties"));
        names.push("NoSuchAttribute");
        let sets = [
            "Lining",
            "IfcDoorLiningProperties",
            "IfcDoorPanelProperties",
            "Pset_Other",
        ];
        for m in &models {
            for set in sets {
                for name in &names {
                    let single = exact_property(m, DOOR, Some(set), name);
                    let listed = exact_properties_where(m, DOOR, |s| s == set, |p| p == *name);
                    match (single, listed) {
                        (Ok(ExactResolution::Present(one)), Ok(entries)) => {
                            assert_eq!(entries.len(), 1, "{set}.{name}");
                            assert_eq!(entries[0].property, one, "{set}.{name}");
                        }
                        (Ok(ExactResolution::Absent), Ok(entries)) => {
                            assert!(entries.is_empty(), "{set}.{name}: {entries:?}");
                        }
                        (Err(single), Err(listed)) => {
                            assert_eq!(single, listed, "{set}.{name}");
                        }
                        (single, listed) => {
                            panic!("{version:?} {set}.{name}: {single:?} but {listed:?}")
                        }
                    }
                    compared += 1;
                }
            }
        }
        // Every listed entry is found by its own set and name.
        let entries = exact_properties(&models[0], DOOR).expect("resolves");
        assert!(entries.len() > 10, "{version:?}");
    }
    assert!(compared > 500, "only {compared} comparisons ran");
}
