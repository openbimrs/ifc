//! Spatial containers come from the declared release's schema table, not
//! from name patterns.
//!
//! IFC4X3 ADD2 adds `IfcFacility` (`IfcBridge`, `IfcBuilding`,
//! `IfcMarineFacility`, `IfcRailway`, `IfcRoad`) and the abstract
//! `IfcFacilityPart` (`IfcBridgePart`, `IfcFacilityPartCommon`,
//! `IfcMarinePart`, `IfcRailwayPart`, `IfcRoadPart`), all
//! `SUBTYPE OF (IfcSpatialStructureElement)`; IFC4 ADD2 TC1 adds
//! `IfcSpatialZone` and `IfcExternalSpatialElement` beside it under
//! `IfcSpatialElement`; IFC2X3 TC1 has only `IfcSpatialStructureElement`.

use std::collections::BTreeSet;

use ifc_model::{Codec, EntityId, Model};
use ifc_schema::{for_version, SchemaVersion};
use ifc_spatial::{SpatialAnomaly, SpatialKind, SpatialTree};
use ifc_step::StepCodec;

const RELEASES: [SchemaVersion; 3] = [
    SchemaVersion::Ifc2x3,
    SchemaVersion::Ifc4,
    SchemaVersion::Ifc4x3,
];

fn upper(names: &[&str]) -> BTreeSet<String> {
    names.iter().map(|n| n.to_ascii_uppercase()).collect()
}

/// The spatial root of each release, as its EXPRESS declares it.
fn root(release: SchemaVersion) -> &'static str {
    match release {
        SchemaVersion::Ifc2x3 => "IfcSpatialStructureElement",
        _ => "IfcSpatialElement",
    }
}

#[test]
fn the_ifc4x3_spatial_elements_are_the_ones_the_express_declares() {
    // Read from references/ifc-spec/ifc4x3-add2/IFC4X3_ADD2.exp by hand, so
    // a table or classifier regression cannot redefine the expectation.
    let expected = upper(&[
        "IfcExternalSpatialStructureElement",
        "IfcExternalSpatialElement",
        "IfcSpatialStructureElement",
        "IfcSpatialZone",
        "IfcFacility",
        "IfcBridge",
        "IfcBuilding",
        "IfcMarineFacility",
        "IfcRailway",
        "IfcRoad",
        "IfcFacilityPart",
        "IfcBridgePart",
        "IfcFacilityPartCommon",
        "IfcMarinePart",
        "IfcRailwayPart",
        "IfcRoadPart",
        "IfcSite",
        "IfcSpace",
        "IfcBuildingStorey",
    ]);
    let table = for_version(SchemaVersion::Ifc4x3).expect("bundled");
    assert_eq!(upper(&table.subtypes("IfcSpatialElement")), expected);
}

#[test]
fn every_release_classifies_exactly_its_spatial_subtype_closure() {
    for release in RELEASES {
        let table = for_version(release).expect("bundled");
        let mut spatial = upper(&table.subtypes(root(release)));
        // The (abstract) root is itself a spatial element.
        spatial.insert(root(release).to_ascii_uppercase());
        assert!(spatial.len() >= 4, "{release:?}: {spatial:?}");
        for name in table.entity_names() {
            let kind = SpatialKind::classify_in(name, release);
            let upper_name = name.to_ascii_uppercase();
            let expected = match upper_name.as_str() {
                "IFCPROJECT" => SpatialKind::Project,
                "IFCSITE" => SpatialKind::Site,
                "IFCBUILDING" => SpatialKind::Building,
                "IFCBUILDINGSTOREY" => SpatialKind::Storey,
                "IFCSPACE" => SpatialKind::Space,
                _ if spatial.contains(&upper_name) => SpatialKind::OtherContainer,
                _ => SpatialKind::Element,
            };
            assert_eq!(kind, expected, "{release:?} {name}");
        }
    }
}

