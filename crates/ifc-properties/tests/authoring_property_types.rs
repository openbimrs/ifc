//! Every property value type authored, then read back by the resolver.
//!
//! `PropertyValue` is the reader's view of what a property carries.
//! Asserting on its variants proves the author wrote the slots the
//! reader actually follows, rather than slots that merely parse.

use ifc_model::codec::Codec;
use ifc_model::{Entity, EntityId, Model, Transaction, Value};
use ifc_properties::{
    add_complex_property, add_element_quantity, add_physical_complex_quantity,
    add_property_bounded_value, add_property_enumerated_value, add_property_list_value,
    add_property_reference_value, add_property_set, add_property_set_template,
    add_property_single_value, add_property_table_value, attach_property_set, attach_template,
    attach_type, create_quantity, properties_of, property_set, property_set_template,
    quantity_sets, template_of_set, PropertyError, PropertyValue, QuantityKind, TableValueDraft,
};
use ifc_step::StepCodec;

/// A length measure, the shape an IFC file uses for a dimensioned scalar.
fn length(value: f64) -> Value {
    Value::Typed {
        type_name: "IFCLENGTHMEASURE".into(),
        value: Box::new(Value::Real(value)),
    }
}

/// A wall to hang property sets on.
fn wall(tx: &mut Transaction, guid: &str) -> EntityId {
    let mut attributes = vec![Value::Null; 8];
    attributes[0] = Value::Text(guid.into());
    tx.create(Entity::new("IFCWALL", attributes))
}

/// Every value type resolves back as the variant it was authored as.
///
/// One set carries all six, so the test also proves they coexist in a
/// single `HasProperties` aggregate.
#[test]
fn every_property_value_type_resolves_as_authored() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let target = wall(&mut tx, "0aBcDeFgHiJkLmNoPqRsTu");
    tx.commit(&mut model).expect("commit the wall");

    let mut tx = Transaction::new(&model);
    let single = add_property_single_value(&mut tx, "Height", None, Some(length(3.2)), None)
        .expect("single");
    let enumerated = add_property_enumerated_value(
        &mut tx,
        "Finish",
        None,
        Some(vec![Value::Text("Painted".into())]),
        None,
    )
    .expect("enumerated");
    let bounded = add_property_bounded_value(
        &mut tx,
        "Thickness",
        None,
        Some(length(0.4)),
        Some(length(0.2)),
        None,
        None,
    )
    .expect("bounded");
    let list = add_property_list_value(
        &mut tx,
        "Layers",
        None,
        Some(vec![length(0.1), length(0.3)]),
        None,
    )
    .expect("list");
    let table = add_property_table_value(
        &mut tx,
        TableValueDraft {
            name: "Deflection",
            defining: Some(vec![length(0.0), length(1.0)]),
            defined: Some(vec![length(0.0), length(0.05)]),
            interpolation: Some("LINEAR"),
            ..TableValueDraft::default()
        },
    )
    .expect("table");
    let reference =
        add_property_reference_value(&mut tx, "Datasheet", None, Some("spec"), Some(target))
            .expect("reference");
    let complex =
        add_complex_property(&mut tx, "UValue", None, "Conditions", &[("Height", single)])
            .expect("complex");
    let pset = add_property_set(
        &mut tx,
        "1aBcDeFgHiJkLmNoPqRsTu",
        "Pset_WallCommon",
        None,
        &[
            ("Height", single),
            ("Finish", enumerated),
            ("Thickness", bounded),
            ("Layers", list),
            ("Deflection", table),
            ("Datasheet", reference),
            ("UValue", complex),
        ],
    )
    .expect("pset");
    attach_property_set(&mut tx, &model, "2aBcDeFgHiJkLmNoPqRsTu", &[target], pset)
        .expect("attached");
    tx.commit(&mut model).expect("commit");

    let sets = properties_of(&model, target);
    assert_eq!(sets.len(), 1);
    let properties = &sets[0].set.properties;
    assert_eq!(properties.len(), 7, "all seven resolve");

    let variant = |name: &str| {
        properties
            .iter()
            .find(|p| p.name.as_deref() == Some(name))
            .map(|p| p.value.clone())
            .unwrap_or_else(|| panic!("{name} resolves"))
    };
    assert!(matches!(variant("Height"), PropertyValue::Single { .. }));
    assert!(matches!(
        variant("Finish"),
        PropertyValue::Enumerated { .. }
    ));
    assert!(matches!(
        variant("Thickness"),
        PropertyValue::Bounded { .. }
    ));
    assert!(matches!(variant("Layers"), PropertyValue::List { .. }));
    assert!(matches!(variant("Deflection"), PropertyValue::Table { .. }));
    assert!(matches!(
        variant("Datasheet"),
        PropertyValue::Reference { .. }
    ));
    assert!(matches!(variant("UValue"), PropertyValue::Complex { .. }));
}

