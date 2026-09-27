//! Reading tables and time series back.
//!
//! Two halves. Author-then-read: records staged through this crate's own
//! writers survive a STEP round trip under IFC4 and IFC4X3 headers and
//! read back clean, with the schema taken from the file header. Invalid
//! records: hand-built models whose defects must each surface as a
//! `TabularIssue`, never be dropped.

use ifc_model::{Codec, Entity, EntityId, Model, Transaction, Value};
use ifc_schema::{for_version, ifc2x3, ifc4, ifc4x3, SchemaVersion};
use ifc_step::StepCodec;
use ifc_tabular::{
    add_irregular_time_series, add_irregular_value, add_regular_time_series, add_table,
    add_table_column, add_table_row, add_time_series_value, ColumnDraft, SeriesDraft, TabularIssue,
    TabularReadError, TabularView, TimeSeriesKind,
};

const HEADERS: [&str; 2] = ["IFC4", "IFC4X3_ADD2"];

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

/// Commit `tx`, write STEP under `header`, and parse it back.
fn round_trip(mut model: Model, tx: Transaction, header: &str) -> Model {
    tx.commit(&mut model).expect("commit");
    model.header_mut().schema = vec![header.to_owned()];
    let mut bytes = Vec::new();
    StepCodec.write(&model, &mut bytes).expect("written");
    StepCodec.read_bytes(&bytes).expect("reparsed")
}

/// A view under the schema the model's own header declares.
fn view(model: &Model) -> TabularView<'_> {
    let token = model.header().schema_token().expect("schema token");
    let version = SchemaVersion::from_header_token(token).expect("known schema");
    TabularView::new(model, for_version(version).expect("bundled")).expect("supported")
}

fn text(value: &str) -> Value {
    Value::Text(value.into())
}

fn real(measure: &str, value: f64) -> Value {
    Value::Typed {
        type_name: measure.into(),
        value: Box::new(Value::Real(value)),
    }
}

#[test]
fn an_authored_table_reads_back_in_both_schemas() {
    for header in HEADERS {
        let model = Model::new();
        let mut tx = Transaction::new(&model);
        let heading = add_table_row(&mut tx, vec![text("Room"), text("Area")], true).unwrap();
        let first = add_table_row(
            &mut tx,
            vec![text("101"), real("IFCAREAMEASURE", 12.5)],
            false,
        )
        .unwrap();
        let second = add_table_row(
            &mut tx,
            vec![text("102"), real("IFCAREAMEASURE", 20.0)],
            false,
        )
        .unwrap();
        let column = add_table_column(
            &mut tx,
            ColumnDraft {
                identifier: Some("area"),
                name: Some("Area"),
                ..ColumnDraft::default()
            },
        )
        .unwrap();
        add_table(
            &mut tx,
            Some("Room schedule"),
            &[(heading, 2, true), (first, 2, false), (second, 2, false)],
            &[column],
        )
        .unwrap();
        let model = round_trip(model, tx, header);

        let view = view(&model);
        let ids = view.table_ids();
        assert_eq!(ids.len(), 1, "{header}");
        let table = view.table(ids[0]).expect("table");
        assert_eq!(table.issues(), &[], "{header}");
        assert_eq!(table.name(), Some("Room schedule"));
        assert_eq!(table.rows().len(), 3);
        let heading = table.heading().expect("one heading");
        assert_eq!(heading.cells(), Some(&[text("Room"), text("Area")][..]));
        let data: Vec<_> = table.data_rows().collect();
        assert_eq!(data.len(), 2);
        assert_eq!(
            data[1].cells().expect("cells")[1],
            real("IFCAREAMEASURE", 20.0)
        );
        assert_eq!(table.columns().len(), 1);
        assert_eq!(table.columns()[0].identifier(), Some("area"));
        assert_eq!(table.columns()[0].name(), Some("Area"));
    }
}