#[test]
fn facilities_and_their_parts_are_containers_only_where_declared() {
    for name in [
        "IFCROAD",
        "IFCBRIDGEPART",
        "IfcFacilityPartCommon",
        "IFCRAILWAY",
    ] {
        assert_eq!(
            SpatialKind::classify_in(name, SchemaVersion::Ifc4x3),
            SpatialKind::OtherContainer,
            "{name}"
        );
        assert_eq!(
            SpatialKind::classify_in(name, SchemaVersion::Ifc4),
            SpatialKind::Element,
            "{name} is not declared in IFC4"
        );
        assert_eq!(SpatialKind::classify(name), SpatialKind::OtherContainer);
    }
    for name in ["IFCEXTERNALSPATIALELEMENT", "IFCSPATIALZONE"] {
        assert_eq!(
            SpatialKind::classify_in(name, SchemaVersion::Ifc4),
            SpatialKind::OtherContainer
        );
        assert_eq!(
            SpatialKind::classify_in(name, SchemaVersion::Ifc2x3),
            SpatialKind::Element
        );
    }
    // A name that merely looks spatial is not one.
    assert_eq!(SpatialKind::classify("IFCSPATIALFOO"), SpatialKind::Element);
    assert_eq!(
        SpatialKind::classify_in("IFCSPATIALFOO", SchemaVersion::Ifc4x3),
        SpatialKind::Element
    );
}

