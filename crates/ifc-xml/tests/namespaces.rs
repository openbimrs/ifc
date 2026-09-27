//! Explicit ifcXML namespace and release-profile contracts.

use ifc_model::{Codec, Entity, EntityId, Model, Value};
use ifc_xml::{XmlCodec, XmlError, XmlProfile};
use quick_xml::{events::Event, name::ResolveResult, reader::NsReader};

const IFC4_NS: &str = "https://standards.buildingsmart.org/IFC/RELEASE/IFC4/ADD2_TC1/XML";

fn strict() -> XmlCodec {
    XmlCodec::strict(XmlProfile::Ifc4Add2Tc1)
}

fn document(namespace: Option<&str>, schema: Option<&str>, body: &str) -> Vec<u8> {
    let namespace = namespace
        .map(|value| format!(" xmlns=\"{value}\""))
        .unwrap_or_default();
    let schema = schema
        .map(|value| format!(" schema=\"{value}\""))
        .unwrap_or_default();
    format!("<?xml version=\"1.0\"?><ifcXML{namespace}{schema}>{body}</ifcXML>").into_bytes()
}

fn assert_qualified_attributes_are_bound(bytes: &[u8]) {
    let mut reader = NsReader::from_reader(bytes);
    let mut buffer = Vec::new();
    loop {
        match reader
            .read_event_into(&mut buffer)
            .expect("writer emits well-formed XML")
        {
            Event::Start(element) | Event::Empty(element) => {
                for attribute in element.attributes() {
                    let attribute = attribute.expect("writer emits valid attributes");
                    let (namespace, _) = reader.resolve_attribute(attribute.key);
                    assert!(
                        !matches!(namespace, ResolveResult::Unknown(_)),
                        "writer emitted an unbound namespace prefix"
                    );
                }
            }
            Event::Eof => break,
            _ => {}
        }
        buffer.clear();
    }
}

#[test]
fn strict_ifc4_accepts_only_its_namespace_and_profile_token() {
    let valid = document(
        Some(IFC4_NS),
        Some("IFC4"),
        "<IFCANNOTATION id=\"i1\" a0=\"guid\"/>",
    );
    assert!(strict().read_bytes(&valid).is_ok());

    for invalid in [
        document(None, Some("IFC4"), ""),
        document(Some("urn:wrong"), Some("IFC4"), ""),
        document(Some(IFC4_NS), None, ""),
        document(Some(IFC4_NS), Some("IFC2X3"), ""),
    ] {
        let error = strict()
            .read_bytes(&invalid)
            .expect_err("strict profile must reject namespace/profile drift");
        let detail = error.to_string();
        assert!(
            detail.contains("namespace") || detail.contains("profile"),
            "unexpected error: {detail}"
        );
    }
}

#[test]
fn strict_reader_requires_the_ifcxml_root() {
    let wrong = format!("<wrong xmlns=\"{IFC4_NS}\" schema=\"IFC4\"/>");
    for invalid in [Vec::new(), wrong.into_bytes()] {
        let error = strict()
            .read_bytes(&invalid)
            .expect_err("strict mode requires an ifcXML root");
        assert!(error.to_string().contains("root"));
    }
}

