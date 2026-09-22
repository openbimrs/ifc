//! Authoring the external spatial element and the project library.
//!
//! Neither is an `IfcSpatialStructureElement`. The external
//! element is a product carrying a placement and representation;
//! the library is an `IfcContext` sharing `IfcProject`'s tail.
//! Slot 5 onward differs in all three shapes.

use ifc_model::{Entity, Model, Transaction, Value};
use ifc_schema::ifc4x3;
use ifc_spatial::{
    create_external_spatial_element, create_project_library, ExternalSpatialDraft,
    ProjectLibraryDraft,
};

const GUID: &str = "1jQ2A$rnvCJhUvFV5RxFtz";
const GUID2: &str = "3rT4B$mkwDKiVwGW6SyGua";

/// The two shapes put different things past slot 4.
#[test]
fn each_shape_lands_its_tail_in_the_declared_slots() {
    let mut model = Model::default();
    let placement = model.push(Entity::new("IFCLOCALPLACEMENT", vec![Value::Null; 2]));
    let units = model.push(Entity::new("IFCUNITASSIGNMENT", vec![Value::Null; 1]));
    let context = model.push(Entity::new(
        "IFCGEOMETRICREPRESENTATIONCONTEXT",
        vec![Value::Null; 6],
    ));
    let mut tx = Transaction::new(&model);
    let external = create_external_spatial_element(
        &mut tx,
        GUID,
        ExternalSpatialDraft {
            name: Some("North air"),
            placement: Some(placement),
            long_name: Some("Air space north of the facade"),
            predefined_type: Some("EXTERNAL_EARTH"),
            ..ExternalSpatialDraft::default()
        },
    )
    .expect("external spatial element");
    let library = create_project_library(
        &mut tx,
        GUID2,
        ProjectLibraryDraft {
            name: Some("Standard types"),
            phase: Some("Design"),
            units: Some(units),
            ..ProjectLibraryDraft::default()
        },
        &[context],
    )
    .expect("project library");
    tx.commit(&mut model).expect("commit");

    let e = model.get(external).expect("external");
    assert_eq!(e.attributes.len(), 9, "IfcExternalSpatialElement arity");
    assert_eq!(e.attributes[5], Value::Ref(placement), "ObjectPlacement");
    assert_eq!(
        e.attributes[7],
        Value::Text("Air space north of the facade".into())
    );
    assert_eq!(e.attributes[8], Value::Enum("EXTERNAL_EARTH".into()));

    let l = model.get(library).expect("library");
    assert_eq!(l.attributes.len(), 9, "IfcProjectLibrary arity");
    assert_eq!(l.attributes[6], Value::Text("Design".into()), "Phase");
    assert_eq!(l.attributes[7], Value::List(vec![Value::Ref(context)]));
    assert_eq!(l.attributes[8], Value::Ref(units), "UnitsInContext");
}

/// USERDEFINED names a kind of exterior space and withholds it.
#[test]
fn userdefined_without_an_object_type_is_refused() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    create_external_spatial_element(
        &mut tx,
        GUID,
        ExternalSpatialDraft {
            predefined_type: Some("USERDEFINED"),
            ..ExternalSpatialDraft::default()
        },
    )
    .expect_err("USERDEFINED needs ObjectType");
    create_external_spatial_element(
        &mut tx,
        GUID,
        ExternalSpatialDraft {
            predefined_type: Some("USERDEFINED"),
            object_type: Some("Acoustic buffer"),
            ..ExternalSpatialDraft::default()
        },
    )
    .expect("named kind is accepted");
    assert_eq!(tx.len(), 1, "only the named draft staged");
}

/// A token outside the enum is refused.
#[test]
fn a_token_outside_the_enum_is_refused() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    create_external_spatial_element(
        &mut tx,
        GUID,
        ExternalSpatialDraft {
            predefined_type: Some("EXTERNAL_AIR"),
            ..ExternalSpatialDraft::default()
        },
    )
    .expect_err("EXTERNAL_AIR is not a declared token");
    assert!(tx.is_empty(), "nothing staged when the token is unknown");
}

/// The hardcoded arities match the schema.
#[test]
fn declared_arities_match_the_schema() {
    for (entity, arity) in [("IfcExternalSpatialElement", 9), ("IfcProjectLibrary", 9)] {
        assert_eq!(
            ifc4x3().attribute_names(entity).len(),
            arity,
            "{entity} arity drifted"
        );
    }
}

/// An absent context set stays absent, not an empty list.
///
/// `RepresentationContexts` is `SET [1:?]`: a present-but-empty
/// aggregate satisfies the type and violates the bound, so the
/// slot is left null rather than written as `()`.
#[test]
fn an_absent_context_set_is_null_not_empty() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let library = create_project_library(
        &mut tx,
        GUID,
        ProjectLibraryDraft {
            name: Some("Types"),
            ..ProjectLibraryDraft::default()
        },
        &[],
    )
    .expect("library without contexts");
    tx.commit(&mut model).expect("commit");
    let l = model.get(library).expect("library");
    assert_eq!(
        l.attributes[7],
        Value::Null,
        "an empty SET [1:?] is written as absent, never as an empty list"
    );
}
