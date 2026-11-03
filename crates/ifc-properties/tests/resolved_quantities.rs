//! Quantity sets by object (#578): both attachment routes, the anomalies,
//! and the separation from property sets.

use ifc_model::{Codec, EntityId, Model};
use ifc_properties::{
    properties_of, property_sets_by_object, quantity_sets_by_object, Attachment, PropertyAnomaly,
};

const WALL: EntityId = EntityId(10);
const TYPE: EntityId = EntityId(11);

fn parse(schema: &str, data: &str) -> Model {
    let text = format!(
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
         FILE_NAME('','',(''),(''),'','','');\nFILE_SCHEMA(('{schema}'));\n\
         ENDSEC;\nDATA;\n{data}\nENDSEC;\nEND-ISO-10303-21;\n"
    );
    let model = ifc_step::StepCodec
        .read_bytes(text.as_bytes())
        .expect("fixture parses");
    assert!(model.diagnostics().is_empty(), "{:?}", model.diagnostics());
    model
}

/// A wall typed by a wall type that holds quantity set #20.
fn ifc4(extra: &str) -> Model {
    parse(
        "IFC4",
        &format!(
            "#10=IFCWALL('0YvctVUKr0kugbFTf53O9L',$,'W',$,$,$,$,$,$);
#11=IFCWALLTYPE('0YvctVUKr0kugbFTf53O9A',$,'A',$,$,(#20),$,$,$,.STANDARD.);
#1=IFCQUANTITYLENGTH('Width',$,$,200.,$);
#3=IFCQUANTITYAREA('NetSideArea',$,$,12.5,$);
#20=IFCELEMENTQUANTITY('0YvctVUKr0kugbFTf53O20',$,'Qto_WallBaseQuantities',$,'BaseQuantities',(#1,#3));
#30=IFCRELDEFINESBYTYPE('0YvctVUKr0kugbFTf53O30',$,$,$,(#10),#11);
{extra}"
        ),
    )
}

#[test]
fn a_types_own_set_is_attached_as_type() {
    let model = ifc4("");
    let (found, anomalies) = quantity_sets_by_object(&model);
    assert!(anomalies.is_empty(), "{anomalies:?}");
    let sets = &found[&TYPE];
    assert_eq!(sets.len(), 1);
    assert_eq!(sets[0].0, Attachment::Type);
    assert_eq!(sets[0].1.id, EntityId(20));
    assert!(!found.contains_key(&WALL));
}

#[test]
fn a_repeated_quantity_name_is_reported_and_the_first_wins() {
    let model = ifc4(
        "#2=IFCQUANTITYLENGTH('Width',$,$,250.,$);
#21=IFCELEMENTQUANTITY('0YvctVUKr0kugbFTf53O21',$,'Qto_Dup',$,$,(#1,#2));
#40=IFCRELDEFINESBYPROPERTIES('0YvctVUKr0kugbFTf53O40',$,$,$,(#10),#21);",
    );
    let (found, anomalies) = quantity_sets_by_object(&model);
    assert_eq!(
        anomalies,
        vec![PropertyAnomaly::DuplicatePropertyName {
            set: EntityId(21),
            kept: EntityId(1),
            rejected: EntityId(2),
        }]
    );
    let (_, set) = &found[&WALL][0];
    assert_eq!(set.quantity("Width").map(|q| q.id()), Some(EntityId(1)));
}

#[test]
fn a_type_attached_by_relationship_is_reported() {
    let model = ifc4("#40=IFCRELDEFINESBYPROPERTIES('0YvctVUKr0kugbFTf53O40',$,$,$,(#11),#20);");
    let (found, anomalies) = quantity_sets_by_object(&model);
    assert_eq!(
        anomalies,
        vec![PropertyAnomaly::TypeAttachedByRelationship {
            relationship: EntityId(40),
            type_object: TYPE,
        }]
    );
    let kinds: Vec<_> = found[&TYPE].iter().map(|(a, _)| *a).collect();
    assert_eq!(kinds, vec![Attachment::Occurrence, Attachment::Type]);
}

#[test]
fn missing_object_and_missing_definition_are_reported() {
    let model = ifc4(
        "#40=IFCRELDEFINESBYPROPERTIES('0YvctVUKr0kugbFTf53O40',$,$,$,(#10,#99),#20);
#41=IFCRELDEFINESBYPROPERTIES('0YvctVUKr0kugbFTf53O41',$,$,$,(#10),#98);",
    );
    let (_, anomalies) = quantity_sets_by_object(&model);
    assert_eq!(
        anomalies,
        vec![
            PropertyAnomaly::MissingObject {
                relationship: EntityId(40),
                object: EntityId(99),
            },
            PropertyAnomaly::MissingDefinition {
                relationship: EntityId(41),
                definition: EntityId(98),
            },
        ]
    );
}

