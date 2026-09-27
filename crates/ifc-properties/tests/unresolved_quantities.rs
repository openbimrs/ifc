//! A simple quantity whose value is `$`, cut off or not a number is listed as
//! `Quantity::Unresolved` in its set, in file order, and still reported as an
//! anomaly (#138).
//!
//! `LengthValue`, `AreaValue`, `VolumeValue`, `CountValue`, `WeightValue`
//! and `TimeValue` are not `OPTIONAL` in IFC2X3 TC1, IFC4 ADD2 TC1 or
//! IFC4X3 ADD2; IFC2X3 records end at the value (no `Formula`), so the
//! fixtures below are written per release, parsed from STEP text.

use ifc_model::{Codec, EntityId, Model};
use ifc_properties::{
    compare, exact_property, quantity_set, quantity_sets, stated_unit, Comparison,
    ComputedQuantity, ExactPropertyError, PropertyAnomaly, Quantity, QuantityKind, Tolerance,
    UnitKind, UnresolvedValue,
};
use ifc_step::StepCodec;

/// Parse `records` under `FILE_SCHEMA((schema))`. A cut-off record is a
/// diagnostic of the parse, not a failure, so diagnostics are allowed.
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

/// The three releases, with each quantity's trailing `Formula` slot (none
/// in IFC2X3) and the `IfcWall` arity (8 in IFC2X3, 9 in IFC4 and IFC4X3).
const RELEASES: [(&str, &str, &str); 3] = [
    ("IFC2X3", "", "$,$,$,$,$"),
    ("IFC4", ",$", "$,$,$,$,$,$"),
    ("IFC4X3_ADD2", ",$", "$,$,$,$,$,$"),
];

/// A wall with one quantity set mixing good and bad quantities:
/// #10 valued, #11 `$`, #12 text, #13 cut off before its value, #14 valued.
fn mixed(schema: &str, formula: &str, wall_tail: &str) -> Model {
    let records = [
        format!("#1=IFCWALL('1xS3BCk291UvhgP2dvNsgp',$,'Wall',{wall_tail});"),
        format!("#10=IFCQUANTITYLENGTH('Length',$,$,2.5{formula});"),
        format!("#11=IFCQUANTITYAREA('Area',$,$,${formula});"),
        format!("#12=IFCQUANTITYVOLUME('Volume',$,$,'lots'{formula});"),
        "#13=IFCQUANTITYCOUNT('Count',$,$);".to_owned(),
        format!("#14=IFCQUANTITYWEIGHT('Weight',$,$,7.{formula});"),
        "#20=IFCELEMENTQUANTITY('0YvctVUKr0kugbFTf53O08',$,'Qto_Mixed',$,$,\
         (#10,#11,#12,#13,#14));"
            .to_owned(),
        "#21=IFCRELDEFINESBYPROPERTIES('0YvctVUKr0kugbFTf53O09',$,$,$,(#1),#20);".to_owned(),
    ];
    let records: Vec<&str> = records.iter().map(String::as_str).collect();
    step(schema, &records)
}

fn reason(quantity: &Quantity) -> Option<&UnresolvedValue> {
    match quantity {
        Quantity::Unresolved { reason, .. } => Some(reason),
        _ => None,
    }
}

#[test]
fn every_member_of_a_mixed_set_is_listed_in_file_order() {
    for (schema, formula, wall) in RELEASES {
        let model = mixed(schema, formula, wall);
        let (set, _) = quantity_set(&model, EntityId(20)).expect("readable");
        let ids: Vec<u64> = set.quantities.iter().map(|q| q.id().0).collect();
        assert_eq!(ids, [10, 11, 12, 13, 14], "{schema}: none dropped");
        let names: Vec<_> = set.quantities.iter().map(Quantity::name).collect();
        assert_eq!(
            names,
            [
                Some("Length"),
                Some("Area"),
                Some("Volume"),
                Some("Count"),
                Some("Weight")
            ],
            "{schema}"
        );
    }
}

#[test]
fn a_bad_value_becomes_unresolved_with_its_kind_and_reason() {
    for (schema, formula, wall) in RELEASES {
        let model = mixed(schema, formula, wall);
        let (set, _) = quantity_set(&model, EntityId(20)).expect("readable");
        let q = &set.quantities;
        assert!(
            matches!(q[0], Quantity::Simple { value, kind: QuantityKind::Length, .. } if value == 2.5),
            "{schema}: {:?}",
            q[0]
        );
        assert!(
            matches!(q[4], Quantity::Simple { value, kind: QuantityKind::Weight, .. } if value == 7.0),
            "{schema}: {:?}",
            q[4]
        );
        for (quantity, kind) in [
            (&q[1], QuantityKind::Area),
            (&q[2], QuantityKind::Volume),
            (&q[3], QuantityKind::Count),
        ] {
            assert!(
                matches!(quantity, Quantity::Unresolved { kind: k, .. } if *k == kind),
                "{schema}: {quantity:?}"
            );
        }
        assert_eq!(
            reason(&q[1]),
            Some(&UnresolvedValue::Missing),
            "{schema}: $"
        );
        match reason(&q[2]) {
            Some(UnresolvedValue::NotNumeric { found }) => {
                assert!(found.contains("lots"), "{schema}: {found}");
            }
            other => panic!("{schema}: expected NotNumeric, got {other:?}"),
        }
        assert_eq!(
            reason(&q[3]),
            Some(&UnresolvedValue::Missing),
            "{schema}: cut off"
        );
    }
}