/// An element quantity attaches like a property set and reads back.
///
/// `IfcElementQuantity` and `IfcPropertySet` are both
/// `IfcPropertySetDefinition`, so the same attachment relationship
/// carries either. This asserts the quantity takeoff path end to end.
#[test]
fn an_authored_element_quantity_attaches_and_reads_back() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let target = wall(&mut tx, "3aBcDeFgHiJkLmNoPqRsTu");
    tx.commit(&mut model).expect("commit the wall");

    let mut tx = Transaction::new(&model);
    let area = create_quantity(&mut tx, QuantityKind::Area, "GrossArea", 12.5);
    let volume = create_quantity(&mut tx, QuantityKind::Volume, "GrossVolume", 3.75);
    let grouped =
        add_physical_complex_quantity(&mut tx, "Gross", None, &[area, volume], "gross measures")
            .expect("complex quantity");
    let quantity_set = add_element_quantity(
        &mut tx,
        "04BcDeFgHiJkLmNoPqRsTu",
        "Qto_WallBaseQuantities",
        Some("BaseQuantities"),
        &[grouped],
    )
    .expect("element quantity");
    attach_property_set(
        &mut tx,
        &model,
        "05BcDeFgHiJkLmNoPqRsTu",
        &[target],
        quantity_set,
    )
    .expect("attached");
    tx.commit(&mut model).expect("commit");

    let (sets, anomalies) = quantity_sets(&model);
    assert!(
        anomalies.is_empty(),
        "authored takeoff is anomaly free: {anomalies:?}"
    );
    assert_eq!(sets.len(), 1, "the takeoff resolves");
    assert_eq!(sets[0].name.as_deref(), Some("Qto_WallBaseQuantities"));
}

/// The schema's per-type rules are enforced at authoring time.
///
/// Every case here produces a file that parses. Each one is refused
/// because what it would mean is broken, not because it is malformed.
#[test]
fn the_schema_rules_for_each_value_type_are_enforced() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);

    // SameUnitUpperLower: bounds of different measures describe two
    // different quantities, so comparing them is meaningless.
    let mass = Value::Typed {
        type_name: "IFCMASSMEASURE".into(),
        value: Box::new(Value::Real(0.2)),
    };
    assert!(
        add_property_bounded_value(
            &mut tx,
            "Thickness",
            None,
            Some(length(0.4)),
            Some(mass),
            None,
            None,
        )
        .is_err(),
        "mixed bound measures are refused"
    );

    // WR21: a row whose input has no output is not a row.
    assert!(
        add_property_table_value(
            &mut tx,
            TableValueDraft {
                name: "Deflection",
                defining: Some(vec![length(0.0), length(1.0)]),
                defined: Some(vec![length(0.0)]),
                ..TableValueDraft::default()
            },
        )
        .is_err(),
        "ragged table columns are refused"
    );

    // WR22: a column mixing measures cannot be interpolated.
    let mixed = Value::Typed {
        type_name: "IFCMASSMEASURE".into(),
        value: Box::new(Value::Real(1.0)),
    };
    assert!(
        add_property_table_value(
            &mut tx,
            TableValueDraft {
                name: "Deflection",
                defining: Some(vec![length(0.0), mixed]),
                defined: Some(vec![length(0.0), length(0.05)]),
                ..TableValueDraft::default()
            },
        )
        .is_err(),
        "a heterogeneous column is refused"
    );
}

/// Empty aggregates and blank names are refused across the value types.
///
/// Each list attribute is typed `LIST [1:?]` or `SET [1:?]`: an empty
/// aggregate is malformed where omission is legal, and the two are
/// not interchangeable.
#[test]
fn empty_aggregates_and_blank_names_are_refused() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let anchor = add_property_single_value(&mut tx, "Anchor", None, None, None).expect("anchor");

    // Assert the attribute named, not merely that something failed: a
    // refusal blaming the wrong field sends a caller to the wrong fix.
    match add_property_enumerated_value(&mut tx, "Finish", None, Some(vec![]), None) {
        Err(PropertyError::AuthoringInvalid {
            entity, attribute, ..
        }) => {
            assert_eq!(entity, "IFCPROPERTYENUMERATEDVALUE");
            assert_eq!(attribute, "EnumerationValues");
        }
        other => panic!("an empty enumeration is refused by name, got {other:?}"),
    }
    assert!(
        add_property_list_value(&mut tx, "Layers", None, Some(vec![]), None).is_err(),
        "an empty list is refused"
    );
    assert!(
        add_complex_property(&mut tx, "UValue", None, "Conditions", &[]).is_err(),
        "a complex property with no children is refused"
    );
    assert!(
        add_complex_property(&mut tx, "UValue", None, "   ", &[("Anchor", anchor)]).is_err(),
        "a blank usage name is refused"
    );
    assert!(
        add_complex_property(
            &mut tx,
            "UValue",
            None,
            "Conditions",
            &[("Anchor", anchor), ("Anchor", anchor)],
        )
        .is_err(),
        "duplicate nested names are refused"
    );
    for name in ["", "   "] {
        assert!(
            add_property_list_value(&mut tx, name, None, Some(vec![length(1.0)]), None).is_err(),
            "a blank property name is refused"
        );
    }
}