#[test]
fn authored_time_series_read_back_in_both_schemas() {
    for header in HEADERS {
        let model = Model::new();
        let mut tx = Transaction::new(&model);
        let a = add_time_series_value(
            &mut tx,
            vec![real("IFCTHERMODYNAMICTEMPERATUREMEASURE", 293.0)],
        )
        .unwrap();
        let b = add_time_series_value(
            &mut tx,
            vec![real("IFCTHERMODYNAMICTEMPERATUREMEASURE", 294.5)],
        )
        .unwrap();
        add_regular_time_series(&mut tx, series("Supply air"), 3600.0, &[a, b]).unwrap();
        let c =
            add_irregular_value(&mut tx, "2026-01-01T06:30:00", vec![Value::Integer(3)]).unwrap();
        add_irregular_time_series(&mut tx, series("Door openings"), &[c]).unwrap();
        let model = round_trip(model, tx, header);

        let view = view(&model);
        let ids = view.series_ids();
        assert_eq!(ids.len(), 2, "{header}");

        let regular = view.time_series(ids[0]).expect("regular");
        assert_eq!(regular.issues(), &[], "{header}");
        assert_eq!(
            regular.kind(),
            TimeSeriesKind::Regular {
                time_step: Some(3600.0)
            }
        );
        assert_eq!(regular.name(), Some("Supply air"));
        assert_eq!(regular.start_time(), Some("2026-01-01T00:00:00"));
        assert_eq!(regular.end_time(), Some("2026-01-02T00:00:00"));
        assert_eq!(regular.data_type(), Some("CONTINUOUS"));
        assert_eq!(regular.data_origin(), Some("MEASURED"));
        assert_eq!(regular.values().len(), 2);
        assert_eq!(regular.values()[0].timestamp(), None);
        assert_eq!(
            regular.values()[1].values(),
            Some(&[real("IFCTHERMODYNAMICTEMPERATUREMEASURE", 294.5)][..])
        );

        let irregular = view.time_series(ids[1]).expect("irregular");
        assert_eq!(irregular.issues(), &[], "{header}");
        assert_eq!(irregular.kind(), TimeSeriesKind::Irregular);
        assert_eq!(irregular.values().len(), 1);
        assert_eq!(
            irregular.values()[0].timestamp(),
            Some("2026-01-01T06:30:00")
        );
        assert_eq!(
            irregular.values()[0].values(),
            Some(&[Value::Integer(3)][..])
        );
    }
}

/// A model built record by record, for defects no writer would produce.
struct Raw(Model);

impl Raw {
    fn new() -> Self {
        Self(Model::new())
    }

    fn put(&mut self, id: u64, type_name: &str, attributes: Vec<Value>) -> EntityId {
        let id = EntityId(id);
        self.0.insert(id, Entity::new(type_name, attributes));
        id
    }
}

fn refs(ids: &[EntityId]) -> Value {
    Value::List(ids.iter().copied().map(Value::Ref).collect())
}

fn cells(n: i64) -> Value {
    Value::List((0..n).map(Value::Integer).collect())
}

/// WR1 is measured against `Rows[1]`; a row with no cells is UNKNOWN in
/// the rule's QUERY and so not ragged.
#[test]
fn wr1_reports_each_ragged_row() {
    let mut raw = Raw::new();
    let wide = raw.put(1, "IFCTABLEROW", vec![cells(3), Value::Bool(false)]);
    let short = raw.put(2, "IFCTABLEROW", vec![cells(2), Value::Bool(false)]);
    let unset = raw.put(3, "IFCTABLEROW", vec![Value::Null, Value::Null]);
    let long = raw.put(4, "IFCTABLEROW", vec![cells(4), Value::Null]);
    let table = raw.put(
        5,
        "IFCTABLE",
        vec![Value::Null, refs(&[wide, short, unset, long]), Value::Null],
    );
    for schema in [ifc4(), ifc4x3()] {
        let view = TabularView::new(&raw.0, schema).unwrap();
        let read = view.table(table).unwrap();
        assert_eq!(
            read.issues(),
            &[
                TabularIssue::RaggedRow {
                    table,
                    row: short,
                    index: 1,
                    expected: 3,
                    found: 2,
                },
                TabularIssue::RaggedRow {
                    table,
                    row: long,
                    index: 3,
                    expected: 3,
                    found: 4,
                },
            ]
        );
        assert_eq!(
            read.rows().len(),
            4,
            "a ragged row is reported, not dropped"
        );
    }
}

/// With no cells in `Rows[1]` there is no reference width, so WR1 holds.
#[test]
fn wr1_has_no_reference_width_without_first_row_cells() {
    let mut raw = Raw::new();
    let first = raw.put(1, "IFCTABLEROW", vec![Value::Null, Value::Null]);
    let a = raw.put(2, "IFCTABLEROW", vec![cells(1), Value::Null]);
    let b = raw.put(3, "IFCTABLEROW", vec![cells(5), Value::Null]);
    let table = raw.put(
        4,
        "IFCTABLE",
        vec![Value::Null, refs(&[first, a, b]), Value::Null],
    );
    let read = TabularView::new(&raw.0, ifc4())
        .unwrap()
        .table(table)
        .unwrap();
    assert_eq!(read.issues(), &[]);
}

