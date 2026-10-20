//! CSV (RFC 4180) and TSV, read and written.
//!
//! CSV quotes a field holding a comma, a quote, a line break, or leading or
//! trailing whitespace, doubling inner quotes; records end in CRLF. A
//! reader accepts CRLF or LF, a leading UTF-8 byte-order mark (spreadsheet
//! applications write one), and refuses a quote inside an unquoted field
//! or an unterminated quoted one by row and column, rather than guessing
//! where the field ends.
//!
//! TSV has no quoting; a field's tab, line break and backslash are escaped
//! as `\t`, `\n`, `\r` and `\\`, the convention of PostgreSQL's text
//! format and "linear TSV". Any other backslash escape is refused.

use super::grid::{Sheet, SheetFormatError};

/// `sheet` as CSV.
pub(super) fn write_csv(sheet: &Sheet) -> Vec<u8> {
    let mut out = String::new();
    for record in std::iter::once(&sheet.header).chain(&sheet.rows) {
        for (index, field) in record.iter().enumerate() {
            if index > 0 {
                out.push(',');
            }
            let quote = field.contains([',', '"', '\n', '\r'])
                || field.starts_with(char::is_whitespace)
                || field.ends_with(char::is_whitespace);
            if quote {
                out.push('"');
                out.push_str(&field.replace('"', "\"\""));
                out.push('"');
            } else {
                out.push_str(field);
            }
        }
        out.push_str("\r\n");
    }
    out.into_bytes()
}

/// `sheet` as TSV.
pub(super) fn write_tsv(sheet: &Sheet) -> Vec<u8> {
    let mut out = String::new();
    for record in std::iter::once(&sheet.header).chain(&sheet.rows) {
        for (index, field) in record.iter().enumerate() {
            if index > 0 {
                out.push('\t');
            }
            for c in field.chars() {
                match c {
                    '\\' => out.push_str("\\\\"),
                    '\t' => out.push_str("\\t"),
                    '\n' => out.push_str("\\n"),
                    '\r' => out.push_str("\\r"),
                    other => out.push(other),
                }
            }
        }
        out.push('\n');
    }
    out.into_bytes()
}

/// The text of a delimited file, without its byte-order mark.
fn text(bytes: &[u8]) -> Result<&str, SheetFormatError> {
    let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
    std::str::from_utf8(bytes).map_err(|error| {
        let row = bytes[..error.valid_up_to()]
            .iter()
            .filter(|byte| **byte == b'\n')
            .count()
            + 1;
        SheetFormatError::Malformed {
            row,
            column: None,
            detail: format!("not UTF-8 at byte {}", error.valid_up_to()),
        }
    })
}

/// A CSV file's records.
pub(super) fn read_csv(bytes: &[u8]) -> Result<Sheet, SheetFormatError> {
    let text = text(bytes)?;
    let mut records: Vec<Vec<String>> = Vec::new();
    let mut record: Vec<String> = Vec::new();
    let mut field = String::new();
    let mut chars = text.chars().peekable();
    // Where the current field started: quoted, and whether it is open.
    let mut quoted = false;
    let mut in_quotes = false;
    let mut at_start = true;
    let malformed = |records: &Vec<Vec<String>>, record: &Vec<String>, detail: &str| {
        SheetFormatError::Malformed {
            row: records.len() + 1,
            column: Some(record.len() + 1),
            detail: detail.to_owned(),
        }
    };
    while let Some(c) = chars.next() {
        if in_quotes {
            if c == '"' {
                if chars.peek() == Some(&'"') {
                    chars.next();
                    field.push('"');
                } else {
                    in_quotes = false;
                }
            } else {
                field.push(c);
            }
            continue;
        }
        match c {
            '"' if at_start => {
                quoted = true;
                in_quotes = true;
                at_start = false;
            }
            '"' => {
                return Err(malformed(
                    &records,
                    &record,
                    "a quote inside an unquoted field; quote the whole field",
                ))
            }
            ',' => {
                record.push(std::mem::take(&mut field));
                quoted = false;
                at_start = true;
            }
            '\r' | '\n' => {
                if c == '\r' && chars.peek() == Some(&'\n') {
                    chars.next();
                }
                record.push(std::mem::take(&mut field));
                records.push(std::mem::take(&mut record));
                quoted = false;
                at_start = true;
            }
            other if quoted => {
                return Err(malformed(
                    &records,
                    &record,
                    &format!("{other:?} after a closing quote"),
                ))
            }
            other => {
                field.push(other);
                at_start = false;
            }
        }
    }
    if in_quotes {
        return Err(malformed(&records, &record, "an unterminated quoted field"));
    }
    // The last record, when the file does not end in a line break.
    if !at_start || quoted || !record.is_empty() {
        record.push(field);
        records.push(record);
    }
    Ok(sheet_of(records))
}

/// A TSV file's records.
pub(super) fn read_tsv(bytes: &[u8]) -> Result<Sheet, SheetFormatError> {
    let text = text(bytes)?;
    let mut records = Vec::new();
    let body = text.strip_suffix('\n').unwrap_or(text);
    if body.is_empty() {
        return Ok(Sheet::default());
    }
    for (row, line) in body.split('\n').enumerate() {
        let line = line.strip_suffix('\r').unwrap_or(line);
        let mut record = Vec::new();
        for (column, raw) in line.split('\t').enumerate() {
            record.push(unescape(raw).map_err(|detail| SheetFormatError::Malformed {
                row: row + 1,
                column: Some(column + 1),
                detail,
            })?);
        }
        records.push(record);
    }
    Ok(sheet_of(records))
}

fn unescape(raw: &str) -> Result<String, String> {
    let mut out = String::with_capacity(raw.len());
    let mut chars = raw.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('\\') => out.push('\\'),
            Some('t') => out.push('\t'),
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some(other) => {
                return Err(format!(
                    "unknown escape \\{other}; write a backslash as \\\\"
                ))
            }
            None => return Err("a backslash ends the field; write it as \\\\".to_owned()),
        }
    }
    Ok(out)
}

/// The first record as the header, the rest as rows.
fn sheet_of(mut records: Vec<Vec<String>>) -> Sheet {
    if records.is_empty() {
        return Sheet::default();
    }
    let header = records.remove(0);
    Sheet {
        header,
        rows: records,
    }
}
