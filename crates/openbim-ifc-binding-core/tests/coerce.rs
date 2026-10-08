//! Plain host values written by name (#342), coerced against the declared
//! type in IFC2X3, IFC4 and IFC4X3, with every refusal's stable code.
//!
//! The declarations come from the normative schemas
//! (`references/ifc-spec/`): `IfcRoot.Name` is `OPTIONAL IfcLabel`
//! (`STRING`) in all three; `IfcWall.PredefinedType` is `OPTIONAL
//! IfcWallTypeEnum` in IFC4 and IFC4X3, and only `IfcWallType` has it in
//! IFC2X3; `IfcTask.Priority` is `OPTIONAL INTEGER` in IFC2X3 and `OPTIONAL
//! IfcInteger` later, `IsMilestone` `BOOLEAN` / `IfcBoolean`;
//! `IfcCurveStyle.CurveWidth` is `OPTIONAL IfcSizeSelect`, a SELECT of
//! `IfcDescriptiveMeasure` (the one `STRING` member) and five `REAL`
//! measures; `IfcCartesianPoint.Coordinates` is `LIST [1:3] OF
//! IfcLengthMeasure`; `IfcPropertySingleValue.NominalValue` is `OPTIONAL
//! IfcValue`; `IfcRelDefinesByProperties.RelatingPropertyDefinition` is
//! `IfcPropertySetDefinition` in IFC2X3 and the SELECT
//! `IfcPropertySetDefinitionSelect` later.
#![cfg(all(feature = "ifc2x3", feature = "ifc4", feature = "ifc4x3"))]

use openbim_ifc_binding_core::value::Tagged;
use openbim_ifc_binding_core::{BindingError, IfcModel, Plain};

fn file(schema: &str, data: &str) -> IfcModel {
    let text = format!(
        "ISO-10303-21;
HEADER;
FILE_DESCRIPTION(('ViewDefinition [CoordinationView]'),'2;1');
FILE_NAME('t.ifc','2026-10-08T00:00:00',('a'),('o'),'p','s','');
FILE_SCHEMA(('{schema}'));
ENDSEC;
DATA;
{data}
ENDSEC;
END-ISO-10303-21;
"
    );
    IfcModel::parse(text.as_bytes()).expect("fixture parses")
}

fn code<T: std::fmt::Debug>(result: Result<T, BindingError>) -> &'static str {
    result.expect_err("refused").code()
}

fn text(value: &str) -> Plain {
    Plain::Text(value.into())
}

fn step(model: &IfcModel) -> String {
    String::from_utf8(model.write().unwrap()).unwrap()
}

const IFC2X3: &str = "#1=IFCWALL('0YvctVUKr0kugbFTf53O9L',$,'W',$,$,#5,$,$);
#2=IFCWALLTYPE('1YvctVUKr0kugbFTf53O9L',$,'T',$,$,$,$,$,$,.NOTDEFINED.);
#3=IFCTASK('2YvctVUKr0kugbFTf53O9L',$,'Pour',$,$,'T1','Planned','Crane',.F.,1);
#4=IFCCURVESTYLE('c',$,$,$);
#5=IFCLOCALPLACEMENT($,#6);
#6=IFCAXIS2PLACEMENT3D(#7,$,$);
#7=IFCCARTESIANPOINT((0.,0.,0.));
#10=IFCPROPERTYSINGLEVALUE('P',$,$,$);
#11=IFCPROPERTYSET('3YvctVUKr0kugbFTf53O9L',$,'Pset',$,(#10));
#12=IFCRELDEFINESBYPROPERTIES('4YvctVUKr0kugbFTf53O9L',$,$,$,(#1),#11);";

const IFC4: &str = "#1=IFCWALL('0YvctVUKr0kugbFTf53O9L',$,'W',$,$,#5,$,$,.NOTDEFINED.);
#2=IFCWALLTYPE('1YvctVUKr0kugbFTf53O9L',$,'T',$,$,$,$,$,$,.NOTDEFINED.);
#3=IFCTASK('2YvctVUKr0kugbFTf53O9L',$,'Pour',$,$,'T1',$,'Planned','Crane',.F.,1,$,.CONSTRUCTION.);
#4=IFCCURVESTYLE('c',$,$,$,$);
#5=IFCLOCALPLACEMENT($,#6);
#6=IFCAXIS2PLACEMENT3D(#7,$,$);
#7=IFCCARTESIANPOINT((0.,0.,0.));
#8=IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.);
#10=IFCPROPERTYSINGLEVALUE('P',$,$,$);
#11=IFCPROPERTYSET('3YvctVUKr0kugbFTf53O9L',$,'Pset',$,(#10));
#12=IFCRELDEFINESBYPROPERTIES('4YvctVUKr0kugbFTf53O9L',$,$,$,(#1),#11);";

const RELEASES: [(&str, &str); 3] = [("IFC2X3", IFC2X3), ("IFC4", IFC4), ("IFC4X3_ADD2", IFC4)];

