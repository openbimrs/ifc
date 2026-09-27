//! Opt-in constituent-fraction policy diagnostic (#103).
//!
//! IFC4 declares no WHERE rule on the sum of a constituent set's fractions,
//! so these are policy findings on valid files: the accessors keep reading
//! the authored values, and only the explicit check reports.

use ifc_material::{ConstituentFractionDiagnostic, MaterialError, MaterialView};
use ifc_model::{Entity, EntityId, Model, Value};

const TOLERANCE: f64 = 1e-6;

fn text(value: &str) -> Value {
    Value::Text(value.into())
}

/// A model with material #1, one constituent per fraction from #10 up, and
/// the constituent set #100 listing them all.
fn set_with(fractions: &[Option<f64>]) -> Model {
    let mut model = Model::new();
    model.insert(
        EntityId(1),
        Entity::new(
            "IFCMATERIAL",
            vec![text("Concrete"), Value::Null, Value::Null],
        ),
    );
    let mut members = Vec::new();
    for (index, fraction) in fractions.iter().enumerate() {
        let id = EntityId(10 + index as u64);
        members.push(Value::Ref(id));
        model.insert(
            id,
            Entity::new(
                "IFCMATERIALCONSTITUENT",
                vec![
                    text(&format!("Part {index}")),
                    Value::Null,
                    Value::Ref(EntityId(1)),
                    fraction.map_or(Value::Null, Value::Real),
                    Value::Null,
                ],
            ),
        );
    }
    model.insert(
        EntityId(100),
        Entity::new(
            "IFCMATERIALCONSTITUENTSET",
            vec![text("Mix"), Value::Null, Value::List(members)],
        ),
    );
    model
}

fn diagnose(
    model: &Model,
    tolerance: f64,
) -> Result<Option<ConstituentFractionDiagnostic>, MaterialError> {
    let view = MaterialView::new(model);
    let set = view.constituent_sets().next().expect("one set");
    view.constituent_fraction_diagnostic(set, tolerance)
}

#[test]
fn fractions_summing_to_one_are_not_reported() {
    let model = set_with(&[Some(0.25), Some(0.5), Some(0.25)]);
    assert_eq!(diagnose(&model, TOLERANCE).unwrap(), None);
}

#[test]
fn a_sum_within_the_tolerance_is_not_reported() {
    // 0.1 + 0.2 + 0.7 is not exactly 1.0 in binary floating point.
    let model = set_with(&[Some(0.1), Some(0.2), Some(0.7)]);
    assert_eq!(diagnose(&model, TOLERANCE).unwrap(), None);
    // Rounded shares from an exporter, 0.333 x 3, within a loose tolerance.
    let model = set_with(&[Some(0.333), Some(0.333), Some(0.333)]);
    assert_eq!(diagnose(&model, 0.01).unwrap(), None);
}

#[test]
fn an_under_sum_is_reported() {
    let model = set_with(&[Some(0.5), Some(0.2)]);
    match diagnose(&model, TOLERANCE).unwrap() {
        Some(ConstituentFractionDiagnostic::SumNotOne {
            set,
            sum,
            tolerance,
        }) => {
            assert_eq!(set, EntityId(100));
            assert!((sum - 0.7).abs() < 1e-12, "{sum}");
            assert_eq!(tolerance, TOLERANCE);
        }
        other => panic!("expected SumNotOne, got {other:?}"),
    }
}

#[test]
fn an_over_sum_is_reported() {
    let model = set_with(&[Some(0.9), Some(0.5)]);
    assert!(matches!(
        diagnose(&model, TOLERANCE).unwrap(),
        Some(ConstituentFractionDiagnostic::SumNotOne { sum, .. }) if (sum - 1.4).abs() < 1e-12
    ));
}

#[test]
fn the_tolerance_is_an_inclusive_bound_on_both_sides() {
    // 0.75 + 0.25 = 1.0 exactly, and so is every difference below: the
    // assertions are free of floating-point rounding.
    let under = set_with(&[Some(0.5), Some(0.25)]);
    assert_eq!(diagnose(&under, 0.25).unwrap(), None, "|0.75 - 1| = 0.25");
    assert!(diagnose(&under, 0.125).unwrap().is_some());
    let over = set_with(&[Some(0.75), Some(0.5)]);
    assert_eq!(diagnose(&over, 0.25).unwrap(), None, "|1.25 - 1| = 0.25");
    assert!(diagnose(&over, 0.125).unwrap().is_some());
}

#[test]
fn a_nan_or_negative_tolerance_admits_no_sum() {
    let model = set_with(&[Some(0.5), Some(0.5)]);
    assert!(diagnose(&model, f64::NAN).unwrap().is_some());
    assert!(diagnose(&model, -1.0).unwrap().is_some());
}

#[test]
fn partially_stated_fractions_are_reported() {
    let model = set_with(&[Some(0.6), None, Some(0.4), None]);
    assert_eq!(
        diagnose(&model, TOLERANCE).unwrap(),
        Some(ConstituentFractionDiagnostic::PartiallyStated {
            set: EntityId(100),
            stated: vec![EntityId(10), EntityId(12)],
            missing: vec![EntityId(11), EntityId(13)],
            stated_sum: 1.0,
        }),
        "a stated sum of 1 does not excuse unstated members"
    );
}

#[test]
fn a_set_stating_no_fractions_is_not_reported() {
    let model = set_with(&[None, None]);
    assert_eq!(diagnose(&model, TOLERANCE).unwrap(), None);
    let mut empty = set_with(&[]);
    empty.insert(
        EntityId(100),
        Entity::new(
            "IFCMATERIALCONSTITUENTSET",
            vec![text("Mix"), Value::Null, Value::Null],
        ),
    );
    assert_eq!(diagnose(&empty, TOLERANCE).unwrap(), None);
}

#[test]
fn the_diagnostic_never_changes_what_the_accessors_read() {
    let model = set_with(&[Some(0.9), Some(0.5)]);
    let view = MaterialView::new(&model);
    let fractions: Vec<_> = view
        .constituents()
        .map(|c| c.fraction().expect("valid per-value range"))
        .collect();
    assert_eq!(fractions, [Some(0.9), Some(0.5)], "no normalisation");
}

#[test]
fn malformed_members_stay_decode_errors() {
    let mut model = set_with(&[Some(0.5)]);
    model.insert(
        EntityId(100),
        Entity::new(
            "IFCMATERIALCONSTITUENTSET",
            vec![
                text("Mix"),
                Value::Null,
                Value::List(vec![Value::Ref(EntityId(10)), Value::Ref(EntityId(99))]),
            ],
        ),
    );
    assert!(matches!(
        diagnose(&model, TOLERANCE),
        Err(MaterialError::DanglingReference { .. })
    ));

    let mut model = set_with(&[Some(0.5)]);
    model.insert(
        EntityId(100),
        Entity::new(
            "IFCMATERIALCONSTITUENTSET",
            vec![
                text("Mix"),
                Value::Null,
                Value::List(vec![Value::Ref(EntityId(1))]),
            ],
        ),
    );
    assert!(matches!(
        diagnose(&model, TOLERANCE),
        Err(MaterialError::ReferenceType { .. })
    ));

    let model = set_with(&[Some(1.5)]);
    assert!(matches!(
        diagnose(&model, TOLERANCE),
        Err(MaterialError::InvalidValue { .. })
    ));
}
