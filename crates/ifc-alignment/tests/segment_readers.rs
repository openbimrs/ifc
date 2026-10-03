//! `read_horizontal_segment` and `read_vertical_segment`, driven directly.
//!
//! Other tests reach the readers through lowering, which only ever feeds
//! them well-formed segments, or build `VerticalSegment` values by hand,
//! which skips the reader entirely. Here every fixture is an entity record,
//! mostly parsed from STEP text, so each accepted kind and each refusal is
//! proved on the path a file actually takes.
//!
//! Slot positions follow `IFC4X3_ADD2.exp`: `IfcAlignmentParameterSegment`
//! contributes `StartTag`/`EndTag` at 0..1, then
//! `IfcAlignmentHorizontalSegment` declares `StartPoint`, `StartDirection`,
//! `StartRadiusOfCurvature`, `EndRadiusOfCurvature`, `SegmentLength`,
//! `GravityCenterLineHeight`, `PredefinedType` (2..8), and
//! `IfcAlignmentVerticalSegment` declares `StartDistAlong`,
//! `HorizontalLength`, `StartHeight`, `StartGradient`, `EndGradient`,
//! `RadiusOfCurvature`, `PredefinedType` (2..8).

use std::f64::consts::PI;
use std::sync::Arc;

use axiolid_curve::ElevationLaw;
use ifc_alignment::{
    elevation_law, read_horizontal_segment, read_vertical_segment, AlignmentError, AlignmentUnits,
    HorizontalSegmentType, VerticalSegmentType,
};
use ifc_model::codec::Codec;
use ifc_model::{Entity, EntityId, Model, Value};
use ifc_step::StepCodec;

/// Millimetres and degrees, so every read proves where units are applied
/// and, just as important, where they are not (gradients are ratios).
fn millimetres_and_degrees() -> AlignmentUnits {
    AlignmentUnits {
        length_to_metres: 0.001,
        angle_to_radians: PI / 180.0,
    }
}

fn metres() -> AlignmentUnits {
    AlignmentUnits {
        length_to_metres: 1.0,
        angle_to_radians: 1.0,
    }
}

const FIXTURE: &str = "ISO-10303-21;
HEADER;
FILE_DESCRIPTION(('ViewDefinition [Alignment]'),'2;1');
FILE_NAME('segment_readers.ifc','2026-09-26T00:00:00',(''),(''),'','','');
FILE_SCHEMA(('IFC4X3_ADD2'));
ENDSEC;
DATA;
#1=IFCCARTESIANPOINT((1000.,2000.));
#2=IFCALIGNMENTHORIZONTALSEGMENT($,$,#1,90.,250000.,250000.,100000.,$,.CIRCULARARC.);
#3=IFCALIGNMENTHORIZONTALSEGMENT($,$,#1,0.,0.,300000.,50000.,1500.,.CLOTHOID.);
#10=IFCALIGNMENTVERTICALSEGMENT($,$,1100000.,200000.,52000.,0.02,-0.03,4000000.,.PARABOLICARC.);
#11=IFCALIGNMENTVERTICALSEGMENT($,$,0.,50000.,100000.,-0.01,0.01,5000000.,.CIRCULARARC.);
#12=IFCALIGNMENTVERTICALSEGMENT($,$,0.,200000.,52000.,0.02,-0.03,$,.PARABOLICARC.);
#13=IFCALIGNMENTVERTICALSEGMENT($,$,0.,50000.,100000.,-0.01,0.01,$,.CIRCULARARC.);
#14=IFCALIGNMENTVERTICALSEGMENT($,$,0.,100000.,50000.,0.02,0.02,1000000.,.CONSTANTGRADIENT.);
#15=IFCALIGNMENTVERTICALSEGMENT($,$,0.,-1.,50000.,0.02,0.02,$,.CONSTANTGRADIENT.);
#16=IFCALIGNMENTVERTICALSEGMENT($,$,0.,100.,50.,'steep',0.02,$,.CONSTANTGRADIENT.);
#17=IFCALIGNMENTVERTICALSEGMENT($,$,0.,100.,50.,0.02,0.02,$,'CONSTANTGRADIENT');
#20=IFCDIRECTION((1.,0.));
#21=IFCALIGNMENTHORIZONTALSEGMENT($,$,#20,0.,0.,0.,100.,$,.LINE.);
#22=IFCCARTESIANPOINT((5.));
#23=IFCALIGNMENTHORIZONTALSEGMENT($,$,#22,0.,0.,0.,100.,$,.LINE.);
#24=IFCALIGNMENTHORIZONTALSEGMENT($,$,#1,0.,0.,0.,0.,$,.LINE.);
#25=IFCALIGNMENTHORIZONTALSEGMENT($,$,#1,0.,0.,0.,-5.,$,.LINE.);
#26=IFCALIGNMENTHORIZONTALSEGMENT($,$,$,0.,0.,0.,100.,$,.LINE.);
#27=IFCALIGNMENTHORIZONTALSEGMENT($,$,#1,0.,0.,0.,100.,$,$);
#28=IFCALIGNMENTHORIZONTALSEGMENT($,$,#1,0.,0.,0.,100.,'high',.LINE.);
#29=IFCALIGNMENTHORIZONTALSEGMENT($,$,1.5,0.,0.,0.,100.,$,.LINE.);
#30=IFCALIGNMENTHORIZONTALSEGMENT($,$,#1,$,0.,0.,100.,$,.LINE.);
ENDSEC;
END-ISO-10303-21;
";