#[test]
fn a_string_writes_a_label_bare_in_every_release() {
    for (schema, data) in RELEASES {
        let mut model = file(schema, data);
        let previous = model
            .set_attribute_by_name_plain(1, "name", text("x"))
            .unwrap();
        assert_eq!(previous, Tagged::Text("W".into()), "{schema}");
        // Not a SELECT, so bare (ISO 10303-21 §12.1.6): 'x', not IFCLABEL('x').
        assert_eq!(
            model.attribute_by_name(1, "Name").unwrap(),
            Tagged::Text("x".into())
        );
        assert!(
            step(&model).contains("#1=IFCWALL('0YvctVUKr0kugbFTf53O9L',$,'x',"),
            "{schema}"
        );
    }
}

#[test]
fn a_string_names_an_enumeration_item_in_any_case() {
    for (schema, data, entity) in [
        ("IFC2X3", IFC2X3, 2),
        ("IFC4", IFC4, 1),
        ("IFC4X3_ADD2", IFC4, 1),
    ] {
        let mut model = file(schema, data);
        model
            .set_attribute_by_name_plain(entity, "PredefinedType", text("standard"))
            .unwrap();
        assert_eq!(
            model.attribute_by_name(entity, "PredefinedType").unwrap(),
            Tagged::Enum("STANDARD".into()),
            "{schema}"
        );
        let refused = model.set_attribute_by_name_plain(entity, "PredefinedType", text("CURVED"));
        let BindingError::TypeMismatch(detail) = refused.unwrap_err() else {
            panic!("type-mismatch");
        };
        assert!(
            detail.contains("IfcWallTypeEnum") && detail.contains("STANDARD"),
            "{detail}"
        );
    }
    // IFC2X3's IfcWall has no PredefinedType at all.
    let mut model = file("IFC2X3", IFC2X3);
    assert_eq!(
        code(model.set_attribute_by_name_plain(1, "PredefinedType", text("STANDARD"))),
        "unknown-attribute"
    );
}

#[test]
fn numbers_and_booleans_follow_the_declaration() {
    for (schema, data) in RELEASES {
        let mut model = file(schema, data);
        model
            .set_attribute_by_name_plain(3, "Priority", Plain::Integer(7))
            .unwrap();
        assert_eq!(
            model.attribute_by_name(3, "Priority").unwrap(),
            Tagged::Integer(7)
        );
        model
            .set_attribute_by_name_plain(3, "IsMilestone", Plain::Bool(true))
            .unwrap();
        assert_eq!(
            model.attribute_by_name(3, "IsMilestone").unwrap(),
            Tagged::Bool(true)
        );
        // A float is not an INTEGER; a string is not a BOOLEAN.
        assert_eq!(
            code(model.set_attribute_by_name_plain(3, "Priority", Plain::Real(2.5))),
            "type-mismatch"
        );
        assert_eq!(
            code(model.set_attribute_by_name_plain(3, "IsMilestone", text("yes"))),
            "type-mismatch"
        );
        // An integer is a REAL when the declaration says so, element by element.
        model
            .set_attribute_by_name_plain(
                7,
                "Coordinates",
                Plain::List(vec![Plain::Integer(1), Plain::Real(2.5), Plain::Integer(0)]),
            )
            .unwrap();
        assert_eq!(
            model.attribute_by_name(7, "Coordinates").unwrap(),
            Tagged::List(vec![
                Tagged::Real(1.0),
                Tagged::Real(2.5),
                Tagged::Real(0.0)
            ]),
            "{schema}"
        );
        assert_eq!(
            code(model.set_attribute_by_name_plain(7, "Coordinates", Plain::Real(1.0))),
            "type-mismatch",
            "an aggregate needs a list"
        );
        assert_eq!(
            code(model.set_attribute_by_name_plain(7, "Coordinates", Plain::List(vec![text("a")]))),
            "type-mismatch"
        );
        assert_eq!(
            code(model.set_attribute_by_name_plain(
                7,
                "Coordinates",
                Plain::List(vec![Plain::Integer(i64::MAX)])
            )),
            "type-mismatch",
            "an integer a REAL cannot hold exactly"
        );
        assert_eq!(
            code(model.set_attribute_by_name_plain(
                7,
                "Coordinates",
                Plain::List(vec![Plain::Real(f64::NAN)])
            )),
            "invalid-value"
        );
    }
}

