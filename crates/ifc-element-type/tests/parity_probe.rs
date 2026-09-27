//! The same probes run against IfcOpenShell, for comparison.
//!
//! Not a parity claim: the two libraries differ in kind. This pins
//! what our writer refuses at construction so the difference is
//! measured rather than asserted.

use ifc_element_type::table::IFCBEAMTYPE;
use ifc_element_type::{create_type, TypeDraft};
use ifc_model::{Model, Transaction};

/// Probe 2: a malformed GlobalId. IfcOpenShell accepts "nope".
#[test]
fn a_malformed_guid_is_refused() {
    let model = Model::new();
    let mut tx = Transaction::new(&model);
    let r = create_type(
        &mut tx,
        IFCBEAMTYPE,
        "nope",
        Some("BEAM"),
        TypeDraft::default(),
    );
    assert!(r.is_err(), "a four-character GUID is not an IFC GUID");
}

/// Probe 3: USERDEFINED without the name it promises.
#[test]
fn userdefined_without_a_name_is_refused() {
    let model = Model::new();
    let mut tx = Transaction::new(&model);
    let r = create_type(
        &mut tx,
        IFCBEAMTYPE,
        "3vB2YO$MX4xv5uCqZZG05x",
        Some("USERDEFINED"),
        TypeDraft::default(),
    );
    assert!(r.is_err(), "USERDEFINED needs ElementType");
}

/// Probe 4: a token outside the entity's enum.
#[test]
fn an_unknown_enum_token_is_refused() {
    let model = Model::new();
    let mut tx = Transaction::new(&model);
    let r = create_type(
        &mut tx,
        IFCBEAMTYPE,
        "3vB2YO$MX4xv5uCqZZG05y",
        Some("BANANA"),
        TypeDraft::default(),
    );
    assert!(r.is_err(), "BANANA is not an IfcBeamTypeEnum member");
}
