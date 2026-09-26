//! The permissive views report the duplicates they resolve (#58).
//!
//! Each case keeps a deterministic winner (the first by id) and says so,
//! instead of silently overwriting one statement with another.

use ifc_model::{Codec, EntityId, Model};
use ifc_properties::{
    property_sets_by_object, property_value, resolved_properties, PropertyAnomaly, PropertyValue,
    Scalar, Source,
};

const WALL: EntityId = EntityId(10);
const TYPE_A: EntityId = EntityId(11);
const TYPE_B: EntityId = EntityId(12);

fn parse(data: &str) -> Model {
    let text = format!(
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
         FILE_NAME('','',(''),(''),'','','');\nFILE_SCHEMA(('IFC4'));\n\
         ENDSEC;\nDATA;\n{data}\nENDSEC;\nEND-ISO-10303-21;\n"
    );
    ifc_step::StepCodec
        .read_bytes(text.as_bytes())
        .expect("fixture parses")
}

/// A wall, two wall types each holding `Pset_WallCommon`, and the property
/// set bodies. `extra` adds the relationships a case needs.
fn model(extra: &str) -> Model {
    parse(&format!(
        "#1=IFCPROPERTYSINGLEVALUE('FireRating',$,IFCLABEL('REI30'),$);
#2=IFCPROPERTYSINGLEVALUE('FireRating',$,IFCLABEL('REI90'),$);
#3=IFCPROPERTYSINGLEVALUE('IsExternal',$,IFCBOOLEAN(.T.),$);
#4=IFCPROPERTYSINGLEVALUE('IsExternal',$,IFCBOOLEAN(.F.),$);
#10=IFCWALL('0YvctVUKr0kugbFTf53O9L',$,'W',$,$,$,$,$,$);
#11=IFCWALLTYPE('0YvctVUKr0kugbFTf53O9A',$,'A',$,$,(#20),$,$,$,.STANDARD.);
#12=IFCWALLTYPE('0YvctVUKr0kugbFTf53O9B',$,'B',$,$,(#21),$,$,$,.STANDARD.);
#20=IFCPROPERTYSET('0YvctVUKr0kugbFTf53O20',$,'Pset_WallCommon',$,(#3));
#21=IFCPROPERTYSET('0YvctVUKr0kugbFTf53O21',$,'Pset_WallCommon',$,(#4));
{extra}"
    ))
}

fn label(value: &PropertyValue) -> Option<&str> {
    match value {
        PropertyValue::Single {
            value: Some(measure),
            ..
        } => match &measure.scalar {
            Scalar::Text(text) => Some(text.as_ref()),
            _ => None,
        },
        _ => None,
    }
}

#[test]
fn an_object_typed_twice_keeps_the_first_type_and_says_so() {
    // #31 is listed first in the file but has the higher id: id order wins.
    let model = model(
        "#31=IFCRELDEFINESBYTYPE('0YvctVUKr0kugbFTf53O31',$,$,$,(#10),#12);
#30=IFCRELDEFINESBYTYPE('0YvctVUKr0kugbFTf53O30',$,$,$,(#10),#11);",
    );
    let (resolved, anomalies) = resolved_properties(&model);
    assert!(anomalies.contains(&PropertyAnomaly::TypedTwice {
        object: WALL,
        kept: TYPE_A,
        rejected: TYPE_B,
        relation: EntityId(31),
    }));
    let sets = &resolved[&WALL];
    assert_eq!(sets.len(), 1);
    assert_eq!(sets[0].source, Source::Type(TYPE_A));
}

#[test]
fn two_same_named_occurrence_sets_keep_the_first_not_a_shadow() {
    let model = model(
        "#22=IFCPROPERTYSET('0YvctVUKr0kugbFTf53O22',$,'Pset_Fire',$,(#1));
#23=IFCPROPERTYSET('0YvctVUKr0kugbFTf53O23',$,'Pset_Fire',$,(#2));
#40=IFCRELDEFINESBYPROPERTIES('0YvctVUKr0kugbFTf53O40',$,$,$,(#10),#23);
#41=IFCRELDEFINESBYPROPERTIES('0YvctVUKr0kugbFTf53O41',$,$,$,(#10),#22);",
    );
    let (resolved, anomalies) = resolved_properties(&model);
    assert!(anomalies.contains(&PropertyAnomaly::DuplicateSetName {
        owner: WALL,
        kept: EntityId(22),
        rejected: EntityId(23),
    }));
    let fire = resolved[&WALL]
        .iter()
        .find(|r| r.set.name.as_deref() == Some("Pset_Fire"))
        .unwrap();
    assert_eq!(fire.set.id, EntityId(22));
    assert!(
        fire.shadowed.is_none(),
        "an occurrence set is not shadowed by another occurrence set"
    );
}

