//! Writing the buildingSMART XSD configuration, one construct at a time.
//!
//! Each written document is read back with the XSD reader and must be the
//! model it was written from; what the configuration cannot carry exactly is
//! refused with a typed error naming where. `xsd_output.rs` validates the
//! same documents against `IFC4.xsd` (opt in).
#![cfg(feature = "schema")]

#[path = "support/xsd_models.rs"]
mod support;

use ifc_model::{Codec, EntityId, Model, ModelError, Value};
use ifc_xml::{XmlCodec, XmlError, XmlProfile};
use std::sync::Arc;
use support::{model, same_model, GEOMETRY, SPATIAL};

const NAMESPACE: &str = "https://standards.buildingsmart.org/IFC/RELEASE/IFC4/ADD2_TC1/XML";

fn codec() -> XmlCodec {
    XmlCodec::xsd(
        Arc::new(ifc_schema::ifc4().clone()),
        XmlProfile::Ifc4Add2Tc1,
    )
}

fn write(model: &Model) -> Result<String, XmlError> {
    ifc_xml::writer::write(&codec(), model).map(|bytes| String::from_utf8(bytes).unwrap())
}

/// Write, read back, and require the same model; returns the document.
fn round_trip(model: &Model) -> String {
    let text = write(model).unwrap_or_else(|error| panic!("write refused: {error}"));
    let read = ifc_xml::reader::read(&codec(), text.as_bytes())
        .unwrap_or_else(|error| panic!("read back refused: {error}\n{text}"));
    if let Err(difference) = same_model(ifc_schema::ifc4(), model, &read) {
        panic!("{difference}\n{text}");
    }
    text
}

/// The refusal of writing `model`, which must carry a path.
fn refused(model: &Model) -> XmlError {
    let error = write(model).expect_err("written, expected a refusal");
    assert!(error.path().is_some(), "refusal without a path: {error}");
    error
}

fn assert_contains(text: &str, expected: &[&str]) {
    for snippet in expected {
        assert!(text.contains(snippet), "missing {snippet:?} in\n{text}");
    }
}

/// `model` with one entity's slot replaced.
fn replace(model: &mut Model, id: u64, slot: usize, value: Value) {
    let mut entity = model.get(EntityId(id)).unwrap().clone();
    entity.attributes[slot] = value;
    model.insert(EntityId(id), entity);
}

/// `SPATIAL` with one entity's slot replaced.
fn spatial_with(id: u64, slot: usize, value: Value) -> Model {
    let mut model = model(SPATIAL);
    replace(&mut model, id, slot, value);
    model
}

#[test]
fn every_construct_model_round_trips() {
    for (name, model) in support::models() {
        assert!(write(&model).is_ok(), "{name}");
        round_trip(&model);
    }
}

#[test]
fn the_root_and_header_are_the_xsd_s() {
    let text = round_trip(&model(SPATIAL));
    assert_contains(
        &text,
        &[
            &format!("<ifcXML xmlns=\"{NAMESPACE}\" xmlns:ifc=\"{NAMESPACE}\""),
            "<header>\n    <name>model.ifc</name>\n    \
             <time_stamp>2026-10-03T12:00:00+02:00</time_stamp>\n    \
             <author>Author</author>\n    <organization>Organisation</organization>\n    \
             <preprocessor_version>preprocessor</preprocessor_version>\n    \
             <originating_system>system</originating_system>\n    \
             <authorization>authorization</authorization>\n    \
             <documentation>ViewDefinition [CoordinationView]</documentation>\n  </header>",
        ],
    );
    // No `schema` attribute and no upper-case STEP names: the native
    // layout's departures from the XSD.
    assert!(!text.contains(" schema="), "{text}");
    assert!(!text.contains("<IFC"), "{text}");
}

#[test]
fn entities_are_top_level_elements_named_as_the_schema_spells_them() {
    let text = round_trip(&model(SPATIAL));
    assert_contains(
        &text,
        &[
            "<IfcProject id=\"i1\" GlobalId=\"0YvctVUKr0kugbFTf53O9L\" Name=\"Project\">",
            "<IfcSIUnit id=\"i31\" UnitType=\"lengthunit\" Prefix=\"milli\" Name=\"metre\"/>",
        ],
    );
}

