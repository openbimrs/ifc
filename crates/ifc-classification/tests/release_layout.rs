//! The bundled tables this crate binds to agree with the normative EXPRESS.
//!
//! `src/release/tests.rs` pins every attribute the crate reads or writes
//! against the bundled IFC2X3, IFC4 and IFC4X3 tables. This closes the chain:
//! for every entity the crate touches, the bundled positional layout, types
//! and optionality, and every select and enumeration it checks against,
//! equal the ones parsed from `references/ifc-spec`. Skips when the
//! references are absent, unless `IFC_SPEC_REQUIRED` is set (as
//! `scripts/gate.sh` does).

use ifc_schema::{for_version, Schema, SchemaVersion};
use std::path::PathBuf;

/// Locate `references/ifc-spec` in either checkout layout.
fn spec_root() -> Option<PathBuf> {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    [
        "../../../../references/ifc-spec",
        "../../references/ifc-spec",
    ]
    .into_iter()
    .map(|rel| crate_dir.join(rel))
    .find(|path| path.is_dir())
}

fn load(rel: &str) -> Option<Schema> {
    let Some(root) = spec_root() else {
        assert!(
            std::env::var_os("IFC_SPEC_REQUIRED").is_none(),
            "IFC_SPEC_REQUIRED is set but references/ifc-spec was not found; \
             run scripts/fetch-ifc-schemas.sh"
        );
        eprintln!("skipped: references/ifc-spec not present");
        return None;
    };
    let bytes = std::fs::read(root.join(rel)).expect("read the reference schema");
    Some(Schema::from_express_bytes(&bytes))
}

const RELEASES: [(SchemaVersion, &str); 3] = [
    (SchemaVersion::Ifc2x3, "ifc2x3-tc1/IFC2X3_TC1.exp"),
    (SchemaVersion::Ifc4, "ifc4-add2-tc1/IFC4.exp"),
    (SchemaVersion::Ifc4x3, "ifc4x3-add2/IFC4X3_ADD2.exp"),
];

/// Every entity the crate reads or writes, in any release.
const ENTITIES: &[&str] = &[
    "IFCCLASSIFICATION",
    "IFCCLASSIFICATIONREFERENCE",
    "IFCCLASSIFICATIONNOTATION",
    "IFCCLASSIFICATIONNOTATIONFACET",
    "IFCDOCUMENTINFORMATION",
    "IFCDOCUMENTREFERENCE",
    "IFCDOCUMENTINFORMATIONRELATIONSHIP",
    "IFCLIBRARYINFORMATION",
    "IFCLIBRARYREFERENCE",
    "IFCEXTERNALREFERENCERELATIONSHIP",
    "IFCRELASSOCIATESCLASSIFICATION",
    "IFCRELASSOCIATESDOCUMENT",
    "IFCRELASSOCIATESLIBRARY",
    "IFCRELDEFINESBYTYPE",
];

/// Every select and enumeration a read or write is checked against.
const TYPES: &[&str] = &[
    "IfcActorSelect",
    "IfcClassificationNotationSelect",
    "IfcClassificationReferenceSelect",
    "IfcClassificationSelect",
    "IfcDefinitionSelect",
    "IfcDocumentConfidentialityEnum",
    "IfcDocumentSelect",
    "IfcDocumentStatusEnum",
    "IfcLibrarySelect",
    "IfcResourceObjectSelect",
];

#[test]
fn bundled_classification_layouts_match_the_reference_express() {
    for (version, path) in RELEASES {
        let Some(reference) = load(path) else {
            return;
        };
        let bundled = for_version(version).unwrap();
        for entity in ENTITIES {
            assert_eq!(
                bundled.entity(entity).map(|e| e.abstract_),
                reference.entity(entity).map(|e| e.abstract_),
                "{version:?} {entity}: declared and abstractness"
            );
            let layout = |schema: &Schema| -> Vec<(String, String, bool)> {
                schema
                    .attributes(entity)
                    .iter()
                    .map(|a| (a.name.clone(), a.type_name.clone(), a.optional))
                    .collect()
            };
            assert_eq!(
                layout(bundled),
                layout(&reference),
                "{version:?} {entity}: positional layout, types, optionality"
            );
        }
        for name in TYPES {
            assert_eq!(
                bundled.type_def(name).map(|t| format!("{:?}", t.kind)),
                reference.type_def(name).map(|t| format!("{:?}", t.kind)),
                "{version:?} {name}"
            );
        }
    }
}

/// The IFC4X3 differences #194 names, read straight from the reference.
#[test]
fn the_ifc4x3_classification_facts_hold_in_the_reference() {
    let (Some(ifc4), Some(ifc4x3)) = (
        load("ifc4-add2-tc1/IFC4.exp"),
        load("ifc4x3-add2/IFC4X3_ADD2.exp"),
    ) else {
        return;
    };
    assert_eq!(ifc4.attribute_names("IfcClassification")[5], "Location");
    assert_eq!(
        ifc4x3.attribute_names("IfcClassification")[5],
        "Specification"
    );
    assert!(!ifc4.accepts_type("IfcResourceObjectSelect", "IfcShapeAspect"));
    assert!(ifc4x3.accepts_type("IfcResourceObjectSelect", "IfcShapeAspect"));
    assert!(ifc4.entity("IfcRoad").is_none());
    assert!(ifc4x3.accepts_type("IfcDefinitionSelect", "IfcRoad"));
    for entity in [
        "IfcClassificationReference",
        "IfcRelAssociatesClassification",
        "IfcDocumentInformation",
        "IfcDocumentReference",
        "IfcLibraryInformation",
        "IfcLibraryReference",
        "IfcExternalReferenceRelationship",
    ] {
        assert_eq!(
            ifc4.attribute_names(entity),
            ifc4x3.attribute_names(entity),
            "{entity} is unchanged from IFC4"
        );
    }
    let Some(ifc2x3) = load("ifc2x3-tc1/IFC2X3_TC1.exp") else {
        return;
    };
    assert_eq!(
        ifc2x3.attribute_names("IfcClassificationReference"),
        ["Location", "ItemReference", "Name", "ReferencedSource"]
    );
    assert_eq!(
        ifc4.attribute_names("IfcClassificationReference")[1],
        "Identification"
    );
    assert!(
        !ifc2x3.attributes("IfcRoot")[1].optional,
        "IFC2X3 OwnerHistory"
    );
    assert!(
        ifc4x3.attributes("IfcRoot")[1].optional,
        "IFC4X3 OwnerHistory"
    );
}
