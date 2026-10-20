//! Tables of values by path (#474): `exportTable` and `importTable`.

use js_sys::Uint8Array;
use openbim_ifc_binding_core::ToRecord;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

use super::IfcModel;
use crate::error::js_error;
use crate::records;

#[wasm_bindgen]
impl IfcModel {
    /// The entities selector `query` selects as a table (#474), as bytes:
    /// `options.format` `"csv"` (default), `"tsv"` or `"xlsx"` (only in a
    /// build with `sheet-xlsx`; the npm package throws
    /// `feature-disabled`), the key column first (`"GlobalId"`, or `"id"`
    /// with `options.key`), then one column per value path of `columns`
    /// (`"Name"`, `"type.Name"`, `"Pset_WallCommon.FireRating"`), each cell
    /// the value the path reads as text, `NULL` as `options.null` (empty by
    /// default). Throws `invalid-selector` for a query or path that does
    /// not parse, `invalid-value` for an unknown format or key, and
    /// `wrong-entity-type` for an entity without a `GlobalId` in a table
    /// keyed by it.
    #[wasm_bindgen(js_name = exportTable)]
    pub fn export_table_js(
        &self,
        query: &str,
        columns: Vec<String>,
        #[wasm_bindgen(unchecked_param_type = "TableOptions | undefined")] options: &JsValue,
    ) -> Result<Uint8Array, JsValue> {
        let format = records::text(options, "format").map_err(js_error)?;
        let key = records::text(options, "key").map_err(js_error)?;
        let null = records::text(options, "null").map_err(js_error)?;
        let table = self
            .0
            .export_table(
                query,
                &columns,
                format.as_deref().unwrap_or("csv"),
                key.as_deref(),
                null.as_deref().unwrap_or(""),
            )
            .map_err(js_error)?;
        Ok(Uint8Array::from(table.as_slice()))
    }

    /// Apply a table (`Uint8Array`) back to the model by its key column
    /// (#474), as one checked transaction: every other cell is written by
    /// its header's value path, typed by the attribute's declared type or
    /// the property's stated one; a cell that reads what the model holds
    /// changes nothing, so an exported table imported unchanged is a no-op.
    /// The answer lists every change and every refusal by row and column;
    /// any refusal means nothing was written, as does `options.dryRun`.
    /// Throws only for a call that cannot start: an unknown format or key,
    /// bytes that are no table of the format.
    #[wasm_bindgen(js_name = importTable, unchecked_return_type = "TableImport")]
    pub fn import_table_js(
        &mut self,
        data: &[u8],
        #[wasm_bindgen(unchecked_param_type = "TableImportOptions | undefined")] options: &JsValue,
    ) -> Result<JsValue, JsValue> {
        let format = records::text(options, "format").map_err(js_error)?;
        let key = records::text(options, "key").map_err(js_error)?;
        let null = records::text(options, "null").map_err(js_error)?;
        let dry_run = records::flag(options, "dryRun").map_err(js_error)?;
        let report = self
            .0
            .import_table(
                data,
                format.as_deref().unwrap_or("csv"),
                key.as_deref(),
                null.as_deref().unwrap_or(""),
                dry_run,
            )
            .map_err(js_error)?;
        Ok(records::record_to_js(&report.to_record()))
    }
}
