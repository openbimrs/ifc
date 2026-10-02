//! Schema-aware reading of the native layout (#266): with a schema, every
//! value is typed from its attribute's declaration and every name must be
//! one the entity declares. [`SchemaReading::Lenient`] keeps the inference
//! the pre-0.4 reader used, and the tests pin where the two differ.
#![cfg(feature = "schema")]

use ifc_model::{Codec, Entity, EntityId, Model, Value};
use ifc_xml::{SchemaReading, XmlCodec, XmlError};
use std::sync::Arc;

const GUID: &str = "0YvctVUKr0kugbFTf53O9L";

fn strict() -> XmlCodec {
    XmlCodec::with_schema(Arc::new(ifc_schema::ifc4().clone()))
}

fn lenient() -> XmlCodec {
    strict().with_reading(SchemaReading::Lenient)
}

fn read(codec: &XmlCodec, body: &str) -> Result<Model, XmlError> {
    let xml =
        format!("<ifcXML xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\">{body}</ifcXML>");
    ifc_xml::reader::read(codec, xml.as_bytes())
}

fn slot(model: &Model, id: u64, name: &str) -> Value {
    let entity = model.get(EntityId(id)).expect("entity");
    let names = ifc_schema::ifc4().attribute_names(&entity.type_name);
    let index = names
        .iter()
        .position(|candidate| *candidate == name)
        .expect("slot");
    entity.attributes[index].clone()
}

#[test]
fn strict_is_the_default_with_a_schema() {
    assert_eq!(strict().reading(), SchemaReading::Strict);
    assert_eq!(XmlCodec::default().reading(), SchemaReading::Strict);
}