#[test]
fn simple_values_are_xml_attributes_in_their_xsd_lexical_form() {
    let text = round_trip(&model(SPATIAL));
    assert_contains(
        &text,
        &[
            // Decimal point always '.', the shortest round-trip form.
            "Precision=\"1e-5\"",
            "Coordinates=\"1.25e20 -3.5e-7\"",
            "OverallHeight=\"2.1\"",
            // A list attribute, whitespace separated.
            "RefLatitude=\"51 30 0 0\"",
            // Enumerations in lower case; escaping.
            "PredefinedType=\"standard\"",
            "Name=\"W &amp; &lt;wall&gt; &quot;quoted&quot; &apos;single&apos;\"",
        ],
    );
    let text = round_trip(&model(GEOMETRY));
    assert_contains(
        &text,
        &[
            // A nested list with fixed inner sizes, flattened.
            "CoordList=\"0.0 0.0 0.0 1.0 0.0 0.0 0.0 1.0 0.0 1.0 1.0 1.0\"",
            // String lists the configuration writes as list attributes.
            "MiddleNames=\"Middle Other\"",
            "Parameter=\"a b\"",
            // Booleans and the logical unknown.
            "RepeatS=\"true\" RepeatT=\"false\"",
            "SelfIntersect=\"unknown\"",
        ],
    );
}

#[test]
fn entity_attributes_are_nil_references_typed_when_a_subtype() {
    let text = round_trip(&model(SPATIAL));
    assert_contains(
        &text,
        &[
            "<Location ref=\"i21\" xsi:nil=\"true\"/>",
            // IfcObjectPlacement is abstract: the XSD needs the type.
            "<ObjectPlacement xsi:type=\"IfcLocalPlacement\" ref=\"i52\" xsi:nil=\"true\"/>",
        ],
    );
}

#[test]
fn selects_hold_entity_elements_or_typed_wrappers() {
    let text = round_trip(&model(SPATIAL));
    assert_contains(
        &text,
        &[
            "<NominalValue>\n      <IfcLabel-wrapper>x  y </IfcLabel-wrapper>\n    </NominalValue>",
            "<IfcComplexNumber-wrapper>1.5 -2.0</IfcComplexNumber-wrapper>",
            "<RelativePlacement>\n      <IfcAxis2Placement3D ref=\"i51\" xsi:nil=\"true\"/>",
            // An aggregate defined type of entities in a SELECT.
            "<IfcPropertySetDefinitionSet-wrapper>\n        \
             <IfcPropertySet ref=\"i16\" xsi:nil=\"true\"/>\n      \
             </IfcPropertySetDefinitionSet-wrapper>",
        ],
    );
}

#[test]
fn aggregates_are_containers_of_their_items() {
    let text = round_trip(&model(GEOMETRY));
    assert_contains(
        &text,
        &[
            // Entities, flattened with the sizes the schema does not fix.
            "<ControlPointsList ifc:arraySize=\"2 2\">\n      \
             <IfcCartesianPoint ref=\"i10\" xsi:nil=\"true\"/>",
            // Simple values the configuration writes as a container.
            "<WeightsData ifc:arraySize=\"2 2\">\n      <IfcReal-wrapper>1.0</IfcReal-wrapper>",
            "<AddressLines>\n      <IfcLabel-wrapper>Line 1</IfcLabel-wrapper>\n      \
             <IfcLabel-wrapper>Line two</IfcLabel-wrapper>",
            // `Seq-` wrapped inner lists.
            "<InnerCoordIndices>\n      \
             <Seq-IfcPositiveInteger-wrapper>2 3 4</Seq-IfcPositiveInteger-wrapper>\n      \
             <Seq-IfcPositiveInteger-wrapper>1 2 4 3</Seq-IfcPositiveInteger-wrapper>",
            // Aggregate defined types in a SELECT list.
            "<IfcLineIndex-wrapper>1 2</IfcLineIndex-wrapper>",
            "<IfcArcIndex-wrapper>1 2 3</IfcArcIndex-wrapper>",
            // A binary is element text, in hex.
            "<RasterCode>89504E47</RasterCode>",
        ],
    );
}