fn fixture() -> Model {
    StepCodec
        .read_bytes(FIXTURE.as_bytes())
        .expect("fixture parses")
}

/// A horizontal segment with one attribute replaced, for values STEP text
/// cannot carry (non-finite reals).
fn horizontal_with(slot: usize, value: Value) -> Model {
    let mut model = Model::new();
    model.insert(
        EntityId(1),
        Entity::new(
            "IFCCARTESIANPOINT",
            vec![Value::List(vec![Value::Real(0.0), Value::Real(0.0)])],
        ),
    );
    let mut attributes = vec![
        Value::Null,
        Value::Null,
        Value::Ref(EntityId(1)),
        Value::Real(0.0),
        Value::Real(0.0),
        Value::Real(0.0),
        Value::Real(100.0),
        Value::Null,
        Value::Enum(Arc::from("LINE")),
    ];
    attributes[slot] = value;
    model.insert(
        EntityId(2),
        Entity::new("IFCALIGNMENTHORIZONTALSEGMENT", attributes),
    );
    model
}

/// A vertical segment with one attribute replaced.
fn vertical_with(slot: usize, value: Value) -> Model {
    let mut attributes = vec![
        Value::Null,
        Value::Null,
        Value::Real(0.0),
        Value::Real(100.0),
        Value::Real(50.0),
        Value::Real(0.02),
        Value::Real(-0.03),
        Value::Real(2000.0),
        Value::Enum(Arc::from("PARABOLICARC")),
    ];
    attributes[slot] = value;
    let mut model = Model::new();
    model.insert(
        EntityId(1),
        Entity::new("IFCALIGNMENTVERTICALSEGMENT", attributes),
    );
    model
}

fn invalid_segment(error: &AlignmentError, id: u64) -> bool {
    matches!(error, AlignmentError::InvalidSegment { entity, .. } if *entity == EntityId(id))
}

// ---------------------------------------------------------------- horizontal

/// A circular arc read from a file: point, bearing, radii and length each
/// scaled by the matching unit factor exactly once.
#[test]
fn reads_a_circular_arc_horizontal_segment_in_file_units() {
    let segment =
        read_horizontal_segment(&fixture(), EntityId(2), millimetres_and_degrees()).expect("arc");
    assert_eq!(segment.segment_type, HorizontalSegmentType::CircularArc);
    assert_eq!(segment.start_point.x, 1.0);
    assert_eq!(segment.start_point.y, 2.0);
    assert!((segment.start_direction - PI / 2.0).abs() < 1e-15);
    assert_eq!(segment.start_radius, 250.0);
    assert_eq!(segment.end_radius, 250.0);
    assert_eq!(segment.segment_length, 100.0);
    assert_eq!(segment.gravity_center_line_height, None);
}

/// A transition token is preserved verbatim, and the optional gravity
/// centre line height is read and scaled when present.
#[test]
fn reads_a_transition_token_verbatim_with_its_gravity_height() {
    let segment = read_horizontal_segment(&fixture(), EntityId(3), millimetres_and_degrees())
        .expect("clothoid");
    assert_eq!(
        segment.segment_type,
        HorizontalSegmentType::Transition("CLOTHOID".to_owned())
    );
    assert_eq!(segment.end_radius, 300.0);
    assert_eq!(segment.gravity_center_line_height, Some(1.5));
}

#[test]
fn refuses_a_horizontal_id_that_is_not_a_horizontal_segment() {
    let model = fixture();
    let error = read_horizontal_segment(&model, EntityId(10), metres()).expect_err("vertical");
    assert!(matches!(
        error,
        AlignmentError::WrongType { entity, expected, ref actual }
            if entity == EntityId(10)
                && expected == "IFCALIGNMENTHORIZONTALSEGMENT"
                && actual == "IFCALIGNMENTVERTICALSEGMENT"
    ));
    assert_eq!(
        read_horizontal_segment(&model, EntityId(999), metres()),
        Err(AlignmentError::MissingEntity {
            entity: EntityId(999)
        })
    );
}

