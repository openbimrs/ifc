//! Staging tables and time series.

use ifc_model::{Model, Transaction, Value};
use ifc_tabular::{
    add_irregular_time_series, add_irregular_value, add_regular_time_series, add_table,
    add_table_column, add_table_row, add_time_series_value, ColumnDraft, SeriesDraft, TabularError,
};

fn series(name: &str) -> SeriesDraft<'_> {
    SeriesDraft {
        name,
        start_time: "2026-01-01T00:00:00",
        end_time: "2026-01-02T00:00:00",
        data_type: "CONTINUOUS",
        data_origin: "MEASURED",
        ..SeriesDraft::default()
    }
}

/// WR1: a ragged table is refused.
///
/// This is the rule worth having. A short row parses, renders, and
/// round-trips; the damage shows up only when a reader indexes a column
/// that row does not have.
#[test]
fn a_ragged_table_is_refused() {
    let model = Model::new();
    let mut tx = Transaction::new(&model);
    let wide = add_table_row(&mut tx, vec![Value::Integer(1), Value::Integer(2)], false).unwrap();
    let narrow = add_table_row(&mut tx, vec![Value::Integer(3)], false).unwrap();
    let err = add_table(
        &mut tx,
        Some("Ragged"),
        &[(wide, 2, false), (narrow, 1, false)],
        &[],
    )
    .expect_err("a short row must be refused");
    assert!(
        matches!(
            err,
            TabularError::RaggedRow {
                row: 1,
                expected: 2,
                found: 1,
                ..
            }
        ),
        "{err}"
    );
}

/// WR2: at most one heading row.
#[test]
fn two_heading_rows_are_refused() {
    let model = Model::new();
    let mut tx = Transaction::new(&model);
    let a = add_table_row(&mut tx, vec![Value::Integer(1)], true).unwrap();
    let b = add_table_row(&mut tx, vec![Value::Integer(2)], true).unwrap();
    let err = add_table(&mut tx, None, &[(a, 1, true), (b, 1, true)], &[])
        .expect_err("two headings must be refused");
    assert!(
        matches!(err, TabularError::TooManyHeadings { found: 2, .. }),
        "{err}"
    );
}

/// One heading plus data rows is the ordinary shape.
#[test]
fn one_heading_and_data_rows_are_accepted() {
    let model = Model::new();
    let mut tx = Transaction::new(&model);
    let head = add_table_row(&mut tx, vec![Value::Text("Size".into())], true).unwrap();
    let data = add_table_row(&mut tx, vec![Value::Integer(42)], false).unwrap();
    add_table(
        &mut tx,
        Some("Sizes"),
        &[(head, 1, true), (data, 1, false)],
        &[],
    )
    .expect("one heading is allowed");
}

/// IfcTable holds exactly three instance slots.
///
/// Its three DERIVE attributes introduce new names rather than redeclaring
/// an inherited one, so they take no slots and must not be written. A
/// writer that emitted counts here would produce a file that can contradict
/// its own rows.
#[test]
fn derived_counts_occupy_no_slots() {
    let mut model = Model::new();
    let mut tx = Transaction::new(&model);
    let row = add_table_row(&mut tx, vec![Value::Integer(1)], false).unwrap();
    let table = add_table(&mut tx, Some("T"), &[(row, 1, false)], &[]).unwrap();
    tx.commit(&mut model).unwrap();
    let entity = model.get(table).expect("table committed");
    assert_eq!(entity.attributes.len(), 3, "IfcTable has three slots");
}