#[test]
fn the_anomalies_are_still_reported_once_each() {
    for (schema, formula, wall) in RELEASES {
        let model = mixed(schema, formula, wall);
        let (sets, anomalies) = quantity_sets(&model);
        assert_eq!(sets.len(), 1, "{schema}");
        match anomalies.as_slice() {
            [PropertyAnomaly::QuantityValueMissing {
                quantity: EntityId(11),
            }, PropertyAnomaly::QuantityValueNotNumeric {
                quantity: EntityId(12),
                found,
            }, PropertyAnomaly::QuantityValueMissing {
                quantity: EntityId(13),
            }] => assert!(found.contains("lots"), "{schema}: {found}"),
            other => panic!("{schema}: unexpected anomalies {other:?}"),
        }
    }
}

#[test]
fn an_unresolved_quantity_is_not_compared_as_zero() {
    for (schema, formula, wall) in RELEASES {
        let model = mixed(schema, formula, wall);
        let (set, _) = quantity_set(&model, EntityId(20)).expect("readable");
        let area = set.quantity("Area").expect("the $ quantity is listed");
        let computed = ComputedQuantity {
            kind: QuantityKind::Area,
            value: 0.0,
            unit: "SQUARE_METRE".into(),
        };
        let result = compare(&model, area, &computed, Tolerance::default());
        assert_eq!(result, Comparison::NotComparable, "{schema}");
    }
}

#[test]
fn an_unresolved_quantity_keeps_its_unit_and_its_unit_rule() {
    for (schema, formula, _) in RELEASES {
        let wrong = format!("#10=IFCQUANTITYAREA('Area',$,#11,${formula});");
        let model = step(
            schema,
            &[
                &wrong,
                "#11=IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.);",
                "#20=IFCELEMENTQUANTITY('0YvctVUKr0kugbFTf53O08',$,'Q',$,$,(#10));",
            ],
        );
        let (set, anomalies) = quantity_set(&model, EntityId(20)).expect("readable");
        let area = &set.quantities[0];
        assert!(
            matches!(
                area,
                Quantity::Unresolved {
                    unit: Some(EntityId(11)),
                    ..
                }
            ),
            "{schema}: {area:?}"
        );
        assert!(
            matches!(stated_unit(&model, area), Some(UnitKind::Si { .. })),
            "{schema}"
        );
        assert_eq!(
            anomalies,
            [
                PropertyAnomaly::QuantityValueMissing {
                    quantity: EntityId(10)
                },
                PropertyAnomaly::QuantityUnitMismatch {
                    quantity: EntityId(10),
                    unit: EntityId(11),
                    expected: "AREAUNIT",
                    found: "LENGTHUNIT".into(),
                },
            ],
            "{schema}"
        );
    }
}

/// The exact API refuses what the permissive view lists as unresolved: a
/// `$` value is `MissingValueSlot` and a non-numeric one `UnsupportedValue`,
/// never a phantom value; a cut-off record refuses the set it belongs to.
#[test]
fn the_exact_api_refuses_the_same_quantities() {
    for (schema, formula, wall) in RELEASES {
        let records = [
            format!("#1=IFCWALL('1xS3BCk291UvhgP2dvNsgp',$,'Wall',{wall});"),
            format!("#11=IFCQUANTITYAREA('Area',$,$,${formula});"),
            format!("#12=IFCQUANTITYVOLUME('Volume',$,$,'lots'{formula});"),
            "#13=IFCQUANTITYCOUNT('Count',$,$);".to_owned(),
            "#20=IFCELEMENTQUANTITY('0YvctVUKr0kugbFTf53O08',$,'Qto_Bad',$,$,(#11,#12));"
                .to_owned(),
            "#21=IFCRELDEFINESBYPROPERTIES('0YvctVUKr0kugbFTf53O09',$,$,$,(#1),#20);".to_owned(),
            "#30=IFCELEMENTQUANTITY('0YvctVUKr0kugbFTf53O30',$,'Qto_Cut',$,$,(#13));".to_owned(),
            "#31=IFCRELDEFINESBYPROPERTIES('0YvctVUKr0kugbFTf53O31',$,$,$,(#1),#30);".to_owned(),
        ];
        let records: Vec<&str> = records.iter().map(String::as_str).collect();
        let model = step(schema, &records);
        let exact = |set, name| exact_property(&model, EntityId(1), Some(set), name);
        assert_eq!(
            exact("Qto_Bad", "Area"),
            Err(ExactPropertyError::MissingValueSlot {
                property: EntityId(11)
            }),
            "{schema}"
        );
        assert_eq!(
            exact("Qto_Bad", "Volume"),
            Err(ExactPropertyError::UnsupportedValue {
                property: EntityId(12)
            }),
            "{schema}"
        );
        assert!(
            matches!(
                exact("Qto_Cut", "Count"),
                Err(ExactPropertyError::MalformedEntitySlots {
                    entity: EntityId(13),
                    ..
                })
            ),
            "{schema}"
        );
    }
}