/// `StartPoint` must be an `IfcCartesianPoint`; the error names the point,
/// not the segment, because that is the record to fix.
#[test]
fn refuses_a_start_point_that_is_not_a_cartesian_point() {
    let error = read_horizontal_segment(&fixture(), EntityId(21), metres()).expect_err("direction");
    assert!(matches!(
        error,
        AlignmentError::WrongType { entity, expected, .. }
            if entity == EntityId(20) && expected == "IFCCARTESIANPOINT"
    ));
}

#[test]
fn refuses_a_start_point_with_fewer_than_two_coordinates() {
    assert_eq!(
        read_horizontal_segment(&fixture(), EntityId(23), metres()),
        Err(AlignmentError::InvalidAttribute {
            entity: EntityId(22),
            index: 0,
            name: "Coordinates",
        })
    );
}

#[test]
fn refuses_a_missing_or_non_reference_start_point() {
    let model = fixture();
    assert_eq!(
        read_horizontal_segment(&model, EntityId(26), metres()),
        Err(AlignmentError::MissingAttribute {
            entity: EntityId(26),
            index: 2,
            name: "StartPoint",
        })
    );
    assert_eq!(
        read_horizontal_segment(&model, EntityId(29), metres()),
        Err(AlignmentError::InvalidAttribute {
            entity: EntityId(29),
            index: 2,
            name: "StartPoint",
        })
    );
    let dangling = horizontal_with(2, Value::Ref(EntityId(404)));
    assert_eq!(
        read_horizontal_segment(&dangling, EntityId(2), metres()),
        Err(AlignmentError::MissingEntity {
            entity: EntityId(404)
        })
    );
}

/// `SegmentLength` is typed `IfcNonNegativeLengthMeasure`, and IFC4.3
/// closes every layout with a zero-length segment (#262), so the reader
/// takes zero and leaves where it may stand to the layout paths. A negative
/// length is still refused.
#[test]
fn reads_a_zero_segment_length_and_refuses_a_negative_one() {
    let model = fixture();
    let zero = read_horizontal_segment(&model, EntityId(24), metres()).expect("zero length");
    assert_eq!(zero.segment_length, 0.0);
    let error = read_horizontal_segment(&model, EntityId(25), metres()).expect_err("negative");
    assert!(invalid_segment(&error, 25), "#25: {error}");
}

#[test]
fn refuses_missing_or_mistyped_horizontal_parameters() {
    let model = fixture();
    assert_eq!(
        read_horizontal_segment(&model, EntityId(27), metres()),
        Err(AlignmentError::MissingAttribute {
            entity: EntityId(27),
            index: 8,
            name: "PredefinedType",
        })
    );
    assert_eq!(
        read_horizontal_segment(&model, EntityId(28), metres()),
        Err(AlignmentError::InvalidAttribute {
            entity: EntityId(28),
            index: 7,
            name: "GravityCenterLineHeight",
        })
    );
    assert_eq!(
        read_horizontal_segment(&model, EntityId(30), metres()),
        Err(AlignmentError::MissingAttribute {
            entity: EntityId(30),
            index: 3,
            name: "StartDirection",
        })
    );
}

/// STEP text cannot spell NaN or infinity, but an in-memory model or a
/// unit factor can produce one; each parameter slot is checked.
#[test]
fn refuses_non_finite_horizontal_values() {
    for (slot, value) in [
        (3, f64::NAN),
        (4, f64::INFINITY),
        (5, f64::NEG_INFINITY),
        (6, f64::INFINITY),
        (7, f64::NAN),
    ] {
        let model = horizontal_with(slot, Value::Real(value));
        let error = read_horizontal_segment(&model, EntityId(2), metres())
            .expect_err("non-finite parameter");
        assert!(invalid_segment(&error, 2), "slot {slot}: {error}");
    }
    let mut model = horizontal_with(3, Value::Real(0.0));
    model.insert(
        EntityId(1),
        Entity::new(
            "IFCCARTESIANPOINT",
            vec![Value::List(vec![Value::Real(f64::NAN), Value::Real(0.0)])],
        ),
    );
    let error = read_horizontal_segment(&model, EntityId(2), metres()).expect_err("NaN point");
    assert!(invalid_segment(&error, 2), "{error}");
}

#[test]
fn refuses_unusable_horizontal_units() {
    let model = fixture();
    for units in [
        AlignmentUnits {
            length_to_metres: 0.0,
            angle_to_radians: 1.0,
        },
        AlignmentUnits {
            length_to_metres: 1.0,
            angle_to_radians: f64::NAN,
        },
    ] {
        assert!(matches!(
            read_horizontal_segment(&model, EntityId(2), units),
            Err(AlignmentError::InvalidUnits { .. })
        ));
    }
}

// ------------------------------------------------------------------ vertical

