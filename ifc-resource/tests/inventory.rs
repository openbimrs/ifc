//! `IfcInventory` projection and group-membership checks.

mod support;

use ifc_resource::{ResourceError, ResourceView};
use ifc_schema::ifc4;

use support::{enumeration, model, named, refs, text, GUID_A};

#[test]
fn inventory_projects_metadata_jurisdiction_and_responsible_persons() {
    let schema = ifc4();
    let mut model = model("IFC4");
    let person = model.push(named(schema, "IfcPerson", &[("FamilyName", text("Doe"))]));
    let org = model.push(named(schema, "IfcOrganization", &[("Name", text("Acme"))]));
    let current = model.push(named(schema, "IfcCostValue", &[]));
    let original = model.push(named(schema, "IfcCostValue", &[]));
    let inventory = model.push(named(
        schema,
        "IfcInventory",
        &[
            ("Name", text("Site tools")),
            ("Description", text("Hand tools")),
            ("PredefinedType", enumeration("ASSETINVENTORY")),
            ("Jurisdiction", ifc_model::Value::Ref(org)),
            ("ResponsiblePersons", refs(&[person])),
            ("LastUpdateDate", text("2026-09-01")),
            ("CurrentValue", ifc_model::Value::Ref(current)),
            ("OriginalValue", ifc_model::Value::Ref(original)),
        ],
    ));

    let view = ResourceView::for_model(&model).unwrap();
    let projected = view.inventory(inventory).unwrap();
    assert_eq!(projected.name().unwrap(), Some("Site tools"));
    assert_eq!(projected.description().unwrap(), Some("Hand tools"));
    assert_eq!(projected.predefined_type().unwrap(), Some("ASSETINVENTORY"));
    assert_eq!(projected.jurisdiction().unwrap(), Some(org));
    assert_eq!(projected.responsible_person_ids().unwrap(), vec![person]);
    assert_eq!(projected.last_update_date().unwrap(), Some("2026-09-01"));
    assert_eq!(projected.current_value().unwrap(), Some(current));
    assert_eq!(projected.original_value().unwrap(), Some(original));
}

#[test]
fn inventory_jurisdiction_accepts_person_and_organization_select_members() {
    let schema = ifc4();
    let mut model = model("IFC4");
    let person = model.push(named(schema, "IfcPerson", &[("FamilyName", text("Doe"))]));
    let inventory = model.push(named(
        schema,
        "IfcInventory",
        &[("Jurisdiction", ifc_model::Value::Ref(person))],
    ));

    let view = ResourceView::for_model(&model).unwrap();
    assert_eq!(
        view.inventory(inventory).unwrap().jurisdiction().unwrap(),
        Some(person)
    );
}

#[test]
fn inventory_jurisdiction_rejects_wrong_reference_type() {
    let schema = ifc4();
    let mut model = model("IFC4");
    let wall = model.push(named(schema, "IfcWall", &[("GlobalId", text(GUID_A))]));
    let inventory = model.push(named(
        schema,
        "IfcInventory",
        &[("Jurisdiction", ifc_model::Value::Ref(wall))],
    ));

    let view = ResourceView::for_model(&model).unwrap();
    assert!(matches!(
        view.inventory(inventory).unwrap().jurisdiction(),
        Err(ResourceError::WrongReferenceType { .. })
    ));
}

#[test]
fn inventory_items_resolve_via_rel_assigns_to_group_authored_order() {
    let schema = ifc4();
    let mut model = model("IFC4");
    let inventory = model.push(named(schema, "IfcInventory", &[("Name", text("Tools"))]));
    let asset_a = model.push(named(
        schema,
        "IfcLaborResource",
        &[("GlobalId", text(GUID_A))],
    ));
    let asset_b = model.push(named(
        schema,
        "IfcLaborResource",
        &[("GlobalId", text("1O2Fr$t4X7Zf8NOew3FLOH"))],
    ));
    model.push(named(
        schema,
        "IfcRelAssignsToGroup",
        &[
            ("RelatedObjects", refs(&[asset_a, asset_b])),
            ("RelatingGroup", ifc_model::Value::Ref(inventory)),
        ],
    ));

    let view = ResourceView::for_model(&model).unwrap();
    assert_eq!(
        view.inventory_items(inventory).unwrap(),
        vec![asset_a, asset_b]
    );
}

#[test]
fn inventory_items_ignores_relations_naming_a_different_group() {
    let schema = ifc4();
    let mut model = model("IFC4");
    let inventory_a = model.push(named(schema, "IfcInventory", &[("Name", text("A"))]));
    let inventory_b = model.push(named(schema, "IfcInventory", &[("Name", text("B"))]));
    let asset = model.push(named(
        schema,
        "IfcLaborResource",
        &[("GlobalId", text(GUID_A))],
    ));
    model.push(named(
        schema,
        "IfcRelAssignsToGroup",
        &[
            ("RelatedObjects", refs(&[asset])),
            ("RelatingGroup", ifc_model::Value::Ref(inventory_b)),
        ],
    ));

    let view = ResourceView::for_model(&model).unwrap();
    assert_eq!(view.inventory_items(inventory_a).unwrap(), Vec::new());
    assert_eq!(view.inventory_items(inventory_b).unwrap(), vec![asset]);
}
