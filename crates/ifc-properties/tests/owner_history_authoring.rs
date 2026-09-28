//! `IfcRoot` writers and the release's `OwnerHistory` rule (#191).
//!
//! From the EXPRESS sources: `IfcRoot.OwnerHistory` is `IfcOwnerHistory` in
//! IFC2X3 TC1 and `OPTIONAL IfcOwnerHistory` in IFC4 ADD2 TC1 and IFC4X3
//! ADD2. A writer that leaves it `$` must refuse an IFC2X3 model; the
//! `*_with_owner_history` variants take one from the caller, which must be an
//! `IfcOwnerHistory`. IFC4 and IFC4X3 output of the old writers is
//! unchanged.

use ifc_model::{Codec, Entity, EntityId, Model, Transaction, Value};
use ifc_properties::{
    add_element_quantity, add_element_quantity_with_owner_history, add_property_set,
    add_property_set_with_owner_history, add_property_single_value, attach_property_set,
    attach_property_set_with_owner_history, attach_type, attach_type_with_owner_history,
    create_quantity, PropertyError, QuantityKind, SchemaVersion,
};
use ifc_schema::for_version;
use ifc_step::StepCodec;

const RELEASES: [(&str, SchemaVersion); 3] = [
    ("IFC2X3", SchemaVersion::Ifc2x3),
    ("IFC4", SchemaVersion::Ifc4),
    ("IFC4X3_ADD2", SchemaVersion::Ifc4x3),
];

/// `#5` is an `IfcOwnerHistory`, `#10` a wall (occurrence) and `#11` a
/// wall type, each with the release's arity and an owner history.
fn model(schema: &str) -> Model {
    let table = for_version(SchemaVersion::from_header_token(schema).expect("known")).unwrap();
    let root = |id: u32, entity: &str, guid: &str| {
        let tail = ",$".repeat(table.attributes(entity).len() - 2);
        format!("#{id}={entity}('{guid}',#5{tail});")
    };
    let records = [
        "#1=IFCPERSON($,'Doe','Jane',$,$,$,$,$);".to_owned(),
        "#2=IFCORGANIZATION($,'Acme',$,$,$);".to_owned(),
        "#3=IFCPERSONANDORGANIZATION(#1,#2,$);".to_owned(),
        "#4=IFCAPPLICATION(#2,'1.0','Test','test');".to_owned(),
        "#5=IFCOWNERHISTORY(#3,#4,$,.NOCHANGE.,$,$,$,1700000000);".to_owned(),
        root(10, "IFCWALL", "1xS3BCk291UvhgP2dvNsgp"),
        root(11, "IFCWALLTYPE", "2xS3BCk291UvhgP2dvNsgp"),
    ];
    let text = format!(
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
         FILE_NAME('','',(''),(''),'','','');\nFILE_SCHEMA(('{schema}'));\nENDSEC;\n\
         DATA;\n{}\nENDSEC;\nEND-ISO-10303-21;\n",
        records.join("\n")
    );
    StepCodec.read_bytes(text.as_bytes()).expect("parses")
}

const OWNER: EntityId = EntityId(5);
const WALL: EntityId = EntityId(10);
const WALL_TYPE: EntityId = EntityId(11);
const G1: &str = "0YvctVUKr0kugbFTf53O08";
const G2: &str = "0YvctVUKr0kugbFTf53O09";

/// Refused before anything is staged.
fn refused(
    model: &Model,
    author: impl Fn(&mut Transaction) -> Result<EntityId, PropertyError>,
) -> PropertyError {
    let mut tx = Transaction::new(model);
    let error = author(&mut tx).expect_err("refused");
    assert!(tx.is_empty(), "a refusal staged {:?}", tx.edits());
    error
}

fn required(entity: &'static str) -> PropertyError {
    PropertyError::AuthoringRequired {
        entity,
        attribute: "OwnerHistory",
        schema: SchemaVersion::Ifc2x3,
    }
}

