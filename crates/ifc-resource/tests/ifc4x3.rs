//! IFC4X3 resource-schema parity checks.

mod support;

use ifc_resource::{Person, ResourceError, ResourceTypeKind, ResourceView};
use ifc_schema::ifc4x3;

use support::{model, named, text};

#[test]
fn ifc4x3_model_projects_labor_resource_type_and_person() {
    let schema = ifc4x3();
    let mut model = model("IFC4X3_ADD2");
    let ty = model.push(named(
        schema,
        "IfcLaborResourceType",
        &[("Name", text("Formwork crew type"))],
    ));
    let person = model.push(named(schema, "IfcPerson", &[("FamilyName", text("Doe"))]));

    let view = ResourceView::for_model(&model).unwrap();
    let projected = view.resource_type(ty).unwrap();
    assert_eq!(projected.kind(), ResourceTypeKind::Labor);
    assert_eq!(projected.name().unwrap(), "Formwork crew type");

    let person_view: Person = view.person(person).unwrap();
    assert_eq!(person_view.family_name().unwrap(), Some("Doe"));
}

#[test]
fn ifc2x3_model_is_a_typed_refusal_because_resource_type_and_time_do_not_exist() {
    let ifc2x3 = model("IFC2X3");
    assert!(matches!(
        ResourceView::for_model(&ifc2x3),
        Err(ResourceError::UnsupportedSchema { .. })
    ));
}
