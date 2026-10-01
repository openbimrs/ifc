//! The vertical seam tolerance follows the precision the file declares
//! (#141).
//!
//! The rounded-but-sound fixture is shaped like buildingSMART's alignment
//! reference datasets (IFC4.x-IF `BC003_*`, `Precision = 1E-4`): stations
//! and heights restated to four decimals, so each seam is off by rounding
//! alone. Values, computed by hand from the IFC definitions:
//!
//! - segment 1, CONSTANTGRADIENT from station 0, height 50, grade
//!   0.012345678 over 123.45678 m, ends at 51.524157652...;
//! - segment 2, PARABOLICARC restated at station 123.4568 (+2.2E-5) and
//!   height 51.5242 (+4.2E-5), grade 0.012345678 to -0.0087654321 over
//!   234.5678 m, ends at `51.5242 + 234.5678 * (g0 + g1) / 2 = 51.944105...`;
//! - segment 3, CONSTANTGRADIENT restated at station 358.0246 and height
//!   51.9441 (-5.2E-6).

use std::sync::Arc;

use ifc_alignment::{
    alignment, alignment_segment, horizontal_layout, horizontal_segment, lower_gradient_curve,
    profile_law, profile_law_within, read_vertical_segment, vertical_layout, vertical_profile_law,
    vertical_segment, AlignmentError, AlignmentUnits, HorizontalSegmentDraft, ProfileSeam,
    SeamTolerance, VerticalSegmentDraft,
};
use ifc_model::{Entity, EntityId, Model, Transaction, Value};

fn metres() -> AlignmentUnits {
    AlignmentUnits {
        length_to_metres: 1.0,
        angle_to_radians: 1.0,
    }
}

const G0: f64 = 0.012_345_678;
const G1: f64 = -0.008_765_432_1;

