//! Tables of values by path (#474).
//!
//! `openbim_ifc_v0_1_model_export_table` writes the entities a selector
//! selects as CSV, TSV or XLSX bytes into the caller's buffer;
//! `openbim_ifc_v0_1_model_import_table` applies a table back by key as one
//! checked transaction and answers with a `TableImport` record tape:
//! applied (`BOOL`), changes (`LIST` of `TableChange`: row, column
//! `INTEGER`; key, path `TEXT`; entity, target entity `REF`; target,
//! before, after `TEXT`), refusals (`LIST` of `TableRefusal`: row, column
//! `INTEGER` or `NULL`; code, message `TEXT`; span `LIST` of two
//! `INTEGER`s or `NULL`). Any refusal means nothing was written.
//!
//! The import never applies a table it cannot report: it plans first, and
//! a size query or a short buffer answers `BufferTooSmall` with the sizes
//! and leaves the model unchanged.

use openbim_ifc_binding_core::{BindingError, IfcModel, TableImport, ToRecord};

use crate::buffer::{bytes, fill, text};
use crate::domains::Out;
use crate::model::{done, with_model, OpenbimIfcModel};
use crate::status::{boundary, OpenbimIfcStatus};
use crate::tape::{OpenbimIfcValueNode, Tape};

/// Table format: RFC 4180 CSV, UTF-8.
pub const OPENBIM_IFC_TABLE_CSV: u32 = 0;
/// Table format: tab-separated values with `\t`, `\n`, `\r`, `\\` escapes.
pub const OPENBIM_IFC_TABLE_TSV: u32 = 1;
/// Table format: an XLSX workbook, every cell text.
pub const OPENBIM_IFC_TABLE_XLSX: u32 = 2;
/// Key column: `GlobalId` for an export; for an import `GlobalId` when the
/// table has one, else `id`.
pub const OPENBIM_IFC_TABLE_KEY_AUTO: u32 = 0;
/// Key column: `GlobalId`.
pub const OPENBIM_IFC_TABLE_KEY_GLOBAL_ID: u32 = 1;
/// Key column: `id`, the entity's `#id`.
pub const OPENBIM_IFC_TABLE_KEY_ID: u32 = 2;
/// `openbim_ifc_v0_1_model_import_table` flag: plan and report only.
pub const OPENBIM_IFC_TABLE_DRY_RUN: u32 = 1;

fn format_name(format: u32) -> Option<&'static str> {
    match format {
        OPENBIM_IFC_TABLE_CSV => Some("csv"),
        OPENBIM_IFC_TABLE_TSV => Some("tsv"),
        OPENBIM_IFC_TABLE_XLSX => Some("xlsx"),
        _ => None,
    }
}

fn key_name(key: u32) -> Result<Option<&'static str>, OpenbimIfcStatus> {
    match key {
        OPENBIM_IFC_TABLE_KEY_AUTO => Ok(None),
        OPENBIM_IFC_TABLE_KEY_GLOBAL_ID => Ok(Some("GlobalId")),
        OPENBIM_IFC_TABLE_KEY_ID => Ok(Some("id")),
        _ => Err(OpenbimIfcStatus::InvalidArgument),
    }
}

/// The entities selector `query` (UTF-8) selects as a table in `format`
/// (`OPENBIM_IFC_TABLE_CSV`, `_TSV`, `_XLSX`), filling `buffer` as
/// `openbim_ifc_v0_1_model_write` does: the key column first (`key`:
/// `OPENBIM_IFC_TABLE_KEY_AUTO` or `_GLOBAL_ID` for `GlobalId`, `_ID` for
/// `id`), then one column per value path of `columns` (`column_count`
/// UTF-8 paths of `column_lengths[i]` bytes, e.g. `type.Name`,
/// `Pset_WallCommon.FireRating`), each cell the value the path reads as
/// text, `NULL` as `null` (UTF-8, may be empty).
///
/// `InvalidSelector` (span through `openbim_ifc_v0_1_last_error_span`) for
/// a query or path that does not parse; `WrongEntityType` for an entity
/// without a `GlobalId` in a table keyed by it; `InvalidArgument` for an
/// unknown format or key, or text that is not UTF-8; `FeatureDisabled`;
/// and as for `openbim_ifc_v0_1_model_select`.
///
/// # Safety
/// `query`, `null` valid for their lengths; `columns` and
/// `column_lengths` null (with `column_count` 0) or valid for
/// `column_count` reads, each column valid for its length; the buffer as
/// for `openbim_ifc_v0_1_model_write`.
#[no_mangle]
pub unsafe extern "C" fn openbim_ifc_v0_1_model_export_table(
    model: OpenbimIfcModel,
    query: *const u8,
    query_len: usize,
    columns: *const *const u8,
    column_lengths: *const usize,
    column_count: usize,
    format: u32,
    key: u32,
    null: *const u8,
    null_len: usize,
    buffer: *mut u8,
    capacity: usize,
    out_required: *mut usize,
) -> OpenbimIfcStatus {
    let (Some(format), Ok(key)) = (format_name(format), key_name(key)) else {
        return OpenbimIfcStatus::InvalidArgument;
    };
    if column_count > 0 && (columns.is_null() || column_lengths.is_null()) {
        return OpenbimIfcStatus::NullPointer;
    }
    boundary(|| {
        // SAFETY: caller contract above.
        let inputs = unsafe { (text(query, query_len), text(null, null_len)) };
        let (query, null) = match inputs {
            (Ok(query), Ok(null)) => (query, null),
            (Err(status), _) | (_, Err(status)) => return status,
        };
        let mut paths = Vec::with_capacity(column_count);
        for index in 0..column_count {
            // SAFETY: both arrays hold `column_count` items (caller contract).
            let (column, length) = unsafe { (*columns.add(index), *column_lengths.add(index)) };
            // SAFETY: each column is valid for its length (caller contract).
            match unsafe { text(column, length) } {
                Ok(path) => paths.push(path.to_owned()),
                Err(status) => return status,
            }
        }
        with_model(model, |m| {
            let table = m.export_table(query, &paths, format, key, null)?;
            // SAFETY: caller contract above.
            done(unsafe { fill(&table, buffer, capacity, out_required) })
        })
    })
}

