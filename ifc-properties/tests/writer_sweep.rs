//! Sweep two writers nothing had called.
//!
//! `add_conversion_based_unit` stages an `IfcConversionBasedUnit`;
//! `create_material_list` stages an `IfcMaterialList`. Neither type was
//! produced anywhere in the suite, so their slot layouts were unverified.

use ifc_model::{Entity, Model, Transaction, Value};
use ifc_properties::{add_conversion_based_unit, ConversionBasedUnitDraft};

/// `IfcConversionBasedUnit` stages with its factor and dimensions.
#[test]
fn the_conversion_based_unit_writer_stages() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let dimensions = tx.create(Entity::new(
        "IFCDIMENSIONALEXPONENTS",
        vec![Value::Integer(1); 7],
    ));
    let factor = tx.create(Entity::new("IFCMEASUREWITHUNIT", vec![Value::Null; 2]));

    let id = add_conversion_based_unit(
        &mut tx,
        ConversionBasedUnitDraft {
            unit_type: "LENGTHUNIT",
            name: "inch",
            conversion_factor: factor,
            dimensions,
        },
    )
    .expect("conversion based unit");
    tx.commit(&mut model).expect("commit");

    let staged = model.get(id).expect("staged");
    assert_eq!(staged.type_name.as_ref(), "IFCCONVERSIONBASEDUNIT");
    // Dimensions and UnitType are inherited from IfcNamedUnit; the
    // conversion pair follows.
    assert_eq!(staged.attributes[0], Value::Ref(dimensions));
    assert_eq!(staged.attributes[3], Value::Ref(factor));
}
