//! Quantity authoring writes the layout of the model's declared release and
//! refuses what that release cannot hold (#138).
//!
//! From the EXPRESS sources: IFC2X3 TC1 `IfcQuantity<Kind>` declares `Name,
//! Description, Unit, <Kind>Value` (four attributes, no `Formula`); IFC4
//! ADD2 TC1 and IFC4X3 ADD2 add `Formula` (five). Only IFC4X3 declares
//! `IfcQuantityNumber`. `IfcCountMeasure` is `NUMBER` in IFC2X3 and IFC4,
//! `INTEGER` in IFC4X3. Every round trip here goes through STEP text and
//! back, then through both the permissive and the exact view.

use ifc_model::{Codec, Entity, EntityId, Model, Transaction, Value};
use ifc_properties::{
    add_element_quantity, attach_property_set, create_quantity, create_quantity_with,
    exact_property, quantity_set, set_quantity_value, ExactResolution, PropertyError, Quantity,
    QuantityExtras, QuantityKind, SchemaVersion,
};
use ifc_step::StepCodec;

/// `(FILE_SCHEMA token, release, IfcWall arity, quantity arity)`.
const RELEASES: [(&str, SchemaVersion, usize, usize); 3] = [
    ("IFC2X3", SchemaVersion::Ifc2x3, 8, 4),
    ("IFC4", SchemaVersion::Ifc4, 9, 5),
    ("IFC4X3_ADD2", SchemaVersion::Ifc4x3, 9, 5),
];

fn declaring(schema: &str) -> Model {
    let mut model = Model::new();
    model.header_mut().schema = vec![schema.to_owned()];
    model
}

/// Author a wall carrying one quantity set of `quantities`, then write the
/// model to STEP and read it back.
fn round_trip(
    schema: &str,
    wall_arity: usize,
    author: impl Fn(&mut Transaction, &Model) -> Vec<EntityId>,
) -> Model {
    let mut model = declaring(schema);
    let mut tx = Transaction::new(&model);
    let mut wall = vec![Value::Null; wall_arity];
    wall[0] = Value::Text("1xS3BCk291UvhgP2dvNsgp".into());
    wall[2] = Value::Text("Wall".into());
    let wall = tx.create(Entity::new("IFCWALL", wall));
    let quantities = author(&mut tx, &model);
    let set = add_element_quantity(
        &mut tx,
        "0YvctVUKr0kugbFTf53O08",
        "Qto_T",
        None,
        &quantities,
    )
    .expect("quantity set");
    tx.commit(&mut model).expect("commit");
    let mut tx = Transaction::new(&model);
    attach_property_set(&mut tx, &model, "0YvctVUKr0kugbFTf53O09", &[wall], set).expect("attach");
    tx.commit(&mut model).expect("commit");

    let bytes = StepCodec.write_bytes(&model).expect("written");
    let back = StepCodec.read_bytes(&bytes).expect("read back");
    assert!(back.diagnostics().is_empty(), "{:?}", back.diagnostics());
    back
}

fn present(resolution: ExactResolution) -> ifc_properties::ExactProperty {
    match resolution {
        ExactResolution::Present(property) => property,
        other => panic!("expected a present quantity, got {other:?}"),
    }
}

#[test]
fn quantities_round_trip_in_their_release_layout() {
    for (schema, _, wall, arity) in RELEASES {
        let model = round_trip(schema, wall, |tx, model| {
            let area = create_quantity_with(
                tx,
                model,
                QuantityKind::Area,
                "GrossArea",
                12.5,
                QuantityExtras {
                    description: Some("Painted"),
                    ..QuantityExtras::default()
                },
            )
            .expect("area");
            let count =
                create_quantity(tx, model, QuantityKind::Count, "Doors", 4.0).expect("count");
            vec![area, count]
        });
        let set_id = model.ids_of_type("IFCELEMENTQUANTITY")[0];
        let (set, anomalies) = quantity_set(&model, set_id).expect("readable");
        assert!(anomalies.is_empty(), "{schema}: {anomalies:?}");
        for quantity in &set.quantities {
            let record = model.get(quantity.id()).expect("written");
            assert_eq!(record.attributes.len(), arity, "{schema}: {record:?}");
        }
        assert!(
            matches!(&set.quantities[..], [
                Quantity::Simple { kind: QuantityKind::Area, value: a, description: Some(d), .. },
                Quantity::Simple { kind: QuantityKind::Count, value: c, .. },
            ] if *a == 12.5 && &**d == "Painted" && *c == 4.0),
            "{schema}: {:?}",
            set.quantities
        );

        let wall = model.ids_of_type("IFCWALL")[0];
        let area = present(
            exact_property(&model, wall, Some("Qto_T"), "GrossArea").expect("exact resolves"),
        );
        assert_eq!(
            area.value_type.as_deref(),
            Some("IFCAREAMEASURE"),
            "{schema}"
        );
        let count =
            present(exact_property(&model, wall, Some("Qto_T"), "Doors").expect("exact resolves"));
        assert_eq!(
            count.value_type.as_deref(),
            Some("IFCCOUNTMEASURE"),
            "{schema}"
        );
    }
}

