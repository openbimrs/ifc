//! Top-level and nested member lists that name no member, or one member
//! twice, are reported rather than silently skipped (#137).
//!
//! `IfcPropertySet.HasProperties`, `IfcComplexProperty.HasProperties`,
//! `IfcElementQuantity.Quantities` and `IfcPhysicalComplexQuantity.
//! HasQuantities` are all `SET [1:?]` of entity references (IFC4 ADD2 TC1).
//! Each fixture below breaks one of them in one way, parsed from STEP text
//! so the malformed item is exactly what an exporter would write.

use ifc_model::{Codec, EntityId, Model};
use ifc_properties::{
    property_checked, property_set_checked, property_sets_by_object, quantity_set,
    resolved_properties, PropertyAnomaly, PropertyValue, Quantity,
};
use ifc_step::StepCodec;

/// Parse IFC4 `records`, requiring a clean parse.
fn ifc4(records: &[&str]) -> Model {
    let text = format!(
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
         FILE_NAME('','',(''),(''),'','','');\nFILE_SCHEMA(('IFC4'));\nENDSEC;\n\
         DATA;\n{}\nENDSEC;\nEND-ISO-10303-21;\n",
        records.join("\n")
    );
    let model = StepCodec
        .read_bytes(text.as_bytes())
        .unwrap_or_else(|e| panic!("fixture must parse: {e:?}"));
    assert!(model.diagnostics().is_empty(), "{:?}", model.diagnostics());
    model
}

const WALL: &str = "#1=IFCWALL('1xS3BCk291UvhgP2dvNsgp',$,'Wall',$,$,$,$,$,$);";
const P10: &str = "#10=IFCPROPERTYSINGLEVALUE('A',$,IFCLABEL('a'),$);";
const P11: &str = "#11=IFCPROPERTYSINGLEVALUE('B',$,IFCLABEL('b'),$);";
const Q10: &str = "#10=IFCQUANTITYLENGTH('Length',$,$,2.5,$);";
const Q11: &str = "#11=IFCQUANTITYAREA('Area',$,$,4.,$);";

fn not_reference(container: u64, attribute: &'static str, found: &str) -> PropertyAnomaly {
    PropertyAnomaly::MemberNotReference {
        container: EntityId(container),
        attribute,
        found: found.to_owned(),
    }
}

fn duplicate(container: u64, attribute: &'static str, member: u64) -> PropertyAnomaly {
    PropertyAnomaly::DuplicateMember {
        container: EntityId(container),
        attribute,
        member: EntityId(member),
    }
}

fn property_ids(model: &Model, set: u64) -> (Vec<EntityId>, Vec<PropertyAnomaly>) {
    let (set, anomalies) = property_set_checked(model, EntityId(set)).expect("a property set");
    (set.properties.iter().map(|p| p.id).collect(), anomalies)
}

