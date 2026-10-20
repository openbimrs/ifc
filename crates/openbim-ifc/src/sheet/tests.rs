//! The grid codecs: CSV and TSV quoting and escapes, malformed records by
//! row and column, and XLSX round trips.

use super::*;

fn sheet(header: &[&str], rows: &[&[&str]]) -> Sheet {
    Sheet::new(
        header.iter().map(|cell| (*cell).to_owned()).collect(),
        rows.iter()
            .map(|row| row.iter().map(|cell| (*cell).to_owned()).collect())
            .collect(),
    )
}

/// Cells that need quoting or escaping in one format or another.
fn awkward() -> Sheet {
    sheet(
        &["GlobalId", "Name", "Description"],
        &[
            &[
                "0TG0wWfbZOvmJHW7AuHJ$k",
                "Wall, \"A\"",
                "line 1\nline 2\r\nend",
            ],
            &["0XLKvWwoErh4p4gJsHE2zX", " padded ", "tab\there \\ back"],
            &["19HFrQ$NB$TW0hRFxhxvxK", "", "=1+1"],
        ],
    )
}

#[test]
fn csv_and_tsv_round_trip_every_cell() {
    let original = awkward();
    for format in [SheetFormat::Csv, SheetFormat::Tsv] {
        let bytes = original.write(format).unwrap();
        assert_eq!(Sheet::read(&bytes, format).unwrap(), original, "{format:?}");
    }
}

#[test]
fn csv_quotes_only_what_needs_it() {
    let bytes = awkward().write(SheetFormat::Csv).unwrap();
    let text = String::from_utf8(bytes).unwrap();
    let mut lines = text.split("\r\n");
    assert_eq!(lines.next(), Some("GlobalId,Name,Description"));
    assert_eq!(
        lines.next(),
        Some("0TG0wWfbZOvmJHW7AuHJ$k,\"Wall, \"\"A\"\"\",\"line 1\nline 2")
    );
}

#[test]
fn tsv_escapes_tabs_breaks_and_backslashes() {
    let bytes = awkward().write(SheetFormat::Tsv).unwrap();
    let text = String::from_utf8(bytes).unwrap();
    assert!(text.contains("line 1\\nline 2\\r\\nend"), "{text}");
    assert!(text.contains("tab\\there \\\\ back"), "{text}");
}

#[test]
fn readers_accept_lf_crlf_a_bom_and_no_final_break() {
    let expected = sheet(&["id", "Name"], &[&["17", "Wall A"], &["18", ""]]);
    for text in [
        "id,Name\r\n17,Wall A\r\n18,\r\n",
        "id,Name\n17,Wall A\n18,",
        "\u{feff}id,Name\n17,\"Wall A\"\n18,\"\"\n",
    ] {
        assert_eq!(
            Sheet::read(text.as_bytes(), SheetFormat::Csv).unwrap(),
            expected,
            "{text:?}"
        );
    }
    let tsv = "id\tName\r\n17\tWall A\n18\t";
    assert_eq!(
        Sheet::read(tsv.as_bytes(), SheetFormat::Tsv).unwrap(),
        expected
    );
    assert_eq!(
        Sheet::read(b"", SheetFormat::Csv).unwrap(),
        Sheet::default()
    );
}

#[test]
fn malformed_records_are_refused_by_row_and_column() {
    let cases: &[(&str, SheetFormat, usize, Option<usize>)] = &[
        ("id,Name\n17,Wa\"ll\n", SheetFormat::Csv, 2, Some(2)),
        ("id,Name\n17,\"Wall\"x\n", SheetFormat::Csv, 2, Some(2)),
        ("id,Name\n17,\"Wall\n", SheetFormat::Csv, 2, Some(2)),
        ("id\tName\n17\tC:\\walls\n", SheetFormat::Tsv, 2, Some(2)),
        ("id\tName\n17\tWall\\\n", SheetFormat::Tsv, 2, Some(2)),
    ];
    for (text, format, row, column) in cases {
        match Sheet::read(text.as_bytes(), *format) {
            Err(SheetFormatError::Malformed {
                row: found_row,
                column: found_column,
                ..
            }) => assert_eq!((found_row, found_column), (*row, *column), "{text:?}"),
            other => panic!("{text:?}: {other:?}"),
        }
    }
    match Sheet::read(b"id,Name\n17,\xFF\n", SheetFormat::Csv) {
        Err(SheetFormatError::Malformed { row: 2, .. }) => {}
        other => panic!("{other:?}"),
    }
}