#[test]
fn property_and_quantity_sets_stay_in_their_own_view() {
    let model = ifc4(
        "#4=IFCPROPERTYSINGLEVALUE('P',$,IFCLABEL('x'),$);
#22=IFCPROPERTYSET('0YvctVUKr0kugbFTf53O22',$,'Qto_WallBaseQuantities',$,(#4));
#40=IFCRELDEFINESBYPROPERTIES('0YvctVUKr0kugbFTf53O40',$,$,$,(#10),#22);",
    );
    let (quantities, qa) = quantity_sets_by_object(&model);
    let (properties, pa) = property_sets_by_object(&model);
    assert!(qa.is_empty() && pa.is_empty(), "{qa:?} {pa:?}");
    assert!(!quantities.contains_key(&WALL));
    assert_eq!(quantities[&TYPE][0].1.id, EntityId(20));
    assert_eq!(properties[&WALL][0].1.id, EntityId(22));
    assert!(!properties.contains_key(&TYPE));
    assert_eq!(properties_of(&model, WALL).len(), 1);
}

// ---- resolution -----------------------------------------------------------------

use ifc_properties::{
    exact_unit, quantities_of, quantity_value, resolved_quantities, Quantity, QuantityKind, Source,
};

fn width(quantity: Option<&Quantity>) -> Option<f64> {
    match quantity? {
        Quantity::Simple { value, .. } => Some(*value),
        _ => None,
    }
}

/// An occurrence set of the same name as the type's, with a different Width.
const OVERRIDE: &str = "#2=IFCQUANTITYLENGTH('Width',$,$,250.,$);
#21=IFCELEMENTQUANTITY('0YvctVUKr0kugbFTf53O21',$,'Qto_WallBaseQuantities',$,$,(#2));
#40=IFCRELDEFINESBYPROPERTIES('0YvctVUKr0kugbFTf53O40',$,$,$,(#10),#21);";

#[test]
fn the_issue_fixture_resolves_the_occurrence_set() {
    let model = parse(
        "IFC4",
        "#7=IFCWALL('0YvctVUKr0kugbFTf53O9L',$,$,$,$,$,$,$,$);
#11=IFCQUANTITYLENGTH('Length',$,$,42.,$);
#9=IFCELEMENTQUANTITY('0YvctVUKr0kugbFTf53O09',$,'Qto_WallBaseQuantities',$,$,(#11));
#13=IFCRELDEFINESBYPROPERTIES('0YvctVUKr0kugbFTf53O13',$,$,$,(#7),#9);",
    );
    let found = quantities_of(&model, EntityId(7));
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].source, Source::Occurrence);
    assert_eq!(found[0].set.id, EntityId(9));
    assert!(found[0].shadowed.is_none());
}

#[test]
fn a_type_set_is_inherited() {
    let model = ifc4("");
    let found = quantities_of(&model, WALL);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].source, Source::Type(TYPE));
    assert!(found[0].shadowed.is_none());
    let width = quantity_value(&found, "Qto_WallBaseQuantities", "Width").unwrap();
    assert!(matches!(
        width,
        Quantity::Simple { kind: QuantityKind::Length, value, .. } if *value == 200.0
    ));
    // The type object itself is not resolved against a type.
    assert!(quantities_of(&model, TYPE).is_empty());
}

#[test]
fn a_type_gets_only_sets_attached_by_the_forbidden_relationship() {
    let model = ifc4("#40=IFCRELDEFINESBYPROPERTIES('0YvctVUKr0kugbFTf53O40',$,$,$,(#11),#20);");
    let found = quantities_of(&model, TYPE);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].source, Source::Occurrence);
    assert_eq!(found[0].set.id, EntityId(20));
}

#[test]
fn an_occurrence_set_shadows_the_type_set_as_a_whole() {
    let model = ifc4(OVERRIDE);
    let (resolved, anomalies) = resolved_quantities(&model);
    assert!(anomalies.is_empty(), "{anomalies:?}");
    let found = &resolved[&WALL];
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].source, Source::Occurrence);
    assert_eq!(found[0].set.id, EntityId(21));
    let shadowed = found[0].shadowed.as_ref().expect("type set shadowed");
    assert_eq!(shadowed.source, Source::Type(TYPE));
    assert_eq!(shadowed.set.id, EntityId(20));
    assert_eq!(
        width(quantity_value(found, "Qto_WallBaseQuantities", "Width")),
        Some(250.0)
    );
    // Per set, not per quantity: the type's NetSideArea is hidden.
    assert!(quantity_value(found, "Qto_WallBaseQuantities", "NetSideArea").is_none());
}

