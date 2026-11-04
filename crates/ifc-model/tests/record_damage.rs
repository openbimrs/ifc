//! `RecordDamage` classifies a model's reader diagnostics (#684).

use ifc_model::{Diagnostic, DiagnosticKind, Entity, EntityId, Model, RecordDamage};

fn model_with(kept: &[u64], diagnostics: Vec<Diagnostic>) -> Model {
    let mut model = Model::new();
    for &id in kept {
        model.insert(EntityId(id), Entity::new("IFCSITE", vec![]));
    }
    for diagnostic in diagnostics {
        model.push_diagnostic(diagnostic);
    }
    model
}

fn at(kind: DiagnosticKind, id: u64) -> Diagnostic {
    Diagnostic::new(kind, "x").with_instance(EntityId(id))
}

#[test]
fn no_diagnostics_report_nothing() {
    let damage = RecordDamage::of(&model_with(&[1], vec![])).unwrap();
    assert_eq!(damage.reported(EntityId(1)), None);
    assert!(!damage.records_lost());
    assert!(!damage.held_id_skipped());
    assert_eq!(damage.diagnostics(), 0);
}

#[test]
fn real_without_point_and_dangling_reference_are_ignored() {
    let model = model_with(
        &[1],
        vec![
            at(DiagnosticKind::RealWithoutPoint, 1),
            at(DiagnosticKind::DanglingReference, 1),
        ],
    );
    let damage = RecordDamage::of(&model).unwrap();
    assert_eq!(damage.reported(EntityId(1)), None);
    assert!(!damage.records_lost());
    assert!(!damage.held_id_skipped());
    assert_eq!(damage.diagnostics(), 2);
}

#[test]
fn a_skip_of_an_unheld_id_loses_a_record() {
    let model = model_with(&[1], vec![at(DiagnosticKind::SkippedRecord, 7)]);
    let damage = RecordDamage::of(&model).unwrap();
    assert_eq!(damage.reported(EntityId(7)), Some(0));
    assert!(damage.records_lost());
    assert!(!damage.held_id_skipped());
}

#[test]
fn a_skip_of_a_held_id_is_flagged() {
    let model = model_with(&[7], vec![at(DiagnosticKind::SkippedRecord, 7)]);
    let damage = RecordDamage::of(&model).unwrap();
    assert_eq!(damage.reported(EntityId(7)), Some(0));
    assert!(damage.records_lost());
    assert!(damage.held_id_skipped());
}

#[test]
fn a_duplicate_is_reported_without_losing_a_record() {
    let model = model_with(&[7], vec![at(DiagnosticKind::DuplicateId, 7)]);
    let damage = RecordDamage::of(&model).unwrap();
    assert_eq!(damage.reported(EntityId(7)), Some(0));
    assert!(!damage.records_lost());
    assert!(!damage.held_id_skipped());
}

#[test]
fn the_first_diagnostic_naming_an_id_is_reported() {
    let model = model_with(
        &[],
        vec![
            at(DiagnosticKind::DuplicateId, 3),
            at(DiagnosticKind::SkippedRecord, 7),
            at(DiagnosticKind::DuplicateId, 7),
        ],
    );
    let damage = RecordDamage::of(&model).unwrap();
    assert_eq!(damage.reported(EntityId(3)), Some(0));
    assert_eq!(damage.reported(EntityId(7)), Some(1));
}

#[test]
fn a_diagnostic_naming_no_record_is_unclassified() {
    let cases = [
        (
            DiagnosticKind::SkippedRecord,
            Diagnostic::new(DiagnosticKind::SkippedRecord, "x"),
        ),
        (
            DiagnosticKind::DuplicateId,
            Diagnostic::new(DiagnosticKind::DuplicateId, "x"),
        ),
        (DiagnosticKind::Other, Diagnostic::unlocated("x")),
    ];
    for (kind, diagnostic) in cases {
        let model = model_with(
            &[],
            vec![at(DiagnosticKind::RealWithoutPoint, 1), diagnostic],
        );
        let error = RecordDamage::of(&model).unwrap_err();
        assert_eq!((error.index, error.kind), (1, kind));
    }
}

#[test]
fn the_unclassified_error_describes_itself() {
    let model = model_with(&[], vec![Diagnostic::unlocated("x")]);
    let error = RecordDamage::of(&model).unwrap_err();
    assert_eq!(
        error.to_string(),
        "diagnostic 0 (Other) names no record a resolution can check"
    );
}
