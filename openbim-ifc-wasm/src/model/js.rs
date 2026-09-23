//! JavaScript surface of [`IfcModel`].
//!
//! Each export converts JS arguments, calls the native method, and converts
//! the result. No IFC logic lives here.

use js_sys::{Array, BigInt, Uint8Array};
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

use super::IfcModel;
use crate::value::js::{from_js, to_js};
use crate::value::Tagged;
use crate::BindingError;

fn index(value: u32) -> usize {
    // wasm32 has a 32-bit usize, so a u32 always fits.
    value as usize
}

fn big(id: u64) -> JsValue {
    BigInt::from(id).into()
}

fn tagged_array(values: &JsValue) -> Result<Vec<Tagged>, BindingError> {
    if !Array::is_array(values) {
        return Err(BindingError::InvalidValue(
            "attributes must be an array".into(),
        ));
    }
    Array::from(values).iter().map(|v| from_js(&v)).collect()
}

#[wasm_bindgen]
impl IfcModel {
    /// An empty model.
    #[wasm_bindgen(constructor)]
    pub fn new() -> IfcModel {
        IfcModel::empty()
    }

    /// Parse a STEP (`.ifc`) file from its bytes.
    #[wasm_bindgen(js_name = parse)]
    pub fn parse_js(bytes: &[u8]) -> Result<IfcModel, JsValue> {
        Ok(IfcModel::parse(bytes)?)
    }

    /// Serialize as STEP bytes.
    #[wasm_bindgen(js_name = write)]
    pub fn write_js(&self) -> Result<Uint8Array, JsValue> {
        Ok(Uint8Array::from(self.write()?.as_slice()))
    }

    /// Number of entities.
    #[wasm_bindgen(getter, js_name = size)]
    pub fn size_js(&self) -> u32 {
        u32::try_from(self.len()).unwrap_or(u32::MAX)
    }

    /// The first `FILE_SCHEMA` token, e.g. `"IFC4"`, or `undefined`.
    #[wasm_bindgen(getter, js_name = schema)]
    pub fn schema_js(&self) -> Option<String> {
        self.schema().map(str::to_owned)
    }

    /// Non-fatal problems found while reading.
    #[wasm_bindgen(js_name = diagnostics)]
    pub fn diagnostics_js(&self) -> Vec<String> {
        self.diagnostics()
    }

    /// Every entity id (`bigint`), in file order.
    #[wasm_bindgen(js_name = ids, unchecked_return_type = "bigint[]")]
    pub fn ids_js(&self) -> Array {
        self.ids().into_iter().map(big).collect()
    }

    /// Ids of every entity of exactly `typeName`, case-insensitive.
    #[wasm_bindgen(js_name = idsOfType, unchecked_return_type = "bigint[]")]
    pub fn ids_of_type_js(&self, #[wasm_bindgen(js_name = typeName)] type_name: &str) -> Array {
        self.ids_of_type(type_name).into_iter().map(big).collect()
    }

    /// Ids of every entity of `typeName` or any subtype, per the file's
    /// declared schema: `IfcWall` also finds `IFCWALLSTANDARDCASE`.
    #[wasm_bindgen(js_name = idsOfTypeIncludingSubtypes, unchecked_return_type = "bigint[]")]
    pub fn ids_of_type_including_subtypes_js(
        &self,
        #[wasm_bindgen(js_name = typeName)] type_name: &str,
    ) -> Result<Array, JsValue> {
        Ok(self
            .ids_of_type_including_subtypes(type_name)?
            .into_iter()
            .map(big)
            .collect())
    }

    /// The upper-case type name of entity `id`.
    #[wasm_bindgen(js_name = typeOf)]
    pub fn type_of_js(&self, id: u64) -> Result<String, JsValue> {
        Ok(self.type_of(id)?.to_owned())
    }

    /// Every attribute of entity `id`, as tagged values.
    #[wasm_bindgen(js_name = attributes, unchecked_return_type = "IfcValue[]")]
    pub fn attributes_js(&self, id: u64) -> Result<Array, JsValue> {
        Ok(self.attributes(id)?.iter().map(to_js).collect())
    }

    /// Attribute `index` of entity `id`, as a tagged value.
    #[wasm_bindgen(js_name = attribute, unchecked_return_type = "IfcValue")]
    pub fn attribute_js(&self, id: u64, slot: u32) -> Result<JsValue, JsValue> {
        Ok(to_js(&self.attribute(id, index(slot))?))
    }

    /// Set attribute `index` of entity `id`; returns the previous value.
    #[wasm_bindgen(js_name = setAttribute, unchecked_return_type = "IfcValue")]
    pub fn set_attribute_js(
        &mut self,
        id: u64,
        slot: u32,
        #[wasm_bindgen(unchecked_param_type = "IfcValue")] value: &JsValue,
    ) -> Result<JsValue, JsValue> {
        let value = from_js(value)?;
        Ok(to_js(&self.set_attribute(id, index(slot), value)?))
    }

    /// Append an entity; returns its id (`bigint`).
    #[wasm_bindgen(js_name = add)]
    pub fn add_js(
        &mut self,
        #[wasm_bindgen(js_name = typeName)] type_name: &str,
        #[wasm_bindgen(unchecked_param_type = "IfcValue[]")] attributes: &JsValue,
    ) -> Result<u64, JsValue> {
        Ok(self.add(type_name, tagged_array(attributes)?)?)
    }

    /// Remove entity `id`, leaving references to it dangling.
    #[wasm_bindgen(js_name = remove)]
    pub fn remove_js(&mut self, id: u64) -> Result<(), JsValue> {
        Ok(self.remove(id)?)
    }

    /// Every `[from, to]` pair (`bigint`s) where `to` does not exist.
    #[wasm_bindgen(
        js_name = danglingReferences,
        unchecked_return_type = "[bigint, bigint][]"
    )]
    pub fn dangling_references_js(&self) -> Array {
        self.dangling_references()
            .into_iter()
            .map(|(from, to)| Array::of2(&big(from), &big(to)))
            .collect()
    }
}
