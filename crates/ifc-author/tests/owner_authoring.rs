//! Ownership authoring.
//!
//! `IfcOwnerHistory` is the provenance record every `IfcRoot` subtype carries.
//! Domain crates in this workspace write a null there; these tests cover the
//! shared write side that lets a caller supply a real one.

use ifc_author::{
    add_application, add_organization, add_owner_history, add_person, add_person_and_organization,
    ApplicationDraft, OrganizationDraft, OwnerHistoryDraft, PersonDraft,
};
use ifc_model::{Model, Transaction};

#[test]
fn a_full_ownership_chain_commits() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let person = add_person(
        &mut tx,
        PersonDraft {
            family_name: Some("Schroedter"),
            ..PersonDraft::default()
        },
    )
    .expect("a named person is valid");
    let org = add_organization(
        &mut tx,
        OrganizationDraft {
            name: "OpenBIM.rs",
            ..OrganizationDraft::default()
        },
    )
    .expect("a named organization is valid");
    let user = add_person_and_organization(&mut tx, person, org);
    let app = add_application(
        &mut tx,
        ApplicationDraft {
            developer: org,
            version: "0.1.0",
            full_name: "openbim-ifc",
            identifier: "openbim-ifc",
        },
    )
    .expect("a complete application is valid");
    let history = add_owner_history(
        &mut tx,
        OwnerHistoryDraft {
            owning_user: user,
            owning_application: app,
            change_action: Some("ADDED"),
            creation_date: 1_700_000_000,
            last_modified_date: None,
        },
    )
    .expect("a well formed ownership record");
    tx.commit(&mut model).expect("ownership chain commits");
    assert!(model.get(history).is_some(), "history reached the model");
}

#[test]
fn a_person_identifying_nobody_is_refused() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    assert!(
        add_person(&mut tx, PersonDraft::default()).is_err(),
        "a person with no identification, family or given name names nobody"
    );
}

#[test]
fn a_change_action_outside_the_enumeration_is_refused() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let person = add_person(
        &mut tx,
        PersonDraft {
            given_name: Some("A"),
            ..PersonDraft::default()
        },
    )
    .expect("person");
    let org = add_organization(
        &mut tx,
        OrganizationDraft {
            name: "Org",
            ..OrganizationDraft::default()
        },
    )
    .expect("org");
    let user = add_person_and_organization(&mut tx, person, org);
    let app = add_application(
        &mut tx,
        ApplicationDraft {
            developer: org,
            version: "1",
            full_name: "f",
            identifier: "i",
        },
    )
    .expect("app");
    let refused = add_owner_history(
        &mut tx,
        OwnerHistoryDraft {
            owning_user: user,
            owning_application: app,
            change_action: Some("CREATED"),
            creation_date: 1,
            last_modified_date: None,
        },
    );
    assert!(
        refused.is_err(),
        "CREATED is not an IfcChangeActionEnum constant; it would serialize as \
         an unparseable enumeration token"
    );
}

#[test]
fn a_record_modified_before_it_existed_is_refused() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let person = add_person(
        &mut tx,
        PersonDraft {
            given_name: Some("A"),
            ..PersonDraft::default()
        },
    )
    .expect("person");
    let org = add_organization(
        &mut tx,
        OrganizationDraft {
            name: "Org",
            ..OrganizationDraft::default()
        },
    )
    .expect("org");
    let user = add_person_and_organization(&mut tx, person, org);
    let app = add_application(
        &mut tx,
        ApplicationDraft {
            developer: org,
            version: "1",
            full_name: "f",
            identifier: "i",
        },
    )
    .expect("app");
    let refused = add_owner_history(
        &mut tx,
        OwnerHistoryDraft {
            owning_user: user,
            owning_application: app,
            change_action: None,
            creation_date: 1_700_000_000,
            last_modified_date: Some(1_600_000_000),
        },
    );
    assert!(
        refused.is_err(),
        "a LastModifiedDate before CreationDate parses and validates while \
         silently corrupting any audit trail built on it"
    );
}
