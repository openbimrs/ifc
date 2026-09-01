mod support;

use ifc_resource::{ResourceError, ResourceView};
use ifc_schema::ifc4;

use support::model;

#[test]
fn bundled_ifc4_slots_match_the_normative_resource_contract() {
    let schema = ifc4();
    assert_eq!(
        schema.attribute_names("IfcConstructionResource"),
        [
            "GlobalId",
            "OwnerHistory",
            "Name",
            "Description",
            "ObjectType",
            "Identification",
            "LongDescription",
            "Usage",
            "BaseCosts",
            "BaseQuantity",
        ]
    );
    assert_eq!(
        schema.attribute_names("IfcRelAssignsToResource"),
        [
            "GlobalId",
            "OwnerHistory",
            "Name",
            "Description",
            "RelatedObjects",
            "RelatedObjectsType",
            "RelatingResource",
        ]
    );
    assert_eq!(
        schema.attribute_names("IfcRelNests"),
        [
            "GlobalId",
            "OwnerHistory",
            "Name",
            "Description",
            "RelatingObject",
            "RelatedObjects",
        ]
    );
}

#[test]
fn view_selects_only_the_proven_ifc4_schema() {
    let missing = ifc_model::Model::new();
    assert!(matches!(
        ResourceView::for_model(&missing),
        Err(ResourceError::MissingSchema)
    ));

    let mut ambiguous = model("IFC4");
    ambiguous.header_mut().schema.push("IFC4".into());
    assert!(matches!(
        ResourceView::for_model(&ambiguous),
        Err(ResourceError::AmbiguousSchema { .. })
    ));

    for token in ["IFC2X3", "IFC4X3_ADD2", "IFC5"] {
        let unsupported = model(token);
        assert!(matches!(
            ResourceView::for_model(&unsupported),
            Err(ResourceError::UnsupportedSchema { .. })
        ));
    }

    let mismatched = model("IFC4X3_ADD2");
    assert!(matches!(
        ResourceView::new(&mismatched, ifc4()),
        Err(ResourceError::UnsupportedSchema { .. })
    ));

    ResourceView::for_model(&model("IFC4")).expect("IFC4 ADD2 TC1 is supported");
}
