//! XLSX: `rust_xlsxwriter` writes a workbook, `calamine` reads one
//! (feature `sheet-xlsx`).
//!
//! Every cell is written as text, so a `GlobalId`, a leading zero or a
//! real's last digit survives a spreadsheet application unchanged; the
//! header row is bold, frozen and filterable. Reading takes the first
//! worksheet from cell A1: a number a user typed reads as the shortest
//! decimal of its value (`5`, `2.75`), a boolean as `TRUE` or `FALSE`. A
//! date, a duration or an error cell is refused by row and column, since
//! no IFC value is written from it.
#![cfg(feature = "sheet-xlsx")]

use std::io::Cursor;

use calamine::{Data, Reader, Xlsx};
use rust_xlsxwriter::{Format, Workbook};

use super::grid::{Sheet, SheetFormatError};

/// The worksheet's limits (Office Open XML as Excel enforces it).
const MAX_ROWS: usize = 1_048_576;
const MAX_COLUMNS: usize = 16_384;
const MAX_CELL_CHARS: usize = 32_767;

/// `sheet` as a workbook of one worksheet named `IFC`.
pub(super) fn write(sheet: &Sheet) -> Result<Vec<u8>, SheetFormatError> {
    let rows = sheet.rows.len() + 1;
    if rows > MAX_ROWS {
        return Err(SheetFormatError::TooLarge(format!(
            "{rows} rows; a worksheet holds {MAX_ROWS}"
        )));
    }
    let records = std::iter::once(&sheet.header).chain(&sheet.rows);
    let width = records.clone().map(Vec::len).max().unwrap_or(0);
    if width > MAX_COLUMNS {
        return Err(SheetFormatError::TooLarge(format!(
            "{width} columns; a worksheet holds {MAX_COLUMNS}"
        )));
    }
    let xlsx = |error: rust_xlsxwriter::XlsxError| SheetFormatError::Xlsx(error.to_string());
    let mut workbook = Workbook::new();
    let bold = Format::new().set_bold();
    let worksheet = workbook.add_worksheet();
    worksheet.set_name("IFC").map_err(xlsx)?;
    for (row, record) in records.enumerate() {
        for (column, text) in record.iter().enumerate() {
            let chars = text.chars().count();
            if chars > MAX_CELL_CHARS {
                return Err(SheetFormatError::TooLarge(format!(
                    "row {}, column {}: {chars} characters; a cell holds {MAX_CELL_CHARS}",
                    row + 1,
                    super::grid::column_name(column + 1)
                )));
            }
            // Bounded by the checks above.
            #[allow(clippy::cast_possible_truncation)]
            let (r, c) = (row as u32, column as u16);
            if row == 0 {
                worksheet
                    .write_string_with_format(r, c, text, &bold)
                    .map_err(xlsx)?;
            } else {
                worksheet.write_string(r, c, text).map_err(xlsx)?;
            }
        }
    }
    if width > 0 {
        worksheet.set_freeze_panes(1, 0).map_err(xlsx)?;
        // Bounded by the checks above.
        #[allow(clippy::cast_possible_truncation)]
        let (last_row, last_column) = ((rows - 1) as u32, (width - 1) as u16);
        worksheet
            .autofilter(0, 0, last_row, last_column)
            .map_err(xlsx)?;
    }
    workbook.save_to_buffer().map_err(xlsx)
}

/// The first worksheet of a workbook, from cell A1.
pub(super) fn read(bytes: &[u8]) -> Result<Sheet, SheetFormatError> {
    let mut workbook =
        Xlsx::new(Cursor::new(bytes)).map_err(|error| SheetFormatError::Xlsx(error.to_string()))?;
    let range = match workbook.worksheet_range_at(0) {
        None => return Ok(Sheet::default()),
        Some(range) => range.map_err(|error| SheetFormatError::Xlsx(error.to_string()))?,
    };
    let Some((last_row, last_column)) = range.end() else {
        return Ok(Sheet::default());
    };
    let mut records = Vec::new();
    for row in 0..=last_row {
        let mut record = Vec::new();
        for column in 0..=last_column {
            let text = match range.get_value((row, column)) {
                None | Some(Data::Empty) => String::new(),
                Some(Data::String(text)) => text.clone(),
                Some(Data::Int(i)) => i.to_string(),
                Some(Data::Float(f)) => f.to_string(),
                Some(Data::Bool(true)) => "TRUE".to_owned(),
                Some(Data::Bool(false)) => "FALSE".to_owned(),
                Some(other) => {
                    let what = match other {
                        Data::Error(error) => format!("an error cell ({error})"),
                        _ => "a date or duration cell; format the column as text".to_owned(),
                    };
                    return Err(SheetFormatError::Malformed {
                        row: row as usize + 1,
                        column: Some(column as usize + 1),
                        detail: what,
                    });
                }
            };
            record.push(text);
        }
        records.push(record);
    }
    // A worksheet's used range is rectangular; trailing empty cells of the
    // header past its last name are no columns.
    let mut header = if records.is_empty() {
        Vec::new()
    } else {
        records.remove(0)
    };
    while header.last().is_some_and(String::is_empty) {
        header.pop();
    }
    let width = header.len();
    let mut rows = Vec::with_capacity(records.len());
    for (index, mut record) in records.into_iter().enumerate() {
        if let Some(extra) = record.iter().skip(width).position(|cell| !cell.is_empty()) {
            return Err(SheetFormatError::Malformed {
                row: index + 2,
                column: Some(width + extra + 1),
                detail: "a value in a column with no header".to_owned(),
            });
        }
        record.truncate(width);
        rows.push(record);
    }
    Ok(Sheet { header, rows })
}
