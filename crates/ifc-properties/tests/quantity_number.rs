//! IFC4X3 `IfcQuantityNumber` in the permissive quantity view (#138).
//!
//! IFC4X3 ADD2 declares `IfcQuantityNumber SUBTYPE OF
//! (IfcPhysicalSimpleQuantity)` with `NumberValue : IfcNumericMeasure` and
//! `Formula : OPTIONAL IfcLabel`, and no WHERE rule: no unit-kind rule and
//! no `>= 0`. IFC2X3 TC1 and IFC4 ADD2 TC1 do not declare it.

use ifc_model::{Codec, EntityId, Model};
use ifc_properties::{
    compare, exact_property, quantity_set, stated_unit, Comparison, ComputedQuantity,
    ExactPropertyError, PropertyAnomaly, Quantity, QuantityKind, Tolerance, UnitKind,
    UnresolvedValue,
};
use ifc_step::StepCodec;

/// Parse `records` under `FILE_SCHEMA((schema))`. A release that does not
/// declare an entity may leave a diagnostic; the permissive view reads on.
fn step(schema: &str, records: &[&str]) -> Model {
    let text = format!(
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
         FILE_NAME('','',(''),(''),'','','');\nFILE_SCHEMA(('{schema}'));\nENDSEC;\n\
         DATA;\n{}\nENDSEC;\nEND-ISO-10303-21;\n",
        records.join("\n")
    );
    StepCodec
        .read_bytes(text.as_bytes())
        .unwrap_or_else(|e| panic!("fixture must parse: {e:?}"))
}

/// Present, `$`, text and negative numbers, in one set.
const NUMBERS: [&str; 7] = [
    "#1=IFCWALL('1xS3BCk291UvhgP2dvNsgp',$,'Wall',$,$,$,$,$,$);",
    "#10=IFCQUANTITYNUMBER('Openings',$,$,3.5,'n = doors + windows');",
    "#11=IFCQUANTITYNUMBER('Missing',$,$,$,$);",
    "#12=IFCQUANTITYNUMBER('Text',$,$,'three',$);",
    "#13=IFCQUANTITYNUMBER('Balance',$,$,-2.,$);",
    "#20=IFCELEMENTQUANTITY('0YvctVUKr0kugbFTf53O08',$,'Qto_Numbers',$,$,(#10,#11,#12,#13));",
    "#21=IFCRELDEFINESBYPROPERTIES('0YvctVUKr0kugbFTf53O09',$,$,$,(#1),#20);",
];

#[test]
fn the_ifc4x3_table_places_number_value_after_unit() {
    let names = ifc_schema::ifc4x3().attribute_names("IFCQUANTITYNUMBER");
    assert_eq!(
        names,
        ["Name", "Description", "Unit", "NumberValue", "Formula"]
    );
    assert!(ifc_schema::ifc4().entity("IFCQUANTITYNUMBER").is_none());
    assert!(ifc_schema::ifc2x3().entity("IFCQUANTITYNUMBER").is_none());
}

#[test]
fn an_ifc4x3_number_is_read_with_its_value_and_formula() {
    let model = step("IFC4X3_ADD2", &NUMBERS);
    let (set, anomalies) = quantity_set(&model, EntityId(20)).expect("readable");
    let ids: Vec<u64> = set.quantities.iter().map(|q| q.id().0).collect();
    assert_eq!(ids, [10, 11, 12, 13], "none dropped, file order kept");
    match &set.quantities[0] {
        Quantity::Simple {
            kind: QuantityKind::Number,
            value,
            formula,
            unit: None,
            ..
        } => {
            assert_eq!(*value, 3.5);
            assert_eq!(formula.as_deref(), Some("n = doors + windows"));
        }
        other => panic!("expected a simple number, got {other:?}"),
    }
    assert!(matches!(
        &set.quantities[1],
        Quantity::Unresolved {
            kind: QuantityKind::Number,
            reason: UnresolvedValue::Missing,
            ..
        }
    ));
    match &set.quantities[2] {
        Quantity::Unresolved {
            kind: QuantityKind::Number,
            reason: UnresolvedValue::NotNumeric { found },
            ..
        } => assert!(found.contains("three"), "{found}"),
        other => panic!("expected an unresolved number, got {other:?}"),
    }
    // No `>= 0` rule: a negative number is a number, not an anomaly.
    assert!(matches!(
        &set.quantities[3],
        Quantity::Simple { kind: QuantityKind::Number, value, .. } if *value == -2.0
    ));
    match anomalies.as_slice() {
        [PropertyAnomaly::QuantityValueMissing {
            quantity: EntityId(11),
        }, PropertyAnomaly::QuantityValueNotNumeric {
            quantity: EntityId(12),
            ..
        }] => {}
        other => panic!("unexpected anomalies {other:?}"),
    }
}

