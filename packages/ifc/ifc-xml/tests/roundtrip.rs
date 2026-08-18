//! Round-trip fidelity for the ifcXML codec, including cross-codec transfer.
//!
//! The contract these tests defend: a value's *kind* survives. XML attribute
//! values are all strings, so a careless codec turns `Real(1.0)` into
//! `Integer(1)` or `Null` into `Text("")` and nobody notices until a
//! downstream consumer misreads a file.

use ifc_model::{Codec, Entity, EntityId, Model, Value};
use ifc_step::StepCodec;
use ifc_xml::XmlCodec;

/// A model exercising every `Value` variant.
fn every_kind() -> Model {
    let mut model = Model::new();
    model.header_mut().schema = vec!["IFC4".to_string()];
    model.header_mut().name = "kinds.ifc".to_string();

    model.insert(
        EntityId(1),
        Entity::new(
            "IFCTESTALLKINDS",
            vec![
                Value::Null,
                Value::Derived,
                Value::Bool(true),
                Value::Bool(false),
                Value::LogicalUnknown,
                Value::Integer(-42),
                Value::Real(1.0),
                Value::Real(-2.5e-7),
                Value::Text("plain".into()),
                Value::Binary("0F3A".into()),
                Value::Enum("ELEMENT".into()),
                Value::Ref(EntityId(2)),
                Value::List(vec![
                    Value::Integer(1),
                    Value::Real(2.0),
                    Value::List(vec![Value::Text("nested".into())]),
                ]),
                Value::Typed {
                    type_name: "IFCLENGTHMEASURE".into(),
                    value: Box::new(Value::Real(0.2)),
                },
            ],
        ),
    );
    model.insert(EntityId(2), Entity::new("IFCTARGET", vec![]));
    model
}

/// Compare entity graphs, ignoring header formatting differences.
fn assert_same_entities(a: &Model, b: &Model, context: &str) {
    assert_eq!(a.len(), b.len(), "{context}: entity count");
    for (id, entity) in a.iter() {
        let other = b
            .get(id)
            .unwrap_or_else(|| panic!("{context}: {id} missing after round-trip"));
        assert_eq!(
            entity.type_name, other.type_name,
            "{context}: {id} type name"
        );
        assert_eq!(
            entity.attributes, other.attributes,
            "{context}: {id} attributes"
        );
    }
}

#[test]
fn xml_roundtrip_preserves_every_value_kind() {
    let model = every_kind();
    let codec = XmlCodec::default();

    let bytes = codec.write_bytes(&model).expect("write");
    let back = codec.read_bytes(&bytes).expect("read");

    assert_same_entities(&model, &back, "xml");
}

/// Real(1.0) must not degrade to Integer(1). XML has no type marker, so this
/// is the failure a naive implementation actually produces.
#[test]
fn reals_do_not_degrade_into_integers() {
    let mut model = Model::new();
    model.insert(
        EntityId(1),
        Entity::new("IFCTEST", vec![Value::Real(1.0), Value::Integer(1)]),
    );

    let codec = XmlCodec::default();
    let back = codec
        .read_bytes(&codec.write_bytes(&model).unwrap())
        .unwrap();
    let attrs = &back.get(EntityId(1)).unwrap().attributes;

    assert!(
        matches!(attrs[0], Value::Real(_)),
        "1.0 should stay a Real, got {:?}",
        attrs[0]
    );
    assert!(
        matches!(attrs[1], Value::Integer(_)),
        "1 should stay an Integer, got {:?}",
        attrs[1]
    );
}

/// Null and Derived are different things in IFC; collapsing them loses data.
#[test]
fn null_and_derived_stay_distinct() {
    let mut model = Model::new();
    model.insert(
        EntityId(1),
        Entity::new("IFCTEST", vec![Value::Null, Value::Derived]),
    );

    let codec = XmlCodec::default();
    let back = codec
        .read_bytes(&codec.write_bytes(&model).unwrap())
        .unwrap();
    let attrs = &back.get(EntityId(1)).unwrap().attributes;

    assert_eq!(attrs[0], Value::Null);
    assert_eq!(attrs[1], Value::Derived);
}

