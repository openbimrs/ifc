//! #202: type definitions written in the model's declared release.
//!
//! From the EXPRESS sources: `IfcRoot.OwnerHistory` is `IfcOwnerHistory` in
//! IFC2X3 TC1 and `OPTIONAL IfcOwnerHistory` in IFC4 ADD2 TC1 and IFC4X3
//! ADD2, and the three releases declare different type lists, layouts and
//! `PredefinedType` enumerations. The catalogue is IFC4X3's, so these walk
//! every row in every release, not one type. `openbim-ifc`'s
//! `release_bound_owner_history_authoring` runs the whole catalogue
//! through `ifc-validate`; the refusals are in `release_refusals.rs`.

mod release_support;

use ifc_element_type::table::IFCWALLTYPE;
use ifc_element_type::{
    create_supertype, create_supertype_with_owner_history, create_type, create_type_in,
    create_type_with_owner_history, ElementTypeError, Family, SupertypeDraft, TypeDraft, ALL,
    ALL_SUPERTYPES, BUILT_ELEMENT_TYPE, TYPE_PRODUCT,
};
use ifc_model::{Codec, Model, Transaction, Value};
use ifc_schema::SchemaVersion;
use ifc_step::StepCodec;
use release_support::{
    declares, members, model, named, slot, staged, table, token, GUID, OWNER, RELEASES,
};

/// Every catalogue row, in every release, with an owner history: a row the
/// release declares is written with that release's arity, owner history
/// and token; a row it lacks is `EntityNotInSchema`; and the only other
/// refusal is a required attribute the draft cannot carry.
#[test]
fn the_whole_catalogue_is_written_in_every_release() {
    for (schema, version) in RELEASES {
        let table = table(version);
        let model = model(&[schema]);
        let (mut written, mut absent, mut required) = (0, 0, Vec::new());
        for kind in ALL {
            let entity = kind.type_name;
            let token = token(table, *kind);
            let mut tx = Transaction::new(&model);
            let result = create_type_with_owner_history(
                &mut tx,
                &model,
                *kind,
                GUID,
                token.as_deref(),
                named(),
                OWNER,
            );
            match result {
                Ok(_) => {
                    assert!(declares(table, entity), "{schema}: {entity} written");
                    let record = staged(&tx);
                    assert_eq!(record.attributes.len(), table.attributes(entity).len());
                    let at = |name| slot(table, entity, name);
                    assert_eq!(record.attributes[at("OwnerHistory")], Value::Ref(OWNER));
                    let slot7 = match kind.family {
                        Family::Element => "Tag",
                        Family::ResourceOrProcess => "LongDescription",
                    };
                    assert_eq!(record.attributes[at(slot7)], Value::Text("T-1".into()));
                    if let Some(token) = &token {
                        assert_eq!(
                            record.attributes[at("PredefinedType")],
                            Value::Enum(token.as_str().into()),
                            "{schema}: {entity}"
                        );
                    }
                    written += 1;
                }
                Err(ElementTypeError::EntityNotInSchema {
                    entity: e,
                    schema: s,
                }) => {
                    assert_eq!((e, s), (entity, version));
                    assert!(!declares(table, entity), "{schema}: {entity} refused");
                    absent += 1;
                }
                Err(ElementTypeError::AuthoringRequired {
                    entity: e,
                    attribute,
                    schema: s,
                }) => {
                    assert_eq!((e, s), (entity, version), "{attribute}");
                    assert_ne!(attribute, "OwnerHistory");
                    assert!(table
                        .attributes(entity)
                        .iter()
                        .any(|found| found.name == attribute && !found.optional));
                    required.push(format!("{entity}.{attribute}"));
                }
                Err(other) => panic!("{schema}: {entity}: {other:?}"),
            }
            assert!(tx.edits().len() <= 1, "{schema}: {entity}");
        }
        assert_eq!(written + absent + required.len(), ALL.len());
        // Minimum counts, so a sweep that silently matched nothing fails.
        // Minimum counts, so a sweep that silently matched nothing fails.
        let (min_written, min_absent) = match version {
            SchemaVersion::Ifc4x3 => (ALL.len() - 4, 0),
            SchemaVersion::Ifc4 => (100, 15),
            SchemaVersion::Ifc2x3 => (65, 50),
        };
        assert!(
            written >= min_written && absent >= min_absent,
            "{schema}: {written} written, {absent} absent"
        );
        // Required attributes `TypeDraft` has no field for. They are
        // refused, never written `$`; `create_type`, which takes no model,
        // still writes them `$` as it did before #202.
        let expected: &[&str] = if version == SchemaVersion::Ifc2x3 {
            &["IFCFURNITURETYPE.AssemblyPlace"]
        } else {
            &[
                "IFCDOORTYPE.OperationType",
                "IFCEVENTTYPE.EventTriggerType",
                "IFCFURNITURETYPE.AssemblyPlace",
                "IFCWINDOWTYPE.PartitioningType",
            ]
        };
        assert_eq!(required, expected, "{schema}");
    }
}

