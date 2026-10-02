//! The zero-length segment that closes every layout (#262).
//!
//! IFC4.3 concept template *Alignment Layout - Horizontal, Vertical and
//! Cant*: "A zero-length segment shall be added, at the end of the list of
//! segments" of each horizontal, vertical and cant layout. The fixture is a
//! hand-built alignment that follows it on all three layouts; every read,
//! lowering and resolve path must accept it, and a zero-length segment
//! anywhere else is refused.
//!
//! Values, worked by hand from the IFC definitions:
//!
//! - horizontal: a 100 m LINE east from the origin, then a CIRCULARARC of
//!   radius 100 turning left through a quarter turn (length 50 pi), which
//!   ends at (200, 100) heading north. The closing LINE starts there.
//! - vertical: +2% from height 50 over 100 m (ends at 52.0), then -1% over
//!   the arc (a grade break at a height-continuous seam, #259), ending at
//!   52 - 0.5 pi. The closing segment restates that height.
//! - cant: a linear ramp from 0 to 0.05 m (left) and 0 to -0.05 m (right)
//!   over the line, held constant over the arc, then the closing segment.

use std::f64::consts::PI;

use axiolid_curve::{Curve3, ElevationLaw};
use ifc_alignment::{
    gradient_curve3, lower_gradient_curve, lower_horizontal_layout,
    lower_horizontal_layout_partial, lower_horizontal_plan, lower_horizontal_segment,
    lower_vertical_segment, vertical_profile_law, AlignmentError, AlignmentUnits, AlignmentView,
    CantLayout, SeamCheck, Stationing, VerticalLayout, VerticalSeamKind,
};
use ifc_model::codec::Codec;
use ifc_model::{EntityId, Model};
use ifc_step::StepCodec;

fn metres() -> AlignmentUnits {
    AlignmentUnits {
        length_to_metres: 1.0,
        angle_to_radians: 1.0,
    }
}

const ALIGNMENT: EntityId = EntityId(401);
const HORIZONTAL: EntityId = EntityId(121);
const VERTICAL: EntityId = EntityId(221);
const CANT: EntityId = EntityId(321);
/// The closing segments' `DesignParameters`.
const H_CLOSING: EntityId = EntityId(105);
const V_CLOSING: EntityId = EntityId(202);
const C_CLOSING: EntityId = EntityId(302);

/// Plan length: 100 + 50 pi.
const PLAN_LENGTH: f64 = 100.0 + 50.0 * PI;

/// The fixture, with each layout's nest list and closing records
/// substitutable.
struct Fixture {
    horizontal_nest: &'static str,
    vertical_nest: &'static str,
    cant_nest: &'static str,
    h_closing: String,
    v_closing: String,
    c_closing: String,
}

impl Default for Fixture {
    fn default() -> Self {
        Self {
            horizontal_nest: "(#111,#113,#115)",
            vertical_nest: "(#211,#213,#215)",
            cant_nest: "(#311,#313,#315)",
            h_closing: "#105=IFCALIGNMENTHORIZONTALSEGMENT($,$,#104,1.5707963267948966,0.,0.,0.,$,.LINE.);".to_owned(),
            v_closing: format!(
                "#202=IFCALIGNMENTVERTICALSEGMENT($,$,{PLAN_LENGTH:?},0.,{:?},-0.01,-0.01,$,.CONSTANTGRADIENT.);",
                52.0 - 0.5 * PI
            ),
            c_closing: format!(
                "#302=IFCALIGNMENTCANTSEGMENT($,$,{PLAN_LENGTH:?},0.,0.05,$,-0.05,$,.CONSTANTCANT.);"
            ),
        }
    }
}