#[test]
fn a_type_with_same_named_sets_is_reported_even_without_occurrences() {
    let model = parse(
        "#3=IFCPROPERTYSINGLEVALUE('IsExternal',$,IFCBOOLEAN(.T.),$);
#11=IFCWALLTYPE('0YvctVUKr0kugbFTf53O9A',$,'A',$,$,(#21,#20),$,$,$,.STANDARD.);
#20=IFCPROPERTYSET('0YvctVUKr0kugbFTf53O20',$,'Pset_WallCommon',$,(#3));
#21=IFCPROPERTYSET('0YvctVUKr0kugbFTf53O21',$,'Pset_WallCommon',$,(#3));",
    );
    let (_, anomalies) = resolved_properties(&model);
    assert_eq!(
        anomalies,
        [PropertyAnomaly::DuplicateSetName {
            owner: TYPE_A,
            kept: EntityId(20),
            rejected: EntityId(21),
        }]
    );
}

#[test]
fn a_set_with_two_same_named_properties_is_reported() {
    let model = model(
        "#22=IFCPROPERTYSET('0YvctVUKr0kugbFTf53O22',$,'Pset_Fire',$,(#1,#2));
#40=IFCRELDEFINESBYPROPERTIES('0YvctVUKr0kugbFTf53O40',$,$,$,(#10),#22);",
    );
    let (_, anomalies) = property_sets_by_object(&model);
    assert_eq!(
        anomalies,
        [PropertyAnomaly::DuplicatePropertyName {
            set: EntityId(22),
            kept: EntityId(1),
            rejected: EntityId(2),
        }]
    );
    // The documented winner: the first in HasProperties order.
    let resolved = resolved_properties(&model).0;
    let value = property_value(&resolved[&WALL], "Pset_Fire", "FireRating").unwrap();
    assert_eq!(label(&value.value), Some("REI30"));
}

#[test]
fn precedence_and_a_valid_model_raise_no_anomaly() {
    // Occurrence Pset_WallCommon overriding the type's is precedence.
    let model = model(
        "#22=IFCPROPERTYSET('0YvctVUKr0kugbFTf53O22',$,'Pset_WallCommon',$,(#1));
#30=IFCRELDEFINESBYTYPE('0YvctVUKr0kugbFTf53O30',$,$,$,(#10),#11);
#32=IFCRELDEFINESBYTYPE('0YvctVUKr0kugbFTf53O32',$,$,$,(#10),#11);
#40=IFCRELDEFINESBYPROPERTIES('0YvctVUKr0kugbFTf53O40',$,$,$,(#10),#22);",
    );
    let (resolved, anomalies) = resolved_properties(&model);
    assert!(anomalies.is_empty(), "{anomalies:?}");
    let set = &resolved[&WALL][0];
    assert_eq!(set.source, Source::Occurrence);
    assert_eq!(set.shadowed.as_ref().map(|s| s.set.id), Some(EntityId(20)));
}

/// A set may be defined by several templates, and all of them are kept (#60).
///
/// `IfcPropertySetDefinition.IsDefinedBy` is `SET [0:?] OF
/// IfcRelDefinesByTemplate`. Before, the map kept only the last template.
#[test]
fn every_template_defining_a_set_is_kept() {
    let model = parse(
        "#3=IFCPROPERTYSINGLEVALUE('IsExternal',$,IFCBOOLEAN(.T.),$);
#20=IFCPROPERTYSET('0YvctVUKr0kugbFTf53O20',$,'Pset_Custom',$,(#3));
#21=IFCPROPERTYSET('0YvctVUKr0kugbFTf53O21',$,'Pset_Other',$,(#3));
#50=IFCSIMPLEPROPERTYTEMPLATE('0YvctVUKr0kugbFTf53O50',$,'IsExternal',$,.P_SINGLEVALUE.,'IfcBoolean',$,$,$,$,$,.READWRITE.);
#51=IFCPROPERTYSETTEMPLATE('0YvctVUKr0kugbFTf53O51',$,'Pset_Custom',$,.PSET_OCCURRENCEDRIVEN.,'IfcWall',(#50));
#52=IFCPROPERTYSETTEMPLATE('0YvctVUKr0kugbFTf53O52',$,'Pset_Custom',$,.PSET_TYPEDRIVENOVERRIDE.,'IfcWall',(#50));
#61=IFCRELDEFINESBYTEMPLATE('0YvctVUKr0kugbFTf53O61',$,$,$,(#20),#52);
#60=IFCRELDEFINESBYTEMPLATE('0YvctVUKr0kugbFTf53O60',$,$,$,(#20,#21),#51);
#62=IFCRELDEFINESBYTEMPLATE('0YvctVUKr0kugbFTf53O62',$,$,$,(#20),#51);",
    );
    let links = ifc_properties::template_of_set(&model);
    assert_eq!(links[&EntityId(20)], [EntityId(51), EntityId(52)]);
    assert_eq!(links[&EntityId(21)], [EntityId(51)]);
}
