//! Writers in this crate that nothing had called.
//!
//! `IfcProfileProperties` and `IfcWindowPanelProperties` already had
//! working writers; no test reached either, so a wrong slot or type
//! name would have compiled and shipped.

use ifc_model::{Entity, Model, Transaction, Value};
use ifc_properties::{
    add_conversion_based_unit, add_conversion_based_unit_with_offset, add_dimensional_exponents,
    add_profile_properties, add_window_panel_properties, ConversionBasedUnitDraft,
};

const GUID: &str = "1jQ2A$rnvCJhUvFV5RxFtz";

/// The window panel writer stages its nine slots.
#[test]
fn the_window_panel_writer_stages() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let id = add_window_panel_properties(
        &mut tx,
        GUID,
        Some("Casement"),
        "SIDEHUNGRIGHTHAND",
        "LEFT",
        (Some(0.06), Some(0.04)),
    )
    .expect("window panel");
    tx.commit(&mut model).expect("commit");

    let staged = model.get(id).expect("staged");
    assert_eq!(staged.type_name.as_ref(), "IFCWINDOWPANELPROPERTIES");
    assert_eq!(staged.attributes.len(), 9);
    assert_eq!(
        staged.attributes[4],
        Value::Enum("SIDEHUNGRIGHTHAND".into()),
        "OperationType is slot 4",
    );
    assert_eq!(staged.attributes[5], Value::Enum("LEFT".into()));
}

/// The profile-properties writer stages, and refuses an empty set.
#[test]
fn the_profile_properties_writer_stages() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let property = tx.create(Entity::new("IFCPROPERTYSINGLEVALUE", vec![Value::Null; 4]));
    let profile = tx.create(Entity::new("IFCCIRCLEPROFILEDEF", vec![Value::Null; 4]));

    assert!(
        add_profile_properties(&mut tx, None, None, &[], profile).is_err(),
        "an empty SET [1:?] was accepted",
    );

    let id = add_profile_properties(&mut tx, Some("Section"), None, &[property], profile)
        .expect("profile properties");
    tx.commit(&mut model).expect("commit");

    let staged = model.get(id).expect("staged");
    assert_eq!(staged.type_name.as_ref(), "IFCPROFILEPROPERTIES");
    assert_eq!(staged.attributes.len(), 4);
    assert_eq!(
        staged.attributes[2],
        Value::List(vec![Value::Ref(property)])
    );
    assert_eq!(staged.attributes[3], Value::Ref(profile));
}

/// The offset unit adds a fifth slot the base form does not have.
#[test]
fn the_offset_unit_carries_its_offset() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let dimensions = add_dimensional_exponents(&mut tx, [0, 0, 0, 0, 1, 0, 0]);
    let factor = tx.create(Entity::new("IFCMEASUREWITHUNIT", vec![Value::Null; 2]));
    let draft = ConversionBasedUnitDraft {
        unit_type: "THERMODYNAMICTEMPERATUREUNIT",
        name: "DEGREE CELSIUS",
        conversion_factor: factor,
        dimensions,
    };

    let plain = add_conversion_based_unit(&mut tx, draft).expect("plain");
    // Celsius converts to kelvin by a factor of one and an offset of
    // 273.15. Without the offset, absolute zero lands at freezing.
    let offset = add_conversion_based_unit_with_offset(&mut tx, draft, 273.15).expect("offset");
    tx.commit(&mut model).expect("commit");

    assert_eq!(model.get(plain).expect("staged").attributes.len(), 4);
    let staged = model.get(offset).expect("staged");
    assert_eq!(
        staged.type_name.as_ref(),
        "IFCCONVERSIONBASEDUNITWITHOFFSET",
    );
    assert_eq!(staged.attributes.len(), 5);
    assert_eq!(staged.attributes[4], Value::Real(273.15));
}