/// The defect #266 names: `Name="1"` is a label, not an integer.
#[test]
fn a_numeric_looking_label_reads_as_a_label() {
    let body = format!(r#"<IFCWALL id="i1" GlobalId="{GUID}" Name="1" Tag="i7"/>"#);
    let model = read(&strict(), &body).unwrap();
    assert_eq!(slot(&model, 1, "Name"), Value::Text("1".into()));
    assert_eq!(slot(&model, 1, "Tag"), Value::Text("i7".into()));

    // The lenient read infers the kind from the text, as before.
    let model = read(&lenient(), &body).unwrap();
    assert_eq!(slot(&model, 1, "Name"), Value::Integer(1));
    assert_eq!(slot(&model, 1, "Tag"), Value::Ref(EntityId(7)));
}

#[test]
fn enumeration_and_number_text_is_typed_from_the_declaration() {
    let model = read(
        &strict(),
        &format!(
            r#"<IFCWALLTYPE id="i1" GlobalId="{GUID}" PredefinedType="NOTDEFINED"/>
               <IFCCARTESIANPOINT id="i2"><Coordinates kind="list"><item kind="real">1.5</item></Coordinates></IFCCARTESIANPOINT>
               <IFCMATERIALLAYER id="i3" LayerThickness="5"/>"#
        ),
    )
    .unwrap();
    assert_eq!(
        slot(&model, 1, "PredefinedType"),
        Value::Enum("NOTDEFINED".into())
    );
    assert_eq!(slot(&model, 3, "LayerThickness"), Value::Real(5.0));
    let error = read(
        &strict(),
        &format!(r#"<IFCWALLTYPE id="i1" GlobalId="{GUID}" PredefinedType="DOOR"/>"#),
    )
    .unwrap_err();
    assert!(
        matches!(error.root_cause(), XmlError::InvalidScalar { .. }),
        "{error:?}"
    );
}

/// The other defect #266 names: a name no attribute declares is refused,
/// naming entity, element and attribute, never placed in a free slot.
#[test]
fn an_undeclared_name_is_refused_with_entity_element_and_attribute() {
    for body in [
        format!(r#"<IFCWALL id="i1" GlobalId="{GUID}" Colour="red"/>"#),
        format!(
            r#"<IFCWALL id="i1" GlobalId="{GUID}"><Colour kind="string">red</Colour></IFCWALL>"#
        ),
    ] {
        let error = read(&strict(), &body).unwrap_err();
        assert!(
            matches!(
                error.root_cause(),
                XmlError::UnknownAttribute { entity, element, attribute }
                    if entity == "IfcWall" && element == "IFCWALL" && attribute == "Colour"
            ),
            "{error:?}"
        );
        assert_eq!(
            error.path().map(ToString::to_string).as_deref(),
            Some("/ifcXML/IFCWALL[@id='i1']")
        );
        // The lenient read keeps it, after the declared slots.
        let model = read(&lenient(), &body).unwrap();
        assert_eq!(
            model.get(EntityId(1)).unwrap().attributes.last(),
            Some(&Value::Text("red".into()))
        );
    }
    // A positional name on a declared entity is not a declared name either.
    let error = read(&strict(), r#"<IFCWALL id="i1" a0="x"/>"#).unwrap_err();
    assert!(
        matches!(error.root_cause(), XmlError::UnknownAttribute { attribute, .. } if attribute == "a0")
    );
}

#[test]
fn an_undeclared_or_abstract_entity_is_refused() {
    let error = read(&strict(), r#"<IFCFUTUREWALL id="i1"/>"#).unwrap_err();
    assert!(
        matches!(error.root_cause(), XmlError::UnknownEntity { name } if name == "IFCFUTUREWALL")
    );
    let error = read(&strict(), r#"<IFCROOT id="i1"/>"#).unwrap_err();
    assert!(matches!(
        error.root_cause(),
        XmlError::AbstractEntity { .. }
    ));
    // Kept by the lenient read, for the round trip of unknown schemas.
    assert_eq!(
        read(&lenient(), r#"<IFCFUTUREWALL id="i1" a0="x"/>"#)
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn an_explicit_kind_the_declaration_does_not_admit_is_refused() {
    for (body, why) in [
        (
            format!(r#"<IFCWALL id="i1" GlobalId="{GUID}"><Name kind="integer">1</Name></IFCWALL>"#),
            "an integer in a label",
        ),
        (
            r#"<IFCPROPERTYSINGLEVALUE id="i1" Name="w"><NominalValue kind="real">1.0</NominalValue></IFCPROPERTYSINGLEVALUE>"#.into(),
            "a bare value in a SELECT",
        ),
        (
            r#"<IFCPROPERTYSINGLEVALUE id="i1" Name="w"><NominalValue kind="typed" type="IFCWALLTYPEENUM"><value kind="enum">NOTDEFINED</value></NominalValue></IFCPROPERTYSINGLEVALUE>"#.into(),
            "a typed value outside the SELECT",
        ),
        (
            r#"<IFCPROPERTYSINGLEVALUE id="i1" Name="w"><NominalValue kind="typed" type="IFCLENGTHMEASURE"><value kind="string">1</value></NominalValue></IFCPROPERTYSINGLEVALUE>"#.into(),
            "a string in a typed measure",
        ),
        (
            format!(r#"<IFCWALL id="i1" GlobalId="{GUID}" ObjectPlacement="i2"/><IFCPERSON id="i2"/>"#),
            "a reference to an entity of the wrong type",
        ),
        (
            format!(r#"<IFCWALL id="i1" GlobalId="{GUID}" ObjectPlacement="i9"/>"#),
            "a reference to no entity",
        ),
    ] {
        let error = read(&strict(), &body).unwrap_err();
        assert!(
            matches!(
                error.root_cause(),
                XmlError::TypeMismatch { .. } | XmlError::UnresolvedReference { .. }
            ),
            "{why}: {error:?}"
        );
    }
    let model = read(
        &strict(),
        r#"<IFCPROPERTYSINGLEVALUE id="i1" Name="w"><NominalValue kind="typed" type="IFCLABEL"><value kind="string">1</value></NominalValue></IFCPROPERTYSINGLEVALUE>"#,
    )
    .unwrap();
    assert_eq!(
        slot(&model, 1, "NominalValue"),
        Value::Typed {
            type_name: "IFCLABEL".into(),
            value: Box::new(Value::Text("1".into())),
        }
    );
}

/// A model the schema does not describe is written losslessly, and the
/// strict read refuses it instead of reading back something else.
#[test]
fn a_non_conformant_model_is_refused_not_changed() {
    let mut model = Model::new();
    let mut wall = vec![Value::Null; ifc_schema::ifc4().attribute_names("IfcWall").len()];
    wall[0] = Value::Text(GUID.into());
    wall[2] = Value::Integer(1); // Name, an IfcLabel
    model.insert(EntityId(1), Entity::new("IFCWALL", wall.clone()));

    let xml = strict().write_bytes(&model).unwrap();
    let error = ifc_xml::reader::read(&strict(), &xml).unwrap_err();
    assert!(
        matches!(error.root_cause(), XmlError::TypeMismatch { .. }),
        "{error:?}"
    );
    // The lenient read of the same bytes still round-trips it.
    let back = lenient().read_bytes(&xml).unwrap();
    assert_eq!(back.get(EntityId(1)).unwrap().attributes, wall);

    // The conformant model round-trips through both reads.
    wall[2] = Value::Text("1".into());
    model.insert(EntityId(1), Entity::new("IFCWALL", wall.clone()));
    let xml = strict().write_bytes(&model).unwrap();
    for codec in [strict(), lenient()] {
        let back = codec.read_bytes(&xml).unwrap();
        assert_eq!(back.get(EntityId(1)).unwrap().attributes, wall);
    }
}
