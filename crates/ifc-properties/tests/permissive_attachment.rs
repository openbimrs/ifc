//! Permissive attachment of sets to type objects and occurrences, on the
//! declared release's tables (#631, #632).
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
    let (p, q) = both(&model);
    assert_eq!(p, vec![missing(2, 99)]);
    assert_eq!(q, vec![missing(2, 99)]);
    // The readable set is still attached.
    assert_eq!(names(&model, 2).len(), 1);
}

#[test]
fn a_present_set_of_another_kind_is_not_reported_missing() {
    let wt = "#2=IFCWALLTYPE('4xS3BCk291UvhgP2dvNsgp',$,'WT',$,$,(#20,#21),$,$,.STANDARD.);";
    let model = step("IFC4", &[wt, P10, PSET20, Q11, QSET21]);
    assert_eq!(both(&model), (vec![], vec![]));
}