impl Fixture {
    fn model(&self) -> Model {
        let arc_length = 50.0 * PI;
        let text = format!(
            "ISO-10303-21;
HEADER;
FILE_DESCRIPTION(('ViewDefinition [Alignment]'),'2;1');
FILE_NAME('closing_segment.ifc','2026-10-02T00:00:00',(''),(''),'','','');
FILE_SCHEMA(('IFC4X3_ADD2'));
ENDSEC;
DATA;
#4=IFCCARTESIANPOINT((0.,0.,0.));
#5=IFCAXIS2PLACEMENT3D(#4,$,$);
#7=IFCGEOMETRICREPRESENTATIONCONTEXT($,'Model',3,1.E-6,#5,$);
#100=IFCCARTESIANPOINT((0.,0.));
#101=IFCALIGNMENTHORIZONTALSEGMENT($,$,#100,0.,0.,0.,100.,$,.LINE.);
#102=IFCCARTESIANPOINT((100.,0.));
#103=IFCALIGNMENTHORIZONTALSEGMENT($,$,#102,0.,100.,100.,{arc_length:?},$,.CIRCULARARC.);
#104=IFCCARTESIANPOINT((200.,100.));
{h_closing}
#111=IFCALIGNMENTSEGMENT('seg-h-0000000000000001',$,$,$,$,$,$,#101);
#113=IFCALIGNMENTSEGMENT('seg-h-0000000000000002',$,$,$,$,$,$,#103);
#115=IFCALIGNMENTSEGMENT('seg-h-0000000000000003',$,$,$,$,$,$,#105);
#121=IFCALIGNMENTHORIZONTAL('align-horiz-0000000001',$,$,$,$,$,$);
#122=IFCRELNESTS('nest-h-000000000000001',$,$,$,#121,{horizontal_nest});
#200=IFCALIGNMENTVERTICALSEGMENT($,$,0.,100.,50.,0.02,0.02,$,.CONSTANTGRADIENT.);
#201=IFCALIGNMENTVERTICALSEGMENT($,$,100.,{arc_length:?},52.,-0.01,-0.01,$,.CONSTANTGRADIENT.);
{v_closing}
#211=IFCALIGNMENTSEGMENT('seg-v-0000000000000001',$,$,$,$,$,$,#200);
#213=IFCALIGNMENTSEGMENT('seg-v-0000000000000002',$,$,$,$,$,$,#201);
#215=IFCALIGNMENTSEGMENT('seg-v-0000000000000003',$,$,$,$,$,$,#202);
#221=IFCALIGNMENTVERTICAL('align-vert-00000000001',$,$,$,$,$,$);
#222=IFCRELNESTS('nest-v-000000000000001',$,$,$,#221,{vertical_nest});
#300=IFCALIGNMENTCANTSEGMENT($,$,0.,100.,0.,0.05,0.,-0.05,.LINEARTRANSITION.);
#301=IFCALIGNMENTCANTSEGMENT($,$,100.,{arc_length:?},0.05,0.05,-0.05,-0.05,.CONSTANTCANT.);
{c_closing}
#311=IFCALIGNMENTSEGMENT('seg-c-0000000000000001',$,$,$,$,$,$,#300);
#313=IFCALIGNMENTSEGMENT('seg-c-0000000000000002',$,$,$,$,$,$,#301);
#315=IFCALIGNMENTSEGMENT('seg-c-0000000000000003',$,$,$,$,$,$,#302);
#321=IFCALIGNMENTCANT('align-cant-00000000001',$,$,$,$,$,$,1.5);
#322=IFCRELNESTS('nest-c-000000000000001',$,$,$,#321,{cant_nest});
#401=IFCALIGNMENT('align-root-00000000001',$,$,$,$,$,$,$);
#402=IFCRELNESTS('nest-a-000000000000001',$,$,$,#401,(#121,#221,#321));
#501=IFCCARTESIANPOINT((0.,0.,0.));
#502=IFCDIRECTION((1.,0.,0.));
#503=IFCVECTOR(#502,1.);
#504=IFCLINE(#501,#503);
#510=IFCPOINTBYDISTANCEEXPRESSION(IFCLENGTHMEASURE(0.),$,$,$,#504);
#513=IFCAXIS2PLACEMENTLINEAR(#510,$,$);
#514=IFCLINEARPLACEMENT($,#513,$);
#520=IFCREFERENT('referent-0000000000001',$,'STA 1+000',$,$,#514,$,.STATION.);
#521=IFCRELNESTS('nest-r-000000000000001',$,$,$,#401,(#520));
#531=IFCPROPERTYSINGLEVALUE('Station',$,IFCLENGTHMEASURE(1000.),$);
#533=IFCPROPERTYSET('pset-stationing-000001',$,'Pset_Stationing',$,(#531));
#534=IFCRELDEFINESBYPROPERTIES('rel-defprop-0000000001',$,$,$,(#520),#533);
ENDSEC;
END-ISO-10303-21;
",
            h_closing = self.h_closing,
            v_closing = self.v_closing,
            c_closing = self.c_closing,
            horizontal_nest = self.horizontal_nest,
            vertical_nest = self.vertical_nest,
            cant_nest = self.cant_nest,
        );
        StepCodec
            .read_bytes(text.as_bytes())
            .expect("fixture parses")
    }
}

fn misplaced(entity: EntityId) -> AlignmentError {
    AlignmentError::SemanticViolation {
        entity: Some(entity),
        rule: "a zero-length alignment segment is allowed only as the last segment of its layout",
    }
}