/// WR2: two rows with `IsHeading` TRUE; an unset flag is not a heading.
#[test]
fn wr2_reports_more_than_one_heading() {
    let mut raw = Raw::new();
    let a = raw.put(1, "IFCTABLEROW", vec![cells(1), Value::Bool(true)]);
    let b = raw.put(2, "IFCTABLEROW", vec![cells(1), Value::Bool(true)]);
    let c = raw.put(3, "IFCTABLEROW", vec![cells(1), Value::Null]);
    let two = raw.put(
        4,
        "IFCTABLE",
        vec![Value::Null, refs(&[a, b, c]), Value::Null],
    );
    let one = raw.put(5, "IFCTABLE", vec![Value::Null, refs(&[a, c]), Value::Null]);
    let view = TabularView::new(&raw.0, ifc4x3()).unwrap();

    let read = view.table(two).unwrap();
    assert_eq!(
        read.issues(),
        &[TabularIssue::TooManyHeadings {
            table: two,
            found: 2
        }]
    );
    assert!(read.heading().is_none(), "no single heading to return");

    let read = view.table(one).unwrap();
    assert_eq!(read.issues(), &[]);
    assert_eq!(read.heading().map(|row| row.id()), Some(a));
    assert_eq!(read.data_rows().count(), 1);
}

/// Every malformed slot and bad reference is reported; the members that
/// did resolve are still returned.
#[test]
fn malformed_table_slots_are_reported_not_dropped() {
    let mut raw = Raw::new();
    let good = raw.put(1, "IFCTABLEROW", vec![cells(2), Value::Bool(false)]);
    let logical = raw.put(2, "IFCTABLEROW", vec![cells(2), Value::LogicalUnknown]);
    let empty = raw.put(3, "IFCTABLEROW", vec![Value::List(vec![]), Value::Null]);
    let bare = raw.put(4, "IFCTABLEROW", vec![Value::Integer(7), Value::Null]);
    let short = raw.put(5, "IFCTABLEROW", vec![cells(2)]);
    let column = raw.put(
        6,
        "IFCTABLECOLUMN",
        vec![
            Value::Real(1.0),
            Value::Null,
            Value::Null,
            Value::Ref(EntityId(404)),
            Value::Null,
        ],
    );
    let rows = Value::List(vec![
        Value::Ref(good),
        Value::Ref(logical),
        Value::Ref(empty),
        Value::Ref(bare),
        Value::Ref(short),
        Value::Integer(9),
        Value::Ref(EntityId(99)),
        Value::Ref(column),
    ]);
    let table = raw.put(
        7,
        "IFCTABLE",
        vec![Value::Integer(1), rows, refs(&[column])],
    );
    let read = TabularView::new(&raw.0, ifc4())
        .unwrap()
        .table(table)
        .unwrap();
    let issues = read.issues();
    let expect = [
        TabularIssue::Malformed {
            entity: table,
            attribute: "Name",
            found: "Integer(1)".into(),
        },
        TabularIssue::Malformed {
            entity: table,
            attribute: "Rows",
            found: "Integer(9)".into(),
        },
        TabularIssue::Dangling {
            entity: table,
            attribute: "Rows",
            target: EntityId(99),
        },
        TabularIssue::WrongReferenceType {
            entity: table,
            attribute: "Rows",
            target: column,
            expected: "IFCTABLEROW",
            actual: "IFCTABLECOLUMN".into(),
        },
        TabularIssue::Malformed {
            entity: logical,
            attribute: "IsHeading",
            found: "LogicalUnknown".into(),
        },
        TabularIssue::EmptyList {
            entity: empty,
            attribute: "RowCells",
        },
        TabularIssue::Malformed {
            entity: bare,
            attribute: "RowCells",
            found: "Integer(7)".into(),
        },
        TabularIssue::Arity {
            entity: short,
            type_name: "IFCTABLEROW",
            expected: 2,
            found: 1,
        },
        TabularIssue::Malformed {
            entity: column,
            attribute: "Identifier",
            found: "Real(1.0)".into(),
        },
        TabularIssue::Dangling {
            entity: column,
            attribute: "Unit",
            target: EntityId(404),
        },
    ];
    for issue in &expect {
        assert!(issues.contains(issue), "missing {issue:?} in {issues:#?}");
    }
    // The empty RowCells list is width 0 against a first row of 2.
    assert!(issues.contains(&TabularIssue::RaggedRow {
        table,
        row: empty,
        index: 2,
        expected: 2,
        found: 0,
    }));
    assert_eq!(issues.len(), expect.len() + 1, "{issues:#?}");
    assert_eq!(read.rows().len(), 5, "every resolved row is returned");
    assert_eq!(read.columns().len(), 1);
    assert_eq!(read.name(), None);
}

