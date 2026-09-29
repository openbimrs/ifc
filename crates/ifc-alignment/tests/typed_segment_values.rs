//! Every segment reader reads a typed parameter exactly like the bare number
//! (#140).
//!
//! A STEP writer may wrap a number in its defined type
//! (`IFCLENGTHMEASURE(1.)`); the codec keeps that as `Value::Typed`. The
//! horizontal reader always unwrapped it; the vertical and cant readers, and
//! the cant layout's `RailHeadDistance`, refused it as the wrong kind. Each
//! test below reads the same records twice, once bare and once with every
//! number typed as its `IFC4X3_ADD2.exp` declaration, and requires the same
//! answer.

use ifc_alignment::{
    read_cant_segment, read_horizontal_segment, read_vertical_segment, AlignmentUnits, CantLayout,
};
use ifc_model::codec::Codec;
use ifc_model::{EntityId, Model};
use ifc_step::StepCodec;

/// Millimetres, so every length read proves the unit is still applied to
/// the unwrapped number.
const MILLIMETRES: AlignmentUnits = AlignmentUnits {
    length_to_metres: 0.001,
    angle_to_radians: 1.0,
};

fn model(data: &str) -> Model {
    let text = format!(
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
         FILE_NAME('typed.ifc','2026-09-28T00:00:00',(''),(''),'','','');\n\
         FILE_SCHEMA(('IFC4X3_ADD2'));\nENDSEC;\nDATA;\n{data}\nENDSEC;\nEND-ISO-10303-21;\n"
    );
    StepCodec
        .read_bytes(text.as_bytes())
        .expect("fixture parses")
}

#[test]
fn the_horizontal_reader_reads_typed_numbers_as_bare() {
    let bare = model(
        "#1=IFCCARTESIANPOINT((1000.,2000.));\n\
         #2=IFCALIGNMENTHORIZONTALSEGMENT($,$,#1,0.5,250000.,-250000.,100000.,1500.,.CIRCULARARC.);",
    );
    let typed = model(
        "#1=IFCCARTESIANPOINT((1000.,2000.));\n\
         #2=IFCALIGNMENTHORIZONTALSEGMENT($,$,#1,IFCPLANEANGLEMEASURE(0.5),\
         IFCLENGTHMEASURE(250000.),IFCLENGTHMEASURE(-250000.),\
         IFCNONNEGATIVELENGTHMEASURE(100000.),IFCPOSITIVELENGTHMEASURE(1500.),.CIRCULARARC.);",
    );
    let read = |m: &Model| read_horizontal_segment(m, EntityId(2), MILLIMETRES);
    assert_eq!(
        read(&typed).expect("typed reads"),
        read(&bare).expect("bare reads")
    );
}

#[test]
fn the_vertical_reader_reads_typed_numbers_as_bare() {
    let bare = model(
        "#10=IFCALIGNMENTVERTICALSEGMENT($,$,1100000.,200000.,52000.,0.02,-0.03,4000000.,.PARABOLICARC.);",
    );
    let typed = model(
        "#10=IFCALIGNMENTVERTICALSEGMENT($,$,IFCLENGTHMEASURE(1100000.),\
         IFCNONNEGATIVELENGTHMEASURE(200000.),IFCLENGTHMEASURE(52000.),IFCRATIOMEASURE(0.02),\
         IFCRATIOMEASURE(-0.03),IFCLENGTHMEASURE(4000000.),.PARABOLICARC.);",
    );
    let read = |m: &Model| read_vertical_segment(m, EntityId(10), MILLIMETRES);
    let segment = read(&typed).expect("typed reads");
    assert_eq!(segment, read(&bare).expect("bare reads"));
    assert_eq!(segment.start_dist_along, 1100.0);
    assert_eq!(segment.radius_of_curvature, Some(4000.0));
    assert_eq!(segment.end_gradient, -0.03);
}

#[test]
fn the_cant_reader_reads_typed_numbers_as_bare() {
    let bare = model(
        "#20=IFCALIGNMENTCANTSEGMENT($,$,5000.,5000.,50.,100.,-50.,-100.,.LINEARTRANSITION.);",
    );
    let typed = model(
        "#20=IFCALIGNMENTCANTSEGMENT($,$,IFCLENGTHMEASURE(5000.),\
         IFCPOSITIVELENGTHMEASURE(5000.),IFCLENGTHMEASURE(50.),IFCLENGTHMEASURE(100.),\
         IFCLENGTHMEASURE(-50.),IFCLENGTHMEASURE(-100.),.LINEARTRANSITION.);",
    );
    let read = |m: &Model| read_cant_segment(m, EntityId(20), MILLIMETRES);
    let segment = read(&typed).expect("typed reads");
    assert_eq!(segment, read(&bare).expect("bare reads"));
    assert_eq!(segment.start_dist_along, 5.0);
    assert_eq!(segment.end_cant_right, Some(-0.1));
}

/// The cant layout reads its own `RailHeadDistance` besides the segments.
#[test]
fn the_cant_layout_reads_a_typed_rail_head_distance_as_bare() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../test/fixtures/synthetic-surfaces/synthetic_alignment_layout.ifc");
    let bare = std::fs::read_to_string(path).expect("fixture");
    let typed = bare
        .replace(
            "#321=IFCALIGNMENTCANT('align-cant-00000000001',$,$,$,$,#320,$,1.5);",
            "#321=IFCALIGNMENTCANT('align-cant-00000000001',$,$,$,$,#320,$,IFCPOSITIVELENGTHMEASURE(1.5));",
        )
        .replace(
            "#300=IFCALIGNMENTCANTSEGMENT($,$,0.,5.,0.,0.05,0.,-0.05,",
            "#300=IFCALIGNMENTCANTSEGMENT($,$,IFCLENGTHMEASURE(0.),IFCPOSITIVELENGTHMEASURE(5.),\
             IFCLENGTHMEASURE(0.),IFCLENGTHMEASURE(0.05),IFCLENGTHMEASURE(0.),IFCLENGTHMEASURE(-0.05),",
        );
    assert_ne!(bare, typed, "both records were rewritten");
    let units = AlignmentUnits {
        length_to_metres: 1.0,
        angle_to_radians: 1.0,
    };
    let read = |text: &str| {
        let m = StepCodec.read_bytes(text.as_bytes()).expect("parses");
        CantLayout::resolve(&m, EntityId(321), units).expect("cant layout resolves")
    };
    let (bare, typed) = (read(&bare), read(&typed));
    assert_eq!(typed.rail_head_distance, 1.5);
    assert_eq!(typed.rail_head_distance, bare.rail_head_distance);
    assert_eq!(typed.segments(), bare.segments());
}
