//! #202: what a release does not declare, or requires and the call leaves
//! unset, is refused with a typed error before anything is staged.

mod release_support;

use ifc_element_type::table::{IFCBEAMTYPE, IFCBEARINGTYPE, IFCDOORTYPE, IFCLABORRESOURCETYPE};
use ifc_element_type::{
    create_supertype_in, create_supertype_with_owner_history, create_type_in,
    create_type_with_owner_history, ElementType, ElementTypeError, SupertypeDraft, ALL,
    BUILT_ELEMENT_TYPE, TYPE_PRODUCT,
};
use ifc_model::{EntityId, Model, Transaction};
use ifc_schema::SchemaVersion;
use release_support::{
    declares, members, model, named, refused, table, GUID, OWNER, RELEASES, WALL,
};

#[test]
fn ifc2x3_without_an_owner_history_is_refused() {
    let model = model(&["IFC2X3"]);
    let required = |entity| ElementTypeError::AuthoringRequired {
        entity,
        attribute: "OwnerHistory",
        schema: SchemaVersion::Ifc2x3,
    };
    assert_eq!(
        refused(&model, |tx| create_type_in(
            tx,
            &model,
            IFCBEAMTYPE,
            GUID,
            Some("BEAM"),
            named()
        )),
        required("IFCBEAMTYPE")
    );
    assert_eq!(
        refused(&model, |tx| create_supertype_in(
            tx,
            &model,
            TYPE_PRODUCT,
            GUID,
            "P",
            SupertypeDraft::default()
        )),
        required("IFCTYPEPRODUCT")
    );
}

#[test]
fn a_wrong_type_or_missing_owner_history_is_refused() {
    for (schema, _) in RELEASES {
        let model = model(&[schema]);
        let author = |tx: &mut Transaction, owner| {
            create_type_with_owner_history(
                tx,
                &model,
                IFCBEAMTYPE,
                GUID,
                Some("BEAM"),
                named(),
                owner,
            )
        };
        let error = refused(&model, |tx| author(tx, WALL));
        assert!(
            matches!(
                &error,
                ElementTypeError::Invalid {
                    attribute: "OwnerHistory",
                    value,
                    ..
                } if value.contains("IFCWALL")
            ),
            "{schema}: {error:?}"
        );
        assert_eq!(
            refused(&model, |tx| author(tx, EntityId(999))),
            ElementTypeError::MissingEntity { id: EntityId(999) }
        );
        let error = refused(&model, |tx| {
            create_supertype_with_owner_history(
                tx,
                &model,
                TYPE_PRODUCT,
                GUID,
                "P",
                SupertypeDraft::default(),
                WALL,
            )
        });
        assert!(
            matches!(error, ElementTypeError::Invalid { .. }),
            "{error:?}"
        );
    }
}

/// A type or token the bound release does not declare is refused, even
/// with an owner history; a header without `FILE_SCHEMA` binds IFC4.
#[test]
fn what_the_release_lacks_is_refused() {
    let absent = |schema: &[&str], kind: ElementType, token, version| {
        let model = model(schema);
        let error = refused(&model, |tx| {
            create_type_with_owner_history(tx, &model, kind, GUID, token, named(), OWNER)
        });
        assert_eq!(
            error,
            ElementTypeError::EntityNotInSchema {
                entity: kind.type_name,
                schema: version,
            }
        );
    };
    absent(
        &["IFC2X3"],
        IFCDOORTYPE,
        Some("DOOR"),
        SchemaVersion::Ifc2x3,
    );
    absent(
        &["IFC2X3"],
        IFCLABORRESOURCETYPE,
        Some("CARPENTRY"),
        SchemaVersion::Ifc2x3,
    );
    absent(&["IFC4"], IFCBEARINGTYPE, Some("POT"), SchemaVersion::Ifc4);
    absent(&[], IFCBEARINGTYPE, Some("POT"), SchemaVersion::Ifc4);
    let model = model(&["IFC4"]);
    assert_eq!(
        refused(&model, |tx| create_supertype_in(
            tx,
            &model,
            BUILT_ELEMENT_TYPE,
            GUID,
            "B",
            SupertypeDraft::default()
        )),
        ElementTypeError::EntityNotInSchema {
            entity: "IFCBUILTELEMENTTYPE",
            schema: SchemaVersion::Ifc4,
        }
    );

    // A token from the IFC4X3 enumeration that IFC4 does not declare, found
    // from the tables rather than a list.
    let ifc4 = table(SchemaVersion::Ifc4);
    let (kind, token) = ALL
        .iter()
        .filter(|kind| declares(ifc4, kind.type_name))
        .find_map(|kind| {
            let declared = members(ifc4, kind.type_name)?;
            let only = kind
                .members
                .iter()
                .find(|m| !declared.iter().any(|t| t == *m))?;
            Some((*kind, *only))
        })
        .expect("an IFC4X3-only token on a type IFC4 declares");
    let error = refused(&model, |tx| {
        create_type_in(tx, &model, kind, GUID, Some(token), named())
    });
    assert_eq!(
        error,
        ElementTypeError::Invalid {
            entity: kind.type_name,
            attribute: "PredefinedType",
            value: token.into(),
        }
    );
}

#[test]
fn multiple_or_unknown_schemas_are_refused() {
    let author = |model: &Model| {
        refused(model, |tx| {
            create_type_in(tx, model, IFCBEAMTYPE, GUID, Some("BEAM"), named())
        })
    };
    assert_eq!(
        author(&model(&["IFC4", "IFC2X3"])),
        ElementTypeError::MultipleSchemas { schemas: 2 }
    );
    assert_eq!(
        author(&model(&["IFC9"])),
        ElementTypeError::UnsupportedSchema {
            schema: "IFC9".into()
        }
    );
}