/// Apply the table `data` (`data_len` bytes in `format`) to the model by
/// its key column (`key`, as for `openbim_ifc_v0_1_model_export_table`) as
/// one checked transaction, `null` (UTF-8) being the cell text of `NULL`.
/// `flags`: `OPENBIM_IFC_TABLE_DRY_RUN`. The answer is a `TableImport`
/// tape: applied (`BOOL`), changes (`LIST` of `TableChange`: row, column
/// `INTEGER`, key, path `TEXT`, entity, target entity `REF`, target,
/// before, after `TEXT`), refusals (`LIST` of `TableRefusal`: row, column
/// `INTEGER` or `NULL`, code, message `TEXT`, span `LIST` of two
/// `INTEGER`s or `NULL`). Any refusal means nothing was written; a size
/// query or a short buffer plans the import and writes nothing.
///
/// `Parse` for bytes that are no table of the format; `InvalidArgument`
/// for an unknown format, key or flag, or a `null` that is not UTF-8;
/// `FeatureDisabled`; `UnsupportedSchema`.
///
/// # Safety
/// `data` and `null` valid for their lengths; the buffers as for
/// `openbim_ifc_v0_1_entity_attribute`.
#[no_mangle]
pub unsafe extern "C" fn openbim_ifc_v0_1_model_import_table(
    model: OpenbimIfcModel,
    data: *const u8,
    data_len: usize,
    format: u32,
    key: u32,
    null: *const u8,
    null_len: usize,
    flags: u32,
    nodes: *mut OpenbimIfcValueNode,
    node_capacity: usize,
    out_nodes_required: *mut usize,
    strings: *mut u8,
    string_capacity: usize,
    out_strings_required: *mut usize,
) -> OpenbimIfcStatus {
    let (Some(format), Ok(key)) = (format_name(format), key_name(key)) else {
        return OpenbimIfcStatus::InvalidArgument;
    };
    if flags & !OPENBIM_IFC_TABLE_DRY_RUN != 0 {
        return OpenbimIfcStatus::InvalidArgument;
    }
    let out = Out {
        nodes,
        node_capacity,
        out_nodes_required,
        strings,
        string_capacity,
        out_strings_required,
    };
    let dry_run = flags & OPENBIM_IFC_TABLE_DRY_RUN != 0;
    boundary(|| {
        // SAFETY: caller contract above.
        let inputs = unsafe { (bytes(data, data_len), text(null, null_len)) };
        let (data, null) = match inputs {
            (Ok(data), Ok(null)) => (data, null),
            (Err(status), _) | (_, Err(status)) => return status,
        };
        with_model(model, |m| {
            let run = |m: &mut IfcModel, plan: bool| -> Result<TableImport, BindingError> {
                m.import_table(data, format, key, null, plan)
            };
            let planned = run(m, true)?;
            let tape = Tape::encode(&planned.to_record().to_tagged());
            let fits =
                tape.nodes.len() <= out.node_capacity && tape.strings.len() <= out.string_capacity;
            let report = if dry_run || !fits || !planned.refusals.is_empty() {
                planned
            } else {
                run(m, false)?
            };
            // SAFETY: caller contract above.
            unsafe { out.write(&report.to_record().to_tagged()) }
        })
    })
}
