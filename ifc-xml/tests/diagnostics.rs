//! Path-rich XML parse diagnostics.

use ifc_xml::{XmlCodec, XmlError};

fn parse(body: &str) -> XmlError {
    let xml = format!("<ifcXML><IFCEXAMPLE id=\"i7\">{body}</IFCEXAMPLE></ifcXML>");
    ifc_xml::reader::read(&XmlCodec::default(), xml.as_bytes())
        .expect_err("fixture is intentionally invalid")
}

#[test]
fn bad_entity_id_retains_the_entity_location() {
    let xml = b"<ifcXML><IFCWALL id=\"broken\"/></ifcXML>";
    let error = ifc_xml::reader::read(&XmlCodec::default(), xml).unwrap_err();

    assert_eq!(
        error.path().map(ToString::to_string).as_deref(),
        Some("/ifcXML/IFCWALL[@id='broken']")
    );
    assert!(matches!(error.root_cause(), XmlError::BadId(value) if value == "broken"));
}

#[test]
fn nested_numeric_failure_retains_entity_attribute_and_list_index() {
    let error = parse(
        "<a3 kind=\"list\"><item kind=\"typed\" type=\"IfcLengthMeasure\"><value kind=\"real\">not-a-real</value></item></a3>",
    );

    assert_eq!(
        error.path().map(ToString::to_string).as_deref(),
        Some("/ifcXML/IFCEXAMPLE[@id='i7']/a3/item[0]/value")
    );
    assert!(matches!(
        error.root_cause(),
        XmlError::InvalidScalar { kind, value }
            if kind == "real" && value == "not-a-real"
    ));
}

#[test]
fn invalid_integer_and_non_finite_real_are_typed_failures() {
    for (kind, value) in [("integer", "12x"), ("real", "NaN"), ("real", "inf")] {
        let error = parse(&format!(r#"<a0 kind="{kind}">{value}</a0>"#));
        assert_eq!(
            error.path().map(ToString::to_string).as_deref(),
            Some("/ifcXML/IFCEXAMPLE[@id='i7']/a0")
        );
        assert!(matches!(
            error.root_cause(),
            XmlError::InvalidScalar {
                kind: actual_kind,
                value: actual_value,
            } if actual_kind == kind && actual_value == value
        ));
    }
}

#[test]
fn nested_reference_and_unknown_kind_fail_instead_of_becoming_null_or_text() {
    let bad_ref = parse("<Target kind=\"ref\">not-an-id</Target>");
    assert_eq!(
        bad_ref.path().map(ToString::to_string).as_deref(),
        Some("/ifcXML/IFCEXAMPLE[@id='i7']/Target")
    );
    assert!(matches!(
        bad_ref.root_cause(),
        XmlError::InvalidScalar { kind, .. } if kind == "ref"
    ));

    let unknown = parse("<Target kind=\"mystery\">payload</Target>");
    assert!(matches!(
        unknown.root_cause(),
        XmlError::UnknownKind(kind) if kind == "mystery"
    ));
}

#[test]
fn writer_refuses_nested_non_finite_reals_with_entity_slot_context() {
    use ifc_model::{Entity, EntityId, Model, Value};

    let mut model = Model::new();
    model.insert(
        EntityId(9),
        Entity::new(
            "IFCEXAMPLE",
            vec![Value::List(vec![Value::Typed {
                type_name: "IFCREAL".into(),
                value: Box::new(Value::Real(f64::INFINITY)),
            }])],
        ),
    );

    let error = ifc_xml::writer::write(&XmlCodec::default(), &model).unwrap_err();
    assert_eq!(
        error.path().map(ToString::to_string).as_deref(),
        Some("/ifcXML/IFCEXAMPLE[@id='i9']/a0")
    );
    assert!(matches!(
        error.root_cause(),
        XmlError::InvalidScalar { kind, value } if kind == "real" && value == "inf"
    ));
}

#[test]
fn malformed_xml_reports_the_open_entity_and_value_path() {
    let error = parse("<a1 kind=\"list\"><item kind=\"integer\">1</wrong>");
    let path = error
        .path()
        .expect("open parser context is retained")
        .to_string();
    assert!(
        path.starts_with("/ifcXML/IFCEXAMPLE[@id='i7']/a1/item[0]"),
        "{path}"
    );
    assert!(matches!(error.root_cause(), XmlError::Malformed(_)));
}
