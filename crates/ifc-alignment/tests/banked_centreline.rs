//! Every cant segment type lowered onto the banked centreline (#93), its
//! seams, and the refusals that remain.
//!
//! Expected cant values are written here from the base formulas
//! `IfcAlignmentCantSegmentTypeEnum` (IFC4X3_ADD2, 8.7.2.1) states, in the
//! whole segment's `xi = s / L`, independently of the piece constructors the
//! lowering uses (the Helmert transition in particular is one formula per
//! half of the WHOLE segment there, two pieces in the lowering):
//!
//! - CONSTANTCANT `D = D1`
//! - LINEARTRANSITION `D1 + xi dD`
//! - BLOSSCURVE `D1 + (3 - 2 xi) xi^2 dD`
//! - HELMERTCURVE `D1 + 2 xi^2 dD` up to `xi = 1/2`, then
//!   `D1 + (1 - 2 (1 - xi)^2) dD`
//! - COSINECURVE `D1 + (1 - cos(pi xi)) / 2 dD`
//! - SINECURVE `D1 + (xi - sin(2 pi xi) / (2 pi)) dD`
//! - VIENNESEBEND `psi = psi1 + dpsi xi^4 (35 - 84 xi + 70 xi^2 - 20 xi^3)`,
//!   `psi = arcsin(D / b)`.

use std::f64::consts::PI;

use axiolid_curve::{BankConvention, Banked3, CantForm, Curve3};
use ifc_alignment::{
    alignment, alignment_segment, cant_layout, cant_segment, gradient_curve3, horizontal_layout,
    horizontal_segment, lower_segmented_reference_curve, segmented_reference_curve3,
    vertical_layout, vertical_segment, AlignmentError, AlignmentUnits, CantLayout,
    CantSegmentDraft, HorizontalSegmentDraft, VerticalSegmentDraft,
};
use ifc_model::{Entity, EntityId, Model, Transaction, Value};

const B: f64 = 1.5;

fn metres() -> AlignmentUnits {
    AlignmentUnits {
        length_to_metres: 1.0,
        angle_to_radians: 1.0,
    }
}

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

