//! Exact resolution on a queried type object (#193).
//!
//! IDS applies facets to type objects (`IfcWallType`), so the exact API
//! answers for an `IfcTypeObject` from its own `HasPropertySets`, which every
//! release declares `OPTIONAL SET [1:?] OF IfcPropertySetDefinition`
//! (`references/ifc-spec/*/*.exp`). The sets are read by the code that reads
//! an occurrence's inherited type sets, so a set gives the same answer
//! whichever of the two is queried, with `ExactSource::Type` of the type.
//! Records are built by attribute name from each release's bundled table.

mod predefined_support;

use ifc_model::{EntityId, Model};
use ifc_properties::{
    exact_predefined_sets, exact_properties, exact_properties_where, exact_property,
    exact_property_sets_where, ExactPropertyEntry, ExactPropertyError, ExactResolution,
    ExactSource, ExactValue, SchemaVersion,
};
use predefined_support::{
    door_model, leaf, lining, parse, present, record, rooted, table, DOOR, DOOR_TYPE, RELEASES,
};

const WALL: EntityId = EntityId(1);
const WALL_TYPE: EntityId = EntityId(2);
const TYPE: ExactSource = ExactSource::Type(WALL_TYPE);

fn single(version: SchemaVersion, id: u64, name: &str, value: &str) -> String {
    let name = format!("'{name}'");
    record(
        table(version),
        id,
        "IfcPropertySingleValue",
        &[("Name", &name), ("NominalValue", value)],
    )
}

fn refs(ids: &[u64]) -> String {
    let list: Vec<String> = ids.iter().map(|id| format!("#{id}")).collect();
    format!("({})", list.join(","))
}

fn pset(version: SchemaVersion, id: u64, name: &str, members: &[u64]) -> String {
    let (name, members) = (format!("'{name}'"), refs(members));
    rooted(
        table(version),
        id,
        "IfcPropertySet",
        &[("Name", &name), ("HasProperties", &members)],
    )
}

fn defines(version: SchemaVersion, id: u64, objects: &[u64], set: u64) -> String {
    let (objects, set) = (refs(objects), format!("#{set}"));
    rooted(
        table(version),
        id,
        "IfcRelDefinesByProperties",
        &[
            ("RelatedObjects", &objects),
            ("RelatingPropertyDefinition", &set),
        ],
    )
}

/// A wall #1 with `Pset_WallCommon` #31 (`FireRating` F30) of its own, typed
/// by wall type #2 whose `HasPropertySets` is `type_sets`. Available sets:
/// #20 `Pset_WallCommon` (`FireRating` F90, `LoadBearing`) and #25
/// `Qto_WallBaseQuantities` (`Length`); `extra` adds more.
fn walls(version: SchemaVersion, type_sets: &str, extra: &[String]) -> Model {
    let schema = table(version);
    let mut records = vec![
        rooted(schema, 1, "IfcWall", &[("Name", "'Wall'")]),
        rooted(
            schema,
            2,
            "IfcWallType",
            &[
                ("Name", "'WT'"),
                ("HasPropertySets", type_sets),
                ("PredefinedType", ".STANDARD."),
            ],
        ),
        rooted(
            schema,
            3,
            "IfcRelDefinesByType",
            &[("RelatedObjects", "(#1)"), ("RelatingType", "#2")],
        ),
        single(version, 10, "FireRating", "IFCLABEL('F90')"),
        single(version, 11, "LoadBearing", "IFCBOOLEAN(.T.)"),
        pset(version, 20, "Pset_WallCommon", &[10, 11]),
        record(
            schema,
            12,
            "IfcQuantityLength",
            &[("Name", "'Length'"), ("LengthValue", "4.5")],
        ),
        rooted(
            schema,
            25,
            "IfcElementQuantity",
            &[
                ("Name", "'Qto_WallBaseQuantities'"),
                ("Quantities", "(#12)"),
            ],
        ),
        single(version, 30, "FireRating", "IFCLABEL('F30')"),
        pset(version, 31, "Pset_WallCommon", &[30]),
        defines(version, 32, &[1], 31),
    ];
    records.extend_from_slice(extra);
    parse(version, &records)
}

