//! `IfcPhysicalSimpleQuantity`/`IfcPhysicalComplexQuantity` projection checks.

mod support;

use ifc_resource::{ResourceError, ResourceView, SimpleQuantityValue};
use ifc_schema::ifc4;

use support::{model, named, refs, text};

#[test]
fn simple_quantity_projects_typed_value_and_metadata() {
    let schema = ifc4();
    let mut model = model("IFC4");
    let quantity = model.push(named(
        schema,
        "IfcQuantityCount",
        &[
            ("Name", text("Crew count")),
            ("CountValue", ifc_model::Value::Real(4.0)),
        ],
    ));

    let view = ResourceView::for_model(&model).unwrap();
    let projected = view.simple_quantity(quantity).unwrap();
    assert_eq!(projected.name().unwrap(), "Crew count");
    assert_eq!(projected.value().unwrap(), SimpleQuantityValue::Count(4.0));
    assert_eq!(projected.unit().unwrap(), None);
}

#[test]
fn simple_quantity_rejects_negative_or_non_finite_values() {
    let schema = ifc4();

    let mut negative = model("IFC4");
    let quantity = negative.push(named(
        schema,
        "IfcQuantityLength",
        &[
            ("Name", text("Span")),
            ("LengthValue", ifc_model::Value::Real(-1.0)),
        ],
    ));
    assert!(matches!(
        ResourceView::for_model(&negative)
            .unwrap()
            .simple_quantity(quantity)
            .unwrap()
            .value(),
        Err(ResourceError::InvalidValue { .. })
    ));

    let mut nan = model("IFC4");
    let quantity = nan.push(named(
        schema,
        "IfcQuantityArea",
        &[
            ("Name", text("Footprint")),
            ("AreaValue", ifc_model::Value::Real(f64::NAN)),
        ],
    ));
    assert!(matches!(
        ResourceView::for_model(&nan)
            .unwrap()
            .simple_quantity(quantity)
            .unwrap()
            .value(),
        Err(ResourceError::InvalidValue { .. })
    ));
}

#[test]
fn complex_quantity_resolves_members_and_rejects_self_reference() {
    let schema = ifc4();
    let mut model = model("IFC4");
    let member = model.push(named(
        schema,
        "IfcQuantityWeight",
        &[
            ("Name", text("Steel weight")),
            ("WeightValue", ifc_model::Value::Real(120.0)),
        ],
    ));
    let complex = model.push(named(
        schema,
        "IfcPhysicalComplexQuantity",
        &[
            ("Name", text("Steel bundle")),
            ("Discrimination", text("material")),
            ("HasQuantities", refs(&[member])),
        ],
    ));

    let view = ResourceView::for_model(&model).unwrap();
    let projected = view.complex_quantity(complex).unwrap();
    assert_eq!(projected.discrimination().unwrap(), "material");
    assert_eq!(projected.member_ids().unwrap(), vec![member]);
}

#[test]
fn complex_quantity_rejects_self_reference() {
    let schema = ifc4();
    let mut model = model("IFC4");
    let self_id = model.next_id();
    let complex = model.push(named(
        schema,
        "IfcPhysicalComplexQuantity",
        &[
            ("Name", text("Bundle")),
            ("Discrimination", text("material")),
            ("HasQuantities", refs(&[self_id])),
        ],
    ));
    assert_eq!(complex, self_id);

    assert!(matches!(
        ResourceView::for_model(&model)
            .unwrap()
            .complex_quantity(complex),
        Err(ResourceError::SemanticViolation { .. })
    ));
}

#[test]
fn construction_resource_base_quantity_resolves_a_simple_quantity() {
    let schema = ifc4();
    let mut model = model("IFC4");
    let quantity = model.push(named(
        schema,
        "IfcQuantityCount",
        &[
            ("Name", text("Crew count")),
            ("CountValue", ifc_model::Value::Real(3.0)),
        ],
    ));
    let labor = model.push(named(
        schema,
        "IfcLaborResource",
        &[
            ("GlobalId", text("2O2Fr$t4X7Zf8NOew3FLOH")),
            ("BaseQuantity", ifc_model::Value::Ref(quantity)),
        ],
    ));

    let view = ResourceView::for_model(&model).unwrap();
    let resource = view.resource(labor).unwrap();
    let base_quantity_id = resource.base_quantity().unwrap().unwrap();
    let base_quantity = view.simple_quantity(base_quantity_id).unwrap();
    assert_eq!(
        base_quantity.value().unwrap(),
        SimpleQuantityValue::Count(3.0)
    );
}
