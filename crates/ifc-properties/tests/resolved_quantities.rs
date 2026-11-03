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