/// `(set, name, source, property id)` of each entry, in result order.
fn summary(entries: &[ExactPropertyEntry]) -> Vec<(&str, &str, ExactSource, EntityId)> {
    entries
        .iter()
        .map(|entry| {
            let property = &entry.property;
            let set = property.property_set.as_ref();
            (
                set,
                entry.name.as_ref(),
                property.source,
                property.property_id,
            )
        })
        .collect()
}

#[test]
fn a_wall_types_own_sets_are_present_with_type_provenance() {
    for (_, version) in RELEASES {
        let m = walls(version, "(#20,#25)", &[]);
        let fire = present(exact_property(
            &m,
            WALL_TYPE,
            Some("Pset_WallCommon"),
            "FireRating",
        ));
        assert_eq!(fire.source, TYPE, "{version:?}");
        assert_eq!(
            (fire.set_id, fire.property_id),
            (EntityId(20), EntityId(10))
        );
        assert_eq!(fire.value, ExactValue::Text("F90".into()));
        let length = present(exact_property(&m, WALL_TYPE, None, "Length"));
        assert_eq!((length.source, length.set_id), (TYPE, EntityId(25)));
        assert_eq!(length.value, ExactValue::Real(4.5));
        assert_eq!(length.value_type.as_deref(), Some("IFCLENGTHMEASURE"));
        assert_eq!(
            summary(&exact_properties(&m, WALL_TYPE).expect("resolves")),
            [
                ("Pset_WallCommon", "FireRating", TYPE, EntityId(10)),
                ("Pset_WallCommon", "LoadBearing", TYPE, EntityId(11)),
                ("Qto_WallBaseQuantities", "Length", TYPE, EntityId(12)),
            ],
            "{version:?}"
        );
    }
}

#[test]
fn a_type_without_the_property_or_any_set_is_an_exact_absence() {
    for (_, version) in RELEASES {
        let m = walls(version, "(#20,#25)", &[]);
        assert_eq!(
            exact_property(&m, WALL_TYPE, Some("Pset_WallCommon"), "AcousticRating"),
            Ok(ExactResolution::Absent)
        );
        // The occurrence's own set is not the type's.
        assert_eq!(
            exact_properties_where(&m, WALL_TYPE, |s| s == "Pset_Other", |_| true),
            Ok(Vec::new())
        );
        // `$` states no sets: every function proves an absence.
        let bare = walls(version, "$", &[]);
        assert_eq!(
            exact_property(&bare, WALL_TYPE, None, "FireRating"),
            Ok(ExactResolution::Absent),
            "{version:?}"
        );
        assert_eq!(exact_properties(&bare, WALL_TYPE), Ok(Vec::new()));
        assert_eq!(
            exact_property_sets_where(&bare, WALL_TYPE, |_| true),
            Ok(Vec::new())
        );
        assert_eq!(
            exact_predefined_sets(&bare, WALL_TYPE, "IfcDoorLiningProperties"),
            Ok(Vec::new())
        );
        // Present but empty breaks `SET [1:?]`: refused, as for an
        // inherited type.
        let empty = walls(version, "()", &[]);
        let malformed = Err(ExactPropertyError::MalformedAggregate {
            entity: WALL_TYPE,
            attribute: "HasPropertySets",
        });
        assert_eq!(
            exact_property(&empty, WALL_TYPE, None, "FireRating"),
            malformed
        );
        assert_eq!(exact_property(&empty, WALL, None, "FireRating"), malformed);
    }
}

