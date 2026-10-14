//! `INVERSE` attributes and the entities referencing one (#462).
//!
//! `attributeByName` reads an `INVERSE` name itself; these list the
//! declarations and the referrers, through the core's cached reverse index.

use js_sys::Array;
use openbim_ifc_binding_core::record::to_records;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

use super::IfcModel;
use crate::error::js_error;
use crate::records;

#[wasm_bindgen]
impl IfcModel {
    /// Every `INVERSE` attribute of entity `id`, inherited ones first, as
    /// the declared release defines them (#462).
    #[wasm_bindgen(
        js_name = inverseAttributeNames,
        unchecked_return_type = "InverseAttributeInfo[]"
    )]
    pub fn inverse_attribute_names_js(&self, id: u64) -> Result<Array, JsValue> {
        let names = self.0.inverse_attribute_names(id).map_err(js_error)?;
        Ok(records::records_to_js(&to_records(&names)))
    }

    /// Every reference to entity `id`: the referencing entity and the
    /// attribute index it references through, in ascending order (#462).
    #[wasm_bindgen(js_name = inverseReferences, unchecked_return_type = "InverseReference[]")]
    pub fn inverse_references_js(&self, id: u64) -> Result<Array, JsValue> {
        let references = self.0.inverse_references(id).map_err(js_error)?;
        Ok(records::records_to_js(&to_records(&references)))
    }

    /// How many distinct entities reference entity `id` (#462).
    #[wasm_bindgen(js_name = totalInverses)]
    pub fn total_inverses_js(&self, id: u64) -> Result<u32, JsValue> {
        let total = self.0.total_inverses(id).map_err(js_error)?;
        Ok(u32::try_from(total).unwrap_or(u32::MAX))
    }
}