#[test]
fn a_closing_horizontal_segment_is_accepted_and_adds_no_geometry() {
    let model = Fixture::default().model();
    let cant = CantLayout::resolve(&model, CANT, metres()).expect("cant");

    let plan = lower_horizontal_plan(&model, HORIZONTAL, metres(), Some(&cant)).expect("plan");
    let axiolid_curve::Curve2::Intrinsic(curve) = &plan.curve else {
        panic!("one intrinsic plan curve");
    };
    assert!((curve.length - PLAN_LENGTH).abs() < 1e-12);
    // Two pieces: the closing segment adds none.
    let axiolid_curve::CurvatureLaw::Piecewise { breaks, laws } = &curve.curvature else {
        panic!("line then arc is piecewise");
    };
    assert_eq!((breaks.as_slice(), laws.len()), (&[100.0][..], 2));
    // Its seam is checked in closed form (after an arc) and reported last.
    assert_eq!(plan.seams.len(), 2);
    let closing = &plan.seams[1];
    assert_eq!(closing.next, H_CLOSING);
    assert_eq!(closing.position, SeamCheck::Verified);
    assert!((closing.distance_along - PLAN_LENGTH).abs() < 1e-12);
    assert_eq!(plan.sources.len(), 3);

    let strict =
        lower_horizontal_layout(&model, HORIZONTAL, metres(), Some(&cant)).expect("strict");
    assert_eq!(strict.seams.len(), 2);
    assert_eq!(strict.seams[1].next, H_CLOSING);
    let axiolid_model::GeometryNode::CurveRelation(axiolid_model::CurveRelation::Composite {
        segments,
    }) = strict.graph.get(strict.root).expect("root")
    else {
        panic!("a composite");
    };
    assert_eq!(
        segments.len(),
        2,
        "the closing segment has no composite piece"
    );

    let partial = lower_horizontal_layout_partial(&model, HORIZONTAL, metres(), Some(&cant))
        .expect("partial");
    assert!(partial.is_complete());
    assert_eq!(partial.runs.len(), 1);
    assert_eq!(partial.runs[0].seams.len(), 2);
    assert_eq!(partial.segment_count, 3);
}

#[test]
fn a_closing_vertical_segment_is_accepted_and_adds_no_piece() {
    let model = Fixture::default().model();
    let layout = VerticalLayout::resolve(&model, VERTICAL, metres()).expect("layout");
    assert_eq!(layout.segments().len(), 3);
    assert!((layout.length() - PLAN_LENGTH).abs() < 1e-12);
    // One seam between the two graded pieces: a grade break (#259). The
    // closing segment has none.
    assert_eq!(layout.seams().len(), 1);
    assert_eq!(layout.seams()[0].kind, VerticalSeamKind::GradeBreak);

    let law = vertical_profile_law(&model, VERTICAL, metres()).expect("profile");
    let ElevationLaw::Piecewise { breaks, laws } = &law else {
        panic!("two pieces");
    };
    assert_eq!((breaks.as_slice(), laws.len()), (&[100.0][..], 2));
    assert_eq!(laws[0].height_at(100.0), Some(52.0));
    assert_eq!(laws[1].height_at(0.0), Some(52.0));
    let end = law.height_at(PLAN_LENGTH).expect("end height");
    assert!((end - (52.0 - 0.5 * PI)).abs() < 1e-12, "end {end}");
}

#[test]
fn a_closing_cant_segment_is_accepted() {
    let model = Fixture::default().model();
    let cant = CantLayout::resolve(&model, CANT, metres()).expect("cant");
    assert_eq!(cant.segments().len(), 3);
    assert!((cant.length() - PLAN_LENGTH).abs() < 1e-12);
    let end = cant.cant_at_distance(PLAN_LENGTH).expect("cant at the end");
    assert_eq!((end.left, end.right), (0.05, -0.05));
}

#[test]
fn the_composed_alignment_stationing_and_hierarchy_accept_closing_segments() {
    let model = Fixture::default().model();
    let lowered = lower_gradient_curve(&model, ALIGNMENT, metres()).expect("gradient curve");
    assert_eq!(lowered.seams.len(), 2);
    let Curve3::Elevated(elevated) = gradient_curve3(&model, ALIGNMENT, metres()).expect("curve")
    else {
        panic!("an elevated curve");
    };
    assert_eq!(elevated.elevation.height_at(100.0), Some(52.0));
    assert_eq!(elevated.elevation.grade_at(100.0), Some(-0.01));

    let stationing = Stationing::resolve(&model, ALIGNMENT, metres()).expect("stationing");
    assert_eq!(stationing.station_at(PLAN_LENGTH), Ok(1000.0 + PLAN_LENGTH));

    let view = AlignmentView::for_model(&model).expect("view");
    let hierarchy = view.hierarchy(ALIGNMENT).expect("hierarchy");
    assert_eq!(
        (
            hierarchy.horizontal.as_slice(),
            hierarchy.vertical.as_slice()
        ),
        (&[HORIZONTAL][..], &[VERTICAL][..])
    );
    assert_eq!(view.layout_segments(HORIZONTAL).expect("segments").len(), 3);
}