/// The IFC4X3 catalogue's tokens and arities are the IFC4X3 table's.
#[test]
fn catalogue_rows_are_the_ifc4x3_tables() {
    let table = table(SchemaVersion::Ifc4x3);
    for kind in ALL {
        let expected: Vec<String> = kind.members.iter().map(|m| (*m).to_owned()).collect();
        assert_eq!(members(table, kind.type_name), Some(expected));
        assert_eq!(table.attributes(kind.type_name).len(), kind.arity);
    }
}

/// `create_type`, which takes no model, still writes the record the
/// catalogue row laid out before #202; and `create_type_in` writes the
/// same record into an IFC4X3 model, and into an IFC4 model wherever IFC4
/// declares the type with the same layout and token.
#[test]
fn ifc4x3_and_ifc4_output_is_unchanged() {
    let ifc4x3 = model(&["IFC4X3_ADD2"]);
    let ifc4 = model(&["IFC4"]);
    let mut compared = [0, 0];
    for kind in ALL {
        let token = if kind.predefined_optional {
            None
        } else {
            Some(kind.members[0])
        };
        let mut draft = TypeDraft {
            name: Some("N"),
            description: Some("D"),
            applicable_occurrence: Some("A"),
            tag_or_long_description: Some("T"),
            ..TypeDraft::default()
        };
        if token == Some("USERDEFINED") {
            draft.fallback = Some("F");
        }
        let mut before = vec![Value::Null; kind.arity];
        before[0] = Value::Text(GUID.into());
        before[2] = Value::Text("N".into());
        before[3] = Value::Text("D".into());
        before[4] = Value::Text("A".into());
        before[7] = Value::Text("T".into());
        before[kind.fallback_slot] = draft
            .fallback
            .map_or(Value::Null, |f| Value::Text(f.into()));
        before[kind.predefined_slot] = token.map_or(Value::Null, |t| Value::Enum(t.into()));

        let mut tx = Transaction::new(&ifc4x3);
        create_type(&mut tx, *kind, GUID, token, draft).expect(kind.type_name);
        assert_eq!(staged(&tx).attributes, before, "{}", kind.type_name);

        for (index, model) in [&ifc4x3, &ifc4].into_iter().enumerate() {
            let version = if index == 0 {
                SchemaVersion::Ifc4x3
            } else {
                SchemaVersion::Ifc4
            };
            let bound = table(version);
            let same = declares(bound, kind.type_name)
                && bound.attribute_names(kind.type_name)
                    == table(SchemaVersion::Ifc4x3).attribute_names(kind.type_name)
                && token.is_none_or(|t| {
                    members(bound, kind.type_name).is_some_and(|m| m.iter().any(|x| x == t))
                });
            if !same {
                continue;
            }
            let mut tx = Transaction::new(model);
            match create_type_in(&mut tx, model, *kind, GUID, token, draft) {
                Ok(_) => {}
                // The four types with a required attribute `TypeDraft`
                // cannot carry (see the sweep above): refused, not `$`.
                Err(ElementTypeError::AuthoringRequired { .. }) => continue,
                Err(other) => panic!("{}: {other:?}", kind.type_name),
            }
            assert_eq!(
                staged(&tx).attributes,
                before,
                "{version:?}: {}",
                kind.type_name
            );
            compared[index] += 1;
        }
    }
    assert_eq!(compared[0], ALL.len() - 4);
    assert!(compared[1] >= 80, "{compared:?}");
}

