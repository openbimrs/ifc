//! A cost item nested under two parents is counted once and reported (#57).
//!
//! IFC4 `IfcObjectDefinition.Nests` (IFC2X3 `Decomposes`) is `SET [0:1]`.
//! Leaf `X` below is nested by both `A` (#20, first) and `B` (#21). Before,
//! `children_of(B)` still listed `X`, so summing the roots' rolled-up totals
//! counted `X`'s 100.00 twice.

use ifc_cost::{
    children_of, descendants_of, nesting_anomalies, parent_of, rolled_up_total, roots, CostAnomaly,
    CostItem, CostView,
};
use ifc_model::{Codec, EntityId, Model};

const A: EntityId = EntityId(10);
const B: EntityId = EntityId(11);
const X: EntityId = EntityId(12);
const Y: EntityId = EntityId(13);

fn parse(nests: &str) -> Model {
    let text = format!(
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
         FILE_NAME('','',(''),(''),'','','');\nFILE_SCHEMA(('IFC4'));\n\
         ENDSEC;\nDATA;\n\
#1=IFCCOSTVALUE('X',$,IFCMONETARYMEASURE(100.),$,$,$,$,$,$,$);
#2=IFCCOSTVALUE('Y',$,IFCMONETARYMEASURE(7.),$,$,$,$,$,$,$);
#10=IFCCOSTITEM('0VWxYL2zn28R_9XQuo5Te9',$,'A',$,$,$,$,$,$);
#11=IFCCOSTITEM('0y8t2zQ1T5fxsp2NrM8p4F',$,'B',$,$,$,$,$,$);
#12=IFCCOSTITEM('0rwC96jAnDder$HMASjSE6',$,'X',$,$,$,$,(#1),$);
#13=IFCCOSTITEM('0rwC96jAnDder$HMASjSE7',$,'Y',$,$,$,$,(#2),$);
{nests}
ENDSEC;\nEND-ISO-10303-21;\n"
    );
    ifc_step::StepCodec
        .read_bytes(text.as_bytes())
        .expect("fixture parses")
}

fn conflicting() -> Model {
    parse(
        "#20=IFCRELNESTS('0aaaaaaaaaaaaaaaaaaaa0',$,$,$,#10,(#12));
#21=IFCRELNESTS('0aaaaaaaaaaaaaaaaaaaa1',$,$,$,#11,(#12,#13));",
    )
}

fn total_over_roots(model: &Model) -> f64 {
    let view = CostView::new(model);
    roots(&view)
        .into_iter()
        .map(|id| {
            let item = CostItem::new(id, model.get(id).unwrap());
            rolled_up_total(&view, &item).unwrap()
        })
        .sum()
}

#[test]
fn the_second_parent_is_reported() {
    assert_eq!(
        nesting_anomalies(&conflicting()),
        [CostAnomaly::NestedTwice {
            item: X,
            kept: A,
            rejected: B,
            relation: EntityId(21),
        }]
    );
}

#[test]
fn every_view_agrees_with_the_kept_parent() {
    let model = conflicting();
    assert_eq!(parent_of(&model, X), Some(A));
    assert_eq!(children_of(&model, A), [X]);
    assert_eq!(children_of(&model, B), [Y], "B keeps its other child");
    assert_eq!(descendants_of(&model, B).unwrap(), [Y]);
}

#[test]
fn a_leaf_with_two_parents_is_counted_once() {
    // X (100) once under A, Y (7) under B.
    assert_eq!(total_over_roots(&conflicting()), 107.0);
}

#[test]
fn a_valid_or_redundant_nesting_has_no_anomaly() {
    let valid = parse("#20=IFCRELNESTS('0aaaaaaaaaaaaaaaaaaaa0',$,$,$,#10,(#12,#13));");
    assert!(nesting_anomalies(&valid).is_empty());
    assert_eq!(total_over_roots(&valid), 107.0);

    // The same parent stated twice, and a child listed twice: redundant.
    let redundant = parse(
        "#20=IFCRELNESTS('0aaaaaaaaaaaaaaaaaaaa0',$,$,$,#10,(#12,#12));
#21=IFCRELNESTS('0aaaaaaaaaaaaaaaaaaaa1',$,$,$,#10,(#12,#13));",
    );
    assert!(nesting_anomalies(&redundant).is_empty());
    assert_eq!(children_of(&redundant, A), [X, Y]);
    assert_eq!(total_over_roots(&redundant), 107.0);
}