#[test]
fn duplicate_names_on_a_type_are_refused_as_on_an_occurrence() {
    for (_, version) in RELEASES {
        let extra = [
            single(version, 41, "FireRating", "IFCLABEL('F60')"),
            pset(version, 40, "Pset_WallCommon", &[41]),
            pset(version, 42, "Pset_Doubled", &[10, 41]),
        ];
        let on_type = walls(version, "(#20,#40,#42)", &extra);
        assert_eq!(
            exact_property(&on_type, WALL_TYPE, Some("Pset_WallCommon"), "FireRating"),
            Err(ExactPropertyError::DuplicateMatchingSets {
                source: TYPE,
                first: EntityId(20),
                second: EntityId(40),
            }),
            "{version:?}"
        );
        assert_eq!(
            exact_property(&on_type, WALL_TYPE, Some("Pset_Doubled"), "FireRating"),
            Err(ExactPropertyError::DuplicateMatchingProperties {
                set: EntityId(42),
                first: EntityId(10),
                second: EntityId(41),
            })
        );
        assert_eq!(
            exact_property_sets_where(&on_type, WALL_TYPE, |s| s == "Pset_WallCommon"),
            Err(ExactPropertyError::DuplicateMatchingSets {
                source: TYPE,
                first: EntityId(20),
                second: EntityId(40),
            })
        );
        // The same two sets on an occurrence: the same refusal, by source.
        let mut direct = extra.to_vec();
        direct.push(defines(version, 43, &[1], 20));
        direct.push(defines(version, 44, &[1], 40));
        let on_occurrence = walls(version, "$", &direct);
        assert_eq!(
            exact_property(&on_occurrence, WALL, Some("Pset_WallCommon"), "FireRating"),
            Err(ExactPropertyError::DuplicateMatchingSets {
                source: ExactSource::Occurrence,
                first: EntityId(31),
                second: EntityId(20),
            })
        );
    }
}

#[test]
fn an_occurrence_query_is_unchanged_and_agrees_with_its_type() {
    for (_, version) in RELEASES {
        let m = walls(version, "(#20,#25)", &[]);
        let occurrence = exact_properties(&m, WALL).expect("resolves");
        assert_eq!(
            summary(&occurrence),
            [
                (
                    "Pset_WallCommon",
                    "FireRating",
                    ExactSource::Occurrence,
                    EntityId(30)
                ),
                ("Pset_WallCommon", "LoadBearing", TYPE, EntityId(11)),
                ("Qto_WallBaseQuantities", "Length", TYPE, EntityId(12)),
            ],
            "{version:?}"
        );
        let fire = present(exact_property(&m, WALL, None, "FireRating"));
        assert_eq!(
            (fire.source, fire.value),
            (ExactSource::Occurrence, ExactValue::Text("F30".into()))
        );
        // Every inherited entry is the type's own entry, field for field.
        let own = exact_properties(&m, WALL_TYPE).expect("resolves");
        for inherited in occurrence.iter().filter(|e| e.property.source == TYPE) {
            assert!(own.contains(inherited), "{version:?}: {inherited:?}");
        }
    }
}

#[test]
fn a_types_sets_are_listed_empty_ones_included() {
    for (_, version) in RELEASES {
        let empty = rooted(
            table(version),
            50,
            "IfcPropertySet",
            &[("Name", "'Pset_Empty'"), ("HasProperties", "()")],
        );
        let m = walls(version, "(#20,#25,#50)", &[empty]);
        let listed = exact_property_sets_where(&m, WALL_TYPE, |_| true).expect("lists");
        let listed: Vec<_> = listed
            .iter()
            .map(|set| (set.name.as_ref(), set.set_id, set.source, set.members))
            .collect();
        assert_eq!(
            listed,
            [
                ("Pset_WallCommon", EntityId(20), TYPE, 2),
                ("Qto_WallBaseQuantities", EntityId(25), TYPE, 1),
                ("Pset_Empty", EntityId(50), TYPE, 0),
            ],
            "{version:?}"
        );
        // The occurrence lists its own set first, then the same type sets.
        let from_wall = exact_property_sets_where(&m, WALL, |_| true).expect("lists");
        let sources: Vec<_> = from_wall.iter().map(|s| (s.set_id, s.source)).collect();
        assert_eq!(
            sources,
            [
                (EntityId(31), ExactSource::Occurrence),
                (EntityId(20), TYPE),
                (EntityId(25), TYPE),
                (EntityId(50), TYPE),
            ]
        );
    }
}