/// The value types survive STEP text with their aggregates intact.
///
/// Slot padding and list encoding only show up once a record is
/// serialised and read back by the parser rather than by the writer.
#[test]
fn the_value_types_survive_step_text() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let table = add_property_table_value(
        &mut tx,
        TableValueDraft {
            name: "Deflection",
            defining: Some(vec![length(0.0), length(1.0)]),
            defined: Some(vec![length(0.0), length(0.05)]),
            expression: Some("y = 0.05x"),
            interpolation: Some("LINEAR"),
            ..TableValueDraft::default()
        },
    )
    .expect("table");
    let pset = add_property_set(
        &mut tx,
        "06BcDeFgHiJkLmNoPqRsTu",
        "Pset_Structural",
        None,
        &[("Deflection", table)],
    )
    .expect("pset");
    tx.commit(&mut model).expect("commit");

    let mut bytes = Vec::new();
    StepCodec.write(&model, &mut bytes).expect("written");
    let reparsed = StepCodec.read_bytes(&bytes).expect("reparsed");

    let set = property_set(&reparsed, pset).expect("the set survives");
    assert_eq!(set.properties.len(), 1);
    let PropertyValue::Table {
        defining, defined, ..
    } = &set.properties[0].value
    else {
        panic!("still a table after the text round trip");
    };
    assert_eq!(defining.len(), 2, "both columns keep their rows");
    assert_eq!(defined.len(), 2);
    // Asymmetric values: equal-length columns would hide a swap of the
    // two slots, which inverts the table's independent variable.
    assert_eq!(
        defining[1].scalar.as_f64(),
        Some(1.0),
        "defining is the input column"
    );
    assert_eq!(
        defined[1].scalar.as_f64(),
        Some(0.05),
        "defined is the output column"
    );
}

/// The quantity entities refuse the same empty and blank cases.
///
/// These mirror the property-type refusals: `Quantities` and
/// `HasQuantities` are both `SET [1:?]`, and a takeoff nobody can
/// name is a takeoff nobody can find.
#[test]
fn the_quantity_entities_refuse_empty_and_blank_input() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let area = create_quantity(&mut tx, QuantityKind::Area, "GrossArea", 12.5);
    let guid = "07BcDeFgHiJkLmNoPqRsTu";

    assert!(
        add_element_quantity(&mut tx, guid, "Qto_Wall", None, &[]).is_err(),
        "an element quantity with no quantities is refused"
    );
    assert!(
        add_element_quantity(&mut tx, guid, "   ", None, &[area]).is_err(),
        "a blank takeoff name is refused"
    );
    assert!(
        add_element_quantity(&mut tx, "not-a-guid", "Qto_Wall", None, &[area]).is_err(),
        "a malformed GUID is refused"
    );
    assert!(
        add_physical_complex_quantity(&mut tx, "Gross", None, &[], "gross").is_err(),
        "a complex quantity with no children is refused"
    );
    assert!(
        add_physical_complex_quantity(&mut tx, "Gross", None, &[area], "  ").is_err(),
        "a blank discrimination is refused"
    );
    assert!(
        add_physical_complex_quantity(&mut tx, "  ", None, &[area], "gross").is_err(),
        "a blank complex quantity name is refused"
    );
}

/// Templates and type attachment read back through their readers.
///
/// A template declares what a set should contain; the relationship
/// binds an authored set to it. Both are what a conformance checker
/// follows, so both must resolve, not merely parse.
#[test]
fn templates_and_type_attachment_read_back() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let simple = tx.create(Entity::new("IFCSIMPLEPROPERTYTEMPLATE", {
        let mut a = vec![Value::Null; 12];
        a[0] = Value::Text("0aBcDeFgHiJkLmNoPqRsTu".into());
        a[2] = Value::Text("Height".into());
        a
    }));
    let template = add_property_set_template(
        &mut tx,
        "1aBcDeFgHiJkLmNoPqRsTu",
        "Pset_WallCommon",
        Some("IfcWall"),
        &[simple],
    )
    .expect("template");
    let height = add_property_single_value(&mut tx, "Height", None, None, None).expect("property");
    let pset = add_property_set(
        &mut tx,
        "2aBcDeFgHiJkLmNoPqRsTu",
        "Pset_WallCommon",
        None,
        &[("Height", height)],
    )
    .expect("pset");
    attach_template(&mut tx, "3aBcDeFgHiJkLmNoPqRsTu", &[pset], template).expect("bound");
    tx.commit(&mut model).expect("commit");

    let found = property_set_template(&model, template).expect("template resolves");
    assert_eq!(found.name.as_deref(), Some("Pset_WallCommon"));
    assert_eq!(found.applicable_entity.as_deref(), Some("IfcWall"));
    assert_eq!(template_of_set(&model).get(&pset), Some(&vec![template]));
}

