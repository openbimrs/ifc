//! Slot layout and binding of every attribute this crate reads or writes,
//! pinned against the three bundled schema tables.
//!
//! `tests/release_layout.rs` checks the bundled tables themselves against
//! the normative EXPRESS under `references/ifc-spec`.

use ifc_model::{EntityId, Model};
use ifc_schema::{for_version, SchemaVersion};

use super::{is_type_object, release_name, Release, IFC4_LAYOUT};
use crate::usage::SELECT_MEMBERS;
use crate::MaterialError;

const ALL: [SchemaVersion; 3] = [
    SchemaVersion::Ifc2x3,
    SchemaVersion::Ifc4,
    SchemaVersion::Ifc4x3,
];

/// IFC2X3 TC1 layouts, transcribed from `IFC2X3_TC1.exp` (inherited
/// attributes first). An entity absent here is not instantiable in IFC2X3.
const IFC2X3_LAYOUT: &[(&str, &[&str])] = &[
    // ENTITY IfcMaterial; Name : IfcLabel;
    ("IFCMATERIAL", &["Name"]),
    (
        "IFCMATERIALCLASSIFICATIONRELATIONSHIP",
        &["MaterialClassifications", "ClassifiedMaterial"],
    ),
    // IfcProductRepresentation (Name, Description, Representations).
    (
        "IFCMATERIALDEFINITIONREPRESENTATION",
        &[
            "Name",
            "Description",
            "Representations",
            "RepresentedMaterial",
        ],
    ),
    // ENTITY IfcMaterialLayer; Material, LayerThickness, IsVentilated.
    (
        "IFCMATERIALLAYER",
        &["Material", "LayerThickness", "IsVentilated"],
    ),
    // ENTITY IfcMaterialLayerSet; MaterialLayers, LayerSetName.
    ("IFCMATERIALLAYERSET", &["MaterialLayers", "LayerSetName"]),
    // No ReferenceExtent before IFC4.
    (
        "IFCMATERIALLAYERSETUSAGE",
        &[
            "ForLayerSet",
            "LayerSetDirection",
            "DirectionSense",
            "OffsetFromReferenceLine",
        ],
    ),
    ("IFCMATERIALLIST", &["Materials"]),
    (
        "IFCRELASSOCIATESMATERIAL",
        &[
            "GlobalId",
            "OwnerHistory",
            "Name",
            "Description",
            "RelatedObjects",
            "RelatingMaterial",
        ],
    ),
    (
        "IFCRELDEFINESBYTYPE",
        &[
            "GlobalId",
            "OwnerHistory",
            "Name",
            "Description",
            "RelatedObjects",
            "RelatingType",
        ],
    ),
];

fn names(version: SchemaVersion, entity: &str) -> Vec<String> {
    for_version(version)
        .unwrap()
        .attribute_names(entity)
        .into_iter()
        .map(str::to_owned)
        .collect()
}

fn owned(attributes: &[&str]) -> Vec<String> {
    attributes.iter().map(|name| (*name).to_owned()).collect()
}

#[test]
fn the_ifc4_layout_is_the_bundled_ifc4_table() {
    for (entity, attributes) in IFC4_LAYOUT {
        assert_eq!(
            names(SchemaVersion::Ifc4, entity),
            owned(attributes),
            "{entity}"
        );
        assert!(Release::Bound(SchemaVersion::Ifc4).instantiates(entity));
    }
}

#[test]
fn ifc4x3_keeps_the_ifc4_layout_but_renames_the_material_expression() {
    for (entity, attributes) in IFC4_LAYOUT {
        let renamed: Vec<&str> = attributes
            .iter()
            .map(|name| release_name(SchemaVersion::Ifc4x3, entity, name))
            .collect();
        assert_eq!(
            names(SchemaVersion::Ifc4x3, entity),
            owned(&renamed),
            "{entity}"
        );
        assert!(Release::Bound(SchemaVersion::Ifc4x3).instantiates(entity));
    }
    assert_eq!(
        names(SchemaVersion::Ifc4x3, "IFCMATERIALRELATIONSHIP")[4],
        "MaterialExpression"
    );
    assert_eq!(
        names(SchemaVersion::Ifc4, "IFCMATERIALRELATIONSHIP")[4],
        "Expression"
    );
}

#[test]
fn the_ifc2x3_layout_is_the_bundled_ifc2x3_table() {
    let release = Release::Bound(SchemaVersion::Ifc2x3);
    for (entity, _) in IFC4_LAYOUT {
        match IFC2X3_LAYOUT.iter().find(|(name, _)| name == entity) {
            Some((_, attributes)) => {
                assert!(release.instantiates(entity), "{entity}");
                assert_eq!(
                    names(SchemaVersion::Ifc2x3, entity),
                    owned(attributes),
                    "{entity}"
                );
            }
            None => assert!(!release.instantiates(entity), "{entity}"),
        }
    }
    // Abstract, not absent: IFC2X3 `IfcMaterialProperties` is ABSTRACT with
    // a single `Material` attribute, so it is refused as an instance.
    let schema = for_version(SchemaVersion::Ifc2x3).unwrap();
    assert!(schema.entity("IFCMATERIALPROPERTIES").unwrap().abstract_);
    assert_eq!(
        schema.attribute_names("IFCMATERIALPROPERTIES"),
        ["Material"]
    );
}