/// A parabolic arc read from a file. Lengths are scaled; gradients are
/// ratios and must not be. The read segment then lowers to the IFC
/// parabola, checked at hand-computed heights:
/// `z(d) = 52 + 0.02 d - 0.05 / 400 d^2` gives 52.75 at 100 m and 51.0 at
/// 200 m.
#[test]
fn reads_a_parabolic_arc_vertical_segment_and_lowers_it() {
    let segment = read_vertical_segment(&fixture(), EntityId(10), millimetres_and_degrees())
        .expect("parabola");
    assert_eq!(segment.predefined_type, VerticalSegmentType::ParabolicArc);
    assert_eq!(segment.start_dist_along, 1100.0);
    assert_eq!(segment.horizontal_length, 200.0);
    assert_eq!(segment.start_height, 52.0);
    assert_eq!(segment.start_gradient, 0.02);
    assert_eq!(segment.end_gradient, -0.03);
    assert_eq!(segment.radius_of_curvature, Some(4000.0));

    let law = elevation_law(&segment).expect("exact parabola");
    assert_eq!(law.height_at(0.0), Some(52.0));
    assert_eq!(law.height_at(100.0), Some(52.75));
    assert_eq!(law.height_at(200.0), Some(51.0));
}

/// A circular vertical arc is read faithfully -- the reader does not judge
/// lowerability -- and `elevation_law` lowers it to the circle itself
/// (#258): start height, start grade and the signed radius.
#[test]
fn reads_a_circular_arc_vertical_segment_and_lowers_it_to_the_circle() {
    let segment = read_vertical_segment(&fixture(), EntityId(11), millimetres_and_degrees())
        .expect("circular arc");
    assert_eq!(segment.predefined_type, VerticalSegmentType::CircularArc);
    assert_eq!(segment.horizontal_length, 50.0);
    assert_eq!(segment.start_height, 100.0);
    assert_eq!(segment.start_gradient, -0.01);
    assert_eq!(segment.end_gradient, 0.01);
    assert_eq!(segment.radius_of_curvature, Some(5000.0));
    assert_eq!(
        elevation_law(&segment),
        Ok(ElevationLaw::CircularArc {
            height: 100.0,
            grade: -0.01,
            radius: 5000.0,
        })
    );
}

/// `RadiusOfCurvature` is required exactly for the two arc families and
/// forbidden otherwise; each direction of that rule is read from a record.
#[test]
fn refuses_a_radius_inconsistent_with_the_vertical_family() {
    let model = fixture();
    for id in [12, 13, 14] {
        let error = read_vertical_segment(&model, EntityId(id), metres()).expect_err("radius rule");
        assert!(
            matches!(&error, AlignmentError::InvalidSegment { entity, detail }
                if *entity == EntityId(id) && detail.contains("radius")),
            "#{id}: {error}"
        );
    }
}

#[test]
fn refuses_a_negative_vertical_horizontal_length() {
    let error = read_vertical_segment(&fixture(), EntityId(15), metres()).expect_err("negative");
    assert!(invalid_segment(&error, 15), "{error}");
}

#[test]
fn refuses_mistyped_vertical_parameters() {
    let model = fixture();
    assert_eq!(
        read_vertical_segment(&model, EntityId(16), metres()),
        Err(AlignmentError::InvalidAttribute {
            entity: EntityId(16),
            index: 5,
            name: "StartGradient",
        })
    );
    assert_eq!(
        read_vertical_segment(&model, EntityId(17), metres()),
        Err(AlignmentError::InvalidAttribute {
            entity: EntityId(17),
            index: 8,
            name: "PredefinedType",
        })
    );
}

#[test]
fn refuses_a_vertical_id_that_is_not_a_vertical_segment() {
    let model = fixture();
    let error = read_vertical_segment(&model, EntityId(2), metres()).expect_err("horizontal");
    assert!(matches!(
        error,
        AlignmentError::WrongType { entity, expected, .. }
            if entity == EntityId(2) && expected == "IFCALIGNMENTVERTICALSEGMENT"
    ));
    assert_eq!(
        read_vertical_segment(&model, EntityId(999), metres()),
        Err(AlignmentError::MissingEntity {
            entity: EntityId(999)
        })
    );
}

#[test]
fn refuses_non_finite_vertical_values() {
    for slot in 2..=7 {
        let model = vertical_with(slot, Value::Real(f64::NAN));
        let error = read_vertical_segment(&model, EntityId(1), metres()).expect_err("NaN");
        assert!(invalid_segment(&error, 1), "slot {slot}: {error}");
    }
}

#[test]
fn refuses_an_unusable_vertical_length_unit() {
    let model = vertical_with(2, Value::Real(0.0));
    assert!(matches!(
        read_vertical_segment(
            &model,
            EntityId(1),
            AlignmentUnits {
                length_to_metres: -1.0,
                angle_to_radians: 1.0,
            }
        ),
        Err(AlignmentError::InvalidUnits { .. })
    ));
}