/// `(StartDistAlong, HorizontalLength, StartHeight, StartGradient,
/// EndGradient, PredefinedType)`.
type Row = (f64, f64, f64, f64, f64, &'static str);

const ROUNDED: [Row; 3] = [
    (0.0, 123.456_78, 50.0, G0, G0, "CONSTANTGRADIENT"),
    (123.4568, 234.5678, 51.5242, G0, G1, "PARABOLICARC"),
    (358.0246, 50.0, 51.9441, G1, G1, "CONSTANTGRADIENT"),
];

/// The same profile with a 5 mm step at the last seam.
const STEPPED: [Row; 3] = [
    ROUNDED[0],
    ROUNDED[1],
    (358.0246, 50.0, 51.9491, G1, G1, "CONSTANTGRADIENT"),
];

fn nest(tx: &mut Transaction, parent: EntityId, children: Vec<EntityId>) {
    tx.create(Entity::new(
        "IFCRELNESTS",
        vec![
            Value::Text("nest".into()),
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Ref(parent),
            Value::List(children.into_iter().map(Value::Ref).collect()),
        ],
    ));
}

/// An alignment over a straight plan, with the profile `rows` (lengths in
/// the unit `scale` metres) and a 3D context declaring `precision`.
/// Returns the model, the alignment and its vertical layout.
fn fixture(rows: &[Row], scale: f64, precision: Option<f64>) -> (Model, EntityId, EntityId) {
    let mut model = Model::default();
    model.header_mut().schema = vec!["IFC4X3_ADD2".to_owned()];
    let mut tx = Transaction::new(&model);
    tx.create(Entity::new(
        "IFCGEOMETRICREPRESENTATIONCONTEXT",
        vec![
            Value::Null,
            Value::Text(Arc::from("Model")),
            Value::Integer(3),
            precision.map_or(Value::Null, Value::Real),
            Value::Null,
            Value::Null,
        ],
    ));
    let mut n = 0;
    let mut guid = || {
        n += 1;
        format!("{n:0>22}")
    };

    let start = tx.create(Entity::new(
        "IFCCARTESIANPOINT",
        vec![Value::List(vec![Value::Real(0.0), Value::Real(0.0)])],
    ));
    let plan_length = rows.iter().map(|r| r.1).sum::<f64>() * scale;
    let h_params = horizontal_segment(
        &mut tx,
        &HorizontalSegmentDraft::new(start, 0.0, 0.0, 0.0, plan_length, "LINE"),
    )
    .expect("line");
    let h_seg = alignment_segment(&mut tx, &guid(), h_params).expect("h segment");
    let h = horizontal_layout(&mut tx, &guid(), Some("H")).expect("h");
    nest(&mut tx, h, vec![h_seg]);

    let mut segments = Vec::new();
    for (start, length, height, entry, exit, kind) in rows {
        let mut draft = VerticalSegmentDraft::new(
            start * scale,
            length * scale,
            height * scale,
            *entry,
            *exit,
            kind,
        );
        if *kind == "PARABOLICARC" {
            draft = draft.radius_of_curvature(length * scale / (exit - entry));
        }
        let params = vertical_segment(&mut tx, &draft).expect("vertical");
        segments.push(alignment_segment(&mut tx, &guid(), params).expect("v segment"));
    }
    let v = vertical_layout(&mut tx, &guid(), Some("V")).expect("v");
    nest(&mut tx, v, segments);

    let a = alignment(&mut tx, &guid(), Some("A"), None).expect("alignment");
    nest(&mut tx, a, vec![h, v]);
    tx.commit(&mut model).expect("commit");
    (model, a, v)
}

/// Rounded to the declared 1E-4 m: accepted, and the heights are the
/// authored ones (each piece starts where the file says it does).
#[test]
fn a_rounded_but_sound_profile_passes_at_the_declared_precision() {
    let (model, _, v) = fixture(&ROUNDED, 1.0, Some(1e-4));
    let law = vertical_profile_law(&model, v, metres()).expect("sound within 1E-4");
    assert_eq!(law.height_at(0.0), Some(50.0));
    // The second piece starts at its restated 51.5242, at the restated seam.
    let seam = law.height_at(123.4568).expect("height");
    assert!((seam - 51.5242).abs() < 1e-12, "seam height {seam}");
    let end = law.height_at(408.0246).expect("height");
    // 51.9441 + 50 * G1.
    assert!((end - (51.9441 + 50.0 * G1)).abs() < 1e-12, "end {end}");
}

/// The composed centreline takes the same tolerance from the model.
#[test]
fn the_gradient_curve_honours_the_declared_precision() {
    let (model, a, _) = fixture(&ROUNDED, 1.0, Some(1e-4));
    lower_gradient_curve(&model, a, metres()).expect("composes within 1E-4");
    let (strict, a, _) = fixture(&ROUNDED, 1.0, None);
    assert!(lower_gradient_curve(&strict, a, metres()).is_err());
}

/// Without a declared precision the old rounding-only rule applies, so the
/// same rounded file is refused: the tolerance comes from the file, not
/// from a guess.
#[test]
fn without_a_declared_precision_rounding_is_still_refused() {
    let (model, _, v) = fixture(&ROUNDED, 1.0, None);
    assert!(matches!(
        vertical_profile_law(&model, v, metres()),
        Err(AlignmentError::InvalidSegment { entity, .. }) if entity != v
    ));
    // And a finer declared precision than the rounding is refused too.
    let (finer, _, v) = fixture(&ROUNDED, 1.0, Some(1e-5));
    assert!(vertical_profile_law(&finer, v, metres()).is_err());
}

/// A genuine 5 mm step is refused at the declared precision, named as a
/// height discontinuity.
#[test]
fn a_genuinely_broken_profile_is_refused_at_the_declared_precision() {
    let (model, _, v) = fixture(&STEPPED, 1.0, Some(1e-4));
    let error = vertical_profile_law(&model, v, metres()).expect_err("5 mm step");
    let AlignmentError::ProfileDiscontinuity {
        seam,
        expected,
        actual,
        ..
    } = error
    else {
        panic!("expected a discontinuity, got {error}");
    };
    assert_eq!(seam, ProfileSeam::Height);
    assert_eq!(actual, 51.9491);
    assert!((expected - 51.944_105_202).abs() < 1e-8, "{expected}");
}

/// Even a file declaring a 10 mm precision cannot join a 5 mm step: the
/// honoured precision is capped at 1 mm.
#[test]
fn a_coarse_declared_precision_cannot_join_a_visible_step() {
    let (model, _, v) = fixture(&STEPPED, 1.0, Some(0.01));
    assert!(matches!(
        vertical_profile_law(&model, v, metres()),
        Err(AlignmentError::ProfileDiscontinuity {
            seam: ProfileSeam::Height,
            ..
        })
    ));
}

/// Unit-aware: the same profile authored in millimetres with
/// `Precision = 0.1` (mm) is accepted, and the 5 mm step still refused.
#[test]
fn the_precision_is_read_in_the_project_length_unit() {
    let millimetres = AlignmentUnits {
        length_to_metres: 0.001,
        angle_to_radians: 1.0,
    };
    let (model, _, v) = fixture(&ROUNDED, 1000.0, Some(0.1));
    vertical_profile_law(&model, v, millimetres).expect("sound within 0.1 mm");
    let (stepped, _, v) = fixture(&STEPPED, 1000.0, Some(0.1));
    assert!(vertical_profile_law(&stepped, v, millimetres).is_err());
}

/// The segment-level entry points: `profile_law` stays strict and
/// `profile_law_within` takes the tolerance explicitly.
#[test]
fn the_slice_entry_points_take_the_tolerance_explicitly() {
    let (model, _, _) = fixture(&ROUNDED, 1.0, Some(1e-4));
    let segments: Vec<_> = model
        .iter()
        .filter(|(_, e)| {
            e.type_name
                .eq_ignore_ascii_case("IFCALIGNMENTVERTICALSEGMENT")
        })
        .map(|(id, _)| read_vertical_segment(&model, id, metres()).expect("reads"))
        .collect();
    assert_eq!(segments.len(), 3);
    assert!(profile_law(&segments).is_err());
    let tolerance = SeamTolerance::from_precision(1e-4).expect("tolerance");
    profile_law_within(&segments, tolerance).expect("sound within 1E-4");
}

/// Runs the seam check over real IFC4X3 exports in a local directory.
///
/// The files are not committed (their licences are not established); fetch
/// them, for example, from buildingSMART/IFC4.x-IF (`tests/_archived/*` and
/// `IFC-files/*`) and run
/// `IFC_ALIGNMENT_EXPORTS=<dir> cargo test -p ifc-alignment --test
/// seam_tolerance -- --ignored --nocapture`. Every file is assumed to use
/// metres. The test prints per layout whether the strict rule and the
/// declared-precision rule accept it, and never fails on a refusal: it is a
/// survey, not a gate.
#[test]
#[ignore = "needs IFC_ALIGNMENT_EXPORTS pointing at local real exports"]
fn survey_real_exports() {
    use ifc_model::Codec;
    let Ok(dir) = std::env::var("IFC_ALIGNMENT_EXPORTS") else {
        eprintln!("IFC_ALIGNMENT_EXPORTS is not set");
        return;
    };
    let mut paths: Vec<_> = std::fs::read_dir(&dir)
        .expect("readable directory")
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| path.extension().is_some_and(|ext| ext == "ifc"))
        .collect();
    paths.sort();
    for path in paths {
        let model = match ifc_step::StepCodec.read_path(&path) {
            Ok(model) => model,
            Err(error) => {
                eprintln!("{}: unreadable ({error})", path.display());
                continue;
            }
        };
        let declared = SeamTolerance::for_model(&model, metres());
        for (id, _) in model
            .iter()
            .filter(|(_, e)| e.type_name.eq_ignore_ascii_case("IFCALIGNMENTVERTICAL"))
        {
            let strict = strict_profile(&model, id);
            let within = vertical_profile_law(&model, id, metres());
            eprintln!(
                "{} #{}: precision {:?}; strict: {}; declared: {}",
                path.file_name().unwrap_or_default().to_string_lossy(),
                id.0,
                declared.as_ref().map(|t| t.length()),
                verdict(&strict),
                verdict(&within),
            );
        }
    }
}

/// The profile of `layout` under the strict rule, walking `IfcRelNests` by
/// hand so the survey compares exactly one variable.
fn strict_profile(
    model: &Model,
    layout: EntityId,
) -> Result<axiolid_curve::ElevationLaw, AlignmentError> {
    let mut segments = Vec::new();
    for (_, nest) in model
        .iter()
        .filter(|(_, e)| e.type_name.eq_ignore_ascii_case("IFCRELNESTS"))
        .filter(|(_, e)| e.attributes.get(4) == Some(&Value::Ref(layout)))
    {
        for child in nest.attributes[5].as_list().unwrap_or_default() {
            let Some(wrapper) = child.as_ref_id().and_then(|id| model.get(id)) else {
                continue;
            };
            if let Some(params) = wrapper.attributes.last().and_then(Value::as_ref_id) {
                segments.push(read_vertical_segment(model, params, metres())?);
            }
        }
    }
    profile_law(&segments)
}

fn verdict<T>(result: &Result<T, AlignmentError>) -> String {
    match result {
        Ok(_) => "accepted".to_owned(),
        Err(error) => format!("refused ({error})"),
    }
}