#[test]
fn a_same_named_property_set_does_not_shadow_a_quantity_set() {
    let model = ifc4(
        "#4=IFCPROPERTYSINGLEVALUE('P',$,IFCLABEL('x'),$);
#22=IFCPROPERTYSET('0YvctVUKr0kugbFTf53O22',$,'Qto_WallBaseQuantities',$,(#4));
#40=IFCRELDEFINESBYPROPERTIES('0YvctVUKr0kugbFTf53O40',$,$,$,(#10),#22);",
    );
    let found = quantities_of(&model, WALL);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].source, Source::Type(TYPE));
    assert!(found[0].shadowed.is_none());
    let props = properties_of(&model, WALL);
    assert_eq!(props.len(), 1);
    assert_eq!(props[0].set.id, EntityId(22));
    assert_eq!(props[0].source, Source::Occurrence);
}

#[test]
fn sets_are_ordered_by_name_not_id() {
    let model = ifc4(
        "#2=IFCQUANTITYLENGTH('Height',$,$,3000.,$);
#21=IFCELEMENTQUANTITY('0YvctVUKr0kugbFTf53O21',$,'Qto_A',$,$,(#2));
#22=IFCELEMENTQUANTITY('0YvctVUKr0kugbFTf53O22',$,'Qto_B',$,$,(#2));
#40=IFCRELDEFINESBYPROPERTIES('0YvctVUKr0kugbFTf53O40',$,$,$,(#10),#22);
#41=IFCRELDEFINESBYPROPERTIES('0YvctVUKr0kugbFTf53O41',$,$,$,(#10),#21);",
    );
    let names: Vec<_> = quantities_of(&model, WALL)
        .iter()
        .map(|r| r.set.name.as_deref().unwrap().to_owned())
        .collect();
    assert_eq!(names, ["Qto_A", "Qto_B", "Qto_WallBaseQuantities"]);
}

#[test]
fn two_same_named_occurrence_sets_keep_the_lower_id() {
    let model = ifc4(
        "#2=IFCQUANTITYLENGTH('Width',$,$,250.,$);
#21=IFCELEMENTQUANTITY('0YvctVUKr0kugbFTf53O21',$,'Qto_X',$,$,(#1));
#22=IFCELEMENTQUANTITY('0YvctVUKr0kugbFTf53O22',$,'Qto_X',$,$,(#2));
#40=IFCRELDEFINESBYPROPERTIES('0YvctVUKr0kugbFTf53O40',$,$,$,(#10),#22);
#41=IFCRELDEFINESBYPROPERTIES('0YvctVUKr0kugbFTf53O41',$,$,$,(#10),#21);",
    );
    let (resolved, anomalies) = resolved_quantities(&model);
    assert_eq!(
        anomalies,
        vec![PropertyAnomaly::DuplicateSetName {
            owner: WALL,
            kept: EntityId(21),
            rejected: EntityId(22),
        }]
    );
    let x: Vec<_> = resolved[&WALL]
        .iter()
        .filter(|r| r.set.name.as_deref() == Some("Qto_X"))
        .collect();
    assert_eq!(x.len(), 1);
    assert_eq!(x[0].set.id, EntityId(21));
    assert!(x[0].shadowed.is_none());
}

#[test]
fn duplicate_sets_on_a_type_without_occurrences_are_reported() {
    let model = parse(
        "IFC4",
        "#11=IFCWALLTYPE('0YvctVUKr0kugbFTf53O9A',$,'A',$,$,(#20,#21),$,$,$,.STANDARD.);
#1=IFCQUANTITYLENGTH('Width',$,$,200.,$);
#20=IFCELEMENTQUANTITY('0YvctVUKr0kugbFTf53O20',$,'Qto_X',$,$,(#1));
#21=IFCELEMENTQUANTITY('0YvctVUKr0kugbFTf53O21',$,'Qto_X',$,$,(#1));",
    );
    let (_, anomalies) = resolved_quantities(&model);
    assert_eq!(
        anomalies,
        vec![PropertyAnomaly::DuplicateSetName {
            owner: TYPE,
            kept: EntityId(20),
            rejected: EntityId(21),
        }]
    );
}

#[test]
fn a_twice_typed_object_is_reported() {
    let model = ifc4(
        "#12=IFCWALLTYPE('0YvctVUKr0kugbFTf53O9B',$,'B',$,$,$,$,$,$,.STANDARD.);
#31=IFCRELDEFINESBYTYPE('0YvctVUKr0kugbFTf53O31',$,$,$,(#10),#12);",
    );
    let (_, anomalies) = resolved_quantities(&model);
    assert_eq!(
        anomalies,
        vec![PropertyAnomaly::TypedTwice {
            object: WALL,
            kept: TYPE,
            rejected: EntityId(12),
            relation: EntityId(31),
        }]
    );
}

