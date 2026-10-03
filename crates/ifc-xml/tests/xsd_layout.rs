//! The buildingSMART XSD configuration of ifcXML, read into the model its
//! STEP form reads into, construct by construct; and refused, with a typed
//! error, where it cannot be read exactly.
//!
//! Every document here is hand-written against the IFC4 ADD2 TC1 XSD. The
//! expected entities are laid out by attribute *name* through the bundled
//! schema, so a slot order is never restated from memory.
#![cfg(feature = "schema")]

use ifc_model::{Entity, EntityId, Model, ModelError, Value};
use ifc_schema::Schema;
use ifc_xml::{XmlCodec, XmlError, XmlLayout, XmlProfile};
use std::sync::Arc;

const NAMESPACE: &str = "https://standards.buildingsmart.org/IFC/RELEASE/IFC4/ADD2_TC1/XML";

fn schema() -> &'static Schema {
    ifc_schema::ifc4()
}

fn codec() -> XmlCodec {
    XmlCodec::xsd(Arc::new(schema().clone()), XmlProfile::Ifc4Add2Tc1)
}

fn document(body: &str) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <ifc:ifcXML xmlns:ifc=\"{NAMESPACE}\" xmlns=\"{NAMESPACE}\" \
         xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\">\n{body}\n</ifc:ifcXML>"
    )
}

fn read(body: &str) -> Result<Model, XmlError> {
    ifc_xml::reader::read(&codec(), document(body).as_bytes())
}

fn refused(body: &str) -> XmlError {
    match read(body) {
        Ok(model) => panic!("expected a refusal, read {} entities", model.len()),
        Err(error) => error,
    }
}

/// An entity laid out by attribute name: unnamed slots are unset, slots the
/// type redeclares DERIVE are derived.
fn entity(name: &str, values: &[(&str, Value)]) -> Entity {
    let schema = schema();
    let declared = schema.entity(name).expect("declared entity");
    let chain: Vec<&str> = std::iter::once(declared.name.as_str())
        .chain(schema.supertypes(name))
        .collect();
    let names = schema.attribute_names(name);
    let mut slots: Vec<Value> = names
        .iter()
        .map(|slot| {
            let derived = chain
                .iter()
                .filter_map(|entity| schema.entity(entity))
                .any(|entity| entity.is_derived(slot));
            if derived {
                Value::Derived
            } else {
                Value::Null
            }
        })
        .collect();
    for (slot, value) in values {
        let index = names
            .iter()
            .position(|candidate| candidate == slot)
            .unwrap_or_else(|| panic!("{name} has no attribute {slot}"));
        slots[index] = value.clone();
    }
    Entity::new(declared.name.to_ascii_uppercase(), slots)
}

fn text(value: &str) -> Value {
    Value::Text(value.into())
}

fn reference(id: u64) -> Value {
    Value::Ref(EntityId(id))
}

fn real(value: f64) -> Value {
    Value::Real(value)
}

fn assert_entities(model: &Model, expected: &[Entity]) {
    assert_eq!(model.len(), expected.len(), "entity count");
    for (index, expected) in expected.iter().enumerate() {
        let id = EntityId(index as u64 + 1);
        let found = model.get(id).unwrap_or_else(|| panic!("{id} missing"));
        assert_eq!(found.type_name, expected.type_name, "{id} type");
        assert_eq!(
            found.attributes, expected.attributes,
            "{id} {}",
            found.type_name
        );
    }
}

const GUID_A: &str = "0YvctVUKr0kugbFTf53O9L";
const GUID_B: &str = "1YvctVUKr0kugbFTf53O9L";
const GUID_C: &str = "2YvctVUKr0kugbFTf53O9L";

