//! What a release cannot hold is refused with a typed error, staging
//! nothing (#202).

mod release_fixture;

use ifc_model::{EntityId, Model, Transaction};
use ifc_schema::SchemaVersion;
use ifc_spatial::authoring::{adhere_to_element_with_owner_history, declare_with_owner_history};
use ifc_spatial::facility::IFCBRIDGE;
use ifc_spatial::{
    aggregate_with_owner_history, create_external_spatial_element_with_owner_history,
    create_facility_with_owner_history, create_project_with_owner_history, create_space_boundary,
    create_space_boundary_with_owner_history, create_spatial_element_with_owner_history,
    BoundaryDraft, BoundaryLevel, ExternalSpatialDraft, FacilityDraft, FacilityError,
    SpatialAuthoringError, SpatialDraft, SpatialKind,
};
use release_fixture::{base, guid, CONTEXT, FEATURE, OWNER, SPACE, UNITS, WALL, WALL2, WALL_TYPE};

use SpatialAuthoringError as E;

fn refused<T: std::fmt::Debug, Err>(tx: &Transaction, result: Result<T, Err>) -> Err {
    assert!(tx.is_empty(), "a refusal staged {:?}", tx.edits());
    result.expect_err("refused")
}

fn boundary(internal: &'static str) -> BoundaryDraft<'static> {
    BoundaryDraft::new(SPACE, WALL, "PHYSICAL", internal)
}

/// #202: the writer that leaves `OwnerHistory` unset used to write `$`
/// into IFC2X3, where it is required.
#[test]
fn ifc2x3_space_boundary_without_owner_history_is_refused() {
    let model = base("IFC2X3", SchemaVersion::Ifc2x3);
    let mut tx = Transaction::new(&model);
    let result = create_space_boundary(
        &mut tx,
        &model,
        BoundaryLevel::Base,
        &guid(1),
        boundary("INTERNAL"),
    );
    assert_eq!(
        refused(&tx, result),
        E::AuthoringRequired {
            entity: "IFCRELSPACEBOUNDARY",
            attribute: "OwnerHistory",
            schema: SchemaVersion::Ifc2x3,
        }
    );
}

#[test]
fn ifc2x3_refuses_what_it_cannot_hold() {
    let v = SchemaVersion::Ifc2x3;
    let model = base("IFC2X3", v);
    let mut tx = Transaction::new(&model);
    let t = &mut tx;
    let absent = |entity| E::EntityNotInSchema { entity, schema: v };
    let required = |entity, attribute| E::AuthoringRequired {
        entity,
        attribute,
        schema: v,
    };
    // IFC2X3 has no `IfcRelDeclares`; it is refused before any reference
    // is resolved, so the relating id is immaterial.
    let result = declare_with_owner_history(t, &model, &guid(1), SPACE, &[WALL_TYPE], OWNER);
    assert_eq!(refused(t, result), absent("IFCRELDECLARES"));
    let result = create_space_boundary_with_owner_history(
        t,
        &model,
        BoundaryLevel::First,
        &guid(2),
        boundary("INTERNAL"),
        OWNER,
    );
    assert_eq!(refused(t, result), absent("IFCRELSPACEBOUNDARY1STLEVEL"));
    let result = create_external_spatial_element_with_owner_history(
        t,
        &model,
        &guid(3),
        ExternalSpatialDraft::default(),
        OWNER,
    );
    assert_eq!(refused(t, result), absent("IFCEXTERNALSPATIALELEMENT"));
    let result = create_space_boundary_with_owner_history(
        t,
        &model,
        BoundaryLevel::Base,
        &guid(4),
        boundary("EXTERNAL_EARTH"),
        OWNER,
    );
    assert_eq!(
        refused(t, result),
        E::AuthoringValueType {
            entity: "IFCRELSPACEBOUNDARY",
            attribute: "InternalOrExternalBoundary",
            declared: "IfcInternalOrExternalEnum",
            schema: v,
        }
    );
    let unset = SpatialDraft::new().name("Site");
    let result = create_spatial_element_with_owner_history(
        t,
        &model,
        SpatialKind::Site,
        &guid(5),
        unset,
        OWNER,
    );
    assert_eq!(refused(t, result), required("IFCSITE", "CompositionType"));
    let composed = unset.composition("ELEMENT");
    let result = create_spatial_element_with_owner_history(
        t,
        &model,
        SpatialKind::Space,
        &guid(6),
        composed,
        OWNER,
    );
    assert_eq!(
        refused(t, result),
        required("IFCSPACE", "InteriorOrExteriorSpace")
    );
    let result =
        create_project_with_owner_history(t, &model, &guid(7), None, Some(UNITS), &[], OWNER);
    assert_eq!(
        refused(t, result),
        required("IFCPROJECT", "RepresentationContexts")
    );
}

/// The IFC4X3-only relationships and facilities are refused in IFC4.
#[test]
fn ifc4_refuses_ifc4x3_entities() {
    let v = SchemaVersion::Ifc4;
    let model = base("IFC4", v);
    let mut tx = Transaction::new(&model);
    let result =
        adhere_to_element_with_owner_history(&mut tx, &model, &guid(1), WALL, &[FEATURE], OWNER);
    assert_eq!(
        refused(&tx, result),
        E::EntityNotInSchema {
            entity: "IFCRELADHERESTOELEMENT",
            schema: v
        }
    );
    let result = create_facility_with_owner_history(
        &mut tx,
        &model,
        IFCBRIDGE,
        &guid(2),
        None,
        FacilityDraft::default(),
        OWNER,
    );
    assert_eq!(
        refused(&tx, result),
        FacilityError::Authoring(E::EntityNotInSchema {
            entity: "IFCBRIDGE",
            schema: v
        })
    );
}

/// The owner history must exist and be an `IfcOwnerHistory`; the header
/// must bind exactly one known release.
#[test]
fn owner_history_and_binding_are_checked() {
    let model = base("IFC2X3", SchemaVersion::Ifc2x3);
    let mut tx = Transaction::new(&model);
    let aggregate = |tx: &mut Transaction, model: &Model, owner: EntityId| {
        aggregate_with_owner_history(tx, model, &guid(1), WALL, &[WALL2], owner)
    };
    let result = aggregate(&mut tx, &model, WALL2);
    assert_eq!(
        refused(&tx, result),
        E::WrongReferenceType {
            entity: "IFCRELAGGREGATES",
            attribute: "OwnerHistory",
            target: WALL2,
            actual: "IFCWALL".into(),
            expected: "IFCOWNERHISTORY",
        }
    );
    let result = aggregate(&mut tx, &model, EntityId(999));
    assert_eq!(
        refused(&tx, result),
        E::MissingReference {
            entity: "IFCRELAGGREGATES",
            attribute: "OwnerHistory",
            target: EntityId(999),
        }
    );
    let mut several = model.clone();
    several.header_mut().schema = vec!["IFC4".into(), "IFC2X3".into()];
    let result = aggregate(&mut tx, &several, OWNER);
    assert_eq!(refused(&tx, result), E::MultipleSchemas { schemas: 2 });
    let mut unknown = model.clone();
    unknown.header_mut().schema = vec!["IFC5".into()];
    let result = aggregate(&mut tx, &unknown, OWNER);
    assert_eq!(
        refused(&tx, result),
        E::UnsupportedSchema {
            schema: "IFC5".into()
        }
    );
    let result = create_space_boundary(
        &mut tx,
        &unknown,
        BoundaryLevel::Base,
        &guid(2),
        boundary("INTERNAL"),
    );
    assert_eq!(
        refused(&tx, result),
        E::UnsupportedSchema {
            schema: "IFC5".into()
        }
    );
    aggregate(&mut tx, &model, OWNER).expect("an owned IFC2X3 aggregation");
}

/// #214: `InteriorOrExteriorSpace` is IFC2X3's, and only a space's. A value
/// anywhere else is refused, never dropped.
#[test]
fn interior_or_exterior_is_refused_where_the_release_does_not_declare_it() {
    let internal = SpatialDraft::new()
        .name("Room")
        .interior_or_exterior("INTERNAL");
    // IFC4 and IFC4X3 declare no `InteriorOrExteriorSpace`.
    for (schema, version) in [
        ("IFC4", SchemaVersion::Ifc4),
        ("IFC4X3_ADD2", SchemaVersion::Ifc4x3),
    ] {
        let model = base(schema, version);
        let mut tx = Transaction::new(&model);
        let result = create_spatial_element_with_owner_history(
            &mut tx,
            &model,
            SpatialKind::Space,
            &guid(1),
            internal,
            OWNER,
        );
        assert_eq!(
            refused(&tx, result),
            E::AuthoringNotInSchema {
                entity: "IFCSPACE",
                attribute: "InteriorOrExteriorSpace",
                schema: version,
            },
            "{schema}"
        );
    }
    let v = SchemaVersion::Ifc2x3;
    let model = base("IFC2X3", v);
    let mut tx = Transaction::new(&model);
    // IFC2X3 declares it on `IfcSpace` alone.
    let result = create_spatial_element_with_owner_history(
        &mut tx,
        &model,
        SpatialKind::Site,
        &guid(2),
        internal.composition("ELEMENT"),
        OWNER,
    );
    assert_eq!(
        refused(&tx, result),
        E::AuthoringNotInSchema {
            entity: "IFCSITE",
            attribute: "InteriorOrExteriorSpace",
            schema: v,
        }
    );
    // A token outside IFC2X3's `IfcInternalOrExternalEnum`.
    let earth = SpatialDraft::new()
        .name("Room")
        .composition("ELEMENT")
        .interior_or_exterior("EXTERNAL_EARTH");
    let result = create_spatial_element_with_owner_history(
        &mut tx,
        &model,
        SpatialKind::Space,
        &guid(3),
        earth,
        OWNER,
    );
    assert_eq!(
        refused(&tx, result),
        E::AuthoringValueType {
            entity: "IFCSPACE",
            attribute: "InteriorOrExteriorSpace",
            declared: "IfcInternalOrExternalEnum",
            schema: v,
        }
    );
    // The plain writer writes IFC4/IFC4X3 only and cannot hold it either.
    let result =
        ifc_spatial::create_spatial_element(&mut tx, SpatialKind::Space, &guid(4), internal);
    assert_eq!(
        refused(&tx, result),
        E::AuthoringNotInSchema {
            entity: "IFCSPACE",
            attribute: "InteriorOrExteriorSpace",
            schema: SchemaVersion::Ifc4,
        }
    );
}

/// #214: each representation context must resolve, be an
/// `IfcRepresentationContext`, not be a sub-context (IFC2X3 `WR32`, IFC4 and
/// IFC4X3 `CorrectContext`), and appear once in the `SET`; IFC2X3 also
/// requires `UnitsInContext`.
#[test]
fn project_contexts_are_checked() {
    for (schema, version) in [
        ("IFC2X3", SchemaVersion::Ifc2x3),
        ("IFC4", SchemaVersion::Ifc4),
    ] {
        let mut model = base(schema, version);
        let mut setup = Transaction::new(&model);
        let sub = setup.create(ifc_model::Entity::new(
            "IFCGEOMETRICREPRESENTATIONSUBCONTEXT",
            vec![ifc_model::Value::Null; 10],
        ));
        setup.commit(&mut model).expect("commit");
        let mut tx = Transaction::new(&model);
        let model = &model;
        let project = |tx: &mut Transaction, contexts: &[EntityId]| {
            create_project_with_owner_history(
                tx,
                model,
                &guid(1),
                Some("P"),
                Some(UNITS),
                contexts,
                OWNER,
            )
        };
        let result = project(&mut tx, &[EntityId(999)]);
        assert_eq!(
            refused(&tx, result),
            E::MissingReference {
                entity: "IFCPROJECT",
                attribute: "RepresentationContexts",
                target: EntityId(999),
            },
            "{schema}"
        );
        let result = project(&mut tx, &[CONTEXT, WALL]);
        assert_eq!(
            refused(&tx, result),
            E::WrongReferenceType {
                entity: "IFCPROJECT",
                attribute: "RepresentationContexts",
                target: WALL,
                actual: "IFCWALL".into(),
                expected: "IFCREPRESENTATIONCONTEXT",
            },
            "{schema}"
        );
        let result = project(&mut tx, &[sub]);
        assert!(
            matches!(
                refused(&tx, result),
                E::Invalid {
                    entity: "IFCPROJECT",
                    attribute: "RepresentationContexts",
                    ..
                }
            ),
            "{schema}: a sub-context"
        );
        let result = project(&mut tx, &[CONTEXT, CONTEXT]);
        assert!(
            matches!(
                refused(&tx, result),
                E::Invalid {
                    entity: "IFCPROJECT",
                    attribute: "RepresentationContexts",
                    ..
                }
            ),
            "{schema}: a repeated context"
        );
        project(&mut tx, &[CONTEXT]).expect("one context");
    }
    let v = SchemaVersion::Ifc2x3;
    let model = base("IFC2X3", v);
    let mut tx = Transaction::new(&model);
    let result = create_project_with_owner_history(
        &mut tx,
        &model,
        &guid(2),
        Some("P"),
        None,
        &[CONTEXT],
        OWNER,
    );
    assert_eq!(
        refused(&tx, result),
        E::AuthoringRequired {
            entity: "IFCPROJECT",
            attribute: "UnitsInContext",
            schema: v,
        }
    );
}
