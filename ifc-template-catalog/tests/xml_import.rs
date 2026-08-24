#![cfg(feature = "xml")]

use ifc_template_catalog::definition::{
    PropertyKind, QuantityKind, QuantitySetType, SetTemplateKind,
};
use ifc_template_catalog::xml::{
    parse_template, parse_template_with_limits, ImportLimits, XmlImportError,
};

const PSD: &str = r#"<PropertySetDef ifdguid="set-guid" templatetype="PSET_TYPEDRIVENOVERRIDE"><IfcVersion version="IFC4"/><Name>Pset_Test</Name><Definition>Set definition</Definition><PsetDefinitionAliases><PsetDefinitionAlias lang="de-DE">Satzdefinition</PsetDefinitionAlias></PsetDefinitionAliases><ApplicableClasses><ClassName>IfcWall</ClassName></ApplicableClasses><ApplicableTypeValue>IfcWall, IfcSlab/FLOOR</ApplicableTypeValue><PropertyDefs>
<PropertyDef ifdguid="p1"><Name>Single</Name><PropertyType><TypePropertySingleValue><DataType type="IfcBoolean"/><UnitType type="LENGTHUNIT"/></TypePropertySingleValue></PropertyType></PropertyDef>
<PropertyDef><Name>Bounded</Name><PropertyType><TypePropertyBoundedValue><DataType type="IfcLengthMeasure"/></TypePropertyBoundedValue></PropertyType></PropertyDef>
<PropertyDef><Name>Enum</Name><PropertyType><TypePropertyEnumeratedValue><EnumList name="PEnum_Test"><EnumItem>A</EnumItem><EnumItem>B</EnumItem></EnumList><ConstantList><ConstantDef><Name>A</Name><Definition>First</Definition><NameAliases><NameAlias lang="de-DE">Eins</NameAlias></NameAliases><DefinitionAliases><DefinitionAlias lang="de-DE">Erster Wert</DefinitionAlias></DefinitionAliases></ConstantDef></ConstantList></TypePropertyEnumeratedValue></PropertyType></PropertyDef>
<PropertyDef><Name>List</Name><PropertyType><TypePropertyListValue><ListValue><DataType type="IfcLabel"/></ListValue></TypePropertyListValue></PropertyType></PropertyDef>
<PropertyDef><Name>Reference</Name><PropertyType><TypePropertyReferenceValue reftype="IfcTimeSeries"/></PropertyType></PropertyDef>
<PropertyDef><Name>Table</Name><PropertyType><TypePropertyTableValue><Expression></Expression><DefiningValue><DataType type="IfcLabel"/></DefiningValue><DefinedValue><DataType type="IfcMassMeasure"/></DefinedValue></TypePropertyTableValue></PropertyType></PropertyDef>
<PropertyDef><Name>Complex</Name><PropertyType><TypeComplexProperty name="Usage"><PropertyDefs><PropertyDef><Name>Child</Name><PropertyType><TypePropertySingleValue><DataType type="IfcReal"/></TypePropertySingleValue></PropertyType></PropertyDef></PropertyDefs></TypeComplexProperty></PropertyType></PropertyDef>
</PropertyDefs></PropertySetDef>"#;

const QTO: &str = r#"<QtoSetDef><Name>Qto_Test</Name><Definition>QTO definition</Definition><QtoDefinitionAliases><QtoDefinitionAlias lang="de-DE">Mengendefinition</QtoDefinitionAlias></QtoDefinitionAliases><ApplicableClasses><ClassName>IfcWall</ClassName></ApplicableClasses><ApplicableTypeValue>IfcWall</ApplicableTypeValue><MethodOfMeasurement>ISO fixture</MethodOfMeasurement><QtoDefs><QtoDefinition><Name>Length</Name><QtoType>Q_LENGTH</QtoType></QtoDefinition><QtoDefinition><Name>Count</Name><QtoType>Q_COUNT</QtoType></QtoDefinition></QtoDefs></QtoSetDef>"#;