/// Nested and inline entities, `ref`/`href` with `xsi:nil`, a forward
/// reference, an inverse attribute, a list attribute and an enumeration:
/// numbered in document order, and the same entities the STEP form holds.
#[test]
fn nested_inline_and_referenced_entities_read_in_document_order() {
    let model = read(&format!(
        r##"<IfcProject GlobalId="{GUID_A}" Name="1">
  <OwnerHistory id="oh" ChangeAction="notdefined" CreationDate="1247473054">
    <OwningUser>
      <ThePerson FamilyName="Doe"/>
      <TheOrganization id="org" Name="Org"/>
    </OwningUser>
    <OwningApplication xsi:nil="true" ref="app"/>
  </OwnerHistory>
  <IsDecomposedBy>
    <IfcRelAggregates GlobalId="{GUID_B}">
      <OwnerHistory xsi:nil="true" ref="oh"/>
      <RelatedObjects>
        <IfcSite GlobalId="{GUID_C}" CompositionType="element" RefLatitude="24 28 0">
          <OwnerHistory xsi:nil="true" href="#oh"/>
        </IfcSite>
      </RelatedObjects>
    </IfcRelAggregates>
  </IsDecomposedBy>
</IfcProject>
<IfcApplication id="app" Version="1" ApplicationFullName="App" ApplicationIdentifier="app">
  <ApplicationDeveloper xsi:nil="true" ref="org"/>
</IfcApplication>"##
    ))
    .unwrap();

    assert_entities(
        &model,
        &[
            entity(
                "IfcProject",
                &[
                    ("GlobalId", text(GUID_A)),
                    ("OwnerHistory", reference(2)),
                    // An IfcLabel: `1` is a label, not an integer.
                    ("Name", text("1")),
                ],
            ),
            entity(
                "IfcOwnerHistory",
                &[
                    ("OwningUser", reference(3)),
                    ("OwningApplication", reference(8)),
                    ("ChangeAction", Value::Enum("NOTDEFINED".into())),
                    ("CreationDate", Value::Integer(1_247_473_054)),
                ],
            ),
            entity(
                "IfcPersonAndOrganization",
                &[
                    ("ThePerson", reference(4)),
                    ("TheOrganization", reference(5)),
                ],
            ),
            entity("IfcPerson", &[("FamilyName", text("Doe"))]),
            entity("IfcOrganization", &[("Name", text("Org"))]),
            // The inverse `IsDecomposedBy` supplies `RelatingObject`.
            entity(
                "IfcRelAggregates",
                &[
                    ("GlobalId", text(GUID_B)),
                    ("OwnerHistory", reference(2)),
                    ("RelatingObject", reference(1)),
                    ("RelatedObjects", Value::List(vec![reference(7)])),
                ],
            ),
            entity(
                "IfcSite",
                &[
                    ("GlobalId", text(GUID_C)),
                    ("OwnerHistory", reference(2)),
                    ("CompositionType", Value::Enum("ELEMENT".into())),
                    (
                        "RefLatitude",
                        Value::List(vec![
                            Value::Integer(24),
                            Value::Integer(28),
                            Value::Integer(0),
                        ]),
                    ),
                ],
            ),
            entity(
                "IfcApplication",
                &[
                    ("ApplicationDeveloper", reference(5)),
                    ("Version", text("1")),
                    ("ApplicationFullName", text("App")),
                    ("ApplicationIdentifier", text("app")),
                ],
            ),
        ],
    );
    assert_eq!(model.header().schema, vec!["IFC4".to_string()]);
}

