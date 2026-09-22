//! Exercise every catalogued element type and supertype row.
//!
//! `create_type` and `create_supertype` build from a table row, so a test
//! that stages one row proves the function rather than the row. Walking
//! the catalogue makes the coverage audit see what the writers can
//! actually produce, and makes a bad arity or slot index fail here.

use ifc_element_type::{
    create_supertype, create_type, SupertypeDraft, TypeDraft, ALL, ALL_SUPERTYPES,
};
use ifc_model::{Model, Transaction};

const GUID: &str = "1hqA$FMcT8$hVvcqsRDBzZ";

fn named() -> TypeDraft<'static> {
    TypeDraft {
        // NameRequired: inherited by every catalogue row, so the sweep
        // cannot use a default draft.
        name: Some("Catalogue sweep"),
        description: None,
        applicable_occurrence: None,
        maps_or_identification: None,
        tag_or_long_description: None,
        fallback: None,
    }
}

/// Every catalogued element type stages with its declared arity.
#[test]
fn every_catalogued_type_stages() {
    assert!(ALL.len() > 100, "catalogue looks truncated: {}", ALL.len());

    for kind in ALL {
        // Most rows make PredefinedType mandatory; the row says which.
        // Passing None unconditionally would test the refusal, not the
        // staging, for every mandatory row.
        let token = if kind.predefined_optional {
            None
        } else {
            Some(kind.members[0])
        };
        let mut draft = named();
        if token == Some("USERDEFINED") {
            draft.fallback = Some("Bespoke");
        }

        let mut model = Model::default();
        let mut tx = Transaction::new(&model);
        let id = create_type(&mut tx, *kind, GUID, token, draft)
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
fn catalogued_type_tokens_round_trip() {
    for kind in ALL {
        let model = Model::default();
        for token in kind.members {
            let mut tx = Transaction::new(&model);
            let mut draft = named();
            if *token == "USERDEFINED" {
                draft.fallback = Some("Bespoke");
            }
            create_type(&mut tx, *kind, GUID, Some(token), draft).unwrap_or_else(|error| {
                panic!(
                    "{} refused its own token {token}: {error:?}",
                    kind.type_name
                )
            });
        }

        let mut tx = Transaction::new(&model);
        assert!(
            create_type(&mut tx, *kind, GUID, Some("__NOT_A_TOKEN__"), named()).is_err(),
            "{} accepted an undeclared token",
            kind.type_name,
        );
    }
}

/// Every supertype row stages.
#[test]
fn every_supertype_stages() {
    for kind in ALL_SUPERTYPES {
        let mut model = Model::default();
        let mut tx = Transaction::new(&model);
        let id = create_supertype(
            &mut tx,
            *kind,
            GUID,
            "Catalogue sweep",
            SupertypeDraft::default(),
        )
        .unwrap_or_else(|error| panic!("{kind:?} refused: {error:?}"));
        tx.commit(&mut model).expect("commit");
        assert!(model.get(id).is_some(), "{kind:?} staged nothing");
    }
}

/// `IfcTypeObject.NameRequired` is inherited by every catalogue row.
///
/// `Name` is OPTIONAL in the slot table and mandatory by rule, so
/// a writer trusting the slot table alone files a nameless type
/// that parses and cannot be referred to.
#[test]
fn a_nameless_type_is_refused() {
    for kind in ALL.iter().take(8) {
        let model = Model::default();
        let mut tx = Transaction::new(&model);
        let token = if kind.predefined_optional {
            None
        } else {
            Some(kind.members[0])
        };
        let mut draft = TypeDraft::default();
        if token == Some("USERDEFINED") {
            draft.fallback = Some("Bespoke");
        }

        assert!(
            create_type(&mut tx, *kind, GUID, token, draft).is_err(),
            "{} accepted a nameless type",
            kind.type_name,
        );
    }
}