/// One cant segment: (start, length, left start, left end, right start,
/// right end, type). `None` ends are omitted (a constant cant).
type Cant = (f64, f64, f64, Option<f64>, f64, Option<f64>, &'static str);

/// A 100 m straight along +x on a 2% grade from 10 m, with `cant` as its
/// cant layout (the closing zero-length segment added after it).
fn track(cant: &[Cant]) -> (Model, EntityId, Vec<EntityId>) {
    let mut model = Model::default();
    model.header_mut().schema = vec!["IFC4X3_ADD2".to_owned()];
    let mut tx = Transaction::new(&model);
    let mut n = 0;
    let mut guid = || {
        n += 1;
        format!("{n:0>22}")
    };
    let start = tx.create(Entity::new(
        "IFCCARTESIANPOINT",
        vec![Value::List(vec![Value::Real(0.0), Value::Real(0.0)])],
    ));
    let line = horizontal_segment(
        &mut tx,
        &HorizontalSegmentDraft::new(start, 0.0, 0.0, 0.0, 100.0, "LINE"),
    )
    .expect("line");
    let line = alignment_segment(&mut tx, &guid(), line).expect("segment");
    let h = horizontal_layout(&mut tx, &guid(), Some("H")).expect("h");
    nest(&mut tx, h, vec![line]);

    let grade = vertical_segment(
        &mut tx,
        &VerticalSegmentDraft::new(0.0, 100.0, 10.0, 0.02, 0.02, "CONSTANTGRADIENT"),
    )
    .expect("grade");
    let grade = alignment_segment(&mut tx, &guid(), grade).expect("segment");
    let v = vertical_layout(&mut tx, &guid(), Some("V")).expect("v");
    nest(&mut tx, v, vec![grade]);

    let mut params = Vec::new();
    let mut wrappers = Vec::new();
    let mut end = (0.0, 0.0, 0.0);
    for (start, length, l1, l2, r1, r2, kind) in cant {
        let mut draft = CantSegmentDraft::new(*start, *length, *l1, *r1, kind);
        if let (Some(l2), Some(r2)) = (l2, r2) {
            draft = draft.end_cant_left(*l2).end_cant_right(*r2);
        }
        let id = cant_segment(&mut tx, &draft).expect("cant segment");
        params.push(id);
        wrappers.push(alignment_segment(&mut tx, &guid(), id).expect("segment"));
        end = (start + length, l2.unwrap_or(*l1), r2.unwrap_or(*r1));
    }
    let closing = cant_segment(
        &mut tx,
        &CantSegmentDraft::new(end.0, 0.0, end.1, end.2, "CONSTANTCANT"),
    )
    .expect("closing");
    wrappers.push(alignment_segment(&mut tx, &guid(), closing).expect("segment"));
    let c = cant_layout(&mut tx, &guid(), Some("C"), B).expect("cant");
    nest(&mut tx, c, wrappers);

    let a = alignment(&mut tx, &guid(), Some("A"), None).expect("alignment");
    nest(&mut tx, a, vec![h, v, c]);
    tx.commit(&mut model).expect("commit");
    (model, a, params)
}

fn banked(model: &Model, a: EntityId) -> Banked3 {
    match segmented_reference_curve3(model, a, metres()).expect("banked") {
        Curve3::Banked(banked) => banked,
        other => panic!("expected a banked curve, got {other:?}"),
    }
}

/// The base formula's shape `f(xi)`, `D = D1 + f(xi) dD`, for the height
/// forms, as IFC4X3_ADD2 writes it.
fn shape(kind: &str, xi: f64) -> f64 {
    match kind {
        "CONSTANTCANT" => 0.0,
        "LINEARTRANSITION" => xi,
        "BLOSSCURVE" => (3.0 - 2.0 * xi) * xi * xi,
        "HELMERTCURVE" if xi <= 0.5 => 2.0 * xi * xi,
        "HELMERTCURVE" => 1.0 - 2.0 * (1.0 - xi) * (1.0 - xi),
        "COSINECURVE" => 0.5 * (1.0 - (PI * xi).cos()),
        "SINECURVE" => xi - (2.0 * PI * xi).sin() / (2.0 * PI),
        other => panic!("{other} is not a height form"),
    }
}

const STATIONS: [f64; 9] = [0.0, 0.1, 0.25, 0.4, 0.5, 0.6, 0.75, 0.9, 1.0];

/// Each height form from 30 mm to 150 mm, rotating about the low (right)
/// rail: the cant law follows the base formula, and the pivot, midway
/// between the rails, follows it at half the height.
#[test]
fn every_height_form_lowers_to_its_ifc_base_formula() {
    for kind in [
        "CONSTANTCANT",
        "LINEARTRANSITION",
        "BLOSSCURVE",
        "HELMERTCURVE",
        "COSINECURVE",
        "SINECURVE",
    ] {
        let (d1, d2) = if kind == "CONSTANTCANT" {
            (0.03, 0.03)
        } else {
            (0.03, 0.15)
        };
        let (model, a, _) = track(&[(0.0, 100.0, d1, Some(d2), 0.0, Some(0.0), kind)]);
        let curve = banked(&model, a);
        assert_eq!(curve.convention, BankConvention::TangentRotation, "{kind}");
        assert_eq!(curve.span(), 100.0, "{kind}");
        let pieces = if kind == "HELMERTCURVE" { 2 } else { 1 };
        assert_eq!(curve.cant.pieces.len(), pieces, "{kind}");
        for xi in STATIONS {
            let expected = d1 + shape(kind, xi) * (d2 - d1);
            let cant = curve.cant_at(100.0 * xi).expect("cant");
            assert!(
                (cant - expected).abs() < 1e-15,
                "{kind} at xi {xi}: {cant} != {expected}"
            );
            let (pivot, _) = curve.pivot_at(100.0 * xi).expect("pivot");
            assert!(
                (pivot - expected / 2.0).abs() < 1e-15,
                "{kind} pivot at xi {xi}: {pivot} != {}",
                expected / 2.0
            );
            let psi = curve.bank_angle_at(100.0 * xi).expect("angle");
            assert!((psi - (expected / B).asin()).abs() < 1e-15, "{kind} angle");
        }
    }
}

/// The Viennese bend is stored as its bank angle, the form IFC writes it
/// in, and rotates about the centreline (left = D / 2, right = -D / 2).
#[test]
fn the_viennese_bend_lowers_as_its_bank_angle() {
    let (model, a, _) = track(&[(
        0.0,
        100.0,
        0.0,
        Some(0.075),
        0.0,
        Some(-0.075),
        "VIENNESEBEND",
    )]);
    let curve = banked(&model, a);
    let psi2 = (0.15_f64 / B).asin();
    assert_eq!(
        curve.cant.pieces[0].form,
        CantForm::VienneseBend {
            start: 0.0,
            change: psi2,
        }
    );
    for xi in STATIONS {
        let blend = xi.powi(4) * (35.0 - 84.0 * xi + 70.0 * xi * xi - 20.0 * xi.powi(3));
        let psi = psi2 * blend;
        // Rounding only: the kernel evaluates the shape in Horner form.
        let angle = curve.bank_angle_at(100.0 * xi).expect("angle");
        assert!((angle - psi).abs() < 1e-15, "psi at {xi}: {angle} != {psi}");
        let cant = curve.cant_at(100.0 * xi).expect("cant");
        assert!(
            (cant - B * psi.sin()).abs() < 1e-14,
            "D = b sin psi at {xi}"
        );
        assert_eq!(curve.pivot_at(100.0 * xi).expect("pivot").0, 0.0);
    }
}

/// A low-rail pivot through a Viennese bend follows the bank angle,
/// `b sin(psi) / 2`: the banked lowering refuses it with a reason naming
/// the angle-form pivot gap (#364), while the section frame evaluates it,
/// the right rail held and the left rail `D(xi)` from the angle law. The
/// two differ here, and only here, until Axiolid has an angle-form pivot.
#[test]
fn a_held_rail_viennese_bend_evaluates_but_does_not_lower() {
    let (model, a, ids) = track(&[(0.0, 100.0, 0.0, Some(0.15), 0.0, Some(0.0), "VIENNESEBEND")]);
    let error = lower_segmented_reference_curve(&model, a, metres()).expect_err("angle pivot");
    assert!(
        matches!(&error, AlignmentError::Unsupported { entity, type_name, detail }
            if *entity == ids[0] && type_name == "VIENNESEBEND"
                && detail.contains("held rail") && detail.contains("AngleInPivot")
                && detail.contains("angle-form pivot")),
        "{error:?}"
    );
    let cant = CantLayout::for_alignment(&model, a, metres()).expect("cant");
    let psi2 = (0.15_f64 / B).asin();
    for xi in STATIONS {
        let frame = cant.frame_at_distance(100.0 * xi).expect("held rail");
        let blend = xi.powi(4) * (35.0 - 84.0 * xi + 70.0 * xi * xi - 20.0 * xi.powi(3));
        let psi = psi2 * blend;
        assert_eq!(frame.right, 0.0, "held rail at {xi}");
        assert!((frame.left - B * psi.sin()).abs() < 1e-15, "left at {xi}");
        assert!((frame.bank_angle - psi).abs() < 1e-15, "psi at {xi}");
        assert!(
            (frame.axis_elevation - B * psi.sin() / 2.0).abs() < 1e-15,
            "pivot at {xi}"
        );
    }
    for (station, left) in [(0.0, 0.0), (100.0, 0.15)] {
        let frame = cant.frame_at_distance(station).expect("authored end");
        assert_eq!((frame.left, frame.right), (left, 0.0), "at {station}");
    }
}

/// A pivot that moves with neither rail held: the rails are not
/// determined, and the lowering and the section frame refuse it alike.
#[test]
fn a_moving_pivot_without_a_held_rail_is_refused() {
    let (model, a, ids) = track(&[(
        0.0,
        100.0,
        0.0,
        Some(0.15),
        0.02,
        Some(-0.01),
        "VIENNESEBEND",
    )]);
    let error = lower_segmented_reference_curve(&model, a, metres()).expect_err("unheld pivot");
    assert!(
        matches!(&error, AlignmentError::Unsupported { entity, type_name, detail }
            if *entity == ids[0] && type_name == "VIENNESEBEND" && detail.contains("neither rail")),
        "{error:?}"
    );
    let cant = CantLayout::for_alignment(&model, a, metres()).expect("cant");
    assert_eq!(cant.frame_at_distance(50.0), Err(error));
    for (station, left, right) in [(0.0, 0.0, 0.02), (100.0, 0.15, -0.01)] {
        let frame = cant.frame_at_distance(station).expect("authored end");
        assert_eq!((frame.left, frame.right), (left, right), "at {station}");
    }
}

/// Inside a Viennese bend the section frame is the banked curve's section
/// (#312): `CantFrame` blends the section's cant angle, `psi = arcsin(D / b)`
/// with `D = left - right`, and places the rails about a rotation point
/// that stays put, 50 mm above the profile, as `Curve3::Banked` does.
#[test]
fn the_cant_frame_agrees_with_the_banked_section_inside_a_viennese_bend() {
    let (model, a, _) = track(&[(
        0.0,
        100.0,
        0.05,
        Some(0.125),
        0.05,
        Some(-0.025),
        "VIENNESEBEND",
    )]);
    let curve = banked(&model, a);
    let Curve3::Elevated(elevated) = gradient_curve3(&model, a, metres()).expect("gradient") else {
        panic!("expected an elevated curve");
    };
    let cant = CantLayout::for_alignment(&model, a, metres()).expect("cant");
    let psi2 = (0.15_f64 / B).asin();
    for xi in STATIONS {
        let d = 100.0 * xi;
        let section = axiolid_evaluate::banked_section(&curve, d).expect("section");
        let frame = cant.frame_at_distance(d).expect("frame");
        let blend = xi.powi(4) * (35.0 - 84.0 * xi + 70.0 * xi * xi - 20.0 * xi.powi(3));
        let psi = psi2 * blend;
        assert!((frame.bank_angle - psi).abs() < 1e-15, "psi at {xi}");
        assert!(
            (frame.bank_angle - section.roll).abs() < 1e-15,
            "roll at {xi}"
        );
        assert!((frame.cant - section.cant).abs() < 1e-15, "cant at {xi}");
        assert!((frame.axis_elevation - 0.05).abs() < 1e-15, "pivot at {xi}");
        assert!(
            (frame.left - (0.05 + B * psi.sin() / 2.0)).abs() < 1e-15,
            "left rail at {xi}"
        );

        let point = axiolid_evaluate::elevated_point(&elevated, d).expect("point");
        let tangent = axiolid_evaluate::elevated_tangent(&elevated, d).expect("tangent");
        let oriented = frame.orient(point, tangent).expect("oriented");
        for (actual, expected, what) in [
            (oriented.origin, section.point, "rotation point"),
            (oriented.x, section.tangent, "tangent"),
            (oriented.y, section.lateral, "rail axis"),
            (oriented.z, section.up, "section up"),
        ] {
            assert!(
                (actual - expected).length() < 1e-12,
                "{what} at {xi}: {actual:?} != {expected:?}"
            );
        }
    }
}

/// A cant layout that stops short of the plan leaves stations without a
/// section: refused, not padded with zero cant.
#[test]
fn a_cant_layout_short_of_the_plan_is_refused() {
    let (model, a, _) = track(&[(
        0.0,
        60.0,
        0.0,
        Some(0.1),
        0.0,
        Some(0.0),
        "LINEARTRANSITION",
    )]);
    let error = lower_segmented_reference_curve(&model, a, metres()).expect_err("short");
    assert!(
        matches!(&error, AlignmentError::Unsupported { type_name, detail, .. }
            if type_name == "IfcAlignmentCant" && detail.contains("whole plan")),
        "{error:?}"
    );
}

/// `|D| > b` has no bank angle; refused at the segment that states it.
#[test]
fn cant_beyond_the_rail_head_distance_is_refused() {
    let (model, a, ids) = track(&[(0.0, 100.0, 0.0, Some(1.6), 0.0, Some(0.0), "BLOSSCURVE")]);
    let error = lower_segmented_reference_curve(&model, a, metres()).expect_err("too much cant");
    assert_eq!(
        error,
        AlignmentError::SemanticViolation {
            entity: Some(ids[0]),
            rule: "cant must not exceed the rail head distance (|D| <= b)",
        }
    );
}

/// Seams between cant segments, through the reference evaluator: the
/// rotation point, the cant and the section axes approach the seam value
/// from both sides. A ramp, a hold, a Helmert run-out (whose own midpoint
/// is a seam between its two pieces) and a hold, rotating about the
/// centreline (left = D / 2, right = -D / 2).
///
/// About the centreline the rotation point is the profile, so the frame is
/// continuous wherever the cant is. A low-rail pivot rises with the cant,
/// and where a linear ramp meets a hold the pivot's rate jumps: that seam is
/// a real kink of the rotation-point curve, which IFC allows (cant is only
/// required C0), so only its position is checked below.
#[test]
fn the_banked_curve_is_continuous_at_its_cant_seams() {
    let (model, a, _) = track(&[
        (
            0.0,
            30.0,
            0.0,
            Some(0.06),
            0.0,
            Some(-0.06),
            "LINEARTRANSITION",
        ),
        (30.0, 30.0, 0.06, None, -0.06, None, "CONSTANTCANT"),
        (
            60.0,
            30.0,
            0.06,
            Some(0.015),
            -0.06,
            Some(-0.015),
            "HELMERTCURVE",
        ),
        (90.0, 10.0, 0.015, None, -0.015, None, "CONSTANTCANT"),
    ]);
    let curve = banked(&model, a);
    assert_eq!(curve.cant.seams(), vec![30.0, 60.0, 75.0, 90.0]);
    let section = |d: f64| axiolid_evaluate::banked_section(&curve, d).expect("section");
    for seam in curve.cant.seams() {
        let at = section(seam);
        for side in [seam - 1e-9, seam + 1e-9] {
            let near = section(side);
            assert!(
                (near.point - at.point).length() < 1e-8,
                "point steps at {seam}: {:?} vs {:?}",
                near.point,
                at.point
            );
            assert!((near.cant - at.cant).abs() < 1e-9, "cant steps at {seam}");
            assert!(
                (near.lateral - at.lateral).length() < 1e-8,
                "lateral axis turns at {seam}"
            );
            assert!((near.up - at.up).length() < 1e-8, "up axis turns at {seam}");
        }
    }
    // The seam values are the authored ones.
    for (seam, cant) in [(30.0, 0.12), (60.0, 0.12), (75.0, 0.075), (90.0, 0.03)] {
        assert!((section(seam).cant - cant).abs() < 1e-15, "cant at {seam}");
    }

    // About the low rail the rotation point still meets itself at a seam.
    let (model, a, _) = track(&[
        (
            0.0,
            30.0,
            0.0,
            Some(0.12),
            0.0,
            Some(0.0),
            "LINEARTRANSITION",
        ),
        (30.0, 70.0, 0.12, None, 0.0, None, "CONSTANTCANT"),
    ]);
    let curve = banked(&model, a);
    let point = |d: f64| axiolid_evaluate::banked_point(&curve, d).expect("point");
    assert!((point(30.0 - 1e-9) - point(30.0)).length() < 1e-8);
    assert!((point(30.0) - point(30.0 + 1e-9)).length() < 1e-8);
}

/// The committed fixture `synthetic_alignment_layout.ifc`: a 17.85 m plan,
/// a 15 m profile and a 10 m cant layout. Its cant layout resolves, and the
/// banked centreline is refused for the first thing that does not cover
/// the plan -- the profile -- not invented past the authored data.
#[test]
fn the_committed_layout_fixture_is_refused_where_it_stops_short() {
    use ifc_model::Codec;
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../test/fixtures/synthetic-surfaces/synthetic_alignment_layout.ifc");
    let model = ifc_step::StepCodec
        .read_path(&path)
        .expect("fixture parses");
    let alignment = *model
        .ids_of_type("IFCALIGNMENT")
        .first()
        .expect("one alignment");
    ifc_alignment::CantLayout::for_alignment(&model, alignment, metres()).expect("cant resolves");
    let error = lower_segmented_reference_curve(&model, alignment, metres()).expect_err("short");
    assert!(
        matches!(&error, AlignmentError::Unsupported { type_name, detail, .. }
            if type_name == "IfcAlignmentVertical" && detail.contains("whole plan")),
        "{error:?}"
    );
}