/// The same entities as their STEP form, read by the STEP codec.
#[test]
fn a_document_reads_into_the_model_of_its_step_form() {
    use ifc_model::Codec;
    let step = format!(
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\nFILE_NAME('','',(''),(''),'','','');\n\
         FILE_SCHEMA(('IFC4'));\nENDSEC;\nDATA;\n\
         #1=IFCPROPERTYSET('{GUID_A}',$,'Pset',$,(#2,#3));\n\
         #2=IFCPROPERTYSINGLEVALUE('Width',$,IFCPOSITIVELENGTHMEASURE(0.25),$);\n\
         #3=IFCPROPERTYENUMERATEDVALUE('Kind',$,(IFCLABEL('a'),IFCLABEL('1')),$);\n\
         ENDSEC;\nEND-ISO-10303-21;\n"
    );
    let from_step = ifc_step::StepCodec.read_bytes(step.as_bytes()).unwrap();
    // `itemType`, `cType` and `arraySize` are global attributes of the XSD,
    // so a valid document qualifies them (`ifc:itemType`); the unqualified
    // spelling some exporters use is read too, to the same model.
    for qualifier in ["ifc:", ""] {
        let from_xml = read(&format!(
            r#"<IfcPropertySet GlobalId="{GUID_A}" Name="Pset">
  <HasProperties>
    <IfcPropertySingleValue Name="Width">
      <NominalValue><IfcPositiveLengthMeasure-wrapper>0.25</IfcPositiveLengthMeasure-wrapper></NominalValue>
    </IfcPropertySingleValue>
    <IfcPropertyEnumeratedValue Name="Kind">
      <EnumerationValues {qualifier}itemType="ifc:IfcValue" {qualifier}cType="list">
        <IfcLabel-wrapper>a</IfcLabel-wrapper>
        <IfcLabel-wrapper>1</IfcLabel-wrapper>
      </EnumerationValues>
    </IfcPropertyEnumeratedValue>
  </HasProperties>
</IfcPropertySet>"#
        ))
        .unwrap();
        assert_eq!(from_xml.len(), from_step.len());
        for (id, expected) in from_step.iter() {
            let found = from_xml.get(id).unwrap();
            assert_eq!(found.type_name, expected.type_name, "{id}");
            assert_eq!(found.attributes, expected.attributes, "{id}");
        }
    }
    // Any other qualified attribute is one the XSD does not declare.
    let refused = read(&format!(
        r#"<IfcPropertySet GlobalId="{GUID_A}" ifc:Name="Pset"/>"#
    ));
    assert!(matches!(
        refused.unwrap_err().root_cause(),
        XmlError::UnknownAttribute { attribute, .. } if attribute == "ifc:Name"
    ));
}

/// `-wrapper` values in a SELECT are typed parameters; a wrapper of the
/// aggregate's own defined type is the bare value; `Seq-` wrappers and
/// aggregate defined types read as lists.
#[test]
fn wrappers_read_as_typed_or_bare_values() {
    let model = read(
        r#"<IfcClassification Name="Uniclass">
  <ReferenceTokens>
    <IfcIdentifier-wrapper>Ss</IfcIdentifier-wrapper>
    <IfcIdentifier-wrapper>25 10</IfcIdentifier-wrapper>
  </ReferenceTokens>
</IfcClassification>
<IfcIndexedPolyCurve>
  <Points xsi:type="IfcCartesianPointList2D" CoordList="0 0 1 0 1 1"/>
  <Segments>
    <IfcLineIndex-wrapper>1 2</IfcLineIndex-wrapper>
    <IfcArcIndex-wrapper>1 2 3</IfcArcIndex-wrapper>
  </Segments>
</IfcIndexedPolyCurve>
<IfcIndexedPolygonalFaceWithVoids CoordIndex="1 2 3">
  <InnerCoordIndices>
    <Seq-IfcPositiveInteger-wrapper>4 5 6</Seq-IfcPositiveInteger-wrapper>
    <Seq-IfcPositiveInteger-wrapper>7 8 9 10</Seq-IfcPositiveInteger-wrapper>
  </InnerCoordIndices>
</IfcIndexedPolygonalFaceWithVoids>"#,
    )
    .unwrap();
    let integers =
        |values: &[i64]| Value::List(values.iter().copied().map(Value::Integer).collect());
    assert_entities(
        &model,
        &[
            entity(
                "IfcClassification",
                &[
                    ("Name", text("Uniclass")),
                    // Wrapper text is the whole value: no splitting.
                    (
                        "ReferenceTokens",
                        Value::List(vec![text("Ss"), text("25 10")]),
                    ),
                ],
            ),
            entity(
                "IfcIndexedPolyCurve",
                &[
                    ("Points", reference(3)),
                    (
                        "Segments",
                        Value::List(vec![
                            Value::Typed {
                                type_name: "IFCLINEINDEX".into(),
                                value: Box::new(integers(&[1, 2])),
                            },
                            Value::Typed {
                                type_name: "IFCARCINDEX".into(),
                                value: Box::new(integers(&[1, 2, 3])),
                            },
                        ]),
                    ),
                ],
            ),
            entity(
                "IfcCartesianPointList2D",
                &[(
                    "CoordList",
                    Value::List(vec![
                        Value::List(vec![real(0.0), real(0.0)]),
                        Value::List(vec![real(1.0), real(0.0)]),
                        Value::List(vec![real(1.0), real(1.0)]),
                    ]),
                )],
            ),
            entity(
                "IfcIndexedPolygonalFaceWithVoids",
                &[
                    ("CoordIndex", integers(&[1, 2, 3])),
                    (
                        "InnerCoordIndices",
                        Value::List(vec![integers(&[4, 5, 6]), integers(&[7, 8, 9, 10])]),
                    ),
                ],
            ),
        ],
    );
}

