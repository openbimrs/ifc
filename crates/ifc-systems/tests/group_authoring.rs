//! Authoring the group family.
//!
//! `IfcGroup` is the supertype; `IfcSystem` and `IfcDistributionSystem`
//! specialise it. The distribution pair puts `LongName` before
//! `PredefinedType`, the inverse of the building systems, which is why
//! the slot positions are asserted rather than assumed.

use ifc_model::{Model, Transaction, Value};
use ifc_schema::{ifc4, ifc4x3};
use ifc_systems::{create_classified_system, create_group, ClassifiedSystemDraft, SystemKind};

const GUID: &str = "1jQ2A$rnvCJhUvFV5RxFtz";

/// A plain group stages with its five inherited slots.
#[test]
fn a_group_stages() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let id = create_group(&mut tx, GUID, Some("Snagging"), Some("Items to fix")).expect("group");
    tx.commit(&mut model).expect("commit");

    let staged = model.get(id).expect("staged");
    assert_eq!(staged.type_name.as_ref(), "IFCGROUP");
    assert_eq!(
        staged.attributes.len(),
        5,
        "IfcGroup adds nothing of its own"
    );
    assert_eq!(staged.attributes[2], Value::Text("Snagging".into()));
    assert_eq!(staged.attributes[3], Value::Text("Items to fix".into()));
}

/// A distribution system stages with LongName before PredefinedType.
#[test]
fn a_distribution_system_stages_with_its_own_slot_order() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let id = create_classified_system(
        &mut tx,
        ifc4x3(),
        SystemKind::DistributionSystem,
        GUID,
        Some("CHW"),
        ClassifiedSystemDraft {
            long_name: Some("Chilled water"),
            predefined_type: Some("CHILLEDWATER"),
            ..ClassifiedSystemDraft::default()
        },
    )
    .expect("distribution system");
    tx.commit(&mut model).expect("commit");

    let staged = model.get(id).expect("staged");
    assert_eq!(staged.type_name.as_ref(), "IFCDISTRIBUTIONSYSTEM");
    assert_eq!(staged.attributes.len(), 7);
    // LongName is slot 5 and PredefinedType slot 6 -- the inverse of
    // IfcBuildingSystem, where the type comes first.
    assert_eq!(staged.attributes[5], Value::Text("Chilled water".into()));
    assert_eq!(staged.attributes[6], Value::Enum("CHILLEDWATER".into()));
}

/// The distribution enum gained members in IFC4X3.
///
/// A token only the newer schema declares must not be accepted when
/// authoring against IFC4, or the file names a system kind that
/// schema does not have.
#[test]
fn a_newer_distribution_token_is_refused_on_ifc4() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);

    let staged_on_x3 = create_classified_system(
        &mut tx,
        ifc4x3(),
        SystemKind::DistributionSystem,
        GUID,
        None,
        ClassifiedSystemDraft {
            predefined_type: Some("MONITORINGSYSTEM"),
            ..ClassifiedSystemDraft::default()
        },
    );
    assert!(staged_on_x3.is_ok(), "IFC4X3 declares MONITORINGSYSTEM");

    let staged_on_ifc4 = create_classified_system(
        &mut tx,
        ifc4(),
        SystemKind::DistributionSystem,
        GUID,
        None,
        ClassifiedSystemDraft {
            predefined_type: Some("MONITORINGSYSTEM"),
            ..ClassifiedSystemDraft::default()
        },
    );
    assert!(
        staged_on_ifc4.is_err(),
        "IFC4 does not declare MONITORINGSYSTEM"
    );
}