#[test]
fn ifc2x3_is_refused_without_an_owner_history() {
    let model = model("IFC2X3");
    let set = EntityId(99);
    assert_eq!(
        refused(&model, |tx| attach_property_set(
            tx,
            &model,
            G1,
            &[WALL],
            set
        )),
        required("IFCRELDEFINESBYPROPERTIES")
    );
    assert_eq!(
        refused(&model, |tx| attach_type(tx, &model, G1, &[WALL], WALL_TYPE)),
        required("IFCRELDEFINESBYTYPE")
    );
}

/// Author a set, a quantity set and both relationships with an owner
/// history, commit, and check every `IfcRoot` written points at it and
/// has its release's arity.
#[test]
fn every_release_accepts_an_owner_history() {
    for (schema, version) in RELEASES {
        let mut model = model(schema);
        let table = for_version(version).unwrap();
        let mut tx = Transaction::new(&model);
        let property = add_property_single_value(&mut tx, "P", None, None, None).expect("p");
        let pset = add_property_set_with_owner_history(
            &mut tx,
            &model,
            G1,
            "Pset_T",
            Some("described"),
            &[("P", property)],
            OWNER,
        )
        .expect(schema);
        let area = create_quantity(&mut tx, &model, QuantityKind::Area, "A", 1.5).expect("q");
        let qto = add_element_quantity_with_owner_history(
            &mut tx,
            &model,
            G2,
            "Qto_T",
            None,
            &[area],
            OWNER,
        )
        .expect(schema);
        tx.commit(&mut model).expect("commit");

        let mut tx = Transaction::new(&model);
        let rels = [
            attach_property_set_with_owner_history(&mut tx, &model, G1, &[WALL], pset, OWNER),
            attach_property_set_with_owner_history(&mut tx, &model, G2, &[WALL], qto, OWNER),
            attach_type_with_owner_history(&mut tx, &model, G1, &[WALL], WALL_TYPE, OWNER),
        ]
        .map(|rel| rel.expect(schema));
        tx.commit(&mut model).expect("commit");

        for id in [pset, qto].into_iter().chain(rels) {
            let record = model.get(id).expect("written");
            let declared = table.attribute_names(&record.type_name);
            assert_eq!(
                record.attributes.len(),
                declared.len(),
                "{schema}: {record:?}"
            );
            assert_eq!(declared[1], "OwnerHistory", "{schema}");
            assert_eq!(
                record.attributes[1],
                Value::Ref(OWNER),
                "{schema}: {record:?}"
            );
        }
        let description = table
            .attribute_names("IFCPROPERTYSET")
            .iter()
            .position(|name| *name == "Description")
            .unwrap();
        assert_eq!(
            model.get(pset).unwrap().attributes[description],
            Value::Text("described".into())
        );
    }
}

/// An owner history staged in the same transaction is accepted: it is
/// usually built right before the records that point at it.
#[test]
fn a_staged_owner_history_is_accepted() {
    let model = model("IFC2X3");
    let mut tx = Transaction::new(&model);
    let staged = tx.create(model.get(OWNER).unwrap().clone());
    let area = create_quantity(&mut tx, &model, QuantityKind::Area, "A", 1.0).expect("q");
    add_element_quantity_with_owner_history(&mut tx, &model, G1, "Qto", None, &[area], staged)
        .expect("a staged IfcOwnerHistory");
}

#[test]
fn a_wrong_type_or_missing_owner_history_is_refused() {
    for (schema, _) in RELEASES {
        let model = model(schema);
        let error = refused(&model, |tx| {
            attach_type_with_owner_history(tx, &model, G1, &[WALL], WALL_TYPE, WALL)
        });
        assert!(
            matches!(
                &error,
                PropertyError::AuthoringInvalid {
                    entity: "IFCRELDEFINESBYTYPE",
                    attribute: "OwnerHistory",
                    value,
                } if value.contains("IFCWALL")
            ),
            "{schema}: {error:?}"
        );
        let error = refused(&model, |tx| {
            add_property_set_with_owner_history(tx, &model, G1, "P", None, &[("P", WALL)], WALL)
        });
        assert!(
            matches!(
                error,
                PropertyError::AuthoringInvalid {
                    attribute: "OwnerHistory",
                    ..
                }
            ),
            "{schema}: {error:?}"
        );
        let missing = EntityId(404);
        assert_eq!(
            refused(&model, |tx| {
                attach_property_set_with_owner_history(tx, &model, G1, &[WALL], WALL, missing)
            }),
            PropertyError::MissingEntity { id: missing },
            "{schema}"
        );
    }
}