#[test]
fn no_release_declares_as_a_non_container_what_another_declares_spatial() {
    // What makes the release-free `classify` safe for headerless models.
    for spatial_in in RELEASES {
        let table = for_version(spatial_in).expect("bundled");
        for name in table.subtypes(root(spatial_in)) {
            for other in RELEASES {
                let other_table = for_version(other).expect("bundled");
                if other_table.entity(name).is_some() {
                    assert!(
                        SpatialKind::classify_in(name, other).is_container(),
                        "{name}: spatial in {spatial_in:?}, not in {other:?}"
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Trees over IFC4X3 infrastructure, parsed from STEP.

fn record(release: SchemaVersion, id: u64, entity: &str, set: &[(&str, &str)]) -> String {
    let names = for_version(release)
        .expect("bundled")
        .attribute_names(entity);
    assert!(!names.is_empty(), "{entity} is declared in {release:?}");
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
    let model = StepCodec.read_bytes(text.as_bytes()).expect("parses");
    assert!(model.diagnostics().is_empty(), "{:?}", model.diagnostics());
    model
}

fn aggregates(release: SchemaVersion, id: u64, whole: &str, parts: &str) -> String {
    record(
        release,
        id,
        "IfcRelAggregates",
        &[("RelatingObject", whole), ("RelatedObjects", parts)],
    )
}

fn placed(
    release: SchemaVersion,
    id: u64,
    entity: &str,
    structure: &str,
    elements: &str,
) -> String {
    record(
        release,
        id,
        entity,
        &[
            ("RelatingStructure", structure),
            ("RelatedElements", elements),
        ],
    )
}

/// project #1 -> site #2 -> road #3 -> road part #4, bridge #5 -> bridge
/// part #6; a proxy on each part, a wall on the road.
fn infrastructure() -> Vec<String> {
    let v = SchemaVersion::Ifc4x3;
    vec![
        record(v, 1, "IfcProject", &[]),
        record(v, 2, "IfcSite", &[]),
        record(v, 3, "IfcRoad", &[]),
        record(v, 4, "IfcRoadPart", &[]),
        record(v, 5, "IfcBridge", &[]),
        record(v, 6, "IfcBridgePart", &[]),
        aggregates(v, 10, "#1", "(#2)"),
        aggregates(v, 11, "#2", "(#3,#5)"),
        aggregates(v, 12, "#3", "(#4)"),
        aggregates(v, 13, "#5", "(#6)"),
        record(v, 20, "IfcBuildingElementProxy", &[]),
        record(v, 21, "IfcBuildingElementProxy", &[]),
        record(v, 22, "IfcWall", &[]),
        placed(v, 30, "IfcRelContainedInSpatialStructure", "#4", "(#20)"),
        placed(v, 31, "IfcRelContainedInSpatialStructure", "#6", "(#21)"),
        placed(v, 32, "IfcRelContainedInSpatialStructure", "#3", "(#22)"),
    ]
}

#[test]
fn an_ifc4x3_road_and_bridge_hold_their_parts_and_elements() {
    let model = parse("IFC4X3_ADD2", &infrastructure());
    let tree = SpatialTree::build(&model);
    assert_eq!(tree.release(), Some(SchemaVersion::Ifc4x3));
    assert_eq!(tree.roots(), [EntityId(1)]);
    assert!(tree.orphans().is_empty());
    assert!(tree.anomalies().is_empty(), "{:?}", tree.anomalies());
    for (facility, part) in [(3, 4), (5, 6)] {
        let node = tree.node(EntityId(facility)).expect("a container");
        assert_eq!(node.kind, SpatialKind::OtherContainer);
        assert_eq!(node.parent, Some(EntityId(2)));
        assert_eq!(node.children, [EntityId(part)]);
        let part_node = tree.node(EntityId(part)).expect("a container");
        assert_eq!(part_node.kind, SpatialKind::OtherContainer);
    }
    assert_eq!(tree.elements_of(EntityId(4)), [EntityId(20)]);
    assert_eq!(tree.elements_of(EntityId(6)), [EntityId(21)]);
    assert_eq!(tree.elements_of(EntityId(3)), [EntityId(22)]);
    assert_eq!(tree.container_of(EntityId(21)), Some(EntityId(6)));
    assert_eq!(
        tree.ancestors(EntityId(6)),
        [EntityId(5), EntityId(2), EntityId(1)]
    );
}

#[test]
fn a_structure_that_is_not_a_container_is_reported_for_both_relationships() {
    let v = SchemaVersion::Ifc4x3;
    let mut records = infrastructure();
    // The wall is not a spatial element: neither relationship can use it.
    records.push(placed(
        v,
        40,
        "IfcRelContainedInSpatialStructure",
        "#22",
        "(#20)",
    ));
    records.push(placed(
        v,
        41,
        "IfcRelReferencedInSpatialStructure",
        "#22",
        "(#21)",
    ));
    let tree = SpatialTree::build(&parse("IFC4X3_ADD2", &records));
    assert_eq!(
        tree.anomalies(),
        [
            SpatialAnomaly::ContainedInNonContainer {
                relation: EntityId(40),
                structure: EntityId(22),
            },
            SpatialAnomaly::ReferencedInNonContainer {
                relation: EntityId(41),
                structure: EntityId(22),
            },
        ]
    );
    assert!(tree.elements_of(EntityId(22)).is_empty());
    assert_eq!(tree.container_of(EntityId(20)), Some(EntityId(4)));
}

#[test]
fn the_declared_release_decides_not_the_name() {
    // IFC2X3 declares no IfcSpatialZone: in an IFC2X3 file it is not a
    // container, and containment into it is reported.
    let v = SchemaVersion::Ifc2x3;
    let records = [
        record(v, 3, "IfcBuildingStorey", &[]),
        "#4=IFCSPATIALZONE('0000000000000000000004',$,$,$,$,$,$,$,$);".to_owned(),
        record(v, 20, "IfcWall", &[]),
        placed(v, 30, "IfcRelContainedInSpatialStructure", "#4", "(#20)"),
    ];
    let tree = SpatialTree::build(&parse("IFC2X3", &records));
    assert_eq!(tree.release(), Some(SchemaVersion::Ifc2x3));
    assert!(tree.node(EntityId(4)).is_none());
    assert_eq!(
        tree.anomalies(),
        [SpatialAnomaly::ContainedInNonContainer {
            relation: EntityId(30),
            structure: EntityId(4),
        }]
    );
    // With no release declared, any bundled release's answer applies.
    let bound = parse("IFC2X3", &records);
    let mut unbound = Model::new();
    for id in bound.ids() {
        unbound.insert(id, bound.get(id).expect("listed").clone());
    }
    let tree = SpatialTree::build(&unbound);
    assert_eq!(tree.release(), None);
    assert_eq!(tree.elements_of(EntityId(4)), [EntityId(20)]);
}

#[test]
fn only_aggregation_and_containment_place_anything() {
    let v = SchemaVersion::Ifc4;
    let records = [
        record(v, 1, "IfcProject", &[]),
        record(v, 3, "IfcBuildingStorey", &[]),
        record(v, 5, "IfcSpace", &[]),
        record(v, 20, "IfcWallType", &[]),
        record(v, 21, "IfcCovering", &[]),
        aggregates(v, 10, "#1", "(#3)"),
        aggregates(v, 11, "#3", "(#5)"),
        // The project declares a type; the space is covered by a ceiling.
        record(
            v,
            12,
            "IfcRelDeclares",
            &[("RelatingContext", "#1"), ("RelatedDefinitions", "(#20)")],
        ),
        record(
            v,
            13,
            "IfcRelCoversSpaces",
            &[("RelatingSpace", "#5"), ("RelatedCoverings", "(#21)")],
        ),
    ];
    let tree = SpatialTree::build(&parse("IFC4", &records));
    assert!(
        tree.elements_of(EntityId(1)).is_empty(),
        "a type is not contained"
    );
    assert!(
        tree.elements_of(EntityId(5)).is_empty(),
        "a covering is not contained"
    );
    assert_eq!(tree.container_of(EntityId(21)), None);
    assert!(tree.anomalies().is_empty());
}
