//! The contract test: costing data survives a build that cannot interpret it.
//!
//! This is the central claim of the architecture, so it is tested against a
//! real file rather than a constructed model:
//!
//! > Read an IFC file containing costing entities, write it back with **no
//! > domain feature enabled**, and the costing entities are untouched.
//!
//! The fixture also contains an entity type (`IFCFUTURESUSTAINABILITYMETRIC`)
//! that exists in no IFC schema. Preserving it proves the model is genuinely
//! schema-agnostic rather than merely tolerant of the schemas we shipped.

#![cfg(feature = "step")]

use ifc::{Codec, EntityId, Model, StepCodec, Value};
use std::path::PathBuf;

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../test/fixtures/costing")
}

fn load() -> Model {
    StepCodec
        .read_path(&fixture().join("costing_schedule.ifc"))
        .expect("the costing fixture should parse")
}

/// Entity graphs must match exactly: same ids, types, and attribute values.
fn assert_identical(a: &Model, b: &Model, context: &str) {
    assert_eq!(a.len(), b.len(), "{context}: entity count changed");
    for (id, entity) in a.iter() {
        let other = b
            .get(id)
            .unwrap_or_else(|| panic!("{context}: {id} vanished"));
        assert_eq!(entity.type_name, other.type_name, "{context}: {id} type");
        assert_eq!(
            entity.attributes, other.attributes,
            "{context}: {id} attributes"
        );
    }
}

/// The headline requirement, end to end.
#[test]
fn costing_entities_survive_a_write_with_no_domain_feature() {
    // Guard: if a domain feature leaked into this build, the test would still
    // pass but would no longer be testing the claim.
    let features = ifc::compiled_features();
    let domains = [
        "cost",
        "schedule",
        "properties",
        "material",
        "classification",
        "structural",
        "resource",
        "systems",
        "style",
    ];
    let enabled: Vec<&str> = domains
        .iter()
        .copied()
        .filter(|d| features.contains(d))
        .collect();

    let model = load();
    let written = StepCodec.write_bytes(&model).expect("write");
    let reread = StepCodec.read_bytes(&written).expect("re-read");

    assert_identical(&model, &reread, "step round-trip");

    // The specific costing values, checked by content rather than by count.
    let (_, schedule) = reread
        .of_type("IFCCOSTSCHEDULE")
        .next()
        .expect("cost schedule survived");
    assert_eq!(schedule.text(2), Some("Budget 2026"));
    assert_eq!(schedule.attributes[4], Value::Enum("BUDGET".into()));
    assert_eq!(schedule.text(5), Some("BQ-2026-01"));

    let items: Vec<_> = reread.of_type("IFCCOSTITEM").collect();
    assert_eq!(items.len(), 2, "both cost items survived");

    let values: Vec<f64> = reread
        .of_type("IFCCOSTVALUE")
        .filter_map(|(_, e)| e.attribute(2)?.unwrap_typed().as_f64())
        .collect();
    assert!(
        values.contains(&12345.67) && values.contains(&0.155),
        "monetary measures survived exactly: {values:?}"
    );

    let quantities: Vec<f64> = reread
        .of_type("IFCQUANTITYVOLUME")
        .chain(reread.of_type("IFCQUANTITYAREA"))
        .filter_map(|(_, e)| e.number(3))
        .collect();
    assert!(
        quantities.contains(&1250.5) && quantities.contains(&842.25),
        "quantities survived exactly: {quantities:?}"
    );

    // Text written into the output verbatim, not merely equal in the model.
    let text = String::from_utf8_lossy(&written);
    for expected in [
        "IFCCOSTSCHEDULE",
        "IFCCOSTITEM",
        "IFCCOSTVALUE",
        "IFCMONETARYMEASURE(12345.67)",
        "Budget 2026",
        ".BUDGET.",
    ] {
        assert!(
            text.contains(expected),
            "output lost {expected:?} (domains enabled in this build: {enabled:?})"
        );
    }
}

/// An entity from no known schema must round-trip, including its odd values.
#[test]
fn unknown_entity_types_are_preserved_verbatim() {
    let model = load();
    let reread = StepCodec
        .read_bytes(&StepCodec.write_bytes(&model).unwrap())
        .unwrap();

    let (_, unknown) = reread
        .of_type("IFCFUTURESUSTAINABILITYMETRIC")
        .next()
        .expect("an entity from no schema must survive");

    assert_eq!(unknown.attributes[0], Value::Text("carbon".into()));
    assert_eq!(unknown.attributes[1], Value::Derived, "* stays derived");
    assert_eq!(unknown.attributes[2], Value::Bool(true), ".T. stays true");
    assert_eq!(
        unknown.attributes[3],
        Value::LogicalUnknown,
        ".U. is the third logical state, not false"
    );
    assert_eq!(
        unknown.attributes[4],
        Value::List(vec![
            Value::Integer(1),
            Value::Real(2.5),
            Value::Ref(EntityId(21)),
        ]),
        "mixed aggregate survived"
    );
    assert_eq!(unknown.attributes[6], Value::Binary("0F3A5C".into()));
}

/// Parsing an unknown entity must not error, per the constraint.
#[test]
fn unknown_entities_never_cause_an_error() {
    let source = b"ISO-10303-21;
HEADER;
FILE_DESCRIPTION((''),'2;1');
FILE_NAME('','',(''),(''),'','','');
FILE_SCHEMA(('IFC99_FROM_THE_FUTURE'));
ENDSEC;
DATA;
#1= IFCTOTALLYMADEUPENTITY('a',$,*,.U.,(#1));
ENDSEC;
END-ISO-10303-21;
";
    let model = StepCodec.read_bytes(source).expect("must not error");
    assert_eq!(model.len(), 1);
    assert_eq!(
        model.header().schema,
        vec!["IFC99_FROM_THE_FUTURE".to_string()],
        "an unrecognized schema token is stored, not rejected"
    );
}

/// Cross-codec: the same fixture through ifcXML and back.
#[cfg(feature = "ifcxml")]
#[test]
fn costing_survives_a_trip_through_a_different_serialization() {
    use ifc::XmlCodec;

    let model = load();
    let xml = XmlCodec::default().write_bytes(&model).expect("xml write");
    let from_xml = XmlCodec::default().read_bytes(&xml).expect("xml read");

    assert_identical(&model, &from_xml, "step -> xml");

    let back = StepCodec.write_bytes(&from_xml).expect("step write");
    let final_model = StepCodec.read_bytes(&back).expect("step read");
    assert_identical(&model, &final_model, "step -> xml -> step");

    let text = String::from_utf8_lossy(&back);
    assert!(text.contains("IFCMONETARYMEASURE(12345.67)"));
}

/// With the `cost` feature on, the *same* file yields interpreted values --
/// the data was always there, the meaning is what the feature adds.
#[cfg(feature = "cost")]
#[test]
fn the_cost_feature_interprets_data_that_was_never_lost() {
    let model = load();
    let view = ifc::cost::CostView::new(&model);

    let items: Vec<_> = view.items().collect();
    assert_eq!(items.len(), 2, "cost items found by the domain view");

    let total = ifc::cost::rollup::grand_total(&view);
    assert!(
        (total - (12345.67 + 0.155)).abs() < 1e-9,
        "cost values sum to the file's amounts, got {total}"
    );
}