/// IFC2X3 and IFC4 do not declare `IfcQuantityNumber`, and a model without
/// one declared release cannot place it: it stays `Unsupported`.
#[test]
fn a_number_outside_ifc4x3_is_not_read_as_one() {
    let foreign = [step("IFC4", &NUMBERS), step("IFC2X3", &NUMBERS)];
    let mut undeclared = Model::new();
    for (id, entity) in step("IFC4X3_ADD2", &NUMBERS).iter() {
        undeclared.insert(id, entity.clone());
    }
    for model in foreign.iter().chain([&undeclared]) {
        let (set, anomalies) = quantity_set(model, EntityId(20)).expect("readable");
        assert_eq!(set.quantities.len(), 4);
        for quantity in &set.quantities {
            assert!(
                matches!(quantity, Quantity::Unsupported { type_name, .. }
                    if &**type_name == "IFCQUANTITYNUMBER"),
                "{:?}: {quantity:?}",
                model.header().schema
            );
        }
        assert!(anomalies.is_empty(), "{anomalies:?}");
    }
}

#[test]
fn a_number_compares_only_with_a_computed_number() {
    let model = step("IFC4X3_ADD2", &NUMBERS);
    let (set, _) = quantity_set(&model, EntityId(20)).expect("readable");
    let computed = |kind, value| ComputedQuantity {
        kind,
        value,
        unit: String::new(),
    };
    let openings = &set.quantities[0];
    let check = |c: &ComputedQuantity| compare(&model, openings, c, Tolerance::default());
    assert!(matches!(
        check(&computed(QuantityKind::Number, 3.5)),
        Comparison::Agrees { .. }
    ));
    assert!(matches!(
        check(&computed(QuantityKind::Number, 4.0)),
        Comparison::Disagrees { .. }
    ));
    assert_eq!(
        check(&computed(QuantityKind::Count, 3.5)),
        Comparison::KindMismatch {
            authored: QuantityKind::Number,
            computed: QuantityKind::Count,
        }
    );
    let missing = &set.quantities[1];
    assert_eq!(
        compare(
            &model,
            missing,
            &computed(QuantityKind::Number, 0.0),
            Tolerance::default()
        ),
        Comparison::NotComparable
    );
    assert_eq!(stated_unit(&model, openings), None, "it states no unit");
}

/// `IfcQuantityNumber` has no unit-kind rule: any stated `IfcNamedUnit` is
/// its unit, and nothing is reported against it.
#[test]
fn a_number_keeps_any_stated_unit_without_a_unit_rule() {
    let model = step(
        "IFC4X3_ADD2",
        &[
            "#10=IFCQUANTITYNUMBER('Span',$,#11,4.,$);",
            "#11=IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.);",
            "#20=IFCELEMENTQUANTITY('0YvctVUKr0kugbFTf53O08',$,'Q',$,$,(#10));",
        ],
    );
    let (set, anomalies) = quantity_set(&model, EntityId(20)).expect("readable");
    assert!(matches!(
        stated_unit(&model, &set.quantities[0]),
        Some(UnitKind::Si { .. })
    ));
    assert!(anomalies.is_empty(), "{anomalies:?}");
}

/// The exact API agrees: a `$` number is `MissingValueSlot`.
#[test]
fn the_exact_api_refuses_a_missing_number() {
    let model = step(
        "IFC4X3_ADD2",
        &[
            NUMBERS[0],
            NUMBERS[2],
            &NUMBERS[5].replace("#10,#11,#12,#13", "#11"),
            NUMBERS[6],
        ],
    );
    assert_eq!(
        exact_property(&model, EntityId(1), Some("Qto_Numbers"), "Missing"),
        Err(ExactPropertyError::MissingValueSlot {
            property: EntityId(11)
        })
    );
}
