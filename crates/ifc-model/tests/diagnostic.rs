//! The `Diagnostic` accessors, builders and `Display`.

use ifc_model::{Diagnostic, DiagnosticKind, EntityId, SourcePosition};

#[test]
fn legacy_constructors_build_other_without_ids() {
    for d in [Diagnostic::warning(2..5, "x"), Diagnostic::unlocated("x")] {
        assert_eq!(d.kind(), DiagnosticKind::Other);
        assert_eq!(d.instance(), None);
        assert_eq!(d.referrer(), None);
        assert_eq!(d.position(), None);
        assert_eq!(d.excerpt(), None);
    }
    assert_eq!(Diagnostic::warning(2..5, "x").byte_range(), Some(&(2..5)));
    assert_eq!(Diagnostic::unlocated("x").byte_range(), None);
}

#[test]
fn builders_set_their_field() {
    let d = Diagnostic::new(DiagnosticKind::DanglingReference, "gone")
        .with_byte_range(1..9)
        .with_position(SourcePosition::new(3, 4))
        .with_instance(EntityId(17))
        .with_referrer(EntityId(18))
        .with_excerpt("#18=X(#17);");
    assert_eq!(d.kind(), DiagnosticKind::DanglingReference);
    assert_eq!(d.byte_range(), Some(&(1..9)));
    assert_eq!(d.position(), Some(SourcePosition::new(3, 4)));
    assert_eq!(d.instance(), Some(EntityId(17)));
    assert_eq!(d.referrer(), Some(EntityId(18)));
    assert_eq!(d.excerpt(), Some("#18=X(#17);"));
    assert_eq!(d.detail(), "gone");
}

#[test]
fn display_with_and_without_position() {
    let base = Diagnostic::new(DiagnosticKind::SkippedRecord, "d");
    assert_eq!(base.to_string(), "warning: d");
    let ranged = base.clone().with_byte_range(1..2);
    assert_eq!(ranged.to_string(), "warning at bytes 1..2: d");
    let both = ranged.with_position(SourcePosition::new(3, 4));
    assert_eq!(
        both.to_string(),
        "warning at line 3, column 4 (bytes 1..2): d"
    );
    let pos_only = base.with_position(SourcePosition::new(3, 4));
    assert_eq!(pos_only.to_string(), "warning at line 3, column 4: d");
}

#[test]
fn source_position_orders_by_line_then_column() {
    assert!(SourcePosition::new(1, 9) < SourcePosition::new(2, 1));
    assert!(SourcePosition::new(2, 1) < SourcePosition::new(2, 2));
    assert_eq!(SourcePosition::new(2, 3).line(), 2);
    assert_eq!(SourcePosition::new(2, 3).column(), 3);
    assert_eq!(SourcePosition::new(2, 3).to_string(), "line 2, column 3");
}