#[test]
fn omitted_attributes_are_written_through_inverses() {
    let text = round_trip(&model(SPATIAL));
    assert_contains(
        &text,
        &[
            // RelatingObject is left off the relationship.
            "<IfcRelAggregates id=\"i3\" GlobalId=\"2YvctVUKr0kugbFTf53O9L\">\n    \
             <RelatedObjects>",
            "<IsDecomposedBy>\n      <IfcRelAggregates ref=\"i3\" xsi:nil=\"true\"/>\n    </IsDecomposedBy>",
            "<ContainsElements>\n      <IfcRelContainedInSpatialStructure ref=\"i7\"",
            // Direct inverses: the element is the one relationship.
            "<IsTypedBy ref=\"i9\" xsi:nil=\"true\"/>",
            "<HasOpenings ref=\"i11\" xsi:nil=\"true\"/>",
            "<HasFillings ref=\"i13\" xsi:nil=\"true\"/>",
            "<HasCoordinateOperation xsi:type=\"IfcMapConversion\" ref=\"i41\" xsi:nil=\"true\"/>",
            "<HasSubContexts>\n      <IfcGeometricRepresentationSubContext ref=\"i23\"",
            // A set written through its members' inverses.
            "<IsDefinedBy>\n      <IfcRelDefinesByProperties ref=\"i17\"",
        ],
    );
    assert!(!text.contains("RelatingObject"), "{text}");
    let text = round_trip(&model(GEOMETRY));
    assert_contains(&text, &["<StyledByItem ref=\"i21\" xsi:nil=\"true\"/>"]);
    assert!(!text.contains("<Item"), "{text}");
}

#[test]
fn a_model_numbered_in_order_reads_back_with_its_ids() {
    let source = model("#1=IFCCARTESIANPOINT((1.,2.));\n#2=IFCAXIS2PLACEMENT2D(#1,$);\n");
    let read = codec()
        .read_bytes(&codec().write_bytes(&source).unwrap())
        .unwrap();
    for (id, entity) in source.iter() {
        assert_eq!(read.get(id).unwrap().attributes, entity.attributes, "{id}");
    }
}

/// Codec-level errors surface as `ModelError::Write`.
#[test]
fn the_codec_reports_refusals_as_write_errors() {
    let model = spatial_with(14, 2, Value::Text("text".into()));
    assert!(matches!(
        codec().write_bytes(&model),
        Err(ModelError::Write(message)) if message.contains("NominalValue")
    ));
}

#[test]
fn values_the_reader_would_read_differently_are_refused() {
    // An integer where a REAL is declared reads back as a real.
    let error = refused(&spatial_with(12, 8, Value::Integer(2)));
    assert!(
        matches!(error.root_cause(), XmlError::Unrepresentable { .. }),
        "{error}"
    );
    assert_eq!(
        error.path().unwrap().as_str(),
        "/ifcXML/IfcDoor[@id='i12']/@OverallHeight"
    );
    // A string with whitespace in a list attribute splits differently.
    let mut model = model(GEOMETRY);
    replace(
        &mut model,
        31,
        3,
        Value::List(vec![Value::Text("van der".into())]),
    );
    let error = refused(&model);
    assert!(
        error.to_string().contains("whitespace-separated list"),
        "{error}"
    );
    // A binary that is not whole bytes.
    let mut model = support::model(GEOMETRY);
    replace(&mut model, 20, 6, Value::Binary("1ABC".into()));
    let error = refused(&model);
    assert!(
        error.to_string().contains("whole number of bytes"),
        "{error}"
    );
}

#[test]
fn values_the_xsd_refuses_are_refused() {
    // xs:normalizedString has no line break.
    let error = refused(&spatial_with(6, 2, Value::Text("two\nlines".into())));
    assert!(error.to_string().contains("normalizedString"), "{error}");
    // IfcLabel is STRING(255).
    let error = refused(&spatial_with(6, 2, Value::Text("x".repeat(256).into())));
    assert!(error.to_string().contains("256 characters"), "{error}");
    // A character XML 1.0 cannot carry.
    let error = refused(&spatial_with(6, 2, Value::Text("bell\u{7}".into())));
    assert!(error.to_string().contains("U+0007"), "{error}");
    // The XSD requires a mandatory attribute's element.
    let error = refused(&spatial_with(52, 1, Value::Null));
    assert!(
        error.to_string().contains("unset `RelativePlacement`"),
        "{error}"
    );
    // Too few items for the declared bounds.
    let error = refused(&spatial_with(16, 4, Value::List(Vec::new())));
    assert!(error.to_string().contains("[1:?]"), "{error}");
    // A non-finite real.
    let error = refused(&spatial_with(12, 8, Value::Real(f64::NAN)));
    assert!(
        matches!(error.root_cause(), XmlError::InvalidScalar { .. }),
        "{error}"
    );
    // A time stamp that is not an xs:dateTime.
    let mut model = model(SPATIAL);
    model.header_mut().time_stamp = "2026-10-03 12:00".into();
    assert!(matches!(
        refused(&model).root_cause(),
        XmlError::InvalidScalar { .. }
    ));
}