#[test]
fn every_release_resolves_inheritance_and_override() {
    let ifc2x3 = |extra: &str| {
        parse(
            "IFC2X3",
            &format!(
                "#10=IFCWALL('0YvctVUKr0kugbFTf53O9L',$,'W',$,$,$,$,$);
#11=IFCWALLTYPE('0YvctVUKr0kugbFTf53O9A',$,'A',$,$,(#20),$,$,$,.STANDARD.);
#1=IFCQUANTITYLENGTH('Width',$,$,200.);
#3=IFCQUANTITYAREA('NetSideArea',$,$,12.5);
#20=IFCELEMENTQUANTITY('0YvctVUKr0kugbFTf53O20',$,'Qto_WallBaseQuantities',$,'BaseQuantities',(#1,#3));
#30=IFCRELDEFINESBYTYPE('0YvctVUKr0kugbFTf53O30',$,$,$,(#10),#11);
{extra}"
            ),
        )
    };
    let ifc4x3 = |extra: &str| {
        let text = format!(
            "#10=IFCWALL('0YvctVUKr0kugbFTf53O9L',$,'W',$,$,$,$,$,$);
#11=IFCWALLTYPE('0YvctVUKr0kugbFTf53O9A',$,'A',$,$,(#20),$,$,$,.STANDARD.);
#1=IFCQUANTITYLENGTH('Width',$,$,200.,$);
#3=IFCQUANTITYNUMBER('Openings',$,$,3.,$);
#20=IFCELEMENTQUANTITY('0YvctVUKr0kugbFTf53O20',$,'Qto_WallBaseQuantities',$,'BaseQuantities',(#1,#3));
#30=IFCRELDEFINESBYTYPE('0YvctVUKr0kugbFTf53O30',$,$,$,(#10),#11);
{extra}"
        );
        parse("IFC4X3_ADD2", &text)
    };
    for model in [ifc2x3(""), ifc4(""), ifc4x3("")] {
        let found = quantities_of(&model, WALL);
        assert_eq!(found[0].source, Source::Type(TYPE));
        assert!(quantity_value(&found, "Qto_WallBaseQuantities", "Width").is_some());
    }
    let number = quantities_of(&ifc4x3(""), WALL);
    assert!(matches!(
        quantity_value(&number, "Qto_WallBaseQuantities", "Openings"),
        Some(Quantity::Simple {
            kind: QuantityKind::Number,
            ..
        })
    ));
    for model in [
        ifc2x3(
            "#2=IFCQUANTITYLENGTH('Width',$,$,250.);
#21=IFCELEMENTQUANTITY('0YvctVUKr0kugbFTf53O21',$,'Qto_WallBaseQuantities',$,$,(#2));
#40=IFCRELDEFINESBYPROPERTIES('0YvctVUKr0kugbFTf53O40',$,$,$,(#10),#21);",
        ),
        ifc4(OVERRIDE),
        ifc4x3(OVERRIDE),
    ] {
        let found = quantities_of(&model, WALL);
        assert_eq!(found[0].source, Source::Occurrence);
        assert_eq!(found[0].shadowed.as_ref().unwrap().set.id, EntityId(20));
        assert_eq!(
            width(quantity_value(&found, "Qto_WallBaseQuantities", "Width")),
            Some(250.0)
        );
    }
}

#[test]
fn the_effective_unit_is_the_project_unit_unless_the_quantity_states_one() {
    let model = ifc4(
        "#50=IFCSIUNIT(*,.LENGTHUNIT.,.MILLI.,.METRE.);
#51=IFCUNITASSIGNMENT((#50));
#52=IFCPROJECT('0YvctVUKr0kugbFTf53O52',$,'P',$,$,$,$,$,#51);
#53=IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.);
#2=IFCQUANTITYLENGTH('Width',$,#53,0.25,$);
#21=IFCELEMENTQUANTITY('0YvctVUKr0kugbFTf53O21',$,'Qto_Own',$,$,(#2));
#40=IFCRELDEFINESBYPROPERTIES('0YvctVUKr0kugbFTf53O40',$,$,$,(#10),#21);",
    );
    let found = quantities_of(&model, WALL);
    let inherited = quantity_value(&found, "Qto_WallBaseQuantities", "Width").unwrap();
    let Quantity::Simple {
        kind, unit, value, ..
    } = inherited
    else {
        panic!("simple quantity expected");
    };
    let effective = exact_unit(&model, kind.measure_type(), *unit).unwrap();
    assert!(effective.from_project);
    assert!((effective.scale - 1e-3).abs() < 1e-15);
    assert!((value * effective.scale - 0.2).abs() < 1e-12);

    let own = quantity_value(&found, "Qto_Own", "Width").unwrap();
    let Quantity::Simple { kind, unit, .. } = own else {
        panic!("simple quantity expected");
    };
    let effective = exact_unit(&model, kind.measure_type(), *unit).unwrap();
    assert!(!effective.from_project);
    assert_eq!(effective.scale, 1.0);
}
