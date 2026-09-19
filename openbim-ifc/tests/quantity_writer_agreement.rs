//! Two crates author `IfcQuantity*`; they must agree byte for byte.
//!
//! # Why this test exists here
//!
//! `ifc-properties` owns quantities as property-set members, and
//! `ifc-cost` owns them as cost-item take-off. Sibling domain crates do
//! not depend on one another, so neither can reuse the other's writer
//! and neither can test the agreement. This facade depends on both and
//! is the only place the two can be compared.
//!
//! Both wrote the same six entity types and disagreed: one emitted four
//! attributes where the schema declares five, dropping `Formula`, and
//! the other encoded `IfcCountMeasure` as a real where EXPRESS declares
//! it `INTEGER`. Both produced files that parse. Comparing the emitted
//! STEP text is what makes a divergence fail rather than accumulate.

#![cfg(all(feature = "cost", feature = "properties", feature = "step"))]

use ifc::cost::mutation::{
    create_quantity as cost_quantity, QuantityDraft, QuantityKind as CostKind,
};
use ifc::properties::{
    create_quantity as property_quantity, create_quantity_with, QuantityExtras,
    QuantityKind as PropertyKind,
};
use ifc::{Codec, Model, StepCodec, Value};
use ifc_model::Transaction;

/// Emit one model as STEP and return the line for its single quantity.
fn quantity_line(model: &Model) -> String {
    let mut bytes = Vec::new();
    StepCodec.write(model, &mut bytes).expect("written");
    String::from_utf8(bytes)
        .expect("utf8")
        .lines()
        .find(|line| line.contains("IFCQUANTITY"))
        .expect("a quantity is written")
        .to_owned()
}

/// Every quantity kind is written identically by both crates.
#[test]
fn both_crates_write_the_same_quantity_record() {
    let pairs = [
        (PropertyKind::Length, CostKind::Length, 2.5),
        (PropertyKind::Area, CostKind::Area, 12.5),
        (PropertyKind::Volume, CostKind::Volume, 3.75),
        (PropertyKind::Count, CostKind::Count, 4.0),
        (PropertyKind::Weight, CostKind::Weight, 12.0),
        (PropertyKind::Time, CostKind::Time, 8.0),
    ];

    for (property_kind, cost_kind, value) in pairs {
        let mut from_properties = Model::default();
        let mut tx = Transaction::new(&from_properties);
        property_quantity(&mut tx, property_kind, "Q", value);
        tx.commit(&mut from_properties).expect("commit");

        let mut from_cost = Model::default();
        let mut tx = Transaction::new(&from_cost);
        let draft = QuantityDraft {
            kind: cost_kind,
            name: "Q",
            description: None,
            unit: None,
            value,
            formula: None,
        };
        cost_quantity(&mut tx, &from_cost, draft).expect("cost quantity");
        tx.commit(&mut from_cost).expect("commit");

        assert_eq!(
            quantity_line(&from_properties),
            quantity_line(&from_cost),
            "{property_kind:?}: the two writers disagree",
        );
    }
}