#[test]
fn a_number_round_trips_in_ifc4x3() {
    let model = round_trip("IFC4X3_ADD2", 9, |tx, model| {
        vec![create_quantity(tx, model, QuantityKind::Number, "Openings", -3.5).expect("number")]
    });
    let set_id = model.ids_of_type("IFCELEMENTQUANTITY")[0];
    let (set, anomalies) = quantity_set(&model, set_id).expect("readable");
    assert!(anomalies.is_empty(), "{anomalies:?}");
    assert!(matches!(
        &set.quantities[..],
        [Quantity::Simple { kind: QuantityKind::Number, value, .. }] if *value == -3.5
    ));
    let wall = model.ids_of_type("IFCWALL")[0];
    let number =
        present(exact_property(&model, wall, Some("Qto_T"), "Openings").expect("exact resolves"));
    assert_eq!(number.value_type.as_deref(), Some("IFCNUMERICMEASURE"));
}

/// Refused before anything is staged.
fn refused(model: &Model, author: impl Fn(&mut Transaction) -> PropertyError) -> PropertyError {
    let mut tx = Transaction::new(model);
    let error = author(&mut tx);
    assert!(tx.is_empty(), "a refusal staged {:?}", tx.edits());
    error
}

#[test]
fn a_kind_or_attribute_the_release_lacks_is_refused() {
    for (schema, version) in [
        ("IFC2X3", SchemaVersion::Ifc2x3),
        ("IFC4", SchemaVersion::Ifc4),
    ] {
        let model = declaring(schema);
        let error = refused(&model, |tx| {
            create_quantity(tx, &model, QuantityKind::Number, "N", 1.0).unwrap_err()
        });
        assert_eq!(
            error,
            PropertyError::EntityNotInSchema {
                entity: "IfcQuantityNumber",
                schema: version,
            }
        );
    }
    // A header without FILE_SCHEMA binds IFC4, which has no number either.
    let bare = Model::new();
    let error = refused(&bare, |tx| {
        create_quantity(tx, &bare, QuantityKind::Number, "N", 1.0).unwrap_err()
    });
    assert!(matches!(
        error,
        PropertyError::EntityNotInSchema {
            schema: SchemaVersion::Ifc4,
            ..
        }
    ));

    let ifc2x3 = declaring("IFC2X3");
    let formula = QuantityExtras {
        formula: Some("l * h"),
        ..QuantityExtras::default()
    };
    let error = refused(&ifc2x3, |tx| {
        create_quantity_with(tx, &ifc2x3, QuantityKind::Area, "A", 1.0, formula).unwrap_err()
    });
    assert_eq!(
        error,
        PropertyError::AuthoringNotInSchema {
            entity: "IfcQuantityArea",
            attribute: "Formula",
            schema: SchemaVersion::Ifc2x3,
        }
    );
}

#[test]
fn a_model_without_one_known_release_is_refused() {
    let mut several = Model::new();
    several.header_mut().schema = vec!["IFC4".to_owned(), "IFC2X3".to_owned()];
    let error = refused(&several, |tx| {
        create_quantity(tx, &several, QuantityKind::Length, "L", 1.0).unwrap_err()
    });
    assert_eq!(error, PropertyError::MultipleSchemas { schemas: 2 });

    let unknown = declaring("IFC5");
    let error = refused(&unknown, |tx| {
        create_quantity(tx, &unknown, QuantityKind::Length, "L", 1.0).unwrap_err()
    });
    assert_eq!(
        error,
        PropertyError::UnsupportedSchema {
            schema: "IFC5".to_owned()
        }
    );
}

