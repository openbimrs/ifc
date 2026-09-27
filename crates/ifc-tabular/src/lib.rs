//! Structured IFC value containers indexed by position or time.
//!
//! A table indexes values by row and column; a time series indexes them
//! by time. Neither carries domain meaning of its own: both bottom out in
//! lists of `IfcValue`, and both appear side by side in the schema's own
//! `IfcMetricValueSelect` and `IfcObjectReferenceSelect`.
//!
//! # Cells are the caller's to type
//!
//! A cell is an `IfcValue`: a SELECT over measure types. `4.2` and
//! `IFCLENGTHMEASURE(4.2)` are different files, and only the second can be
//! converted or compared dimensionally. This crate never invents a measure;
//! the caller passes a `Value` and chooses.
//!
//! # Derived attributes are not stored
//!
//! `IfcTable` DERIVEs three counts from its rows. They are derived under new
//! names rather than redeclaring an inherited attribute, so they occupy no
//! instance slot: nothing is written for them, not even `*`, and they are
//! never computed into the file.
//!
//! # Reading back
//!
//! [`TabularView`] borrows a model and returns [`Table`] and
//! [`TimeSeries`] views under a declared IFC4 or IFC4X3 schema. A read
//! reports every malformed slot and WR1/WR2 violation as a
//! [`TabularIssue`] rather than dropping it.

mod error;
mod read;
mod series;
mod table;

pub use error::{TabularError, TabularReadError, TabularReadResult, TabularResult};
pub use read::{SeriesValue, TabularIssue, TabularView, TimeSeries, TimeSeriesKind};
pub use read::{Table, TableColumn, TableRow};
pub use series::{add_irregular_time_series, add_regular_time_series, SeriesDraft};
pub use series::{add_irregular_value, add_time_series_value};
pub use table::{add_table, add_table_column, add_table_row, ColumnDraft};
