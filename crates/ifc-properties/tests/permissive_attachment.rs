//! Permissive attachment of sets to type objects and occurrences, on the
//! declared release's tables (#631, #632, #630).
//!
//! Models are parsed from STEP text, so the file is exactly what an exporter
//! would write.

use ifc_model::{Codec, EntityId, Model};
use ifc_properties::{
    property_sets_by_object, quantity_sets_by_object, Attachment, PropertyAnomaly,
};
use ifc_step::StepCodec;

fn step(schema: &str, records: &[&str]) -> Model {
    let text = format!(
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
         FILE_NAME('','',(''),(''),'','','');\nFILE_SCHEMA(('{schema}'));\nENDSEC;\n\
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
const PSET20: &str = "#20=IFCPROPERTYSET('2xS3BCk291UvhgP2dvNsgp',$,'Pset_A',$,(#10));";
const Q11: &str = "#11=IFCQUANTITYLENGTH('Length',$,$,2.5,$);";
const QSET21: &str = "#21=IFCELEMENTQUANTITY('3xS3BCk291UvhgP2dvNsgp',$,'Qto_A',$,$,(#11));";

fn missing(relationship: u64, definition: u64) -> PropertyAnomaly {
    PropertyAnomaly::MissingDefinition {
        relationship: EntityId(relationship),
        definition: EntityId(definition),
    }
}

fn names(model: &Model, object: u64) -> Vec<(Attachment, Option<String>)> {
    let (sets, _) = property_sets_by_object(model);
    sets.get(&EntityId(object))
        .map(|v| {
            v.iter()
                .map(|(a, s)| (*a, s.name.as_deref().map(str::to_owned)))
                .collect()
        })
        .unwrap_or_default()
}

fn qnames(model: &Model, object: u64) -> Vec<(Attachment, Option<String>)> {
    let (sets, _) = quantity_sets_by_object(model);
    sets.get(&EntityId(object))
        .map(|v| {
            v.iter()
                .map(|(a, s)| (*a, s.name.as_deref().map(str::to_owned)))
                .collect()
        })
        .unwrap_or_default()
}

fn both(model: &Model) -> (Vec<PropertyAnomaly>, Vec<PropertyAnomaly>) {
    (
        property_sets_by_object(model).1,
        quantity_sets_by_object(model).1,
    )
}

// --- #631 ---------------------------------------------------------------

#[cfg(feature = "ifc4x3")]
#[test]
fn an_ifc4x3_only_type_object_carries_its_sets() {
    let bearing =
        "#2=IFCBEARINGTYPE('4xS3BCk291UvhgP2dvNsgp',$,'B',$,$,(#20,#21),$,.ELASTOMERIC.);";
    let model = step("IFC4X3_ADD2", &[bearing, P10, PSET20, Q11, QSET21]);
    assert_eq!(
        names(&model, 2),
        vec![(Attachment::Type, Some("Pset_A".to_owned()))]
    );
    assert_eq!(
        qnames(&model, 2),
        vec![(Attachment::Type, Some("Qto_A".to_owned()))]
    );
}

// --- #632 ---------------------------------------------------------------

#[test]
fn a_type_set_id_absent_from_the_file_is_reported() {
    let wt = "#2=IFCWALLTYPE('4xS3BCk291UvhgP2dvNsgp',$,'WT',$,$,(#20,#99),$,$,.STANDARD.);";
    let model = step("IFC4", &[wt, P10, PSET20, Q11, QSET21]);
    let missing_set = PropertyAnomaly::MissingTypeSet {
        type_object: EntityId(2),
        definition: EntityId(99),
    };
    let (p, q) = both(&model);
    assert_eq!(p, vec![missing_set.clone()]);
    assert_eq!(q, vec![missing_set]);
    // The readable set is still attached.
    assert_eq!(names(&model, 2).len(), 1);
}

#[test]
fn a_present_set_of_another_kind_is_not_reported_missing() {
    let wt = "#2=IFCWALLTYPE('4xS3BCk291UvhgP2dvNsgp',$,'WT',$,$,(#20,#21),$,$,.STANDARD.);";
    let model = step("IFC4", &[wt, P10, PSET20, Q11, QSET21]);
    assert_eq!(both(&model), (vec![], vec![]));
}

// --- #630 ---------------------------------------------------------------

fn rel(definition: &str) -> String {
    format!("#30=IFCRELDEFINESBYPROPERTIES('5xS3BCk291UvhgP2dvNsgp',$,$,$,(#1),{definition});")
}

#[test]
fn a_definition_set_is_read_member_by_member() {
    for definition in ["(#20,#21)", "IFCPROPERTYSETDEFINITIONSET((#20,#21))"] {
        let r = rel(definition);
        let model = step("IFC4", &[WALL, P10, PSET20, Q11, QSET21, &r]);
        assert_eq!(
            names(&model, 1),
            vec![(Attachment::Occurrence, Some("Pset_A".to_owned()))],
            "{definition}"
        );
        assert_eq!(
            qnames(&model, 1),
            vec![(Attachment::Occurrence, Some("Qto_A".to_owned()))],
            "{definition}"
        );
        assert_eq!(both(&model), (vec![], vec![]), "{definition}");
    }
}

#[test]
fn a_mixed_definition_set_reports_each_unreadable_member() {
    let r = rel("(#20,#99,'x',#21)");
    let model = step("IFC4", &[WALL, P10, PSET20, Q11, QSET21, &r]);
    assert_eq!(names(&model, 1).len(), 1);
    assert_eq!(qnames(&model, 1).len(), 1);
    let malformed = PropertyAnomaly::MalformedAttribute {
        entity: EntityId(30),
        attribute: "RelatingPropertyDefinition",
        found: "Text(\"x\")".to_owned(),
    };
    let (p, q) = both(&model);
    for anomalies in [p, q] {
        assert!(anomalies.contains(&missing(30, 99)), "{anomalies:?}");
        assert!(anomalies.contains(&malformed), "{anomalies:?}");
        assert_eq!(anomalies.len(), 2, "{anomalies:?}");
    }
}

#[test]
fn a_definition_set_attaches_to_every_related_object() {
    let r = "#30=IFCRELDEFINESBYPROPERTIES('5xS3BCk291UvhgP2dvNsgp',$,$,$,(#1,#98),(#20));";
    let model = step("IFC4", &[WALL, P10, PSET20, r]);
    assert_eq!(names(&model, 1).len(), 1);
    let (_, a) = property_sets_by_object(&model);
    assert_eq!(
        a,
        vec![PropertyAnomaly::MissingObject {
            relationship: EntityId(30),
            object: EntityId(98)
        }]
    );
}

// --- review: names the declared table does not know ----------------------

const DOOR_TYPE: &str =
    "#2=IFCDOORTYPE('4xS3BCk291UvhgP2dvNsgp',$,'DT',$,$,(#20),$,$,.DOOR.,.NOTDEFINED.,$);";

#[cfg(all(feature = "ifc2x3", feature = "ifc4"))]
#[test]
fn a_type_the_declared_release_does_not_define_still_carries_its_sets() {
    let model = step("IFC2X3", &[DOOR_TYPE, P10, PSET20]);
    assert_eq!(
        names(&model, 2),
        vec![(Attachment::Type, Some("Pset_A".to_owned()))]
    );
}

#[cfg(all(feature = "ifc2x3", feature = "ifc4"))]
#[test]
fn such_a_type_attached_by_a_relationship_is_reported() {
    let r = "#30=IFCRELDEFINESBYPROPERTIES('5xS3BCk291UvhgP2dvNsgp',$,$,$,(#2),#20);";
    let model = step("IFC2X3", &[DOOR_TYPE, P10, PSET20, r]);
    let (p, _) = both(&model);
    assert_eq!(
        p,
        vec![PropertyAnomaly::TypeAttachedByRelationship {
            relationship: EntityId(30),
            type_object: EntityId(2),
        }]
    );
}

// --- review: HasPropertySets and empty lists ------------------------------

fn malformed(entity: u64, attribute: &'static str, found: &str) -> PropertyAnomaly {
    PropertyAnomaly::MalformedAttribute {
        entity: EntityId(entity),
        attribute,
        found: found.to_owned(),
    }
}

#[test]
fn an_empty_definition_set_is_reported() {
    for definition in ["()", "IFCPROPERTYSETDEFINITIONSET(())"] {
        let r = rel(definition);
        let model = step("IFC4", &[WALL, P10, PSET20, &r]);
        let (p, q) = both(&model);
        let expected = vec![malformed(30, "RelatingPropertyDefinition", "List([])")];
        assert_eq!(p, expected, "{definition}");
        assert_eq!(q, expected, "{definition}");
    }
}

#[test]
fn a_malformed_has_property_sets_is_reported() {
    let cases = [
        ("(#20,'x')", "Text(\"x\")", 1),
        ("()", "List([])", 0),
        ("#20", "Ref(EntityId(20))", 0),
    ];
    for (value, found, attached) in cases {
        let wt =
            format!("#2=IFCWALLTYPE('4xS3BCk291UvhgP2dvNsgp',$,'WT',$,$,{value},$,$,.STANDARD.);");
        let model = step("IFC4", &[&wt, P10, PSET20]);
        let (p, q) = both(&model);
        let expected = vec![malformed(2, "HasPropertySets", found)];
        assert_eq!(p, expected, "{value}");
        assert_eq!(q, expected, "{value}");
        assert_eq!(names(&model, 2).len(), attached, "{value}");
    }
}
