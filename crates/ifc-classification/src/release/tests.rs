//! Slot layout, declared types and binding of every attribute this crate
//! reads or writes, pinned against the three bundled schema tables.
//!
//! `tests/release_layout.rs` checks the bundled tables themselves against
//! the normative EXPRESS under `references/ifc-spec`.

use ifc_model::{EntityId, Model, Value};
use ifc_schema::{for_version, SchemaVersion};

use super::{enumeration, is_text_type, release_name, Release};
use crate::ClassificationError;

const ALL: [SchemaVersion; 3] = [
    SchemaVersion::Ifc2x3,
    SchemaVersion::Ifc4,
    SchemaVersion::Ifc4x3,
];

const ROOT: [&str; 4] = ["GlobalId", "OwnerHistory", "Name", "Description"];

/// Every attribute this crate reads or writes, per entity, under its IFC4
/// ADD2 TC1 name and in IFC4 positional order (inherited attributes first).
/// Accessors never index it: they ask the bound release by name.
const IFC4_LAYOUT: &[(&str, &[&str])] = &[
    (
        "IFCCLASSIFICATION",
        &[
            "Source",
            "Edition",
            "EditionDate",
            "Name",
            "Description",
            "Location",
            "ReferenceTokens",
        ],
    ),
    (
        "IFCCLASSIFICATIONREFERENCE",
        &[
            "Location",
            "Identification",
            "Name",
            "ReferencedSource",
            "Description",
            "Sort",
        ],
    ),
    (
        "IFCDOCUMENTINFORMATION",
        &[
            "Identification",
            "Name",
            "Description",
            "Location",
            "Purpose",
            "IntendedUse",
            "Scope",
            "Revision",
            "DocumentOwner",
            "Editors",
            "CreationTime",
            "LastRevisionTime",
            "ElectronicFormat",
            "ValidFrom",
            "ValidUntil",
            "Confidentiality",
            "Status",
        ],
    ),
    (
        "IFCDOCUMENTREFERENCE",
        &[
            "Location",
            "Identification",
            "Name",
            "Description",
            "ReferencedDocument",
        ],
    ),
    (
        "IFCLIBRARYINFORMATION",
        &[
            "Name",
            "Version",
            "Publisher",
            "VersionDate",
            "Location",
            "Description",
        ],
    ),
    (
        "IFCLIBRARYREFERENCE",
        &[
            "Location",
            "Identification",
            "Name",
            "Description",
            "Language",
            "ReferencedLibrary",
        ],
    ),
    (
        "IFCEXTERNALREFERENCERELATIONSHIP",
        &[
            "Name",
            "Description",
            "RelatingReference",
            "RelatedResourceObjects",
        ],
    ),
    (
        "IFCDOCUMENTINFORMATIONRELATIONSHIP",
        &[
            "Name",
            "Description",
            "RelatingDocument",
            "RelatedDocuments",
            "RelationshipType",
        ],
    ),
];

/// The `IfcRoot` relationships, each `ROOT` plus these two.
const ROOT_RELATIONS: &[(&str, [&str; 2])] = &[
    (
        "IFCRELASSOCIATESCLASSIFICATION",
        ["RelatedObjects", "RelatingClassification"],
    ),
    (
        "IFCRELASSOCIATESDOCUMENT",
        ["RelatedObjects", "RelatingDocument"],
    ),
    (
        "IFCRELASSOCIATESLIBRARY",
        ["RelatedObjects", "RelatingLibrary"],
    ),
    ("IFCRELDEFINESBYTYPE", ["RelatedObjects", "RelatingType"]),
];