#[test]
fn a_select_takes_its_one_fitting_member_typed_and_refuses_several() {
    for (schema, data) in RELEASES {
        let mut model = file(schema, data);
        model
            .set_attribute_by_name_plain(4, "CurveWidth", text("by layer"))
            .unwrap();
        // A SELECT writes a typed parameter (ISO 10303-21 §12.1.8).
        assert_eq!(
            model.attribute_by_name(4, "CurveWidth").unwrap(),
            Tagged::Typed {
                type_name: "IFCDESCRIPTIVEMEASURE".into(),
                value: Box::new(Tagged::Text("by layer".into())),
            },
            "{schema}"
        );
        assert!(step(&model).contains("IFCDESCRIPTIVEMEASURE('by layer')"));
        let refused = model.set_attribute_by_name_plain(4, "CurveWidth", Plain::Real(2.5));
        let BindingError::AmbiguousValue(detail) = refused.unwrap_err() else {
            panic!("ambiguous-value");
        };
        for member in [
            "IfcLengthMeasure",
            "IfcPositiveLengthMeasure",
            "IfcRatioMeasure",
        ] {
            assert!(detail.contains(member), "{detail} names {member}");
        }
        assert!(!detail.contains("IfcDescriptiveMeasure"), "{detail}");
        // The refusal changed nothing; an explicit wrapper still works.
        assert!(matches!(
            model.attribute_by_name(4, "CurveWidth").unwrap(),
            Tagged::Typed { .. }
        ));
        let exact = Tagged::Typed {
            type_name: "IFCPOSITIVELENGTHMEASURE".into(),
            value: Box::new(Tagged::Real(2.5)),
        };
        model
            .set_attribute_by_name_plain(4, "CurveWidth", Plain::Exact(exact.clone()))
            .unwrap();
        assert_eq!(model.attribute_by_name(4, "CurveWidth").unwrap(), exact);
        // IfcValue: a string is a label, a text, an identifier, ...
        assert_eq!(
            code(model.set_attribute_by_name_plain(10, "NominalValue", text("x"))),
            "ambiguous-value"
        );
        assert_eq!(
            code(model.set_attribute_by_name_plain(4, "CurveWidth", Plain::Bool(true))),
            "type-mismatch"
        );
    }
}

#[test]
fn an_entity_handle_is_a_checked_reference() {
    for (schema, data) in RELEASES {
        let mut model = file(schema, data);
        // An entity-typed slot (IfcObjectPlacement), and a SELECT of
        // entities in IFC4 and IFC4X3 (IfcPropertySetDefinitionSelect).
        model
            .set_attribute_by_name_plain(1, "ObjectPlacement", Plain::Ref(5))
            .unwrap();
        model
            .set_attribute_by_name_plain(12, "RelatingPropertyDefinition", Plain::Ref(11))
            .unwrap();
        assert_eq!(
            model
                .attribute_by_name(12, "RelatingPropertyDefinition")
                .unwrap(),
            Tagged::Ref(11)
        );
        assert_eq!(
            code(model.set_attribute_by_name_plain(1, "ObjectPlacement", Plain::Ref(7))),
            "type-mismatch",
            "{schema}: a point is no placement"
        );
        assert_eq!(
            code(model.set_attribute_by_name_plain(
                12,
                "RelatingPropertyDefinition",
                Plain::Ref(1)
            )),
            "type-mismatch",
            "{schema}: a wall is no property set"
        );
        assert_eq!(
            code(model.set_attribute_by_name_plain(1, "ObjectPlacement", Plain::Ref(99))),
            "missing-reference"
        );
        assert_eq!(
            code(model.set_attribute_by_name_plain(1, "Name", Plain::Ref(5))),
            "type-mismatch"
        );
        // An exact reference is written as given, unchecked, as before.
        model
            .set_attribute_by_name_plain(1, "ObjectPlacement", Plain::Exact(Tagged::Ref(99)))
            .unwrap();
    }
}

#[test]
fn null_unsets_and_other_refusals_keep_their_codes() {
    for (schema, data) in RELEASES {
        let mut model = file(schema, data);
        model
            .set_attribute_by_name_plain(1, "Name", Plain::Null)
            .unwrap();
        assert_eq!(
            model.attribute_by_name(1, "Name").unwrap(),
            Tagged::Null,
            "{schema}"
        );
        assert_eq!(
            code(model.set_attribute_by_name_plain(1, "Nope", text("x"))),
            "unknown-attribute"
        );
        assert_eq!(
            code(model.set_attribute_by_name_plain(99, "Name", text("x"))),
            "missing-entity"
        );
        // A number is no GlobalId (STRING(22) FIXED).
        assert_eq!(
            code(model.set_attribute_by_name_plain(1, "GlobalId", Plain::Integer(1))),
            "type-mismatch"
        );
    }
    let mut model = file("IFC4", IFC4);
    assert_eq!(
        code(model.set_attribute_by_name_plain(8, "Dimensions", Plain::Null)),
        "derived-attribute"
    );
    // coerce_attribute answers without writing.
    let before = step(&model);
    assert_eq!(
        model
            .coerce_attribute(1, "PredefinedType", text("Standard"))
            .unwrap(),
        Tagged::Enum("STANDARD".into())
    );
    assert_eq!(step(&model), before);
}

#[test]
fn an_undeclared_release_or_entity_is_refused() {
    let mut model = file("IFC9", "#1=IFCWALL('0YvctVUKr0kugbFTf53O9L',$,'W');");
    assert_eq!(
        code(model.set_attribute_by_name_plain(1, "Name", text("x"))),
        "unsupported-schema"
    );
    let mut model = file("IFC4", "#1=IFCNOSUCHTHING('x');");
    assert_eq!(
        code(model.set_attribute_by_name_plain(1, "Name", text("x"))),
        "unsupported-schema"
    );
}
