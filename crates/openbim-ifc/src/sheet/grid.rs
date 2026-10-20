//! [`Sheet`]: a header and rows of text, read from and written to CSV,
//! TSV and XLSX.

use std::fmt;

/// A table of text: a header naming each column and rows of cells.
///
/// Rows are numbered as a spreadsheet numbers them: the header is row 1,
/// `rows[0]` row 2. Reading keeps every record as it is, short or long;
/// [`plan_import`](super::plan_import) refuses a row whose width differs
/// from the header's.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct Sheet {
    /// The first row: one value path per column.
    pub header: Vec<String>,
    /// The other rows.
    pub rows: Vec<Vec<String>>,
}

impl Sheet {
    /// A sheet of `header` and `rows`.
    #[must_use]
    pub fn new(header: Vec<String>, rows: Vec<Vec<String>>) -> Self {
        Self { header, rows }
    }

    /// Read `bytes` as `format`. The first record is the header; a file
    /// with no record reads as an empty sheet.
    ///
    /// # Errors
    ///
    /// [`SheetFormatError::Malformed`] for text that is not UTF-8 or a
    /// record that does not follow the format, by row and column;
    /// [`SheetFormatError::Xlsx`] for a workbook that cannot be read;
    /// [`SheetFormatError::FeatureDisabled`] for XLSX in a build without
    /// `sheet-xlsx`.
    pub fn read(bytes: &[u8], format: SheetFormat) -> Result<Self, SheetFormatError> {
        match format {
            SheetFormat::Csv => super::delimited::read_csv(bytes),
            SheetFormat::Tsv => super::delimited::read_tsv(bytes),
            SheetFormat::Xlsx => read_xlsx(bytes),
        }
    }

    /// The sheet as `format`: CSV with CRLF line ends, TSV with escapes, or
    /// an XLSX workbook of one worksheet whose every cell is text.
    ///
    /// # Errors
    ///
    /// [`SheetFormatError::TooLarge`] for a sheet beyond a worksheet's
    /// limits; [`SheetFormatError::Xlsx`] when the workbook cannot be
    /// written; [`SheetFormatError::FeatureDisabled`] for XLSX in a build
    /// without `sheet-xlsx`.
    pub fn write(&self, format: SheetFormat) -> Result<Vec<u8>, SheetFormatError> {
        match format {
            SheetFormat::Csv => Ok(super::delimited::write_csv(self)),
            SheetFormat::Tsv => Ok(super::delimited::write_tsv(self)),
            SheetFormat::Xlsx => write_xlsx(self),
        }
    }
}

#[cfg(feature = "sheet-xlsx")]
fn read_xlsx(bytes: &[u8]) -> Result<Sheet, SheetFormatError> {
    super::xlsx::read(bytes)
}

#[cfg(not(feature = "sheet-xlsx"))]
fn read_xlsx(_bytes: &[u8]) -> Result<Sheet, SheetFormatError> {
    Err(SheetFormatError::FeatureDisabled("sheet-xlsx"))
}

#[cfg(feature = "sheet-xlsx")]
fn write_xlsx(sheet: &Sheet) -> Result<Vec<u8>, SheetFormatError> {
    super::xlsx::write(sheet)
}

#[cfg(not(feature = "sheet-xlsx"))]
fn write_xlsx(_sheet: &Sheet) -> Result<Vec<u8>, SheetFormatError> {
    Err(SheetFormatError::FeatureDisabled("sheet-xlsx"))
}

/// A file format of a [`Sheet`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SheetFormat {
    /// RFC 4180 comma-separated values, UTF-8.
    Csv,
    /// Tab-separated values, UTF-8, with `\t`, `\n`, `\r` and `\\`
    /// escapes.
    Tsv,
    /// An Office Open XML workbook (`.xlsx`); feature `sheet-xlsx`.
    Xlsx,
}

impl SheetFormat {
    /// The format a file extension (any case, without the dot) names:
    /// `csv`, `tsv` or `tab`, `xlsx`.
    #[must_use]
    pub fn from_extension(extension: &str) -> Option<Self> {
        match extension.to_ascii_lowercase().as_str() {
            "csv" => Some(Self::Csv),
            "tsv" | "tab" => Some(Self::Tsv),
            "xlsx" => Some(Self::Xlsx),
            _ => None,
        }
    }

    /// The format a name (any case) names: `csv`, `tsv`, `xlsx`.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_ascii_lowercase().as_str() {
            "csv" => Some(Self::Csv),
            "tsv" => Some(Self::Tsv),
            "xlsx" => Some(Self::Xlsx),
            _ => None,
        }
    }

    /// The format's name and usual extension: `csv`, `tsv`, `xlsx`.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Csv => "csv",
            Self::Tsv => "tsv",
            Self::Xlsx => "xlsx",
        }
    }
}

/// Why a sheet could not be read or written.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SheetFormatError {
    /// A record does not follow the format.
    Malformed {
        /// The one-based row, the header being row 1.
        row: usize,
        /// The one-based column, when the fault is in one field.
        column: Option<usize>,
        /// What is wrong.
        detail: String,
    },
    /// The sheet exceeds what the format holds: an XLSX worksheet's
    /// 1,048,576 rows, 16,384 columns or 32,767 characters per cell.
    TooLarge(String),
    /// The XLSX workbook could not be read or written.
    Xlsx(String),
    /// The format needs a facade feature this build leaves out
    /// (`sheet-xlsx`).
    FeatureDisabled(&'static str),
}

impl fmt::Display for SheetFormatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Malformed {
                row,
                column: Some(column),
                detail,
            } => write!(f, "row {row}, column {}: {detail}", column_name(*column)),
            Self::Malformed {
                row,
                column: None,
                detail,
            } => write!(f, "row {row}: {detail}"),
            Self::TooLarge(detail) => write!(f, "too large for the format: {detail}"),
            Self::Xlsx(detail) => write!(f, "XLSX: {detail}"),
            Self::FeatureDisabled(feature) => {
                write!(f, "this build leaves out the `{feature}` feature")
            }
        }
    }
}

impl std::error::Error for SheetFormatError {}

/// A one-based column as a spreadsheet names it: 1 is `A`, 27 `AA`.
#[must_use]
pub fn column_name(column: usize) -> String {
    let mut name = Vec::new();
    let mut rest = column;
    while rest > 0 {
        let digit = (rest - 1) % 26;
        // `digit` < 26.
        #[allow(clippy::cast_possible_truncation)]
        name.push(b'A' + digit as u8);
        rest = (rest - 1) / 26;
    }
    name.reverse();
    String::from_utf8(name).unwrap_or_default()
}
