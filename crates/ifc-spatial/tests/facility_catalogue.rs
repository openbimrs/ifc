//! Exercise every catalogued facility row.
//!
//! `create_facility` builds from a table row, so a test that stages one
//! row proves the function rather than the row. Coverage auditing by
//! "did a test build this type name" therefore under-reports every row
//! no test happened to pick -- which was all of them but `IfcBridge`.
//!
//! Walking the catalogue also makes a bad arity or slot index fail here
//! rather than in a consumer's file.

use ifc_model::{Model, Transaction, Value};
use ifc_spatial::facility::ALL;
use ifc_spatial::{create_facility, FacilityDraft};

const GUID: &str = "1hqA$FMcT8$hVvcqsRDBzZ";

/// A draft naming itself, so `USERDEFINED` tokens are legal.
fn named() -> FacilityDraft<'static> {
    FacilityDraft {
        name: Some("Sweep"),
        description: None,
        object_type: Some("Bespoke"),
        placement: None,
        long_name: None,
        composition: None,
        usage: None,
    }
}

/// Every catalogued facility stages, with the arity its row declares.
#[test]
fn every_catalogued_facility_stages() {
    assert_eq!(ALL.len(), 10, "catalogue size");

    for kind in ALL {
        let mut model = Model::default();
        let mut tx = Transaction::new(&model);
        let mut draft = named();
        // A part declares a UsageType slot and requires it; a facility
        // declares none and refuses one.
        if kind.usage_slot.is_some() {
            draft.usage = Some("LONGITUDINAL");
        }
        let token = kind.predefined_slot.map(|_| kind.members[0]);

        let id = create_facility(&mut tx, *kind, GUID, token, draft)
            .unwrap_or_else(|error| panic!("{} refused: {error:?}", kind.type_name));
        tx.commit(&mut model).expect("commit");

        let staged = model.get(id).expect("staged");
        assert_eq!(staged.type_name.as_ref(), kind.type_name);
        assert_eq!(
            staged.attributes.len(),
            kind.arity,
            "{} arity disagrees with its catalogue row",
            kind.type_name,
        );
    }
}

/// Every row accepts its own tokens and refuses a foreign one.
#[test]
fn catalogued_facility_tokens_round_trip() {
    for kind in ALL {
        let Some(slot) = kind.predefined_slot else {
            continue;
        };
        let mut draft = named();
        if kind.usage_slot.is_some() {
            draft.usage = Some("LONGITUDINAL");
        }

        for token in kind.members {
            let model = Model::default();
            let mut tx = Transaction::new(&model);
            let id =
                create_facility(&mut tx, *kind, GUID, Some(token), draft).unwrap_or_else(|error| {
                    panic!(
                        "{} refused its own token {token}: {error:?}",
                        kind.type_name
                    )
                });
            let mut committed = model.clone();
            tx.commit(&mut committed).expect("commit");
            assert_eq!(
                committed.get(id).expect("staged").attributes[slot],
                Value::Enum(token.to_string().into()),
                "{} wrote {token} to the wrong slot",
                kind.type_name,
            );
        }

        let model = Model::default();
        let mut tx = Transaction::new(&model);
        assert!(
            create_facility(&mut tx, *kind, GUID, Some("__NOT_A_TOKEN__"), draft).is_err(),
            "{} accepted an undeclared token",
            kind.type_name,
        );
    }
}

/// The usage slot is required exactly where the row declares one.
///
/// `UsageType` is the only non-optional attribute these entities add, and
/// it exists on the parts but not the facilities. A writer that ignored
/// the row would file a part with an empty required slot.
#[test]
fn the_usage_slot_follows_the_row() {
    for kind in ALL {
        let model = Model::default();
        let mut tx = Transaction::new(&model);

        if kind.usage_slot.is_some() {
            assert!(
                create_facility(&mut tx, *kind, GUID, kind.members.first().copied(), named())
                    .is_err(),
                "{} staged without its required UsageType",
                kind.type_name,
            );
        } else {
            let mut draft = named();
            draft.usage = Some("LONGITUDINAL");
            assert!(
                create_facility(&mut tx, *kind, GUID, kind.members.first().copied(), draft)
                    .is_err(),
                "{} accepted a UsageType it does not declare",
                kind.type_name,
            );
        }
    }
}