#[test]
fn a_count_is_never_truncated() {
    let ifc4x3 = declaring("IFC4X3_ADD2");
    let error = refused(&ifc4x3, |tx| {
        create_quantity(tx, &ifc4x3, QuantityKind::Count, "C", 2.5).unwrap_err()
    });
    assert!(
        matches!(
            error,
            PropertyError::AuthoringInvalid {
                attribute: "CountValue",
                ..
            }
        ),
        "{error:?}"
    );
    let error = refused(&ifc4x3, |tx| {
        create_quantity(tx, &ifc4x3, QuantityKind::Length, "L", f64::NAN).unwrap_err()
    });
    assert!(
        matches!(error, PropertyError::AuthoringInvalid { .. }),
        "{error:?}"
    );

    // IFC2X3 and IFC4 declare IfcCountMeasure as NUMBER: a fraction is kept.
    for schema in ["IFC2X3", "IFC4"] {
        let mut model = declaring(schema);
        let mut tx = Transaction::new(&model);
        let count = create_quantity(&mut tx, &model, QuantityKind::Count, "C", 2.5).expect(schema);
        tx.commit(&mut model).expect("commit");
        let Value::Typed { value, .. } = &model.get(count).expect("written").attributes[3] else {
            panic!("{schema}: a typed measure");
        };
        assert_eq!(**value, Value::Real(2.5), "{schema}");
    }
}

/// Parse `records` under `schema`.
fn step(schema: &str, records: &[&str]) -> Model {
    let text = format!(
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
         FILE_NAME('','',(''),(''),'','','');\nFILE_SCHEMA(('{schema}'));\nENDSEC;\n\
         DATA;\n{}\nENDSEC;\nEND-ISO-10303-21;\n",
        records.join("\n")
    );
    StepCodec.read_bytes(text.as_bytes()).expect("parses")
}

/// `set_quantity_value` repairs a `$` value into a simple quantity.
#[test]
fn setting_a_missing_value_repairs_the_quantity() {
    for (schema, _, _, arity) in RELEASES {
        let tail = if arity == 5 { ",$" } else { "" };
        let area = format!("#10=IFCQUANTITYAREA('A',$,$,${tail});");
        let mut model = step(
            schema,
            &[
                &area,
                "#20=IFCELEMENTQUANTITY('0YvctVUKr0kugbFTf53O08',$,'Q',$,$,(#10));",
            ],
        );
        let before = quantity_set(&model, EntityId(20)).expect("readable").0;
        assert!(matches!(before.quantities[0], Quantity::Unresolved { .. }));

        let mut tx = Transaction::new(&model);
        set_quantity_value(&mut tx, &model, EntityId(10), 7.25).expect(schema);
        tx.commit(&mut model).expect("commit");
        let (after, anomalies) = quantity_set(&model, EntityId(20)).expect("readable");
        assert!(anomalies.is_empty(), "{schema}: {anomalies:?}");
        assert!(
            matches!(after.quantities[0], Quantity::Simple { value, .. } if value == 7.25),
            "{schema}: {:?}",
            after.quantities
        );
        assert_eq!(
            model.get(EntityId(10)).expect("kept").attributes.len(),
            arity
        );
    }
}

#[test]
fn setting_a_value_refuses_what_the_release_cannot_hold() {
    // A record cut off before its value: no slot in it can be trusted.
    let cut = step("IFC4", &["#10=IFCQUANTITYAREA('A',$,$);"]);
    let error = refused(&cut, |tx| {
        set_quantity_value(tx, &cut, EntityId(10), 1.0).unwrap_err()
    });
    assert_eq!(
        error,
        PropertyError::MalformedEntitySlots {
            id: EntityId(10),
            type_name: "IFCQUANTITYAREA".to_owned(),
            expected: 5,
            actual: 3,
        }
    );
    // An IFC4X3 number in an IFC4 model is foreign to it.
    let foreign = step("IFC4", &["#10=IFCQUANTITYNUMBER('N',$,$,1.,$);"]);
    let error = refused(&foreign, |tx| {
        set_quantity_value(tx, &foreign, EntityId(10), 2.0).unwrap_err()
    });
    assert!(matches!(
        error,
        PropertyError::EntityNotInSchema {
            entity: "IfcQuantityNumber",
            schema: SchemaVersion::Ifc4
        }
    ));
}