/// Enumerations, booleans and logicals are typed by their declaration, never
/// left as text; binary reads as the STEP form.
#[test]
fn enumeration_boolean_and_binary_text_is_typed_from_the_schema() {
    let model = read(
        r#"<IfcWallType GlobalId="0YvctVUKr0kugbFTf53O9L" PredefinedType="NOTDEFINED"/>
<IfcBlobTexture RepeatS="true" RepeatT="0" RasterFormat="PNG">
  <RasterCode>
    89504e47
  </RasterCode>
</IfcBlobTexture>
<IfcMaterialLayer LayerThickness="5" IsVentilated="unknown"/>"#,
    )
    .unwrap();
    assert_entities(
        &model,
        &[
            entity(
                "IfcWallType",
                &[
                    ("GlobalId", text(GUID_A)),
                    ("PredefinedType", Value::Enum("NOTDEFINED".into())),
                ],
            ),
            entity(
                "IfcBlobTexture",
                &[
                    ("RepeatS", Value::Bool(true)),
                    ("RepeatT", Value::Bool(false)),
                    ("RasterFormat", text("PNG")),
                    ("RasterCode", Value::Binary("089504E47".into())),
                ],
            ),
            entity(
                "IfcMaterialLayer",
                &[
                    // An integer literal in a REAL slot is a real.
                    ("LayerThickness", real(5.0)),
                    ("IsVentilated", Value::LogicalUnknown),
                ],
            ),
        ],
    );
}

/// An inverse element may be the relationship itself rather than a
/// container of them: with XML attributes, or with only child elements.
#[test]
fn inverse_attributes_supply_the_attribute_they_invert() {
    let model = read(&format!(
        r#"<IfcWall GlobalId="{GUID_A}">
  <HasOpenings GlobalId="{GUID_B}">
    <RelatedOpeningElement xsi:type="IfcOpeningElement" GlobalId="{GUID_C}"/>
  </HasOpenings>
</IfcWall>
<IfcPolyline>
  <Points><IfcCartesianPoint Coordinates="0 0"/><IfcCartesianPoint Coordinates="1 0"/></Points>
  <StyledByItem>
    <Styles><IfcCurveStyle Name="thin"/></Styles>
  </StyledByItem>
</IfcPolyline>
<IfcGeometricRepresentationContext ContextType="Model" CoordinateSpaceDimension="3">
  <WorldCoordinateSystem><IfcAxis2Placement3D><Location Coordinates="0 0 0"/></IfcAxis2Placement3D></WorldCoordinateSystem>
  <HasSubContexts>
    <IfcGeometricRepresentationSubContext ContextIdentifier="Body" ContextType="Model" CoordinateSpaceDimension="3" TargetView="model_view"/>
  </HasSubContexts>
</IfcGeometricRepresentationContext>"#
    ))
    .unwrap();
    assert_entities(
        &model,
        &[
            entity("IfcWall", &[("GlobalId", text(GUID_A))]),
            entity(
                "IfcRelVoidsElement",
                &[
                    ("GlobalId", text(GUID_B)),
                    ("RelatingBuildingElement", reference(1)),
                    ("RelatedOpeningElement", reference(3)),
                ],
            ),
            entity("IfcOpeningElement", &[("GlobalId", text(GUID_C))]),
            entity(
                "IfcPolyline",
                &[("Points", Value::List(vec![reference(5), reference(6)]))],
            ),
            entity(
                "IfcCartesianPoint",
                &[("Coordinates", Value::List(vec![real(0.0), real(0.0)]))],
            ),
            entity(
                "IfcCartesianPoint",
                &[("Coordinates", Value::List(vec![real(1.0), real(0.0)]))],
            ),
            entity(
                "IfcStyledItem",
                &[
                    ("Item", reference(4)),
                    ("Styles", Value::List(vec![reference(8)])),
                ],
            ),
            entity("IfcCurveStyle", &[("Name", text("thin"))]),
            entity(
                "IfcGeometricRepresentationContext",
                &[
                    ("ContextType", text("Model")),
                    ("CoordinateSpaceDimension", Value::Integer(3)),
                    ("WorldCoordinateSystem", reference(10)),
                ],
            ),
            entity("IfcAxis2Placement3D", &[("Location", reference(11))]),
            entity(
                "IfcCartesianPoint",
                &[(
                    "Coordinates",
                    Value::List(vec![real(0.0), real(0.0), real(0.0)]),
                )],
            ),
            // The subtype redeclares CoordinateSpaceDimension DERIVE: the
            // XSD still admits the attribute, and the slot reads as `*`.
            entity(
                "IfcGeometricRepresentationSubContext",
                &[
                    ("ContextIdentifier", text("Body")),
                    ("ContextType", text("Model")),
                    ("ParentContext", reference(9)),
                    ("TargetView", Value::Enum("MODEL_VIEW".into())),
                ],
            ),
        ],
    );
}

