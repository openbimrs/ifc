//! Vertical profile seams: a step in height is refused, a grade break with
//! continuous height is accepted (#259).
//!
//! Each segment restates its own `StartHeight` and `StartGradient`. When the
//! height disagrees with where the previous segment ends, joining the pieces
//! would shift every downstream height while the profile still looks well
//! formed. A change of grade at a height-continuous seam is legal IFC4.3
//! ADD2 (`IfcAlignmentVerticalSegment`: "The transition at the segment
//! connection is not enforced to be tangential") and is carried exactly by
//! the piecewise law. Fixtures are entity records read through
//! `read_vertical_segment`, so the check is proved on the path a file takes.

use std::sync::Arc;

use axiolid_curve::ElevationLaw;
use ifc_alignment::{
    profile_law, read_vertical_segment, AlignmentError, AlignmentUnits, ProfileSeam,
    VerticalSegment,
};
use ifc_model::{Entity, EntityId, Model, Value};

fn metres() -> AlignmentUnits {
    AlignmentUnits {
        length_to_metres: 1.0,
        angle_to_radians: 1.0,
    }
}

/// `(StartDistAlong, HorizontalLength, StartHeight, StartGradient,
/// EndGradient, RadiusOfCurvature, PredefinedType)`, slots 2..8 of
/// `IfcAlignmentVerticalSegment` in `IFC4X3_ADD2.exp`.
type Row = (f64, f64, f64, f64, f64, Option<f64>, &'static str);

/// Insert the rows as vertical segment records and read them back.
fn read_profile(rows: &[Row]) -> Vec<VerticalSegment> {
    let mut model = Model::new();
    for (index, (start, length, height, entry, exit, radius, kind)) in rows.iter().enumerate() {
        model.insert(
            EntityId(index as u64 + 1),
            Entity::new(
                "IFCALIGNMENTVERTICALSEGMENT",
                vec![
                    Value::Null,
                    Value::Null,
                    Value::Real(*start),
                    Value::Real(*length),
                    Value::Real(*height),
                    Value::Real(*entry),
                    Value::Real(*exit),
                    radius.map_or(Value::Null, Value::Real),
                    Value::Enum(Arc::from(*kind)),
                ],
            ),
        );
    }
    (1..=rows.len() as u64)
        .map(|id| read_vertical_segment(&model, EntityId(id), metres()).expect("reads"))
        .collect()
}

/// 2% from 50.0 over 100 m ends at 52.0 with grade 0.02; a 200 m crest from
/// +2% to -3% then starts exactly there.
const GRADE: Row = (1000.0, 100.0, 50.0, 0.02, 0.02, None, "CONSTANTGRADIENT");
const CREST: Row = (
    1100.0,
    200.0,
    52.0,
    0.02,
    -0.03,
    Some(4000.0),
    "PARABOLICARC",
);

#[test]
fn a_matched_seam_is_accepted() {
    let law = profile_law(&read_profile(&[GRADE, CREST])).expect("continuous");
    assert_eq!(law.height_at(100.0), Some(52.0));
    assert_eq!(law.height_at(200.0), Some(52.75));
}

/// The seam after a parabola: its end height is 52 + (0.02 - 0.03) / 2 *
/// 200 = 51.0 and its end grade -0.03, both of which the next grade meets.
#[test]
fn a_matched_seam_after_a_parabola_is_accepted() {
    let tail = (1300.0, 50.0, 51.0, -0.03, -0.03, None, "CONSTANTGRADIENT");
    let law = profile_law(&read_profile(&[GRADE, CREST, tail])).expect("continuous");
    let end = law.height_at(350.0).expect("height");
    assert!((end - 49.5).abs() < 1e-12, "profile end was {end}");
}

/// Realistic magnitudes: a seam computed in floating point (312.456 +
/// 0.0123 * 87.65) differs from the authored decimal by rounding only, and
/// the magnitude-scaled tolerance accepts it.
#[test]
fn a_seam_equal_up_to_rounding_is_accepted() {
    let first = (
        23_456.789,
        87.65,
        312.456,
        0.0123,
        0.0123,
        None,
        "CONSTANTGRADIENT",
    );
    let second = (
        23_544.439,
        40.0,
        313.534_095,
        0.0123,
        0.0123,
        None,
        "CONSTANTGRADIENT",
    );
    profile_law(&read_profile(&[first, second])).expect("equal up to rounding");
}

/// A 10 mm step at the seam is refused, naming both segments and both
/// heights.
#[test]
fn a_height_step_is_refused() {
    let stepped = (
        1100.0,
        200.0,
        52.01,
        0.02,
        -0.03,
        Some(4000.0),
        "PARABOLICARC",
    );
    let error = profile_law(&read_profile(&[GRADE, stepped])).expect_err("step");
    let AlignmentError::ProfileDiscontinuity {
        entity,
        previous,
        seam,
        expected,
        actual,
    } = error
    else {
        panic!("expected a typed discontinuity, got {error}");
    };
    assert_eq!(entity, EntityId(2));
    assert_eq!(previous, EntityId(1));
    assert_eq!(seam, ProfileSeam::Height);
    assert!((expected - 52.0).abs() < 1e-12, "expected {expected}");
    assert_eq!(actual, 52.01);
}

/// Even a millimetre step at a 52 m height lies far outside the rounding
/// tolerance: the check is not a loose "close enough".
#[test]
fn a_millimetre_step_is_refused() {
    let stepped = (
        1100.0,
        200.0,
        52.001,
        0.02,
        -0.03,
        Some(4000.0),
        "PARABOLICARC",
    );
    assert!(matches!(
        profile_law(&read_profile(&[GRADE, stepped])),
        Err(AlignmentError::ProfileDiscontinuity {
            seam: ProfileSeam::Height,
            ..
        })
    ));
}

/// The two pieces either side of the seam at plan distance `at`.
fn pieces(law: &ElevationLaw, at: f64) -> (&ElevationLaw, &ElevationLaw, f64) {
    let ElevationLaw::Piecewise { breaks, laws } = law else {
        panic!("a multi-segment profile is piecewise: {law:?}");
    };
    let index = breaks.iter().position(|b| *b == at).expect("seam");
    let start = if index == 0 { 0.0 } else { breaks[index - 1] };
    (&laws[index], &laws[index + 1], at - start)
}

/// Height agrees but the grade changes from 2% to 3%: a grade break. It is
/// accepted, and both sides keep their own grade at one exact height.
#[test]
fn a_grade_break_with_continuous_height_is_accepted() {
    let broken = (1100.0, 100.0, 52.0, 0.03, 0.03, None, "CONSTANTGRADIENT");
    let law = profile_law(&read_profile(&[GRADE, broken])).expect("grade break");
    let (before, after, local_end) = pieces(&law, 100.0);
    assert_eq!(before.height_at(local_end), Some(52.0));
    assert_eq!(after.height_at(0.0), Some(52.0));
    assert_eq!(before.grade_at(local_end), Some(0.02));
    assert_eq!(after.grade_at(0.0), Some(0.03));
    // Through the whole law: the seam belongs to the piece starting there.
    assert_eq!(law.height_at(100.0), Some(52.0));
    assert_eq!(law.grade_at(100.0), Some(0.03));
    assert_eq!(law.height_at(200.0), Some(55.0));
}

/// A grade break after a parabola: the next piece starts at the parabola's
/// end height (51.0) but at +2%, not its exit grade of -3%.
#[test]
fn a_grade_break_after_a_parabola_is_accepted() {
    let rising = (1300.0, 50.0, 51.0, 0.02, 0.02, None, "CONSTANTGRADIENT");
    let law = profile_law(&read_profile(&[GRADE, CREST, rising])).expect("grade break");
    let (before, after, local_end) = pieces(&law, 300.0);
    assert_eq!(before.height_at(local_end), Some(51.0));
    assert_eq!(after.height_at(0.0), Some(51.0));
    let exit = before.grade_at(local_end).expect("grade");
    assert!((exit + 0.03).abs() < 1e-15, "parabola exit grade {exit}");
    assert_eq!(after.grade_at(0.0), Some(0.02));
    assert_eq!(law.height_at(350.0), Some(52.0));
}

/// A grade break does not excuse a step: height is still checked at the
/// same seam.
#[test]
fn a_grade_break_with_a_height_step_is_refused() {
    let stepped = (1100.0, 100.0, 52.5, 0.03, 0.03, None, "CONSTANTGRADIENT");
    assert_eq!(
        profile_law(&read_profile(&[GRADE, stepped])),
        Err(AlignmentError::ProfileDiscontinuity {
            entity: EntityId(2),
            previous: EntityId(1),
            seam: ProfileSeam::Height,
            expected: 52.0,
            actual: 52.5,
        })
    );
}