#[test]
fn a_door_types_predefined_sets_resolve_with_type_provenance() {
    for (_, version) in RELEASES {
        let sets = [
            lining(version, 40, &[("LiningDepth", "50.")]),
            leaf(version, 41, ".LEFT."),
        ];
        // IFC2X3 types the door with `IfcDoorStyle`, IFC4 and IFC4X3 with
        // `IfcDoorType`.
        let m = door_model(version, &[], &[40, 41], &sets);
        let own = exact_predefined_sets(&m, DOOR_TYPE, "IfcDoorPanelProperties").expect("lists");
        assert_eq!(own.len(), 1, "{version:?}");
        assert_eq!(
            (own[0].source, own[0].set_id),
            (ExactSource::Type(DOOR_TYPE), EntityId(41))
        );
        assert_eq!(
            own[0].attribute("PanelOperation").map(|p| &p.value),
            Some(&ExactValue::Enum("SWINGING".into()))
        );
        // The door inherits exactly the type's answer.
        assert_eq!(
            exact_predefined_sets(&m, DOOR, "IfcDoorPanelProperties"),
            Ok(own)
        );
        let depth = present(exact_property(&m, DOOR_TYPE, None, "LiningDepth"));
        assert_eq!(
            (depth.source, depth.set_id, depth.value),
            (
                ExactSource::Type(DOOR_TYPE),
                EntityId(40),
                ExactValue::Real(50.0)
            )
        );
        let listed = exact_property_sets_where(&m, DOOR_TYPE, |_| true).expect("lists");
        let names: Vec<_> = listed.iter().map(|s| s.name.as_ref()).collect();
        assert_eq!(names, ["IfcDoorLiningProperties", "IfcDoorPanelProperties"]);
    }
}

#[test]
fn every_type_object_of_the_release_is_accepted_by_its_table() {
    for (_, version) in RELEASES {
        let schema = table(version);
        let mut types = schema.subtypes("IfcTypeObject");
        types.push("IfcTypeObject");
        if version == SchemaVersion::Ifc2x3 {
            assert!(types.contains(&"IfcDoorStyle") && types.contains(&"IfcWindowStyle"));
        }
        let records: Vec<String> = types
            .iter()
            .enumerate()
            .map(|(index, name)| rooted(schema, 100 + index as u64, name, &[("Name", "'T'")]))
            .collect();
        let m = parse(version, &records);
        for (index, name) in types.iter().enumerate() {
            let id = EntityId(100 + index as u64);
            assert_eq!(
                exact_property(&m, id, None, "FireRating"),
                Ok(ExactResolution::Absent),
                "{version:?} {name}"
            );
        }
    }
}

#[test]
fn a_type_in_a_property_relationship_is_refused_not_ignored() {
    // IFC2X3 admits only `IfcObject` there; IFC4 and IFC4X3 forbid a type
    // object by `NoRelatedTypeObject`, whose documentation puts its sets in
    // `HasPropertySets`. A type with both is refused, never answered from one.
    for (_, version) in RELEASES {
        let m = walls(version, "(#20)", &[defines(version, 45, &[2], 25)]);
        let refused = Some(ExactPropertyError::InvalidOccurrenceTarget {
            relationship: EntityId(45),
            object: WALL_TYPE,
        });
        assert_eq!(
            exact_property(&m, WALL_TYPE, None, "Length").err(),
            refused,
            "{version:?}"
        );
        assert_eq!(exact_properties(&m, WALL_TYPE).err(), refused);
        assert_eq!(
            exact_property_sets_where(&m, WALL_TYPE, |_| true).err(),
            refused
        );
        assert_eq!(
            exact_predefined_sets(&m, WALL_TYPE, "IfcDoorLiningProperties").err(),
            refused
        );
        // What is neither an object nor a type is still refused as a query.
        assert!(matches!(
            exact_properties(&walls(version, "(#20)", &[]), EntityId(20)),
            Err(ExactPropertyError::InvalidQueryObject {
                object: EntityId(20),
                ..
            })
        ));
    }
}
