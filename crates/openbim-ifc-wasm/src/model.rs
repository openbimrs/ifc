//! JavaScript surface of the shared [`Core`] model.
//!
//! Each export converts JS arguments, calls the core method, and converts
//! the result. No IFC logic lives here.

use js_sys::{Array, BigInt, Uint8Array};
use openbim_ifc_binding_core::value::Tagged;
use openbim_ifc_binding_core::{BindingError, IfcModel as Core};
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

use crate::error::js_error;
use crate::value::{from_js, to_js};

mod types;

/// An IFC model: entities keyed by their `#id`, in file order.
///
/// A newtype because `wasm_bindgen` can only export a type defined in this
/// crate; all state and behaviour are the core's.
#[wasm_bindgen]
#[derive(Debug, Default)]
pub struct IfcModel(Core);

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
        IfcModel(Core::empty())
    }

    /// Parse a STEP (`.ifc`) file from its bytes.
    #[wasm_bindgen(js_name = parse)]
    pub fn parse_js(bytes: &[u8]) -> Result<IfcModel, JsValue> {
        Core::parse(bytes).map(IfcModel).map_err(js_error)
    }

    /// Serialize as STEP bytes.
    #[wasm_bindgen(js_name = write)]
    pub fn write_js(&self) -> Result<Uint8Array, JsValue> {
        Ok(Uint8Array::from(
            self.0.write().map_err(js_error)?.as_slice(),
        ))
    }

    /// Number of entities.
    #[wasm_bindgen(getter, js_name = size)]
    pub fn size_js(&self) -> u32 {
        u32::try_from(self.0.len()).unwrap_or(u32::MAX)
    }

    /// The first `FILE_SCHEMA` token, e.g. `"IFC4"`, or `undefined`.
    #[wasm_bindgen(getter, js_name = schema)]
    pub fn schema_js(&self) -> Option<String> {
        self.0.schema().map(str::to_owned)
    }

    /// Non-fatal problems found while reading.
    #[wasm_bindgen(js_name = diagnostics)]
    pub fn diagnostics_js(&self) -> Vec<String> {
        self.0.diagnostics()
    }

    /// Every entity id (`bigint`), in file order.
    #[wasm_bindgen(js_name = ids, unchecked_return_type = "bigint[]")]
    pub fn ids_js(&self) -> Array {
        self.0.ids().into_iter().map(big).collect()
    }

    /// Ids of every entity of exactly `typeName`, case-insensitive.
    #[wasm_bindgen(js_name = idsOfType, unchecked_return_type = "bigint[]")]
    pub fn ids_of_type_js(&self, #[wasm_bindgen(js_name = typeName)] type_name: &str) -> Array {
        self.0.ids_of_type(type_name).into_iter().map(big).collect()
    }

    /// Ids of every entity of `typeName` or any subtype, per the file's
    /// declared schema: `IfcWall` also finds `IFCWALLSTANDARDCASE`.
    #[wasm_bindgen(js_name = idsOfTypeIncludingSubtypes, unchecked_return_type = "bigint[]")]
    pub fn ids_of_type_including_subtypes_js(
        &self,
        #[wasm_bindgen(js_name = typeName)] type_name: &str,
    ) -> Result<Array, JsValue> {
        Ok(self
            .0
            .ids_of_type_including_subtypes(type_name)
            .map_err(js_error)?
            .into_iter()
            .map(big)
            .collect())
    }

    /// The upper-case type name of entity `id`.
    #[wasm_bindgen(js_name = typeOf)]
    pub fn type_of_js(&self, id: u64) -> Result<String, JsValue> {
        Ok(self.0.type_of(id).map_err(js_error)?.to_owned())
    }

    /// Every attribute of entity `id`, as tagged values.
    #[wasm_bindgen(js_name = attributes, unchecked_return_type = "IfcValue[]")]
    pub fn attributes_js(&self, id: u64) -> Result<Array, JsValue> {
        Ok(self
            .0
            .attributes(id)
            .map_err(js_error)?
            .iter()
            .map(to_js)
            .collect())
    }

    /// Attribute `index` of entity `id`, as a tagged value.
    #[wasm_bindgen(js_name = attribute, unchecked_return_type = "IfcValue")]
    pub fn attribute_js(&self, id: u64, slot: u32) -> Result<JsValue, JsValue> {
        Ok(to_js(&self.0.attribute(id, index(slot)).map_err(js_error)?))
    }

    /// Set attribute `index` of entity `id`; returns the previous value.
    #[wasm_bindgen(js_name = setAttribute, unchecked_return_type = "IfcValue")]
    pub fn set_attribute_js(
        &mut self,
        id: u64,
        slot: u32,
        #[wasm_bindgen(unchecked_param_type = "IfcValue")] value: &JsValue,
    ) -> Result<JsValue, JsValue> {
        let value = from_js(value).map_err(js_error)?;
        Ok(to_js(
            &self
                .0
                .set_attribute(id, index(slot), value)
                .map_err(js_error)?,
        ))
    }

    /// Append an entity; returns its id (`bigint`).
    #[wasm_bindgen(js_name = add)]
    pub fn add_js(
        &mut self,
        #[wasm_bindgen(js_name = typeName)] type_name: &str,
        #[wasm_bindgen(unchecked_param_type = "IfcValue[]")] attributes: &JsValue,
    ) -> Result<u64, JsValue> {
        let attributes = tagged_array(attributes).map_err(js_error)?;
        self.0.add(type_name, attributes).map_err(js_error)
    }

    /// Remove entity `id`, leaving references to it dangling.
    #[wasm_bindgen(js_name = remove)]
    pub fn remove_js(&mut self, id: u64) -> Result<(), JsValue> {
        self.0.remove(id).map_err(js_error)
    }

    /// Every `[from, to]` pair (`bigint`s) where `to` does not exist.
    #[wasm_bindgen(
        js_name = danglingReferences,
        unchecked_return_type = "[bigint, bigint][]"
    )]
    pub fn dangling_references_js(&self) -> Array {
        self.0
            .dangling_references()
            .into_iter()
            .map(|(from, to)| Array::of2(&big(from), &big(to)))
            .collect()
    }
}
