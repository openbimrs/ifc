//! Validation, with and without EXPRESS rules (#538).

use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

use super::{index, IfcModel};
use crate::error::js_error;
use crate::records;

#[wasm_bindgen]
impl IfcModel {
    /// Validate against the schema the header declares; findings are
    /// sorted by severity, rule, entity and slot. `maxFindings` caps the
    /// report (default 10,000) and sets `truncated` when reached.
    #[wasm_bindgen(js_name = validate, unchecked_return_type = "ValidationReport")]
    pub fn validate_js(
        &self,
        #[wasm_bindgen(js_name = maxFindings)] max_findings: Option<u32>,
    ) -> Result<JsValue, JsValue> {
        let report = self.0.validate(max_findings.map(index)).map_err(js_error)?;
        Ok(records::report_to_js(&report))
    }

    /// Validate with the EXPRESS rules of the declared release, read from
    /// `express`, the text of that release's `.exp` file (#538). Throws
    /// `express-mismatch` for text that declares another schema or
    /// disagrees with the release's tables, and `invalid-express` for text
    /// that does not parse. `maxFindings` is as for `validate`.
    #[wasm_bindgen(js_name = validateExpress, unchecked_return_type = "ValidationReport")]
    pub fn validate_express_js(
        &self,
        express: &str,
        #[wasm_bindgen(js_name = maxFindings)] max_findings: Option<u32>,
    ) -> Result<JsValue, JsValue> {
        let report = self
            .0
            .validate_express(express, max_findings.map(index))
            .map_err(js_error)?;
        Ok(records::report_to_js(&report))
    }
}