/// Agreement is necessary but not sufficient: both could be wrong.
///
/// The comparison above only proves the two writers match. These
/// assertions anchor them to the schema instead of to each other:
/// five attributes on every subtype, `IfcCountMeasure` as an integer
/// with no decimal point, every other measure as a real, and `Unit`
/// and `Formula` present when supplied.
#[test]
fn the_agreed_record_matches_the_schema() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let unit = tx.create(ifc_model::Entity::new(
        "IFCSIUNIT",
        vec![
            ifc_model::Value::Derived,
            ifc_model::Value::Enum("AREAUNIT".into()),
            ifc_model::Value::Null,
            ifc_model::Value::Enum("SQUARE_METRE".into()),
        ],
    ));
    let area = create_quantity_with(
        &mut tx,
        PropertyKind::Area,
        "GrossArea",
        12.5,
        QuantityExtras {
            description: Some("Painted face"),
            unit: Some(unit),
            formula: Some("l * h"),
        },
    );
    let count = property_quantity(&mut tx, PropertyKind::Count, "Doors", 4.0);
    tx.commit(&mut model).expect("commit");

    // Five attributes: Name, Description, Unit, the measure, Formula.
    assert_eq!(model.get(area).expect("area").attributes.len(), 5);
    assert_eq!(model.get(count).expect("count").attributes.len(), 5);

    let mut bytes = Vec::new();
    StepCodec.write(&model, &mut bytes).expect("written");
    let text = String::from_utf8(bytes).expect("utf8");

    let area_line = text
        .lines()
        .find(|line| line.contains("IFCQUANTITYAREA"))
        .expect("area written");
    assert!(area_line.contains("Painted face"), "{area_line}");
    assert!(area_line.contains("l * h"), "Formula survives: {area_line}");
    assert!(
        area_line.contains("IFCAREAMEASURE(12.5)"),
        "a real measure keeps its decimal: {area_line}",
    );
    // Unit is slot 2 and must reference the authored IFCSIUNIT; dropping
    // it silently reinterprets the quantity in the project default.
    assert_eq!(
        model.get(area).expect("area").attributes[2],
        ifc_model::Value::Ref(unit),
        "Unit is written at slot 2",
    );

    let count_line = text
        .lines()
        .find(|line| line.contains("IFCQUANTITYCOUNT"))
        .expect("count written");
    // IfcCountMeasure = INTEGER, so the value is 4 and never 4.
    //
    // Asserted on the closing delimiter, not on a bare "(4)": that is a
    // substring of "(4.)" and would hold for the real encoding too.
    assert!(
        count_line.contains("IFCCOUNTMEASURE(4),"),
        "a count is an integer, not a real: {count_line}",
    );
    assert!(
        !count_line.contains("IFCCOUNTMEASURE(4."),
        "a count carries no decimal point: {count_line}",
    );
}

/// Four crates author `IfcRelNests`; the ends must not transpose.
///
/// `ifc-schedule` nests tasks, `ifc-systems` nests ports,
/// `ifc-cost` nests cost items and `ifc-resource` nests resources.
/// None can see the others. `RelatingObject` is slot 4 and
/// `RelatedObjects` slot 5; swapping them makes the children the
/// parent, which parses and inverts every nesting query.
///
/// `ifc-resource` resolves its slots by attribute name from the
/// schema, so it is used here as the reference the index-addressed
/// writers are checked against.
#[cfg(all(feature = "schedule", feature = "systems", feature = "resource"))]
#[test]
fn every_nesting_writer_agrees_on_which_end_is_the_parent() {
    use ifc::resource::{NestingDraft, ResourceDraft, ResourceEditor, ResourceKind};
    use ifc::schedule::nest_tasks;
    use ifc::systems::nest_ports;

    const PARENT: &str = "0aBcDeFgHiJkLmNoPqRsTu";
    const CHILD: &str = "1aBcDeFgHiJkLmNoPqRsTu";
    const REL: &str = "2aBcDeFgHiJkLmNoPqRsTu";

    // ifc-resource: slots resolved by name from the schema, which it
    // reads from the model header.
    let mut model = Model::new();
    model.header_mut().schema = vec!["IFC4".to_owned()];
    let mut editor = ResourceEditor::for_model(&mut model).expect("editor");
    let parent = editor
        .create_resource(ResourceDraft::new(ResourceKind::Crew, PARENT))
        .expect("parent resource");
    let child = editor
        .create_resource(ResourceDraft::new(ResourceKind::Labor, CHILD))
        .expect("child resource");
    let by_name = editor
        .create_nesting(NestingDraft::new(REL, parent, vec![child]))
        .expect("resource nesting");
    let reference = model.get(by_name).expect("written").clone();
    assert_eq!(
        reference.attributes[4],
        Value::Ref(parent),
        "schema: parent at 4"
    );

    // ifc-schedule and ifc-systems address the same slots by index.
    let mut other = Model::default();
    let mut tx = Transaction::new(&other);
    let sched = nest_tasks(&mut tx, REL, parent, &[child]).expect("task nesting");
    let ports = nest_ports(&mut tx, REL, parent, &[child]).expect("port nesting");
    tx.commit(&mut other).expect("commit");

    for (id, crate_name) in [(sched, "ifc-schedule"), (ports, "ifc-systems")] {
        let entity = other.get(id).expect("written");
        assert_eq!(
            entity.attributes[4], reference.attributes[4],
            "{crate_name}: RelatingObject must match the schema-addressed writer",
        );
        assert_eq!(
            entity.attributes[5], reference.attributes[5],
            "{crate_name}: RelatedObjects must match the schema-addressed writer",
        );
    }
}
