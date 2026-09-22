//! Authoring the two linear supertypes.
//!
//! `IfcLinearElement` and `IfcLinearPositioningElement` are both
//! IFC4X3 additions and both arity 7, but they differ on whether a
//! placement is optional.

use ifc_alignment::{linear_element, linear_positioning_element};
use ifc_model::{Entity, Model, Transaction, Value};
use ifc_schema::{ifc4, ifc4x3};

const GUID: &str = "1hqA$QMHj8nB$JURcqgIm7";

/// A linear element carries nothing of its own beyond the product
/// slots, and its placement is genuinely optional.
#[test]
fn a_linear_element_needs_no_placement() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let id = linear_element(&mut tx, GUID, Some("Carriageway"), None).expect("linear element");
    tx.commit(&mut model).expect("commit");

    let staged = model.get(id).expect("staged");
    assert_eq!(staged.type_name.as_ref(), "IFCLINEARELEMENT");
    assert_eq!(staged.attributes.len(), 7, "product arity");
    assert_eq!(
        staged.attributes[5],
        Value::Null,
        "ObjectPlacement stays absent",
    );
}

/// HasPlacement: the positioning element inherits a rule the
/// linear element does not have.
///
/// The slot is OPTIONAL in the schema for both, so only the rule
/// distinguishes them. This writer encodes the rule in its signature:
/// the placement is a required EntityId, so a caller cannot express a
/// positioning element that positions nothing.
#[test]
fn a_positioning_element_requires_its_placement() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);

    let placement = tx.create(Entity::new("IFCLOCALPLACEMENT", vec![Value::Null; 2]));
    let id =
        linear_positioning_element(&mut tx, GUID, None, placement).expect("positioning element");
    tx.commit(&mut model).expect("commit");

    let staged = model.get(id).expect("staged");
    assert_eq!(staged.type_name.as_ref(), "IFCLINEARPOSITIONINGELEMENT",);
    assert_eq!(staged.attributes[5], Value::Ref(placement));
}

/// Both refuse a malformed GlobalId.
#[test]
fn a_malformed_global_id_is_refused() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let placement = tx.create(Entity::new("IFCLOCALPLACEMENT", vec![Value::Null; 2]));
    assert!(linear_element(&mut tx, "short", None, None).is_err());
    assert!(linear_positioning_element(&mut tx, "short", None, placement).is_err(),);
}

/// Both entities are IFC4X3 additions: IFC4 declares neither.
///
/// The hardcoded arity 7 is only safe because it matches the
/// schema, so assert the schema rather than trusting the constant.
#[test]
fn both_are_ifc4x3_additions() {
    for name in ["IfcLinearElement", "IfcLinearPositioningElement"] {
        assert!(
            ifc4().entity(name).is_none(),
            "{name} should be absent from IFC4",
        );
        assert_eq!(
            ifc4x3().attribute_names(name).len(),
            7,
            "{name} arity drifted from the schema",
        );
    }
    assert_eq!(
        ifc4x3().attribute_names("IfcLinearElement")[5],
        "ObjectPlacement",
        "slot 5 is the placement the rule talks about",
    );
}