#[test]
fn columns_are_named_as_a_spreadsheet_names_them() {
    let names: Vec<String> = [1, 2, 26, 27, 52, 703].map(column_name).to_vec();
    assert_eq!(names, ["A", "B", "Z", "AA", "AZ", "AAA"]);
}

#[test]
fn formats_and_keys_are_named() {
    assert_eq!(SheetFormat::from_extension("CSV"), Some(SheetFormat::Csv));
    assert_eq!(SheetFormat::from_extension("tab"), Some(SheetFormat::Tsv));
    assert_eq!(SheetFormat::from_name("xlsx"), Some(SheetFormat::Xlsx));
    assert_eq!(SheetFormat::from_extension("ods"), None);
    assert_eq!(KeyColumn::from_name("GUID"), Some(KeyColumn::GlobalId));
    assert_eq!(KeyColumn::from_name("id"), Some(KeyColumn::Id));
    assert!(KeyColumn::GlobalId.is(&ValuePath::parse("globalid").unwrap()));
    assert!(!KeyColumn::Id.is(&ValuePath::parse("ID").unwrap()));
    assert!(!KeyColumn::GlobalId.is(&ValuePath::parse("type.GlobalId").unwrap()));
}

#[cfg(feature = "sheet-xlsx")]
#[test]
fn xlsx_round_trips_every_cell_as_text() {
    let original = awkward();
    let bytes = original.write(SheetFormat::Xlsx).unwrap();
    assert!(bytes.starts_with(b"PK"), "an Office Open XML package");
    assert_eq!(Sheet::read(&bytes, SheetFormat::Xlsx).unwrap(), original);
}

#[cfg(feature = "sheet-xlsx")]
#[test]
fn xlsx_reads_typed_cells_and_refuses_dates() {
    let mut workbook = rust_xlsxwriter::Workbook::new();
    let worksheet = workbook.add_worksheet();
    worksheet.write_string(0, 0, "id").unwrap();
    worksheet.write_string(0, 1, "Width").unwrap();
    worksheet.write_string(0, 2, "IsExternal").unwrap();
    worksheet.write_number(1, 0, 17.0).unwrap();
    worksheet.write_number(1, 1, 2.75).unwrap();
    worksheet.write_boolean(1, 2, true).unwrap();
    let bytes = workbook.save_to_buffer().unwrap();
    assert_eq!(
        Sheet::read(&bytes, SheetFormat::Xlsx).unwrap(),
        sheet(&["id", "Width", "IsExternal"], &[&["17", "2.75", "TRUE"]])
    );

    let mut workbook = rust_xlsxwriter::Workbook::new();
    let worksheet = workbook.add_worksheet();
    worksheet.write_string(0, 0, "id").unwrap();
    let date = rust_xlsxwriter::ExcelDateTime::from_ymd(2026, 10, 20).unwrap();
    let format = rust_xlsxwriter::Format::new().set_num_format("yyyy-mm-dd");
    worksheet
        .write_datetime_with_format(1, 0, &date, &format)
        .unwrap();
    let bytes = workbook.save_to_buffer().unwrap();
    match Sheet::read(&bytes, SheetFormat::Xlsx) {
        Err(SheetFormatError::Malformed {
            row: 2,
            column: Some(1),
            ..
        }) => {}
        other => panic!("{other:?}"),
    }
}

#[cfg(not(feature = "sheet-xlsx"))]
#[test]
fn xlsx_needs_its_feature() {
    assert_eq!(
        awkward().write(SheetFormat::Xlsx),
        Err(SheetFormatError::FeatureDisabled("sheet-xlsx"))
    );
}
