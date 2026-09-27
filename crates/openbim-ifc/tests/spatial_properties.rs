//! Element properties grouped by spatial container (#121), over two storeys
//! in IFC2X3, IFC4 and IFC4X3.
//!
//! Every fixture is STEP text parsed by the strict codec. Records are laid
//! out from the declared release's own attribute table, so each carries that
//! release's arity without restating it here.
//!
//! The model, identical in the three releases:
//!
//! ```text
//! #1 project -> #2 building -> #3 Ground -> #5 Room (a space)
//!                           -> #4 Upper
//! #20 curtain wall  contained in #3, referenced in #4   Pset_Test
//! #25 stair         contained in #3                     no properties
//! #26 stair flight  part of #25                         Pset_Test
//! #23 proxy         contained in #5                     Pset_Test
//! #21 wall          contained in #4                     malformed set
//! #22 column        contained in #4                     no properties
//! ```

#![cfg(all(feature = "step", feature = "spatial", feature = "properties"))]

use ifc::{
    spatial_properties, Codec, ContainerElements, ContainerName, ElementProperties, EntityId,
    Model, SpatialKind, SpatialMembership, StepCodec,
};
use ifc_properties::{ExactPropertyError, ExactSource, SchemaVersion};

const RELEASES: [(SchemaVersion, &str); 3] = [
    (SchemaVersion::Ifc2x3, "IFC2X3"),
    (SchemaVersion::Ifc4, "IFC4"),
    (SchemaVersion::Ifc4x3, "IFC4X3_ADD2"),
];

/// `#id=ENTITY(...)` with `set` attributes by name, a GlobalId, and `$`
/// elsewhere, in `version`'s positional order.
fn record(version: SchemaVersion, id: u64, entity: &str, set: &[(&str, &str)]) -> String {
    let schema = ifc_schema::for_version(version).expect("bundled");
    let names = schema.attribute_names(entity);
    assert!(!names.is_empty(), "{entity} is declared in {version:?}");
    for (name, _) in set {
        assert!(names.contains(name), "{entity}.{name} in {version:?}");
    }
    let values: Vec<String> = names
        .iter()
        .map(|name| match set.iter().find(|(key, _)| key == name) {
            Some((_, value)) => (*value).to_owned(),
            None if *name == "GlobalId" => format!("'{id:0>22}'"),
            None => "$".to_owned(),
        })
        .collect();
    format!(
        "#{id}={}({});",
        entity.to_ascii_uppercase(),
        values.join(",")
    )
}

fn parse(token: &str, records: &[String]) -> Model {
    let text = format!(
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
         FILE_NAME('','',(''),(''),'','','');\nFILE_SCHEMA(('{token}'));\nENDSEC;\n\
         DATA;\n{}\nENDSEC;\nEND-ISO-10303-21;\n",
        records.join("\n")
    );
    let model = StepCodec
        .read_bytes(text.as_bytes())
        .unwrap_or_else(|e| panic!("fixture must parse: {e:?}"));
    assert!(model.diagnostics().is_empty(), "{:?}", model.diagnostics());
    model
}