#[test]
fn strict_reader_rejects_multiple_or_nested_roots() {
    let root = format!(r#"<ifcXML xmlns="{IFC4_NS}" schema="IFC4"/>"#);
    let multiple = format!("{root}{root}");
    let nested =
        format!(r#"<ifcXML xmlns="{IFC4_NS}" schema="IFC4"><ifcXML schema="IFC4"/></ifcXML>"#);
    for invalid in [multiple, nested] {
        let error = strict()
            .read_bytes(invalid.as_bytes())
            .expect_err("strict mode requires one document root");
        assert!(error.to_string().contains("root"), "{error}");
    }
}

#[test]
fn strict_reader_resolves_prefixes_and_rejects_local_name_spoofing() {
    let valid = format!(
        "<?xml version=\"1.0\"?><ifc:ifcXML xmlns:ifc=\"{IFC4_NS}\" schema=\"IFC4\"><ifc:IFCANNOTATION id=\"i1\" a0=\"guid\"/></ifc:ifcXML>"
    );
    assert!(strict().read_bytes(valid.as_bytes()).is_ok());

    let spoofed = format!(
        "<?xml version=\"1.0\"?><ifc:ifcXML xmlns:ifc=\"{IFC4_NS}\" xmlns:evil=\"urn:evil\" schema=\"IFC4\"><evil:IFCANNOTATION id=\"i1\" a0=\"guid\"/></ifc:ifcXML>"
    );
    let error = strict()
        .read_bytes(spoofed.as_bytes())
        .expect_err("matching local names in another namespace are not IFC");
    assert!(error.to_string().contains("namespace"));
}

#[test]
fn strict_writer_emits_the_official_namespace_and_round_trips() {
    let mut model = Model::new();
    model.header_mut().schema = vec!["IFC4".into()];
    model.insert(
        EntityId(1),
        Entity::new("IFCANNOTATION", vec![Value::Text("guid".into())]),
    );

    let bytes = strict().write_bytes(&model).unwrap();
    let xml = String::from_utf8(bytes.clone()).unwrap();
    assert!(xml.contains(&format!("xmlns=\"{IFC4_NS}\"")));
    assert!(strict().read_bytes(&bytes).is_ok());

    model.header_mut().schema = vec!["IFC2X3".into()];
    let error = strict()
        .write_bytes(&model)
        .expect_err("writer must not relabel an IFC2X3 model as IFC4");
    assert!(error.to_string().contains("profile"));
}

#[test]
fn writer_binds_xsi_nil_and_round_trips_null_values() {
    for codec in [XmlCodec::default(), strict()] {
        let mut model = Model::new();
        model.header_mut().schema = vec!["IFC4".into()];
        model.insert(EntityId(2), Entity::new("IFCANNOTATION", vec![Value::Null]));

        let bytes = codec.write_bytes(&model).expect("null values are writable");
        assert_qualified_attributes_are_bound(&bytes);
        let xml = String::from_utf8(bytes.clone()).unwrap();
        assert!(xml.contains("xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\""));
        let read = codec
            .read_bytes(&bytes)
            .expect("written null XML is readable");
        assert_eq!(
            read.get(EntityId(2)).unwrap().attribute(0),
            Some(&Value::Null)
        );
    }
}

#[test]
fn self_closing_typed_values_do_not_bypass_scalar_validation() {
    for kind in ["integer", "real", "ref", "mystery"] {
        let xml = document(
            Some(IFC4_NS),
            Some("IFC4"),
            &format!("<IFCANNOTATION id=\"i1\"><a0 kind=\"{kind}\"/></IFCANNOTATION>"),
        );
        let codec = strict();
        let error = ifc_xml::reader::read(&codec, &xml)
            .expect_err("empty typed values are not implicit nulls");
        assert_eq!(
            error.path().map(ToString::to_string).as_deref(),
            Some("/ifcXML/IFCANNOTATION[@id='i1']/a0")
        );
        if kind == "mystery" {
            assert!(matches!(
                error.root_cause(),
                XmlError::UnknownKind(actual) if actual == kind
            ));
        } else {
            assert!(matches!(
                error.root_cause(),
                XmlError::InvalidScalar { kind: actual, value }
                    if actual == kind && value.is_empty()
            ));
        }
    }

    let spoofed_nil = document(
        Some(IFC4_NS),
        Some("IFC4"),
        "<IFCANNOTATION id=\"i1\"><a0 xmlns:evil=\"urn:evil\" evil:nil=\"true\" kind=\"ref\"/></IFCANNOTATION>",
    );
    let codec = strict();
    let error = ifc_xml::reader::read(&codec, &spoofed_nil)
        .expect_err("a local-name spoof is not an XSI null marker");
    assert!(matches!(
        error.root_cause(),
        XmlError::InvalidScalar { kind, value } if kind == "ref" && value.is_empty()
    ));

    let explicit_nil = document(
        Some(IFC4_NS),
        Some("IFC4"),
        "<IFCANNOTATION id=\"i1\"><a0 xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" xsi:nil=\"true\"/></IFCANNOTATION>",
    );
    let model = strict()
        .read_bytes(&explicit_nil)
        .expect("explicit nil remains supported");
    assert_eq!(
        model.get(EntityId(1)).unwrap().attribute(0),
        Some(&Value::Null)
    );
}

#[test]
fn compatibility_codec_still_round_trips_unknown_schema_documents() {
    let legacy = document(
        Some("http://www.buildingsmart-tech.org/ifcXML/UNKNOWN/final"),
        Some("VENDOR_SCHEMA"),
        "<VENDORTHING id=\"i7\" a0=\"opaque\"/>",
    );
    let model = XmlCodec::default().read_bytes(&legacy).unwrap();
    assert_eq!(model.get(EntityId(7)).unwrap().text(0), Some("opaque"));
}
