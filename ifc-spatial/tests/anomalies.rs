//! Second parents are reported, and every view agrees with the kept one (#54).

use ifc_model::{Codec, EntityId, Model};
use ifc_spatial::{SpatialAnomaly, SpatialTree};

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

const HIERARCHY: &str = "\
#1=IFCPROJECT('p',$,'P',$,$,$,$,$,$);
#2=IFCBUILDING('b1',$,'B1',$,$,$,$,$,.ELEMENT.,$,$,$);
#3=IFCBUILDING('b2',$,'B2',$,$,$,$,$,.ELEMENT.,$,$,$);
#4=IFCBUILDINGSTOREY('s1',$,'S1',$,$,$,$,$,.ELEMENT.,$);
#5=IFCSPACE('r1',$,'R1',$,$,$,$,$,.ELEMENT.,.INTERNAL.,$);
#6=IFCSPACE('r2',$,'R2',$,$,$,$,$,.ELEMENT.,.INTERNAL.,$);
#7=IFCWALL('w',$,'W',$,$,$,$,$,$);
#10=IFCRELAGGREGATES('a1',$,$,$,#1,(#2,#3));
#11=IFCRELAGGREGATES('a2',$,$,$,#2,(#4));
#13=IFCRELAGGREGATES('a4',$,$,$,#4,(#5,#6));
#20=IFCRELCONTAINEDINSPATIALSTRUCTURE('c1',$,$,$,(#7),#5);";

/// The reproduction from issue #54.
fn conflicting() -> Model {
    parse(&format!(
        "{HIERARCHY}\n\
         #12=IFCRELAGGREGATES('a3',$,$,$,#3,(#4));\n\
         #21=IFCRELCONTAINEDINSPATIALSTRUCTURE('c2',$,$,$,(#7),#6);"
    ))
}

#[test]
fn double_containment_and_double_aggregation_are_each_reported_once() {
    let tree = SpatialTree::build(&conflicting());
    assert_eq!(
        tree.anomalies(),
        [
            SpatialAnomaly::AggregatedTwice {
                child: EntityId(4),
                kept: EntityId(2),
                rejected: EntityId(3),
                relation: EntityId(12),
            },
            SpatialAnomaly::ContainedTwice {
                element: EntityId(7),
                kept: EntityId(5),
                rejected: EntityId(6),
                relation: EntityId(21),
            },
        ]
    );
}

#[test]
fn the_rejected_container_does_not_list_the_element() {
    let tree = SpatialTree::build(&conflicting());
    assert_eq!(tree.container_of(EntityId(7)), Some(EntityId(5)));
    assert_eq!(tree.elements_of(EntityId(5)), [EntityId(7)]);
    assert!(tree.elements_of(EntityId(6)).is_empty());
    assert_eq!(
        tree.elements_recursive(EntityId(1)),
        [EntityId(7)],
        "the element is counted once from the root"
    );
}

#[test]
fn the_rejected_parent_does_not_list_the_child() {
    let tree = SpatialTree::build(&conflicting());
    assert_eq!(tree.node(EntityId(4)).unwrap().parent, Some(EntityId(2)));
    assert_eq!(tree.node(EntityId(2)).unwrap().children, [EntityId(4)]);
    assert!(tree.node(EntityId(3)).unwrap().children.is_empty());
}

#[test]
fn a_valid_hierarchy_has_no_anomalies() {
    let tree = SpatialTree::build(&parse(HIERARCHY));
    assert!(tree.anomalies().is_empty(), "{:?}", tree.anomalies());
    assert_eq!(tree.container_of(EntityId(7)), Some(EntityId(5)));
}

/// Stating the same parent twice is redundant, not contradictory.
#[test]
fn a_repeated_statement_of_the_same_parent_is_not_an_anomaly() {
    let tree = SpatialTree::build(&parse(&format!(
        "{HIERARCHY}\n\
         #12=IFCRELAGGREGATES('a3',$,$,$,#2,(#4));\n\
         #21=IFCRELCONTAINEDINSPATIALSTRUCTURE('c2',$,$,$,(#7,#7),#5);"
    )));
    assert!(tree.anomalies().is_empty(), "{:?}", tree.anomalies());
    assert_eq!(tree.elements_of(EntityId(5)), [EntityId(7)]);
    assert_eq!(tree.node(EntityId(2)).unwrap().children, [EntityId(4)]);
}