#[test]
fn relationships_the_configuration_has_no_place_for_are_refused() {
    // A second opening: the XSD allows one HasOpenings element.
    let mut model = model(SPATIAL);
    let mut second = model.get(EntityId(11)).unwrap().clone();
    second.attributes[0] = Value::Text("GYvctVUKr0kugbFTf53O9L".into());
    model.insert(EntityId(60), second);
    let error = refused(&model);
    assert!(error.to_string().contains("HasOpenings"), "{error}");
    // IfcRelDefinesByObject.RelatingObject has no form at all.
    let model = support::model(&format!(
        "{SPATIAL}#61=IFCRELDEFINESBYOBJECT('HYvctVUKr0kugbFTf53O9L',$,$,$,(#6),#12);\n"
    ));
    let error = refused(&model);
    assert!(error.to_string().contains("RelatingObject"), "{error}");
    // An IFC4 IfcOrientedEdge cannot carry StyledByItem: its XSD type
    // restricts the inverse away.
    let model = support::model(
        "#1=IFCCARTESIANPOINT((0.,0.,0.));\n#2=IFCVERTEXPOINT(#1);\n\
         #3=IFCEDGE(#2,#2);\n#4=IFCORIENTEDEDGE(*,*,#3,.T.);\n\
         #5=IFCSTYLEDITEM(#4,(#6),$);\n#6=IFCSURFACESTYLE($,.BOTH.,(#7));\n\
         #7=IFCSURFACESTYLESHADING(#8,$);\n#8=IFCCOLOURRGB($,1.,1.,1.);\n",
    );
    let error = refused(&model);
    assert!(error.to_string().contains("no inverse element"), "{error}");
}

#[test]
fn models_the_schema_does_not_describe_are_refused() {
    let mut unknown = model(SPATIAL);
    unknown.insert(
        EntityId(70),
        ifc_model::Entity::new("IFCNOTANENTITY", Vec::new()),
    );
    assert!(matches!(
        refused(&unknown).root_cause(),
        XmlError::UnknownEntity { .. }
    ));
    let dangling = spatial_with(22, 0, Value::Ref(EntityId(99)));
    assert!(matches!(
        refused(&dangling).root_cause(),
        XmlError::UnresolvedReference { id } if id == "i99"
    ));
    // A reference to an entity the declaration does not admit.
    let wrong = spatial_with(22, 0, Value::Ref(EntityId(31)));
    assert!(matches!(
        refused(&wrong).root_cause(),
        XmlError::TypeMismatch { .. }
    ));
    // A value in a slot the subtype derives.
    let derived = spatial_with(23, 2, Value::Integer(3));
    assert!(refused(&derived).to_string().contains("derives"));
}

#[test]
fn the_profile_and_schema_must_match_the_model() {
    let mut model = model(SPATIAL);
    model.header_mut().schema = vec!["IFC4X3_ADD2".into()];
    assert!(matches!(
        write(&model).unwrap_err(),
        XmlError::Profile {
            expected: "IFC4",
            ..
        }
    ));
    let mismatched = XmlCodec::xsd(
        Arc::new(ifc_schema::ifc4x3().clone()),
        XmlProfile::Ifc4Add2Tc1,
    );
    assert!(matches!(
        ifc_xml::writer::write(&mismatched, &support::model(SPATIAL)).unwrap_err(),
        XmlError::SchemaMismatch { .. }
    ));
    let mut authors = support::model(SPATIAL);
    authors.header_mut().author = vec!["One".into(), "Two".into()];
    assert!(refused(&authors)
        .to_string()
        .contains("2 header `author` entries"));
}