/// Every attribute the crate reads resolves to its own position in each
/// release, or to a typed absence -- never to another attribute's slot.
#[test]
fn every_slot_resolves_per_release_or_is_absent_by_schema() {
    let id = EntityId(1);
    for version in ALL {
        let release = Release::Bound(version);
        for (entity, attributes) in IFC4_LAYOUT {
            for (ifc4_slot, attribute) in attributes.iter().enumerate() {
                let resolved = release.slot(entity, id, attribute);
                let expected = match version {
                    SchemaVersion::Ifc4 | SchemaVersion::Ifc4x3 => Ok(ifc4_slot),
                    SchemaVersion::Ifc2x3 => {
                        match IFC2X3_LAYOUT.iter().find(|(name, _)| name == entity) {
                            None => Err(MaterialError::EntityNotInSchema {
                                entity,
                                id: Some(id),
                                schema: version,
                            }),
                            Some((_, layout)) => layout
                                .iter()
                                .position(|name| name == attribute)
                                .ok_or(MaterialError::NotInSchema {
                                    entity,
                                    id,
                                    attribute,
                                    schema: version,
                                }),
                        }
                    }
                };
                assert_eq!(resolved, expected, "{version:?} {entity}.{attribute}");
            }
        }
    }
}

#[test]
fn the_layer_thickness_measure_differs_by_release() {
    for (version, measure) in [
        (SchemaVersion::Ifc2x3, "IfcPositiveLengthMeasure"),
        (SchemaVersion::Ifc4, "IfcNonNegativeLengthMeasure"),
        (SchemaVersion::Ifc4x3, "IfcNonNegativeLengthMeasure"),
    ] {
        let declared = Release::Bound(version)
            .declared("IFCMATERIALLAYER", "LayerThickness")
            .unwrap();
        assert_eq!(declared.type_name, measure, "{version:?}");
    }
}

/// `SELECT_MEMBERS` restricted to what a release instantiates is exactly
/// that release's `IfcMaterialSelect` closure.
#[test]
fn the_select_dispatch_matches_each_release() {
    for version in ALL {
        let schema = for_version(version).unwrap();
        let release = Release::Bound(version);
        let mut closure: Vec<String> = schema
            .entity_names()
            .filter(|name| release.instantiates(name))
            .filter(|name| schema.accepts_type("IfcMaterialSelect", name))
            .map(str::to_ascii_uppercase)
            .collect();
        closure.sort();
        let mut dispatched: Vec<String> = SELECT_MEMBERS
            .iter()
            .filter(|name| release.instantiates(name))
            .map(|name| (*name).to_owned())
            .collect();
        dispatched.sort();
        assert_eq!(closure, dispatched, "{version:?}");
    }
    let ifc2x3 = Release::Bound(SchemaVersion::Ifc2x3);
    assert_eq!(
        SELECT_MEMBERS
            .iter()
            .filter(|name| ifc2x3.instantiates(name))
            .count(),
        5,
        "IFC2X3: material, list, layer, layer set, layer set usage"
    );
}

#[test]
fn type_objects_come_from_the_release_table() {
    let count = |version| {
        let schema = for_version(version).unwrap();
        schema
            .entity_names()
            .filter(|name| is_type_object(Release::Bound(version), name).unwrap())
            .count()
    };
    // The generated IFC4 inventory this replaces listed 119 entities.
    assert_eq!(count(SchemaVersion::Ifc4), 119);
    let ifc2x3 = Release::Bound(SchemaVersion::Ifc2x3);
    let ifc4x3 = Release::Bound(SchemaVersion::Ifc4x3);
    assert!(is_type_object(ifc2x3, "IfcWallType").unwrap());
    assert!(is_type_object(ifc2x3, "IFCDOORSTYLE").unwrap());
    assert!(!is_type_object(ifc2x3, "IFCTASKTYPE").unwrap());
    assert!(!is_type_object(ifc4x3, "IFCDOORSTYLE").unwrap());
    assert!(is_type_object(ifc4x3, "IFCDOORTYPE").unwrap());
    assert!(!is_type_object(ifc4x3, "IFCRELDEFINESBYTYPE").unwrap());
}

#[test]
fn the_header_binds_one_release_or_fails_closed() {
    let bind = |schemas: &[&str]| {
        let mut model = Model::new();
        model.header_mut().schema = schemas.iter().map(|s| (*s).to_owned()).collect();
        Release::of(&model).bound().map(|(version, _)| version)
    };
    assert_eq!(bind(&[]), Ok(SchemaVersion::Ifc4));
    assert_eq!(bind(&["IFC2X3"]), Ok(SchemaVersion::Ifc2x3));
    assert_eq!(bind(&["IFC4"]), Ok(SchemaVersion::Ifc4));
    assert_eq!(bind(&["ifc4x3_add2"]), Ok(SchemaVersion::Ifc4x3));
    assert_eq!(bind(&["IFC4X3"]), Ok(SchemaVersion::Ifc4x3));
    assert_eq!(
        bind(&["IFC5"]),
        Err(MaterialError::UnsupportedSchema {
            schema: "IFC5".to_owned()
        })
    );
    assert_eq!(
        bind(&["IFC4", "IFC4"]),
        Err(MaterialError::MultipleSchemas { schemas: 2 })
    );
}
