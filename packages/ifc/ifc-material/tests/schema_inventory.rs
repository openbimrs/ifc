use ifc_material::{
    CardinalPointReference, DirectionSense, LayerSetDirection, IFC4_MATERIAL_RESOURCE_ENTITIES,
    IFC4_MATERIAL_RESOURCE_TYPES,
};

#[test]
fn pins_complete_ifc4_material_resource_declaration_inventory() {
    assert_eq!(IFC4_MATERIAL_RESOURCE_ENTITIES.len(), 18);
    assert_eq!(IFC4_MATERIAL_RESOURCE_TYPES.len(), 4);
    for required in [
        "IFCMATERIAL",
        "IFCMATERIALDEFINITION",
        "IFCMATERIALLAYERWITHOFFSETS",
        "IFCMATERIALPROFILESETUSAGETAPERING",
        "IFCMATERIALPROPERTIES",
        "IFCMATERIALUSAGEDEFINITION",
    ] {
        assert!(IFC4_MATERIAL_RESOURCE_ENTITIES.contains(&required));
    }
    assert_eq!(
        DirectionSense::parse("negative"),
        Some(DirectionSense::Negative)
    );
    assert_eq!(
        LayerSetDirection::parse("axis3"),
        Some(LayerSetDirection::Axis3)
    );
    assert!(CardinalPointReference::new(0).is_none());
    assert_eq!(CardinalPointReference::new(20).unwrap().get(), 20);
}