#[test]
fn a_property_set_reports_a_member_that_is_not_a_reference() {
    let m = ifc4(&[
        P10,
        P11,
        "#20=IFCPROPERTYSET('0000000000000000000020',$,'Pset_X',$,(#10,'B',#11));",
    ]);
    let (ids, anomalies) = property_ids(&m, 20);
    assert_eq!(ids, [EntityId(10), EntityId(11)]);
    assert_eq!(
        anomalies,
        [not_reference(20, "HasProperties", r#"Text("B")"#)]
    );
}

#[test]
fn a_property_set_reports_a_member_listed_twice_and_reads_it_once() {
    let m = ifc4(&[
        P10,
        P11,
        "#20=IFCPROPERTYSET('0000000000000000000020',$,'Pset_X',$,(#10,#11,#10));",
    ]);
    let (ids, anomalies) = property_ids(&m, 20);
    assert_eq!(ids, [EntityId(10), EntityId(11)]);
    assert_eq!(anomalies, [duplicate(20, "HasProperties", 10)]);
}

#[test]
fn set_level_readers_carry_the_member_anomalies_once_per_set() {
    let m = ifc4(&[
        WALL,
        P10,
        "#20=IFCPROPERTYSET('0000000000000000000020',$,'Pset_X',$,(#10,#10,42));",
        "#30=IFCRELDEFINESBYPROPERTIES('0000000000000000000030',$,$,$,(#1),#20);",
    ]);
    let expected = [
        duplicate(20, "HasProperties", 10),
        not_reference(20, "HasProperties", "Integer(42)"),
    ];
    let (by_object, anomalies) = property_sets_by_object(&m);
    assert_eq!(anomalies, expected);
    // Read once, so the repeat is not also a duplicate property name.
    let sets = &by_object[&EntityId(1)];
    assert_eq!(sets[0].1.properties.len(), 1);
    let (_, anomalies) = resolved_properties(&m);
    assert_eq!(anomalies, expected);
}

#[test]
fn a_complex_property_reports_both_member_faults() {
    let m = ifc4(&[
        P10,
        P11,
        "#20=IFCCOMPLEXPROPERTY('C',$,'usage',(#10,$,#11,#11));",
    ]);
    let (property, anomalies) = property_checked(&m, EntityId(20)).expect("a property");
    let PropertyValue::Complex { properties, .. } = property.value else {
        panic!("a complex property");
    };
    let ids: Vec<_> = properties.iter().map(|p| p.id).collect();
    assert_eq!(ids, [EntityId(10), EntityId(11)]);
    assert_eq!(
        anomalies,
        [
            not_reference(20, "HasProperties", "Null"),
            duplicate(20, "HasProperties", 11),
        ]
    );
}

#[test]
fn an_element_quantity_reports_a_member_that_is_not_a_reference() {
    let m = ifc4(&[
        Q10,
        Q11,
        "#20=IFCELEMENTQUANTITY('0000000000000000000020',$,'Qto_X',$,$,(#10,2.5,#11));",
    ]);
    let (set, anomalies) = quantity_set(&m, EntityId(20)).expect("a quantity set");
    let ids: Vec<_> = set.quantities.iter().map(Quantity::id).collect();
    assert_eq!(ids, [EntityId(10), EntityId(11)]);
    assert_eq!(anomalies, [not_reference(20, "Quantities", "Real(2.5)")]);
}

#[test]
fn an_element_quantity_reports_a_member_listed_twice_and_reads_it_once() {
    let m = ifc4(&[
        Q10,
        Q11,
        "#20=IFCELEMENTQUANTITY('0000000000000000000020',$,'Qto_X',$,$,(#11,#10,#11));",
    ]);
    let (set, anomalies) = quantity_set(&m, EntityId(20)).expect("a quantity set");
    let ids: Vec<_> = set.quantities.iter().map(Quantity::id).collect();
    assert_eq!(ids, [EntityId(11), EntityId(10)]);
    assert_eq!(anomalies, [duplicate(20, "Quantities", 11)]);
}

#[test]
fn a_complex_quantity_reports_both_member_faults() {
    let m = ifc4(&[
        Q10,
        Q11,
        "#20=IFCPHYSICALCOMPLEXQUANTITY('Layer',$,(#10,#10,'x',#11),'layer',$,$);",
        "#21=IFCELEMENTQUANTITY('0000000000000000000021',$,'Qto_X',$,$,(#20));",
    ]);
    let (set, anomalies) = quantity_set(&m, EntityId(21)).expect("a quantity set");
    let Quantity::Complex { quantities, .. } = &set.quantities[0] else {
        panic!("a complex quantity");
    };
    let ids: Vec<_> = quantities.iter().map(Quantity::id).collect();
    assert_eq!(ids, [EntityId(10), EntityId(11)]);
    assert_eq!(
        anomalies,
        [
            duplicate(20, "HasQuantities", 10),
            not_reference(20, "HasQuantities", r#"Text("x")"#),
        ]
    );
}

#[test]
fn a_well_formed_member_list_reports_nothing() {
    let m = ifc4(&[
        P10,
        "#20=IFCPROPERTYSET('0000000000000000000020',$,'Pset_X',$,(#10));",
    ]);
    let (ids, anomalies) = property_ids(&m, 20);
    assert_eq!(ids, [EntityId(10)]);
    assert!(anomalies.is_empty(), "{anomalies:?}");
}