/// A flattened nested aggregate nests by `arraySize` when its inner sizes
/// are not fixed, and is refused without one.
#[test]
fn a_nested_aggregate_needs_fixed_inner_sizes_or_an_array_size() {
    let surface = |array_size: &str| {
        format!(
            r#"<IfcBSplineSurfaceWithKnots UDegree="1" VDegree="1" SurfaceForm="plane_surf" UClosed="false" VClosed="false" SelfIntersect="false" UMultiplicities="2 2" VMultiplicities="2 2" UKnots="0 1" VKnots="0 1" KnotSpec="unspecified">
  <ControlPointsList{array_size}>
    <IfcCartesianPoint Coordinates="0 0 0"/><IfcCartesianPoint Coordinates="0 1 0"/>
    <IfcCartesianPoint Coordinates="1 0 0"/><IfcCartesianPoint Coordinates="1 1 0"/>
  </ControlPointsList>
</IfcBSplineSurfaceWithKnots>"#
        )
    };
    let error = refused(&surface(""));
    assert!(
        matches!(error.root_cause(), XmlError::Unsupported { construct } if construct.contains("arraySize")),
        "{error}"
    );
    assert!(
        error
            .path()
            .unwrap()
            .as_str()
            .ends_with("/ControlPointsList"),
        "{error}"
    );

    let model = read(&surface(r#" arraySize="2 2""#)).unwrap();
    let surface = model.get(EntityId(1)).unwrap();
    let names = schema().attribute_names("IfcBSplineSurfaceWithKnots");
    let slot = names
        .iter()
        .position(|name| *name == "ControlPointsList")
        .unwrap();
    assert_eq!(
        surface.attributes[slot],
        Value::List(vec![
            Value::List(vec![reference(2), reference(3)]),
            Value::List(vec![reference(4), reference(5)]),
        ])
    );
    // An arraySize that does not describe the items is refused too.
    assert!(read(
        r#"<IfcBSplineSurfaceWithKnots><ControlPointsList arraySize="3 2"><IfcCartesianPoint Coordinates="0 0 0"/></ControlPointsList></IfcBSplineSurfaceWithKnots>"#
    )
    .is_err());
}

// ------------------------------------------------------------- refusals

#[test]
fn an_unknown_xml_attribute_is_refused_naming_entity_element_and_attribute() {
    let error = refused(&format!(
        r#"<IfcProject GlobalId="{GUID_A}"><OwnerHistory Colour="red"/></IfcProject>"#
    ));
    assert!(
        matches!(
            error.root_cause(),
            XmlError::UnknownAttribute { entity, element, attribute }
                if entity == "IfcOwnerHistory" && element == "OwnerHistory" && attribute == "Colour"
        ),
        "{error:?}"
    );
}

#[test]
fn an_unknown_child_element_is_refused_naming_entity_element_and_attribute() {
    let error = refused(&format!(
        r#"<IfcProject GlobalId="{GUID_A}"><Colour/></IfcProject>"#
    ));
    assert!(
        matches!(
            error.root_cause(),
            XmlError::UnknownAttribute { entity, element, attribute }
                if entity == "IfcProject" && element == "IfcProject" && attribute == "Colour"
        ),
        "{error:?}"
    );
    assert_eq!(
        error.path().unwrap().as_str(),
        "/ifcXML/IfcProject[#1]/Colour"
    );
}

#[test]
fn values_the_declaration_does_not_admit_are_refused() {
    for (body, why) in [
        (
            r#"<IfcCartesianPoint Coordinates="0,5 1"/>"#,
            "decimal comma",
        ),
        (
            r#"<IfcCartesianPoint Coordinates="INF 1"/>"#,
            "non-finite real",
        ),
        (
            r#"<IfcWallType PredefinedType="door"/>"#,
            "foreign enumeration member",
        ),
        (
            r#"<IfcBlobTexture RepeatS="unknown"/>"#,
            "unknown in a BOOLEAN",
        ),
        (r#"<IfcWall GlobalId="short"/>"#, "STRING(22) FIXED width"),
        (
            r#"<IfcOwnerHistory CreationDate="soon"/>"#,
            "text in an INTEGER",
        ),
    ] {
        let error = refused(body);
        assert!(
            matches!(error.root_cause(), XmlError::InvalidScalar { .. }),
            "{why}: {error:?}"
        );
    }
    let error = refused(r#"<IfcCartesianPointList3D CoordList="0 0 0 1"/>"#);
    assert!(
        matches!(error.root_cause(), XmlError::TypeMismatch { .. }),
        "a list that is not a multiple of 3: {error:?}"
    );
}

#[test]
fn constructs_the_reader_cannot_read_exactly_are_refused() {
    for (body, why) in [
        // An entity reference can only be an element.
        (
            r#"<IfcProject OwnerHistory="i1"/>"#,
            "reference as XML attribute",
        ),
        // A simple value can only be an XML attribute.
        (
            r#"<IfcProject><Name>x</Name></IfcProject>"#,
            "simple value as element",
        ),
        (
            r#"<IfcProject><OwnerHistory ref="missing" xsi:nil="true"/></IfcProject>"#,
            "undefined id",
        ),
        (r#"<IfcPerson id="a"/><IfcPerson id="a"/>"#, "duplicate id"),
        (
            r#"<IfcProject><OwnerHistory href="other.ifcxml#i1" xsi:nil="true"/></IfcProject>"#,
            "external href",
        ),
        (r#"<IfcPerson pos="1"/>"#, "positional addressing"),
        (r#"<IfcRoot/>"#, "abstract entity"),
        (r#"<IfcFooBar/>"#, "unknown entity"),
        (
            r#"<IfcProject><OwnerHistory xsi:type="IfcPerson"/></IfcProject>"#,
            "xsi:type not a subtype",
        ),
        (
            r#"<IfcPropertySingleValue><NominalValue><IfcWallTypeEnum-wrapper>notdefined</IfcWallTypeEnum-wrapper></NominalValue></IfcPropertySingleValue>"#,
            "wrapper outside the SELECT",
        ),
        (
            r#"<IfcPropertySingleValue><NominalValue><IfcLabel-wrapper>a</IfcLabel-wrapper><IfcLabel-wrapper>b</IfcLabel-wrapper></NominalValue></IfcPropertySingleValue>"#,
            "two values in one SELECT",
        ),
        (
            r#"<IfcPropertySingleValue><NominalValue><IfcLabel-wrapper id="v">a</IfcLabel-wrapper></NominalValue></IfcPropertySingleValue>"#,
            "id on a wrapper",
        ),
        (
            r#"<IfcBlobTexture><RasterCode extraBits="4">FF</RasterCode></IfcBlobTexture>"#,
            "partial-byte binary",
        ),
        (
            r#"<IfcProject><OwnerHistory id="oh"/></IfcProject><IfcRelAggregates><RelatingObject ref="oh" xsi:nil="true"/></IfcRelAggregates>"#,
            "reference to an entity of the wrong type",
        ),
        (
            r#"<IfcGeometricRepresentationSubContext><WorldCoordinateSystem/></IfcGeometricRepresentationSubContext>"#,
            "derived attribute as element",
        ),
        (
            r#"<IfcPerson>stray text</IfcPerson>"#,
            "text inside an entity",
        ),
    ] {
        let error = refused(body);
        assert!(
            !matches!(error.root_cause(), XmlError::Malformed(_)),
            "{why}: refused as malformed XML rather than for its content: {error:?}"
        );
    }
}

/// An inverse implying a value its relationship contradicts, or a position
/// in an ordered aggregate, is refused.
#[test]
fn an_inverse_that_contradicts_or_cannot_place_its_value_is_refused() {
    let conflict = refused(&format!(
        r#"<IfcProject id="p" GlobalId="{GUID_A}"/>
<IfcSite id="s" GlobalId="{GUID_B}">
  <IsDecomposedBy>
    <IfcRelAggregates GlobalId="{GUID_C}"><RelatingObject ref="p" xsi:nil="true"/></IfcRelAggregates>
  </IsDecomposedBy>
</IfcSite>"#
    ));
    assert!(
        matches!(conflict.root_cause(), XmlError::InverseConflict { .. }),
        "{conflict:?}"
    );

    // `Nests` inverts IfcRelNests.RelatedObjects, a LIST: where this task
    // goes in it is unknown.
    let ordered = refused(&format!(
        r#"<IfcTask id="other" GlobalId="{GUID_A}" IsMilestone="false"/>
<IfcTask GlobalId="{GUID_B}" IsMilestone="false">
  <Nests>
    <IfcRelNests GlobalId="{GUID_C}"><RelatedObjects><IfcTask ref="other" xsi:nil="true"/></RelatedObjects></IfcRelNests>
  </Nests>
</IfcTask>"#
    ));
    assert!(
        matches!(ordered.root_cause(), XmlError::InverseConflict { .. }),
        "{ordered:?}"
    );
}

#[test]
fn the_namespace_and_schema_must_match_the_profile() {
    let foreign = br#"<ifcXML xmlns="http://www.buildingsmart-tech.org/ifcXML/IFC4/final"><IfcPerson/></ifcXML>"#;
    assert!(matches!(
        ifc_xml::reader::read(&codec(), foreign)
            .unwrap_err()
            .root_cause(),
        XmlError::Namespace { .. }
    ));
    // The namespace the release's own Annex E examples declare.
    let legacy = br#"<ifcXML xmlns="http://www.buildingsmart-tech.org/ifcXML/IFC4/Add2"><IfcPerson/></ifcXML>"#;
    assert_eq!(ifc_xml::reader::read(&codec(), legacy).unwrap().len(), 1);

    let mismatched = XmlCodec::xsd(
        Arc::new(ifc_schema::ifc4x3().clone()),
        XmlProfile::Ifc4Add2Tc1,
    );
    assert!(matches!(
        ifc_xml::reader::read(&mismatched, document("").as_bytes()).unwrap_err(),
        XmlError::SchemaMismatch { .. }
    ));
}

/// The layout is read and written; an empty model writes the bare root,
/// once its header declares the profile's schema.
#[test]
fn the_xsd_layout_writes_too() {
    use ifc_model::Codec;
    let codec = codec();
    assert_eq!(codec.layout(), XmlLayout::Xsd);
    let error = codec.write_bytes(&Model::new()).unwrap_err();
    assert!(matches!(error, ModelError::Write(_)), "{error}");
    let mut model = Model::new();
    model.header_mut().schema = vec!["IFC4".into()];
    let bytes = codec.write_bytes(&model).unwrap();
    assert_eq!(codec.read_bytes(&bytes).unwrap().len(), 0);
}

#[test]
fn ifc4x3_documents_read_with_their_own_profile() {
    let codec = XmlCodec::xsd(
        Arc::new(ifc_schema::ifc4x3().clone()),
        XmlProfile::Ifc4x3Add2,
    );
    let xml = format!(
        r#"<ifcXML xmlns="{}"><IfcSite GlobalId="{GUID_A}" RefLatitude="52 31 0"/></ifcXML>"#,
        XmlProfile::Ifc4x3Add2.namespace()
    );
    let model = ifc_xml::reader::read(&codec, xml.as_bytes()).unwrap();
    assert_eq!(&*model.get(EntityId(1)).unwrap().type_name, "IFCSITE");
    assert_eq!(model.header().schema, vec!["IFC4X3_ADD2".to_string()]);
}