/// The cell's measure wrapper survives exactly as the caller wrote it.
///
/// `4.2` and `IFCLENGTHMEASURE(4.2)` are different files and only the
/// second is dimensionally meaningful. The crate must neither add nor
/// strip the wrapper.
#[test]
fn cell_measures_are_passed_through_untouched() {
    let mut model = Model::new();
    let mut tx = Transaction::new(&model);
    let typed = Value::Typed {
        type_name: "IFCLENGTHMEASURE".into(),
        value: Box::new(Value::Real(4.2)),
    };
    let row = add_table_row(&mut tx, vec![typed.clone(), Value::Real(4.2)], false).unwrap();
    tx.commit(&mut model).unwrap();
    let entity = model.get(row).expect("row committed");
    let Some(Value::List(cells)) = entity.attributes.first() else {
        panic!("RowCells is a list");
    };
    assert_eq!(cells[0], typed, "the wrapper must survive");
    assert_eq!(cells[1], Value::Real(4.2), "a bare literal stays bare");
}

/// A regular series states one step; an irregular one timestamps each value.
#[test]
fn both_series_kinds_stage_with_their_own_shape() {
    let mut model = Model::new();
    let mut tx = Transaction::new(&model);
    let plain = add_time_series_value(&mut tx, vec![Value::Real(1.0)]).unwrap();
    let stamped =
        add_irregular_value(&mut tx, "2026-01-01T06:00:00", vec![Value::Real(2.0)]).unwrap();
    let regular = add_regular_time_series(&mut tx, series("Load"), 3600.0, &[plain]).unwrap();
    let irregular = add_irregular_time_series(&mut tx, series("Events"), &[stamped]).unwrap();
    tx.commit(&mut model).unwrap();
    assert_eq!(model.get(regular).unwrap().attributes.len(), 10);
    assert_eq!(model.get(irregular).unwrap().attributes.len(), 9);
    let step = &model.get(regular).unwrap().attributes[8];
    assert_eq!(*step, Value::Real(3600.0), "TimeStep sits at slot 8");
}

/// Every LIST [1:?] refuses an empty list rather than writing one.
#[test]
fn required_lists_refuse_to_be_empty() {
    let model = Model::new();
    let mut tx = Transaction::new(&model);
    assert!(matches!(
        add_table_row(&mut tx, Vec::new(), false),
        Err(TabularError::EmptyList { .. })
    ));
    assert!(matches!(
        add_time_series_value(&mut tx, Vec::new()),
        Err(TabularError::EmptyList { .. })
    ));
    assert!(matches!(
        add_irregular_time_series(&mut tx, series("E"), &[]),
        Err(TabularError::EmptyList { .. })
    ));
}

/// A blank required label is refused: it satisfies EXISTS while naming nothing.
#[test]
fn blank_required_text_is_refused() {
    let model = Model::new();
    let mut tx = Transaction::new(&model);
    let mut draft = series("ok");
    draft.name = "   ";
    assert!(matches!(
        add_irregular_time_series(&mut tx, draft, &[]),
        Err(TabularError::BlankRequired { .. })
    ));
    assert!(matches!(
        add_table_column(
            &mut tx,
            ColumnDraft {
                identifier: Some(" "),
                ..ColumnDraft::default()
            }
        ),
        Err(TabularError::BlankRequired { .. })
    ));
}

/// Rows and columns land in their own slots.
///
/// Both are lists of refs, so swapping them yields a file that parses and
/// validates structurally while reporting every column as a row.
#[test]
fn rows_and_columns_do_not_share_a_slot() {
    let mut model = Model::new();
    let mut tx = Transaction::new(&model);
    let row = add_table_row(&mut tx, vec![Value::Integer(1)], false).unwrap();
    let column = add_table_column(
        &mut tx,
        ColumnDraft {
            identifier: Some("A"),
            ..ColumnDraft::default()
        },
    )
    .unwrap();
    let table = add_table(&mut tx, None, &[(row, 1, false)], &[column]).unwrap();
    tx.commit(&mut model).unwrap();
    let entity = model.get(table).expect("table committed");
    assert_eq!(
        entity.attributes[1],
        Value::List(vec![Value::Ref(row)]),
        "Rows sit at slot 1"
    );
    assert_eq!(
        entity.attributes[2],
        Value::List(vec![Value::Ref(column)]),
        "Columns sit at slot 2"
    );
}