/// Required attributes, the subtype's own value record, and value slots
/// are all checked on a time series.
#[test]
fn malformed_time_series_are_reported_not_dropped() {
    let mut raw = Raw::new();
    let empty = raw.put(1, "IFCTIMESERIESVALUE", vec![Value::List(vec![])]);
    let foreign = raw.put(2, "IFCIRREGULARTIMESERIESVALUE", vec![text("t"), cells(1)]);
    let regular = raw.put(
        3,
        "IFCREGULARTIMESERIES",
        vec![
            Value::Null,
            Value::Null,
            text("2026-01-01T00:00:00"),
            text("2026-01-02T00:00:00"),
            text("CONTINUOUS"),
            Value::Enum("MEASURED".into()),
            Value::Null,
            Value::Null,
            text("hourly"),
            refs(&[empty, foreign]),
        ],
    );
    let no_stamp = raw.put(
        4,
        "IFCIRREGULARTIMESERIESVALUE",
        vec![Value::Null, cells(1)],
    );
    let irregular = raw.put(
        5,
        "IFCIRREGULARTIMESERIES",
        vec![
            text("Openings"),
            Value::Null,
            text("2026-01-01T00:00:00"),
            Value::Null,
            Value::Enum("DISCRETE".into()),
            Value::Enum("MEASURED".into()),
            Value::Null,
            Value::Null,
            refs(&[no_stamp]),
        ],
    );
    let view = TabularView::new(&raw.0, ifc4x3()).unwrap();

    let read = view.time_series(regular).unwrap();
    assert_eq!(
        read.issues(),
        &[
            TabularIssue::Missing {
                entity: regular,
                attribute: "Name",
            },
            TabularIssue::Malformed {
                entity: regular,
                attribute: "TimeSeriesDataType",
                found: "Text(\"CONTINUOUS\")".into(),
            },
            TabularIssue::Malformed {
                entity: regular,
                attribute: "TimeStep",
                found: "Text(\"hourly\")".into(),
            },
            TabularIssue::WrongReferenceType {
                entity: regular,
                attribute: "Values",
                target: foreign,
                expected: "IFCTIMESERIESVALUE",
                actual: "IFCIRREGULARTIMESERIESVALUE".into(),
            },
            TabularIssue::EmptyList {
                entity: empty,
                attribute: "ListValues",
            },
        ]
    );
    assert_eq!(read.kind(), TimeSeriesKind::Regular { time_step: None });
    assert_eq!(read.values().len(), 1);

    let read = view.time_series(irregular).unwrap();
    assert_eq!(
        read.issues(),
        &[
            TabularIssue::Missing {
                entity: irregular,
                attribute: "EndTime",
            },
            TabularIssue::Missing {
                entity: no_stamp,
                attribute: "TimeStamp",
            },
        ]
    );
    assert_eq!(read.values()[0].values(), Some(&[Value::Integer(0)][..]));
}

/// An empty or missing `Values` list is reported on the series itself.
#[test]
fn a_series_without_values_is_reported() {
    let mut raw = Raw::new();
    let mut attributes = vec![
        text("S"),
        Value::Null,
        text("2026-01-01T00:00:00"),
        text("2026-01-02T00:00:00"),
        Value::Enum("CONTINUOUS".into()),
        Value::Enum("MEASURED".into()),
        Value::Null,
        Value::Null,
    ];
    let mut irregular = attributes.clone();
    irregular.push(Value::List(vec![]));
    attributes.push(Value::Real(60.0));
    attributes.push(Value::Null);
    let regular = raw.put(1, "IFCREGULARTIMESERIES", attributes);
    let irregular = raw.put(2, "IFCIRREGULARTIMESERIES", irregular);
    let view = TabularView::new(&raw.0, ifc4()).unwrap();
    assert_eq!(
        view.time_series(regular).unwrap().issues(),
        &[TabularIssue::Missing {
            entity: regular,
            attribute: "Values",
        }]
    );
    assert_eq!(
        view.time_series(irregular).unwrap().issues(),
        &[TabularIssue::EmptyList {
            entity: irregular,
            attribute: "Values",
        }]
    );
}

#[test]
fn entry_refusals() {
    let mut raw = Raw::new();
    let row = raw.put(1, "IFCTABLEROW", vec![cells(1), Value::Null]);
    let view = TabularView::new(&raw.0, ifc4()).unwrap();
    assert_eq!(
        view.table(EntityId(2)).unwrap_err(),
        TabularReadError::UnknownEntity { id: EntityId(2) }
    );
    assert!(matches!(
        view.table(row).unwrap_err(),
        TabularReadError::WrongEntityType { actual, .. } if actual == "IFCTABLEROW"
    ));
    assert!(matches!(
        view.time_series(row).unwrap_err(),
        TabularReadError::WrongEntityType { .. }
    ));
    assert!(matches!(
        TabularView::new(&raw.0, ifc2x3()).unwrap_err(),
        TabularReadError::UnsupportedSchema { .. }
    ));
}