/// Type attachment is the route properties reach occurrences.
///
/// IfcRelDefinesByProperties refuses a type object; this is the
/// relationship that carries a type's properties to its instances.
#[test]
fn type_attachment_refuses_the_cases_the_schema_forbids() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let wall_type = tx.create(Entity::new("IFCWALLTYPE", vec![Value::Null; 10]));
    let occurrence = wall(&mut tx, "0aBcDeFgHiJkLmNoPqRsTu");
    tx.commit(&mut model).expect("commit");

    let mut tx = Transaction::new(&model);
    let rel = attach_type(
        &mut tx,
        &model,
        "1aBcDeFgHiJkLmNoPqRsTu",
        &[occurrence],
        wall_type,
    )
    .expect("a type attaches to an occurrence");
    tx.commit(&mut model).expect("commit");
    let written = model.get(rel).expect("written");
    assert_eq!(
        written.attributes[5],
        Value::Ref(wall_type),
        "RelatingType at 5"
    );

    let mut tx = Transaction::new(&model);
    assert!(
        attach_type(&mut tx, &model, "2aBcDeFgHiJkLmNoPqRsTu", &[], wall_type).is_err(),
        "an empty occurrence list is refused",
    );
    assert!(
        attach_type(
            &mut tx,
            &model,
            "3aBcDeFgHiJkLmNoPqRsTu",
            &[wall_type],
            wall_type,
        )
        .is_err(),
        "a type object cannot be its own occurrence",
    );
    assert!(
        attach_type(
            &mut tx,
            &model,
            "04BcDeFgHiJkLmNoPqRsTu",
            &[occurrence],
            occurrence,
        )
        .is_err(),
        "an occurrence is not a type",
    );
}

/// Templates refuse the empty aggregates the schema forbids.
///
/// HasPropertyTemplates and RelatedPropertySets are both `SET [1:?]`:
/// a template declaring no properties constrains nothing, and a
/// binding to no sets binds nothing.
#[test]
fn templates_refuse_empty_aggregates() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let simple = tx.create(Entity::new(
        "IFCSIMPLEPROPERTYTEMPLATE",
        vec![Value::Null; 12],
    ));
    let template =
        add_property_set_template(&mut tx, "0aBcDeFgHiJkLmNoPqRsTu", "Pset_X", None, &[simple])
            .expect("template");

    assert!(
        add_property_set_template(&mut tx, "1aBcDeFgHiJkLmNoPqRsTu", "Pset_X", None, &[],).is_err(),
        "a template with no property templates is refused",
    );
    assert!(
        attach_template(&mut tx, "2aBcDeFgHiJkLmNoPqRsTu", &[], template).is_err(),
        "a binding to no property sets is refused",
    );
    assert!(
        add_property_set_template(&mut tx, "bad-guid", "Pset_X", None, &[simple]).is_err(),
        "a malformed GUID is refused",
    );
    assert!(
        add_property_set_template(&mut tx, "3aBcDeFgHiJkLmNoPqRsTu", "   ", None, &[simple],)
            .is_err(),
        "a blank template name is refused",
    );
}

/// All six IfcQuantity subtypes are authorable and read back typed.
///
/// They are written through one QuantityKind path rather than six
/// functions, which is easy to mistake for a gap when grepping for
/// type-name literals. This pins the actual behaviour.
#[test]
fn every_quantity_subtype_is_authorable() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let kinds = [
        (QuantityKind::Length, "IFCQUANTITYLENGTH"),
        (QuantityKind::Area, "IFCQUANTITYAREA"),
        (QuantityKind::Volume, "IFCQUANTITYVOLUME"),
        (QuantityKind::Count, "IFCQUANTITYCOUNT"),
        (QuantityKind::Weight, "IFCQUANTITYWEIGHT"),
        (QuantityKind::Time, "IFCQUANTITYTIME"),
    ];
    let mut ids = Vec::new();
    for (kind, _) in kinds {
        ids.push(create_quantity(&mut tx, kind, "Q", 2.0));
    }
    tx.commit(&mut model).expect("commit");

    for (id, (_, expected)) in ids.iter().zip(kinds) {
        let entity = model.get(*id).expect("written");
        assert!(
            entity.type_name.eq_ignore_ascii_case(expected),
            "expected {expected}, got {}",
            entity.type_name,
        );
    }
}