/// `add_property_set` takes no model and writes `$`; in IFC2X3 the attach
/// is where that set is caught, since it would make the file invalid.
#[test]
fn ifc2x3_refuses_to_attach_a_set_without_an_owner_history() {
    let mut model = model("IFC2X3");
    let mut tx = Transaction::new(&model);
    let property = add_property_single_value(&mut tx, "P", None, None, None).expect("p");
    let set = add_property_set(&mut tx, G1, "Pset_T", None, &[("P", property)]).expect("set");
    tx.commit(&mut model).expect("commit");
    let error = refused(&model, |tx| {
        attach_property_set_with_owner_history(tx, &model, G2, &[WALL], set, OWNER)
    });
    assert!(
        matches!(
            error,
            PropertyError::AuthoringInvalid {
                attribute: "RelatingPropertyDefinition",
                ..
            }
        ),
        "{error:?}"
    );
    // The same set is accepted in IFC4, where OwnerHistory is optional.
    let mut model = self::model("IFC4");
    let mut tx = Transaction::new(&model);
    let property = add_property_single_value(&mut tx, "P", None, None, None).expect("p");
    let set = add_property_set(&mut tx, G1, "Pset_T", None, &[("P", property)]).expect("set");
    tx.commit(&mut model).expect("commit");
    let mut tx = Transaction::new(&model);
    attach_property_set_with_owner_history(&mut tx, &model, G2, &[WALL], set, OWNER)
        .expect("IFC4 OwnerHistory is optional");
}

/// The writers that leave `OwnerHistory` unset write exactly what they
/// wrote before in IFC4 and IFC4X3.
#[test]
fn ifc4_and_ifc4x3_output_is_unchanged() {
    for schema in ["IFC4", "IFC4X3_ADD2"] {
        let mut model = model(schema);
        let mut tx = Transaction::new(&model);
        let area = create_quantity(&mut tx, &model, QuantityKind::Area, "A", 1.0).expect("q");
        let qto = add_element_quantity(&mut tx, G1, "Qto", None, &[area]).expect("qto");
        let rel = attach_property_set(&mut tx, &model, G2, &[WALL], qto).expect("attach");
        let typed = attach_type(&mut tx, &model, G2, &[WALL], WALL_TYPE).expect("type");
        tx.commit(&mut model).expect("commit");
        let text = |id| Value::Text(id);
        assert_eq!(
            model.get(rel).unwrap(),
            &Entity::new(
                "IFCRELDEFINESBYPROPERTIES",
                vec![
                    text(G2.into()),
                    Value::Null,
                    Value::Null,
                    Value::Null,
                    Value::List(vec![Value::Ref(WALL)]),
                    Value::Ref(qto),
                ]
            ),
            "{schema}"
        );
        assert_eq!(
            model.get(typed).unwrap(),
            &Entity::new(
                "IFCRELDEFINESBYTYPE",
                vec![
                    text(G2.into()),
                    Value::Null,
                    Value::Null,
                    Value::Null,
                    Value::List(vec![Value::Ref(WALL)]),
                    Value::Ref(WALL_TYPE),
                ]
            ),
            "{schema}"
        );
    }
}

/// The attach writers now bind the release like the quantity writers: a
/// model declaring several schemas or an unknown one is refused.
#[test]
fn a_model_without_one_known_release_is_refused() {
    let mut several = model("IFC4");
    several.header_mut().schema = vec!["IFC4".to_owned(), "IFC2X3".to_owned()];
    assert_eq!(
        refused(&several, |tx| attach_property_set(
            tx,
            &several,
            G1,
            &[WALL],
            WALL
        )),
        PropertyError::MultipleSchemas { schemas: 2 }
    );
    let mut unknown = model("IFC4");
    unknown.header_mut().schema = vec!["IFC5".to_owned()];
    assert_eq!(
        refused(&unknown, |tx| attach_type(
            tx,
            &unknown,
            G1,
            &[WALL],
            WALL_TYPE
        )),
        PropertyError::UnsupportedSchema {
            schema: "IFC5".to_owned()
        }
    );
}
