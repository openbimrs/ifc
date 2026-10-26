//! Tables of values by path, exported and imported (#474): the facade's
//! counterpart of IfcOpenShell's `ifccsv`.
//!
//! [`export`] writes entities as rows of a [`Sheet`], one column per
//! [`ValuePath`] (`Name`, `type.Name`,
//! `Pset_WallCommon.FireRating`, `Qto_WallBaseQuantities.Length`,
//! `container.Name`), each header the path's canonical text, keyed by the
//! entity's `GlobalId` or `id` in the first column. [`plan_import`] reads
//! a sheet back: rows are matched by their key, and every other cell is
//! written by its header's path, typed by the attribute's declared type or
//! the property's stated one, as one transaction. [`import`] commits it.
//!
//! # Semantics
//!
//! - A cell whose text is what its path reads now changes nothing, so an
//!   exported table imported unchanged is a no-op: no write, no new
//!   revision.
//! - Every refusal is collected by row and column -- a header that is no
//!   path, a missing, empty, unknown, ambiguous or repeated key, a row of
//!   another width, a cell that does not read as its target's type, two
//!   rows writing one type's name differently -- and any refusal refuses
//!   the whole sheet. Nothing is written unless everything can be.
//! - The empty cell is `NULL` (`$`) unless [`SheetOptions::null`] names
//!   another text (IfcOpenShell's `--null`); with the default an empty
//!   string cannot be written.
//! - Writes go through the checked set-by-path API: an attribute's value is
//!   checked against the release, a property through the checked property
//!   edit, `Pset_` and `Qto_` sets against the PSD/QTO catalog when the
//!   build has one.
//!
//! # Formats
//!
//! [`Sheet::read`] and [`Sheet::write`] take CSV (RFC 4180) and TSV
//! always, and XLSX with feature `sheet-xlsx` (the `rust_xlsxwriter`
//! writer and the `calamine` reader, both pure Rust). Every XLSX cell is
//! written as text.
//!
//! ```
//! # #[cfg(all(feature = "step", feature = "ifc4"))] {
//! use ifc::selector::{SelectorIndex, ValuePath};
//! use ifc::sheet::{export, import, Sheet, SheetFormat, SheetOptions};
//! use ifc::{Codec, EntityId, StepCodec};
//!
//! let mut model = StepCodec.read_bytes(b"ISO-10303-21;HEADER;FILE_DESCRIPTION((''),'2;1');\
//!     FILE_NAME('','',(''),(''),'','','');FILE_SCHEMA(('IFC4'));ENDSEC;DATA;\
//!     #1=IFCWALL('31t64Uzj98DhdKplMxBpF5',$,'Wall',$,$,$,$,$,$);\
//!     ENDSEC;END-ISO-10303-21;").unwrap();
//! let schema = ifc::schema::ifc4();
//! let options = SheetOptions::default();
//!
//! let columns = [ValuePath::parse("Name").unwrap()];
//! let index = SelectorIndex::new(&model, schema);
//! let sheet = export(&index, &[EntityId(1)], &columns, &options).unwrap();
//! let csv = String::from_utf8(sheet.write(SheetFormat::Csv).unwrap()).unwrap();
//! assert_eq!(csv, "GlobalId,Name\r\n31t64Uzj98DhdKplMxBpF5,Wall\r\n");
//!
//! let edited = Sheet::read(csv.replace(",Wall", ",Wall 1").as_bytes(), SheetFormat::Csv).unwrap();
//! let changes = import(&mut model, schema, &edited, &options, ifc::fresh_seed()).unwrap();
//! assert_eq!(changes.len(), 1);
//! # }
//! ```
#![cfg(feature = "sheet")]

mod delimited;
mod export;
mod grid;
mod import;
mod xlsx;

pub use export::{export, ExportError};
pub use grid::{column_name, Sheet, SheetFormat, SheetFormatError};
pub use import::{import, plan_import, CellChange, ImportPlan, SheetRefusal, SheetRefusalReason};

use crate::selector::{Key, ValuePath};

/// The column a row is matched by.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum KeyColumn {
    /// `GlobalId`: stable across files and tools; only `IfcRoot` entities
    /// carry one.
    #[default]
    GlobalId,
    /// `id`, the entity's `#id` in this file: any entity has one, but it
    /// may change when another tool writes the file.
    Id,
}

impl KeyColumn {
    /// The column's header, which is also the path it reads.
    #[must_use]
    pub const fn header(self) -> &'static str {
        match self {
            Self::GlobalId => "GlobalId",
            Self::Id => "id",
        }
    }

    /// The column a name (any case) names: `globalid`, `guid`, `id`.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_ascii_lowercase().as_str() {
            "globalid" | "global-id" | "guid" => Some(Self::GlobalId),
            "id" => Some(Self::Id),
            _ => None,
        }
    }

    pub(crate) fn path(self) -> ValuePath {
        // Both headers are paths of one plain key.
        ValuePath::parse(self.header()).unwrap_or_else(|_| unreachable!("a one-key path"))
    }

    /// Whether `path` reads this column: `GlobalId` in any case, as the
    /// attribute resolves, or exactly `id`, the keyword.
    pub(crate) fn is(self, path: &ValuePath) -> bool {
        let [only] = path.keys.as_slice() else {
            return false;
        };
        let Key::Text(text) = &only.key else {
            return false;
        };
        match self {
            Self::GlobalId => text.eq_ignore_ascii_case("GlobalId"),
            Self::Id => text == "id",
        }
    }
}

/// How a sheet is keyed and how `NULL` is written.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct SheetOptions {
    /// The key column. Export writes `GlobalId` when `None`; import uses
    /// `GlobalId` when the header has it, else `id`.
    pub key: Option<KeyColumn>,
    /// The text of `NULL` (`$`), empty by default.
    pub null: String,
}

impl SheetOptions {
    /// These options keyed by `key`.
    #[must_use]
    pub fn with_key(mut self, key: KeyColumn) -> Self {
        self.key = Some(key);
        self
    }

    /// These options writing `NULL` as `null`.
    #[must_use]
    pub fn with_null(mut self, null: impl Into<String>) -> Self {
        self.null = null.into();
        self
    }
}

#[cfg(test)]
mod tests;