/// IFC2X3 TC1 layouts, transcribed from `IFC2X3_TC1.exp` (inherited first).
/// `IfcExternalReferenceRelationship` is absent there.
const IFC2X3_LAYOUT: &[(&str, &[&str])] = &[
    // ENTITY IfcClassification; Source, Edition, EditionDate, Name.
    (
        "IFCCLASSIFICATION",
        &["Source", "Edition", "EditionDate", "Name"],
    ),
    // IfcExternalReference (Location, ItemReference, Name) + ReferencedSource.
    (
        "IFCCLASSIFICATIONREFERENCE",
        &["Location", "ItemReference", "Name", "ReferencedSource"],
    ),
    (
        "IFCDOCUMENTINFORMATION",
        &[
            "DocumentId",
            "Name",
            "Description",
            "DocumentReferences",
            "Purpose",
            "IntendedUse",
            "Scope",
            "Revision",
            "DocumentOwner",
            "Editors",
            "CreationTime",
            "LastRevisionTime",
            "ElectronicFormat",
            "ValidFrom",
            "ValidUntil",
            "Confidentiality",
            "Status",
        ],
    ),
    (
        "IFCDOCUMENTREFERENCE",
        &["Location", "ItemReference", "Name"],
    ),
    (
        "IFCLIBRARYINFORMATION",
        &[
            "Name",
            "Version",
            "Publisher",
            "VersionDate",
            "LibraryReference",
        ],
    ),
    (
        "IFCLIBRARYREFERENCE",
        &["Location", "ItemReference", "Name"],
    ),
    // Not an IfcResourceLevelRelationship in IFC2X3: no Name/Description.
    (
        "IFCDOCUMENTINFORMATIONRELATIONSHIP",
        &["RelatingDocument", "RelatedDocuments", "RelationshipType"],
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

fn root_layout(tail: [&str; 2]) -> Vec<String> {
    owned(&ROOT).into_iter().chain(owned(&tail)).collect()
}

#[test]
fn the_ifc4_layout_is_the_bundled_ifc4_table() {
    for (entity, attributes) in IFC4_LAYOUT {
        assert_eq!(
            names(SchemaVersion::Ifc4, entity),
            owned(attributes),
            "{entity}"
        );
    }
    for (entity, tail) in ROOT_RELATIONS {
        for version in ALL {
            assert_eq!(
                names(version, entity),
                root_layout(*tail),
                "{version:?} {entity}"
            );
        }
    }
}

/// IFC4X3 ADD2 keeps every IFC4 layout this crate touches except one name:
/// `IfcClassification.Location` is `Specification`, same position.
#[test]
fn ifc4x3_keeps_the_ifc4_layout_but_renames_the_classification_location() {
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
    }
    assert_eq!(
        names(SchemaVersion::Ifc4x3, "IFCCLASSIFICATION")[5],
        "Specification"
    );
    assert_eq!(
        names(SchemaVersion::Ifc4, "IFCCLASSIFICATION")[5],
        "Location"
    );
    let slot = |version| Release::Bound(version).slot("IFCCLASSIFICATION", EntityId(1), "Location");
    assert_eq!(slot(SchemaVersion::Ifc4), Ok(5));
    assert_eq!(slot(SchemaVersion::Ifc4x3), Ok(5));
}

#[test]
fn the_ifc2x3_layout_is_the_bundled_ifc2x3_table() {
    for (entity, attributes) in IFC2X3_LAYOUT {
        assert_eq!(
            names(SchemaVersion::Ifc2x3, entity),
            owned(attributes),
            "{entity}"
        );
    }
    let schema = for_version(SchemaVersion::Ifc2x3).unwrap();
    assert!(schema.entity("IFCEXTERNALREFERENCERELATIONSHIP").is_none());
}

/// Every attribute the crate touches resolves to its own position in each
/// release, or to a typed absence -- never to another attribute's slot.
#[test]
fn every_slot_resolves_per_release_or_is_absent_by_schema() {
    let id = EntityId(1);
    for version in ALL {
        let release = Release::Bound(version);
        for (entity, attributes) in IFC4_LAYOUT {
            for (ifc4_slot, attribute) in attributes.iter().enumerate() {
                let expected = match version {
                    SchemaVersion::Ifc4 | SchemaVersion::Ifc4x3 => Ok(ifc4_slot),
                    SchemaVersion::Ifc2x3 => IFC2X3_LAYOUT
                        .iter()
                        .find(|(name, _)| name == entity)
                        .and_then(|(_, layout)| {
                            let name = release_name(version, entity, attribute);
                            layout.iter().position(|declared| *declared == name)
                        })
                        .ok_or(ClassificationError::NotInSchema {
                            entity,
                            id,
                            attribute,
                            schema: version,
                        }),
                };
                assert_eq!(
                    release.slot(entity, id, attribute),
                    expected,
                    "{version:?} {entity}.{attribute}"
                );
            }
        }
    }
}

/// The declared types that decide what a read or write accepts, per release.
#[test]
fn the_declared_types_that_differ_by_release() {
    let declared = |version, entity, attribute| {
        Release::Bound(version)
            .declared(entity, attribute)
            .map(|a| (a.type_name.as_str(), a.optional))
    };
    use SchemaVersion::{Ifc2x3, Ifc4, Ifc4x3};
    let rel = "IFCRELASSOCIATESCLASSIFICATION";
    assert_eq!(
        declared(Ifc2x3, rel, "OwnerHistory"),
        Ok(("IfcOwnerHistory", false))
    );
    assert_eq!(
        declared(Ifc4, rel, "OwnerHistory"),
        Ok(("IfcOwnerHistory", true))
    );
    assert_eq!(
        declared(Ifc4x3, rel, "OwnerHistory"),
        Ok(("IfcOwnerHistory", true))
    );
    assert_eq!(
        declared(Ifc2x3, rel, "RelatedObjects"),
        Ok(("IfcRoot", false))
    );
    for version in [Ifc4, Ifc4x3] {
        assert_eq!(
            declared(version, rel, "RelatedObjects"),
            Ok(("IfcDefinitionSelect", false))
        );
        assert_eq!(
            declared(version, rel, "RelatingClassification"),
            Ok(("IfcClassificationSelect", false))
        );
        assert_eq!(
            declared(version, "IFCCLASSIFICATIONREFERENCE", "ReferencedSource"),
            Ok(("IfcClassificationReferenceSelect", true))
        );
        assert_eq!(
            declared(version, "IFCCLASSIFICATION", "Location"),
            Ok(("IfcURIReference", true))
        );
        assert_eq!(
            declared(version, "IFCCLASSIFICATION", "Source"),
            Ok(("IfcLabel", true))
        );
    }
    assert_eq!(
        declared(Ifc2x3, rel, "RelatingClassification"),
        Ok(("IfcClassificationNotationSelect", false))
    );
    assert_eq!(
        declared(Ifc2x3, "IFCCLASSIFICATIONREFERENCE", "ReferencedSource"),
        Ok(("IfcClassification", true))
    );
    assert_eq!(
        declared(Ifc2x3, "IFCCLASSIFICATION", "Source"),
        Ok(("IfcLabel", false))
    );
    assert_eq!(
        declared(Ifc2x3, "IFCCLASSIFICATION", "EditionDate"),
        Ok(("IfcCalendarDate", true))
    );
    assert_eq!(
        declared(Ifc2x3, "IFCLIBRARYINFORMATION", "Publisher"),
        Ok(("IfcOrganization", true))
    );

    // IfcResourceObjectSelect gained IfcShapeAspect in IFC4X3.
    let resource = |version| {
        let schema = for_version(version).unwrap();
        schema.accepts_type("IfcResourceObjectSelect", "IFCSHAPEASPECT")
    };
    assert!(!resource(Ifc4));
    assert!(resource(Ifc4x3));
}

#[test]
fn text_types_resolve_to_string_and_records_do_not() {
    for version in ALL {
        let schema = for_version(version).unwrap();
        for text in ["IfcLabel", "IfcIdentifier", "IfcText"] {
            assert!(is_text_type(schema, text), "{version:?} {text}");
        }
        assert!(!is_text_type(schema, "IfcOrganization"), "{version:?}");
    }
    for version in [SchemaVersion::Ifc4, SchemaVersion::Ifc4x3] {
        let schema = for_version(version).unwrap();
        for text in ["IfcURIReference", "IfcDate", "IfcDateTime", "IfcLanguageId"] {
            assert!(is_text_type(schema, text), "{version:?} {text}");
        }
    }
    let ifc2x3 = for_version(SchemaVersion::Ifc2x3).unwrap();
    for record in [
        "IfcCalendarDate",
        "IfcDateAndTime",
        "IfcDocumentElectronicFormat",
    ] {
        assert!(!is_text_type(ifc2x3, record), "{record}");
    }
}

/// The document enumerations: the same members in every release, in
/// different order in IFC4X3, and `FINALDRAFT` everywhere.
#[test]
fn document_enumerations_come_from_the_release() {
    let sorted = |version, name| {
        let mut values = enumeration(for_version(version).unwrap(), name);
        values.sort_unstable();
        values
    };
    for version in ALL {
        assert_eq!(
            sorted(version, "IfcDocumentConfidentialityEnum"),
            [
                "CONFIDENTIAL",
                "NOTDEFINED",
                "PERSONAL",
                "PUBLIC",
                "RESTRICTED",
                "USERDEFINED"
            ],
            "{version:?}"
        );
        assert_eq!(
            sorted(version, "IfcDocumentStatusEnum"),
            ["DRAFT", "FINAL", "FINALDRAFT", "NOTDEFINED", "REVISION"],
            "{version:?}"
        );
    }
    let order = |version| {
        enumeration(
            for_version(version).unwrap(),
            "IfcDocumentConfidentialityEnum",
        )[0]
    };
    assert_eq!(order(SchemaVersion::Ifc4), "PUBLIC");
    assert_eq!(order(SchemaVersion::Ifc4x3), "CONFIDENTIAL");
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
    assert_eq!(bind(&["IFC4X3"]), Ok(SchemaVersion::Ifc4x3));
    assert_eq!(bind(&["ifc4x3_add2"]), Ok(SchemaVersion::Ifc4x3));
    assert_eq!(
        bind(&["IFC4X1"]),
        Err(ClassificationError::UnsupportedSchema {
            schema: "IFC4X1".to_owned()
        })
    );
    assert_eq!(
        bind(&["IFC4", "IFC4X3"]),
        Err(ClassificationError::MultipleSchemas { schemas: 2 })
    );
}

#[test]
fn records_refuse_what_the_release_cannot_hold() {
    let ifc2x3 = Release::Bound(SchemaVersion::Ifc2x3);
    let text = |s: &str| Value::Text(s.into());
    assert_eq!(
        ifc2x3.record(
            "IFCCLASSIFICATION",
            vec![("Name", text("U")), ("Description", text("d"))]
        ),
        Err(ClassificationError::AuthoringNotInSchema {
            entity: "IFCCLASSIFICATION",
            attribute: "Description",
            schema: SchemaVersion::Ifc2x3,
        })
    );
    assert_eq!(
        ifc2x3.record(
            "IFCCLASSIFICATION",
            vec![
                ("Source", text("s")),
                ("Edition", text("e")),
                ("EditionDate", text("2024-01-01")),
                ("Name", text("U"))
            ]
        ),
        Err(ClassificationError::AuthoringValueType {
            entity: "IFCCLASSIFICATION",
            attribute: "EditionDate",
            declared: "IfcCalendarDate",
            schema: SchemaVersion::Ifc2x3,
        })
    );
    assert_eq!(
        ifc2x3.record("IFCCLASSIFICATION", vec![("Name", text("U"))]),
        Err(ClassificationError::AuthoringRequired {
            entity: "IFCCLASSIFICATION",
            attribute: "Source",
            schema: SchemaVersion::Ifc2x3,
        })
    );
    assert_eq!(
        ifc2x3.record("IFCEXTERNALREFERENCERELATIONSHIP", vec![]),
        Err(ClassificationError::EntityNotInSchema {
            entity: "IFCEXTERNALREFERENCERELATIONSHIP",
            schema: SchemaVersion::Ifc2x3,
        })
    );
    // IFC4X3 writes the IFC4-named Location into Specification.
    let record = Release::Bound(SchemaVersion::Ifc4x3)
        .record(
            "IFCCLASSIFICATION",
            vec![("Name", text("U")), ("Location", text("https://x"))],
        )
        .unwrap();
    assert_eq!(record.attributes.len(), 7);
    assert_eq!(record.attributes[5], text("https://x"));
}