fn two_storeys(version: SchemaVersion) -> Vec<String> {
    let r = |id, entity, set: &[(&str, &str)]| record(version, id, entity, set);
    let aggregates = |id, whole: &str, parts: &str| {
        r(
            id,
            "IfcRelAggregates",
            &[("RelatingObject", whole), ("RelatedObjects", parts)],
        )
    };
    let placed = |id, entity, structure: &str, elements: &str| {
        r(
            id,
            entity,
            &[
                ("RelatingStructure", structure),
                ("RelatedElements", elements),
            ],
        )
    };
    vec![
        r(1, "IfcProject", &[("Name", "'P'")]),
        r(2, "IfcBuilding", &[]),
        r(3, "IfcBuildingStorey", &[("Name", "'Ground'")]),
        r(4, "IfcBuildingStorey", &[("Name", "'Upper'")]),
        r(5, "IfcSpace", &[("Name", "'Room'")]),
        aggregates(10, "#1", "(#2)"),
        aggregates(11, "#2", "(#3,#4)"),
        aggregates(12, "#3", "(#5)"),
        r(20, "IfcCurtainWall", &[("Name", "'Curtain'")]),
        r(21, "IfcWall", &[]),
        r(22, "IfcColumn", &[]),
        r(23, "IfcBuildingElementProxy", &[]),
        r(25, "IfcStair", &[]),
        r(26, "IfcStairFlight", &[]),
        placed(30, "IfcRelContainedInSpatialStructure", "#3", "(#25,#20)"),
        placed(31, "IfcRelContainedInSpatialStructure", "#4", "(#22,#21)"),
        placed(32, "IfcRelContainedInSpatialStructure", "#5", "(#23)"),
        placed(33, "IfcRelReferencedInSpatialStructure", "#4", "(#20)"),
        aggregates(40, "#25", "(#26)"),
        "#50=IFCPROPERTYSINGLEVALUE('IsExternal',$,IFCBOOLEAN(.T.),$);".to_owned(),
        r(
            51,
            "IfcPropertySet",
            &[("Name", "'Pset_Test'"), ("HasProperties", "(#50)")],
        ),
        r(
            52,
            "IfcRelDefinesByProperties",
            &[
                ("RelatedObjects", "(#20,#23,#26)"),
                ("RelatingPropertyDefinition", "#51"),
            ],
        ),
        // `HasProperties` is a SET: naming #53 twice is malformed.
        "#53=IFCPROPERTYSINGLEVALUE('A',$,IFCLABEL('a'),$);".to_owned(),
        r(
            54,
            "IfcPropertySet",
            &[("Name", "'Pset_Broken'"), ("HasProperties", "(#53,#53)")],
        ),
        r(
            55,
            "IfcRelDefinesByProperties",
            &[
                ("RelatedObjects", "(#21)"),
                ("RelatingPropertyDefinition", "#54"),
            ],
        ),
    ]
}

/// `(element, membership)` per container, the listing's shape.
fn shape(container: &ContainerElements<'_>) -> Vec<(u64, SpatialMembership)> {
    container
        .members()
        .iter()
        .map(|member| (member.element.0, member.membership))
        .collect()
}

/// Property names of a resolved element, or the element's error.
fn names(element: &ElementProperties<'_>) -> Result<Vec<String>, ExactPropertyError> {
    element
        .properties
        .clone()
        .map(|entries| entries.iter().map(|e| e.name.to_string()).collect())
}

#[test]
fn containers_list_contained_part_and_referenced_elements_in_every_release() {
    use SpatialMembership::{Contained, Part, Referenced};
    for (version, token) in RELEASES {
        let model = parse(token, &two_storeys(version));
        let view = spatial_properties(&model).expect("a supported release");
        assert_eq!(view.schema(), version);

        // Tree order: the space nested in Ground precedes Upper.
        let order: Vec<u64> = view.containers().map(|c| c.container.id.0).collect();
        assert_eq!(order, [1, 2, 3, 5, 4], "{token}");

        let ground = view.container(EntityId(3)).expect("a container");
        assert_eq!(ground.container.kind, SpatialKind::Storey);
        assert_eq!(ground.container.type_name, "IFCBUILDINGSTOREY");
        assert_eq!(ground.container.name, ContainerName::Text("Ground"));
        assert_eq!(ground.container.parent, Some(EntityId(2)));
        assert_eq!(
            shape(&ground),
            [
                (20, Contained),
                (25, Contained),
                (
                    26,
                    Part {
                        whole: EntityId(25)
                    }
                )
            ],
            "{token}"
        );

        let room = view.container(EntityId(5)).expect("a container");
        assert_eq!(room.container.kind, SpatialKind::Space);
        assert_eq!(room.container.parent, Some(EntityId(3)));
        assert_eq!(shape(&room), [(23, Contained)], "{token}");

        let upper = view.container(EntityId(4)).expect("a container");
        assert_eq!(upper.container.name, ContainerName::Text("Upper"));
        assert_eq!(
            shape(&upper),
            [(20, Referenced), (21, Contained), (22, Contained)],
            "{token}"
        );

        let building = view.container(EntityId(2)).expect("a container");
        assert_eq!(building.container.name, ContainerName::Unset);
        assert!(building.members().is_empty(), "nothing is folded upwards");
    }
}

