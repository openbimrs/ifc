//! Exercise every catalogued occurrence row.
//!
//! The catalogue-driven writer takes a table row and builds from it, so a
//! test that stages one row proves the function, not the row. Coverage
//! auditing by "did a test build this type name" therefore under-reports
//! every row no test happened to pick.
//!
//! This walks the whole catalogue so the audit sees what the writer can
//! actually produce, and so a row with a broken arity or slot index fails
//! here rather than in a consumer's file.

use ifc_model::{Model, Transaction};
use ifc_occurrence::table::ALL;
use ifc_occurrence::{create, OccurrenceDraft};

const GUID: &str = "1hqA$FMcT8$hVvcqsRDBzZ";

/// Every catalogued occurrence stages, with the arity its row declares.
#[test]
fn every_catalogued_occurrence_stages() {
    assert!(ALL.len() > 100, "catalogue looks truncated: {}", ALL.len());

    for kind in ALL {
        let mut model = Model::default();
        let mut tx = Transaction::new(&model);
        let id = create(
            &mut tx,
            &model,
            *kind,
            GUID,
            None,
            None,
            OccurrenceDraft::default(),
        )
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

/// Every row accepts its own declared tokens and refuses a foreign one.
#[test]
fn catalogued_predefined_tokens_round_trip() {
    let model = Model::default();
    for kind in ALL {
        if kind.predefined_slot.is_none() {
            continue;
        }
        for token in kind.members {
            if *token == "USERDEFINED" {
                continue;
            }
            let mut tx = Transaction::new(&model);
            create(
                &mut tx,
                &model,
                *kind,
                GUID,
                Some(token),
                None,
                OccurrenceDraft::default(),
            )
            .unwrap_or_else(|error| {
                panic!(
                    "{} refused its own token {token}: {error:?}",
                    kind.type_name
                )
            });
        }

        let mut tx = Transaction::new(&model);
        assert!(
            create(
                &mut tx,
                &model,
                *kind,
                GUID,
                Some("__NOT_A_TOKEN__"),
                None,
                OccurrenceDraft::default(),
            )
            .is_err(),
            "{} accepted an undeclared token",
            kind.type_name,
        );
    }
}