/// A zero-length segment before the end of its layout is refused on every
/// path, naming it.
#[test]
fn a_misplaced_zero_length_segment_is_refused_on_every_path() {
    let model = Fixture {
        horizontal_nest: "(#111,#115,#113)",
        vertical_nest: "(#211,#215,#213)",
        cant_nest: "(#311,#315,#313)",
        ..Fixture::default()
    }
    .model();
    let h = Err(misplaced(H_CLOSING));
    assert_eq!(
        lower_horizontal_plan(&model, HORIZONTAL, metres(), None).map(|_| ()),
        h
    );
    assert_eq!(
        lower_horizontal_layout(&model, HORIZONTAL, metres(), None).map(|_| ()),
        h
    );
    assert_eq!(
        lower_horizontal_layout_partial(&model, HORIZONTAL, metres(), None).map(|_| ()),
        h
    );
    let v = Err(misplaced(V_CLOSING));
    assert_eq!(
        VerticalLayout::resolve(&model, VERTICAL, metres()).map(|_| ()),
        v
    );
    assert_eq!(
        vertical_profile_law(&model, VERTICAL, metres()).map(|_| ()),
        v
    );
    assert_eq!(
        CantLayout::resolve(&model, CANT, metres()).map(|_| ()),
        Err(misplaced(C_CLOSING))
    );
    assert!(matches!(
        lower_gradient_curve(&model, ALIGNMENT, metres()),
        Err(AlignmentError::SemanticViolation { .. })
    ));
}

/// The closing segment restates where its layout ends, and that seam is
/// checked like any other: a wrong point, height or cant is refused.
#[test]
fn a_closing_segment_that_does_not_meet_its_layout_end_is_refused() {
    let model = Fixture {
        h_closing: "#105=IFCALIGNMENTHORIZONTALSEGMENT($,$,#102,1.5707963267948966,0.,0.,0.,$,.LINE.);"
            .to_owned(),
        v_closing: format!(
            "#202=IFCALIGNMENTVERTICALSEGMENT($,$,{PLAN_LENGTH:?},0.,51.,-0.01,-0.01,$,.CONSTANTGRADIENT.);"
        ),
        c_closing: format!(
            "#302=IFCALIGNMENTCANTSEGMENT($,$,{PLAN_LENGTH:?},0.,0.,$,0.,$,.CONSTANTCANT.);"
        ),
        ..Fixture::default()
    }
    .model();
    assert!(matches!(
        lower_horizontal_plan(&model, HORIZONTAL, metres(), None),
        Err(AlignmentError::SemanticViolation { entity: Some(e), .. }) if e == H_CLOSING
    ));
    assert!(matches!(
        lower_horizontal_layout(&model, HORIZONTAL, metres(), None),
        Err(AlignmentError::SemanticViolation { entity: Some(e), .. }) if e == H_CLOSING
    ));
    assert!(matches!(
        vertical_profile_law(&model, VERTICAL, metres()),
        Err(AlignmentError::ProfileDiscontinuity { entity, .. }) if entity == V_CLOSING
    ));
    assert!(matches!(
        CantLayout::resolve(&model, CANT, metres()),
        Err(AlignmentError::SemanticViolation { entity: Some(e), .. }) if e == C_CLOSING
    ));
}

/// Lowered on its own, a zero-length segment has no geometry: refused.
#[test]
fn a_zero_length_segment_alone_has_no_geometry() {
    let model = Fixture::default().model();
    assert!(matches!(
        lower_horizontal_segment(&model, H_CLOSING, metres()),
        Err(AlignmentError::InvalidSegment { entity, .. }) if entity == H_CLOSING
    ));
    assert!(matches!(
        lower_vertical_segment(&model, V_CLOSING, metres()),
        Err(AlignmentError::InvalidSegment { entity, .. }) if entity == V_CLOSING
    ));
}

/// A layout of nothing but its closing segment states no geometry.
#[test]
fn a_layout_of_only_a_closing_segment_is_refused() {
    let model = Fixture {
        horizontal_nest: "(#115)",
        vertical_nest: "(#215)",
        cant_nest: "(#315)",
        ..Fixture::default()
    }
    .model();
    for result in [
        lower_horizontal_plan(&model, HORIZONTAL, metres(), None).map(|_| ()),
        VerticalLayout::resolve(&model, VERTICAL, metres()).map(|_| ()),
        vertical_profile_law(&model, VERTICAL, metres()).map(|_| ()),
        CantLayout::resolve(&model, CANT, metres()).map(|_| ()),
    ] {
        assert!(
            matches!(result, Err(AlignmentError::SemanticViolation { .. })),
            "{result:?}"
        );
    }
}