/// Every supertype in every release: written where the release declares
/// it instantiable, `EntityNotInSchema` where it does not.
#[test]
fn every_supertype_is_written_in_every_release() {
    for (schema, version) in RELEASES {
        let table = table(version);
        let model = model(&[schema]);
        let mut written = 0;
        for kind in ALL_SUPERTYPES {
            let mut tx = Transaction::new(&model);
            let result = create_supertype_with_owner_history(
                &mut tx,
                &model,
                *kind,
                GUID,
                "Sweep",
                SupertypeDraft::default(),
                OWNER,
            );
            if declares(table, kind.type_name) {
                result.expect(kind.type_name);
                let record = staged(&tx);
                assert_eq!(
                    record.attributes.len(),
                    table.attributes(kind.type_name).len()
                );
                let at = slot(table, kind.type_name, "OwnerHistory");
                assert_eq!(record.attributes[at], Value::Ref(OWNER));
                written += 1;
            } else {
                assert_eq!(
                    result,
                    Err(ElementTypeError::EntityNotInSchema {
                        entity: kind.type_name,
                        schema: version,
                    })
                );
            }
        }
        assert!(written >= 3, "{schema}: {written}");
    }
    // Unchanged without a model: the IFC4X3 layout, as before.
    let model = Model::new();
    let mut tx = Transaction::new(&model);
    create_supertype(
        &mut tx,
        BUILT_ELEMENT_TYPE,
        GUID,
        "B",
        SupertypeDraft::default(),
    )
    .expect("IFC4X3 layout");
    assert_eq!(staged(&tx).attributes.len(), BUILT_ELEMENT_TYPE.arity);
}

/// A sample per release written, read back with `ifc-step`, and read by
/// attribute name from that release's table.
#[test]
fn a_sample_round_trips_in_every_release() {
    for (schema, version) in RELEASES {
        let table = table(version);
        let mut model = model(&[schema]);
        let mut tx = Transaction::new(&model);
        let wall = create_type_with_owner_history(
            &mut tx,
            &model,
            IFCWALLTYPE,
            GUID,
            Some("STANDARD"),
            named(),
            OWNER,
        )
        .expect(schema);
        let product = create_supertype_with_owner_history(
            &mut tx,
            &model,
            TYPE_PRODUCT,
            "2hqA$FMcT8$hVvcqsRDBzZ",
            "Product",
            SupertypeDraft {
                tag: Some("P-1"),
                ..SupertypeDraft::default()
            },
            OWNER,
        )
        .expect(schema);
        tx.commit(&mut model).expect("commit");
        let bytes = StepCodec.write_bytes(&model).expect("written");
        let back = StepCodec.read_bytes(&bytes).expect("read back");
        assert!(back.diagnostics().is_empty(), "{:?}", back.diagnostics());
        for (id, entity) in [(wall, "IFCWALLTYPE"), (product, "IFCTYPEPRODUCT")] {
            let record = back.get(id).expect("read back");
            assert_eq!(record.attributes.len(), table.attributes(entity).len());
            assert_eq!(
                record.attributes[slot(table, entity, "OwnerHistory")],
                Value::Ref(OWNER)
            );
        }
        assert_eq!(
            back.get(wall).unwrap().attributes[slot(table, "IFCWALLTYPE", "PredefinedType")],
            Value::Enum("STANDARD".into())
        );
    }
}