/// The claim that serialization is pluggable, tested end to end: STEP in,
/// XML out, STEP back, identical entity graph.
#[test]
fn step_to_xml_to_step_preserves_the_model() {
    let source = b"ISO-10303-21;
HEADER;
FILE_DESCRIPTION((''),'2;1');
FILE_NAME('x.ifc','2026-01-01T00:00:00',(''),(''),'','','');
FILE_SCHEMA(('IFC4'));
ENDSEC;
DATA;
#1= IFCCOSTSCHEDULE('3vB2YO$MX4xv5uCqZZG05x',$,'Schedule',$,.BUDGET.,$,$,$);
#2= IFCCOSTITEM('1a$Bn5MX4xv5uCqZZG05y',$,'Excavation',$,$,(#3),$);
#3= IFCCOSTVALUE('0xY2n5MX4xv5uCqZZG05z',$,IFCMONETARYMEASURE(12345.67),$,$,$,$);
#4= IFCUNKNOWNFUTUREENTITY('data',*,.T.,(1,2.5,#1));
ENDSEC;
END-ISO-10303-21;
";

    let step = StepCodec;
    let xml = XmlCodec::default();

    let from_step = step.read_bytes(source).expect("step read");
    let as_xml = xml.write_bytes(&from_step).expect("xml write");
    let from_xml = xml.read_bytes(&as_xml).expect("xml read");
    assert_same_entities(&from_step, &from_xml, "step->xml");

    let back_to_step = step.write_bytes(&from_xml).expect("step write");
    let final_model = step.read_bytes(&back_to_step).expect("step reread");
    assert_same_entities(&from_step, &final_model, "step->xml->step");

    // The specific promise: cost data survived a trip through a codec that
    // knows nothing about cost, in a build with no cost crate compiled.
    let cost_value = final_model
        .of_type("IFCCOSTVALUE")
        .next()
        .expect("cost value survived")
        .1;
    assert_eq!(
        cost_value.attribute(2).unwrap().unwrap_typed().as_f64(),
        Some(12345.67)
    );

    // And so did an entity from no schema that exists.
    let unknown = final_model
        .of_type("IFCUNKNOWNFUTUREENTITY")
        .next()
        .expect("unknown entity survived")
        .1;
    assert_eq!(unknown.attributes[1], Value::Derived);
    assert_eq!(unknown.attributes[2], Value::Bool(true));
}

/// Unicode must survive both encodings.
#[test]
fn unicode_and_xml_special_characters_survive() {
    let mut model = Model::new();
    model.insert(
        EntityId(1),
        Entity::new(
            "IFCWALL",
            vec![
                Value::Text("Außenwand <&> \"quoted\"".into()),
                Value::Text("\u{30d3}\u{30eb}".into()),
            ],
        ),
    );

    let codec = XmlCodec::default();
    let back = codec
        .read_bytes(&codec.write_bytes(&model).unwrap())
        .unwrap();
    let attrs = &back.get(EntityId(1)).unwrap().attributes;

    assert_eq!(attrs[0], Value::Text("Außenwand <&> \"quoted\"".into()));
    assert_eq!(attrs[1], Value::Text("\u{30d3}\u{30eb}".into()));
}

/// Numeric-looking strings must stay strings.
///
/// Regression: `IfcApplication.Version` is commonly `"0.1"`. Writing it as a
/// plain XML attribute made the reader infer `Real(0.1)`, silently changing
/// the value's kind. Caught by a realistic fixture, not by a unit test.
#[test]
fn strings_that_look_numeric_stay_strings() {
    let mut model = Model::new();
    model.insert(
        EntityId(1),
        Entity::new(
            "IFCAPPLICATION",
            vec![
                Value::Text("0.1".into()),
                Value::Text("42".into()),
                Value::Text("i7".into()),
                Value::Text("1e5".into()),
                Value::Real(0.1),
                Value::Integer(42),
            ],
        ),
    );

    let codec = XmlCodec::default();
    let back = codec
        .read_bytes(&codec.write_bytes(&model).unwrap())
        .unwrap();
    let attrs = &back.get(EntityId(1)).unwrap().attributes;

    assert_eq!(attrs[0], Value::Text("0.1".into()), "version string");
    assert_eq!(attrs[1], Value::Text("42".into()), "integer-like string");
    assert_eq!(attrs[2], Value::Text("i7".into()), "reference-like string");
    assert_eq!(attrs[3], Value::Text("1e5".into()), "exponent-like string");
    assert_eq!(attrs[4], Value::Real(0.1), "an actual real");
    assert_eq!(attrs[5], Value::Integer(42), "an actual integer");
}

/// With a schema, attribute names are the real ones rather than `a0`, `a1`.
#[cfg(feature = "schema")]
#[test]
fn schema_produces_conformant_attribute_names() {
    use ifc_schema::Schema;
    use std::sync::Arc;

    let schema = Arc::new(Schema::from_express(
        "SCHEMA IFC4;\n\
         ENTITY IfcRoot; GlobalId : IfcGloballyUniqueId; END_ENTITY;\n\
         ENTITY IfcCostItem SUBTYPE OF (IfcRoot); Identification : IfcLabel; END_ENTITY;\n\
         END_SCHEMA;",
    ));

    let mut model = Model::new();
    model.insert(
        EntityId(1),
        Entity::new(
            "IFCCOSTITEM",
            vec![Value::Text("guid".into()), Value::Text("A-01".into())],
        ),
    );

    let codec = XmlCodec::with_schema(schema);
    let xml = String::from_utf8(codec.write_bytes(&model).unwrap()).unwrap();

    assert!(
        xml.contains("GlobalId=\"guid\""),
        "expected schema attribute names, got:\n{xml}"
    );
    assert!(
        xml.contains("Identification=\"A-01\""),
        "expected inherited slot ordering, got:\n{xml}"
    );
    assert!(
        !xml.contains("a0="),
        "positional fallback should not appear"
    );
}