#[test]
fn a_malformed_element_is_reported_and_the_rest_still_resolve() {
    for (version, token) in RELEASES {
        let model = parse(token, &two_storeys(version));
        let view = spatial_properties(&model).expect("a supported release");

        let upper: Vec<_> = view
            .container(EntityId(4))
            .expect("a container")
            .elements()
            .collect();
        // The broken wall sits between two good elements: neither is lost.
        let [curtain, wall, column] = &upper[..] else {
            panic!("{token}: three elements, got {upper:?}");
        };
        assert_eq!(names(curtain), Ok(vec!["IsExternal".to_owned()]), "{token}");
        let entry = &curtain.properties.as_ref().expect("resolved")[0];
        assert_eq!(entry.property.property_set.as_ref(), "Pset_Test");
        assert_eq!(entry.property.source, ExactSource::Occurrence);
        assert_eq!(column.type_name, "IFCCOLUMN");
        assert_eq!(names(column), Ok(vec![]), "{token}: a proven absence");
        assert_eq!(
            wall.properties,
            Err(ExactPropertyError::DuplicateAggregateMember {
                entity: EntityId(54),
                attribute: "HasProperties",
                member: EntityId(53),
            }),
            "{token}"
        );

        // The same element answers identically under each container.
        let ground: Vec<_> = view
            .container(EntityId(3))
            .expect("a container")
            .elements()
            .collect();
        assert_eq!(ground[0].properties, curtain.properties, "{token}");
        assert_eq!(names(&ground[1]), Ok(vec![]), "{token}: the stair");
        assert_eq!(
            names(&ground[2]),
            Ok(vec!["IsExternal".to_owned()]),
            "{token}: the part's own set"
        );
        let room: Vec<_> = view
            .container(EntityId(5))
            .expect("a container")
            .elements()
            .collect();
        assert_eq!(
            names(&room[0]),
            Ok(vec!["IsExternal".to_owned()]),
            "{token}"
        );
    }
}

#[test]
fn selectors_narrow_every_element_alike() {
    let model = parse("IFC4", &two_storeys(SchemaVersion::Ifc4));
    let view = spatial_properties(&model).expect("IFC4");
    let upper = view.container(EntityId(4)).expect("a container");
    let answers: Vec<_> = upper
        .elements_where(|set| set == "Pset_Test", |_| true)
        .map(|element| names(&element))
        .collect();
    assert_eq!(answers[0], Ok(vec!["IsExternal".to_owned()]));
    // The broken set is not selected, so it is skipped unread, as
    // `exact_properties_where` documents: the wall proves an absence.
    assert_eq!(answers[1], Ok(vec![]));
    assert_eq!(answers[2], Ok(vec![]));
    assert_eq!(answers.len(), 3);
    let none: Vec<_> = upper
        .elements_where(|_| true, |_| false)
        .map(|element| names(&element))
        .collect();
    assert_eq!(none[0], Ok(vec![]), "nothing selected is a proven absence");
}

#[test]
fn a_model_level_refusal_is_returned_once() {
    let mut records = two_storeys(SchemaVersion::Ifc4);
    records.truncate(5);
    let model = parse("IFC5", &records);
    assert_eq!(
        spatial_properties(&model).err(),
        Some(ExactPropertyError::UnsupportedSchema {
            schema: "IFC5".to_owned()
        })
    );
}

#[test]
fn containers_in_an_aggregation_cycle_are_still_listed() {
    let version = SchemaVersion::Ifc4;
    let r = |id, entity, set: &[(&str, &str)]| record(version, id, entity, set);
    let model = parse(
        "IFC4",
        &[
            r(3, "IfcBuildingStorey", &[]),
            r(4, "IfcBuildingStorey", &[]),
            r(
                10,
                "IfcRelAggregates",
                &[("RelatingObject", "#3"), ("RelatedObjects", "(#4)")],
            ),
            r(
                11,
                "IfcRelAggregates",
                &[("RelatingObject", "#4"), ("RelatedObjects", "(#3)")],
            ),
        ],
    );
    let view = spatial_properties(&model).expect("IFC4");
    assert!(view.tree().roots().is_empty(), "no root reaches either");
    let order: Vec<u64> = view.containers().map(|c| c.container.id.0).collect();
    assert_eq!(order, [3, 4]);
}

#[test]
fn a_dangling_part_is_reported_not_listed() {
    let mut records = two_storeys(SchemaVersion::Ifc4);
    records.push(record(
        SchemaVersion::Ifc4,
        41,
        "IfcRelAggregates",
        &[("RelatingObject", "#25"), ("RelatedObjects", "(#99)")],
    ));
    let model = parse("IFC4", &records);
    let view = spatial_properties(&model).expect("IFC4");
    assert_eq!(view.dangling_parts(), [(EntityId(41), EntityId(99))]);
    let ground = view.container(EntityId(3)).expect("a container");
    assert_eq!(ground.members().len(), 3);
}