#[test]
fn parses_every_psd_property_form_and_normalizes_applicability() {
    let set = parse_template(PSD).unwrap();
    assert_eq!(set.name, "Pset_Test");
    assert_eq!(set.applicability.len(), 2);
    assert_eq!(set.definition_aliases[0].text, "Satzdefinition");
    assert_eq!(
        set.applicability[1].predefined_type.as_deref(),
        Some("FLOOR")
    );
    let SetTemplateKind::Property { properties, .. } = set.kind else {
        panic!()
    };
    assert_eq!(properties.len(), 7);
    assert!(matches!(
        properties[0].kind,
        PropertyKind::SingleValue { .. }
    ));
    let PropertyKind::EnumeratedValue {
        values, constants, ..
    } = &properties[2].kind
    else {
        panic!()
    };
    assert_eq!(values, &["A", "B"]);
    assert_eq!(constants[0].name, "A");
    assert_eq!(constants[0].definition.as_deref(), Some("First"));
    assert_eq!(constants[0].name_aliases[0].text, "Eins");
    let PropertyKind::TableValue { expression, .. } = &properties[5].kind else {
        panic!()
    };
    assert_eq!(expression.as_deref(), Some(""));
    assert!(matches!(properties[6].kind, PropertyKind::Complex { .. }));
}

#[test]
fn parses_qto_measurement_types() {
    let set = parse_template(QTO).unwrap();
    let SetTemplateKind::Quantity {
        set_type,
        quantities,
        method_of_measurement,
    } = set.kind
    else {
        panic!()
    };
    assert_eq!(set_type, QuantitySetType::Unspecified);
    assert_eq!(method_of_measurement.as_deref(), Some("ISO fixture"));
    assert_eq!(set.definition_aliases[0].text, "Mengendefinition");
    assert_eq!(quantities[0].kind, QuantityKind::Length);
    assert_eq!(quantities[1].kind, QuantityKind::Count);
}

#[test]
fn parses_every_qto_set_classification_and_rejects_unknown_values() {
    for (attribute, expected) in [
        ("", QuantitySetType::Unspecified),
        (
            " templatetype=\"QTO_TYPEDRIVENOVERRIDE\"",
            QuantitySetType::TypeDrivenOverride,
        ),
        (
            " templatetype=\"QTO_TYPEDRIVENONLY\"",
            QuantitySetType::TypeDrivenOnly,
        ),
        (
            " templatetype=\"QTO_OCCURRENCEDRIVEN\"",
            QuantitySetType::OccurrenceDriven,
        ),
    ] {
        let xml = format!("<QtoSetDef{attribute}><Name>Qto_Test</Name></QtoSetDef>");
        let set = parse_template(&xml).unwrap();
        let SetTemplateKind::Quantity { set_type, .. } = set.kind else {
            panic!("expected quantity set")
        };
        assert_eq!(set_type, expected, "attribute: {attribute}");
    }

    let xml = "<QtoSetDef templatetype=\"QTO_UNKNOWN\"><Name>Qto_Test</Name></QtoSetDef>";
    assert!(matches!(
        parse_template(xml),
        Err(XmlImportError::UnsupportedSetType { value, .. }) if value == "QTO_UNKNOWN"
    ));
}

#[test]
fn rejects_unknown_typed_content() {
    let xml = "<PropertySetDef><Name>Pset_Bad</Name><PropertyDefs><PropertyDef><Name>X</Name><PropertyType><TypeMagic/></PropertyType></PropertyDef></PropertyDefs></PropertySetDef>";
    let error = parse_template(xml).unwrap_err();
    assert!(matches!(
        error,
        XmlImportError::UnsupportedPropertyType { .. }
    ));
}

#[test]
fn preserves_cdata_text() {
    let xml = "<QtoSetDef><Name>Qto_Test</Name><Definition><![CDATA[Area < gross]]></Definition></QtoSetDef>";
    let set = parse_template(xml).unwrap();
    assert_eq!(set.definition.as_deref(), Some("Area < gross"));
}

#[test]
fn importer_limits_untrusted_input_before_building_a_tree() {
    let limits = ImportLimits {
        max_bytes: 8,
        ..ImportLimits::default()
    };
    assert!(matches!(
        parse_template_with_limits(PSD, limits),
        Err(XmlImportError::LimitExceeded { kind: "bytes", .. })
    ));
}

#[test]
fn rejects_document_type_declarations() {
    let xml = "<!DOCTYPE x><PropertySetDef/>";
    assert!(matches!(parse_template(xml), Err(XmlImportError::Xml(_))));
}
