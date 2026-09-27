//! The bundled tables this crate binds to agree with the normative EXPRESS.
//!
//! `src/release/tests.rs` pins every attribute the crate reads or writes
//! against the bundled IFC2X3, IFC4 and IFC4X3 tables. This closes the chain:
//! for every MaterialResource entity (and the two relationships the crate
//! reads) the bundled positional layout and abstractness equal the ones
//! parsed from `references/ifc-spec`. Skips when the references are absent,
//! unless `IFC_SPEC_REQUIRED` is set (as `scripts/gate.sh` does).

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

/// Every entity name the crate reads or writes, in any release.
const READ: &[&str] = &[
    "IFCMATERIAL",
    "IFCMATERIALCLASSIFICATIONRELATIONSHIP",
    "IFCMATERIALCONSTITUENT",
    "IFCMATERIALCONSTITUENTSET",
    "IFCMATERIALDEFINITIONREPRESENTATION",
    "IFCMATERIALLAYER",
    "IFCMATERIALLAYERSET",
    "IFCMATERIALLAYERSETUSAGE",
    "IFCMATERIALLAYERWITHOFFSETS",
    "IFCMATERIALLIST",
    "IFCMATERIALPROFILE",
    "IFCMATERIALPROFILESET",
    "IFCMATERIALPROFILESETUSAGE",
    "IFCMATERIALPROFILESETUSAGETAPERING",
    "IFCMATERIALPROFILEWITHOFFSETS",
    "IFCMATERIALPROPERTIES",
    "IFCMATERIALRELATIONSHIP",
    "IFCRELASSOCIATESMATERIAL",
    "IFCRELDEFINESBYTYPE",
    "IFCTYPEOBJECT",
];

#[test]
fn bundled_material_layouts_match_the_reference_express() {
    for (version, path) in RELEASES {
        let Some(reference) = load(path) else {
            return;
        };
        let bundled = for_version(version).unwrap();
        for entity in READ {
            let declared = reference.entity(entity);
            assert_eq!(
                bundled.entity(entity).is_some(),
                declared.is_some(),
                "{version:?} {entity}: declared"
            );
            let Some(declared) = declared else {
                continue;
            };
            assert_eq!(
                bundled.entity(entity).unwrap().abstract_,
                declared.abstract_,
                "{version:?} {entity}: abstractness"
            );
            assert_eq!(
                bundled.attribute_names(entity),
                reference.attribute_names(entity),
                "{version:?} {entity}: positional layout"
            );
            let types = |schema: &Schema| -> Vec<String> {
                schema
                    .attributes(entity)
                    .iter()
                    .map(|attribute| attribute.type_name.clone())
                    .collect()
            };
            assert_eq!(
                types(bundled),
                types(&reference),
                "{version:?} {entity}: declared types"
            );
        }
        // The type objects a release admits as `RelatingType`.
        let concrete_types = |schema: &Schema| -> Vec<String> {
            let mut names: Vec<String> = schema
                .entity_names()
                .filter(|name| schema.is_a(name, "IFCTYPEOBJECT"))
                .filter(|name| !schema.entity(name).unwrap().abstract_)
                .map(str::to_ascii_uppercase)
                .collect();
            names.sort();
            names
        };
        assert_eq!(
            concrete_types(bundled),
            concrete_types(&reference),
            "{version:?}: concrete IfcTypeObject subtypes"
        );
    }
}

/// The IFC2X3 facts #77 names, read straight from the reference EXPRESS.
#[test]
fn the_ifc2x3_material_facts_hold_in_the_reference() {
    let Some(ifc2x3) = load("ifc2x3-tc1/IFC2X3_TC1.exp") else {
        return;
    };
    assert_eq!(ifc2x3.attribute_names("IfcMaterial"), ["Name"]);
    assert_eq!(
        ifc2x3.attribute_names("IfcMaterialLayer"),
        ["Material", "LayerThickness", "IsVentilated"]
    );
    assert_eq!(
        ifc2x3.attribute_names("IfcMaterialLayerSet"),
        ["MaterialLayers", "LayerSetName"]
    );
    assert_eq!(
        ifc2x3.attribute_names("IfcMaterialLayerSetUsage"),
        [
            "ForLayerSet",
            "LayerSetDirection",
            "DirectionSense",
            "OffsetFromReferenceLine"
        ]
    );
    for absent in [
        "IfcMaterialConstituentSet",
        "IfcMaterialProfileSet",
        "IfcMaterialRelationship",
        "IfcMaterialLayerWithOffsets",
    ] {
        assert!(ifc2x3.entity(absent).is_none(), "{absent}");
    }
    let Some(ifc4x3) = load("ifc4x3-add2/IFC4X3_ADD2.exp") else {
        return;
    };
    assert_eq!(
        ifc4x3.attribute_names("IfcMaterialRelationship")[4],
        "MaterialExpression"
    );
}
