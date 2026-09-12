//! The material inventory must match the schema, not itself.
//!
//! `schema_inventory.rs` asserts the list length against a number written
//! beside it, so trimming the list trims the expectation. This derives the
//! expectation from the normative EXPRESS source instead.

use ifc_schema::completeness::audit_inventory;
use ifc_schema::Schema;
use std::path::PathBuf;

/// Locate `references/ifc-spec` in either checkout layout.
fn spec_root() -> Option<PathBuf> {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    ["../../../references/ifc-spec", "../references/ifc-spec"]
        .into_iter()
        .map(|rel| crate_dir.join(rel))
        .find(|path| path.is_dir())
}

/// The roots the MaterialResource inventory covers.
///
/// `IfcMaterialDefinition` is the hierarchy root, but the schema also puts
/// three standalone entities and one representation subtype in this schema:
/// `IfcMaterialDefinitionRepresentation` descends from
/// `IfcProductRepresentation`, not from `IfcMaterialDefinition`, so a
/// hierarchy walk alone would miss it -- which is exactly how it went
/// missing from the published list.
const ROOTS: &[&str] = &[
    "IfcMaterialDefinition",
    "IfcMaterialDefinitionRepresentation",
    "IfcMaterialClassificationRelationship",
    "IfcMaterialLayerSetUsage",
    "IfcMaterialList",
    "IfcMaterialProfileSetUsage",
    "IfcMaterialProperties",
    "IfcMaterialRelationship",
    "IfcMaterialUsageDefinition",
];

#[test]
fn the_published_inventory_names_every_entity_the_schema_declares() {
    let Some(root) = spec_root() else {
        assert!(
            std::env::var_os("IFC_SPEC_REQUIRED").is_none(),
            "IFC_SPEC_REQUIRED is set but references/ifc-spec was not found"
        );
        eprintln!("skipped: references/ifc-spec not present");
        return;
    };
    let bytes = std::fs::read(root.join("ifc4-add2-tc1/IFC4.exp")).expect("read IFC4.exp");
    let schema = Schema::from_express_bytes(&bytes);

    let gap = audit_inventory(
        &schema,
        ROOTS,
        ifc_material::IFC4_MATERIAL_RESOURCE_ENTITIES,
    );
    assert!(gap.is_empty(), "inventory disagrees with IFC4:\n{gap}");
}
