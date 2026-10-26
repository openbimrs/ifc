//! Selectors and values by path (#464): `select`, `getValue`, `setValue`.

use js_sys::{Array, BigInt, Object};
use openbim_ifc_binding_core::{canonical_selector, Selected, SELECTOR_GRAMMAR_VERSION};
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

use super::IfcModel;
use crate::authoring;
use crate::error::js_error;
use crate::geometry;
use crate::value::{plain_from_js, to_js};

/// A value read by path: a scalar as [`to_js`] encodes it, a list as
/// `{ kind: "list", items }`, a property set as
/// `{ kind: "properties", entries: [name, value][] }`.
fn selected_to_js(value: &Selected) -> JsValue {
    match value {
        Selected::Value(tagged) => to_js(tagged),
        Selected::List(items) => {
            let object = Object::new();
            let items: Array = items.iter().map(selected_to_js).collect();
            // Setting a property on a fresh plain object cannot fail.
            let _ = js_sys::Reflect::set(&object, &"kind".into(), &"list".into());
            let _ = js_sys::Reflect::set(&object, &"items".into(), &items);
            object.into()
        }
        Selected::Properties(members) => {
            let object = Object::new();
            let entries: Array = members
                .iter()
                .map(|(name, member)| {
                    let pair = Array::new();
                    pair.push(&name.as_str().into());
                    pair.push(&selected_to_js(member));
                    JsValue::from(pair)
                })
                .collect();
            let _ = js_sys::Reflect::set(&object, &"kind".into(), &"properties".into());
            let _ = js_sys::Reflect::set(&object, &"entries".into(), &entries);
            object.into()
        }
    }
}

#[wasm_bindgen]
impl IfcModel {
    /// The ids selector `query` selects, ascending (#464); with `base`, from
    /// those entities alone.
    ///
    /// Class facets (subtypes included) add entities, `!` removes them;
    /// attribute, property and `type=`, `material=`, `classification=`,
    /// `location=` and `group=` facets filter; `,` chains facets and `+`
    /// unites lists. Throws `invalid-selector` (with `span`, the byte range
    /// in the UTF-8 query) for a query that does not parse or names a
    /// class or attribute the file's release does not declare.
    #[wasm_bindgen(js_name = select, unchecked_return_type = "bigint[]")]
    pub fn select_js(
        &self,
        query: &str,
        #[wasm_bindgen(unchecked_param_type = "bigint[] | BigUint64Array | undefined")]
        base: &JsValue,
    ) -> Result<Array, JsValue> {
        let ids = match geometry::ids(base).map_err(js_error)? {
            None => self.0.select(query),
            Some(base) => self.0.select_from(query, &base),
        }
        .map_err(js_error)?;
        Ok(ids
            .into_iter()
            .map(|id| JsValue::from(BigInt::from(id)))
            .collect())
    }

    /// The value `path` reads from entity `id`: `"type.Name"`,
    /// `"Pset_WallCommon.FireRating"`, `"materials.Name"`,
    /// `"occurrences.count"`.
    #[wasm_bindgen(js_name = getValue, unchecked_return_type = "IfcSelectedValue")]
    pub fn get_value_js(&self, id: u64, path: &str) -> Result<JsValue, JsValue> {
        Ok(selected_to_js(
            &self.0.get_value(id, path).map_err(js_error)?,
        ))
    }

    /// Write `value` at `path` of entity `id` as one checked transaction:
    /// an attribute coerced as `setAttributeByName` coerces a plain value,
    /// or a property from a typed value, or a plain one when the property
    /// already states its type. A refused write changes nothing.
    #[wasm_bindgen(js_name = setValue)]
    pub fn set_value_js(
        &mut self,
        id: u64,
        path: &str,
        #[wasm_bindgen(unchecked_param_type = "IfcValue | IfcPlainValue")] value: &JsValue,
    ) -> Result<(), JsValue> {
        let value = plain_from_js(value).map_err(js_error)?;
        self.0
            .set_value_seeded(id, path, value, authoring::seed())
            .map_err(js_error)
    }

    /// Selector `query` in its canonical form: spacing, quoting and
    /// aliases normalised. Throws `invalid-selector`.
    #[wasm_bindgen(js_name = canonicalSelector)]
    pub fn canonical_selector_js(query: &str) -> Result<String, JsValue> {
        canonical_selector(query).map_err(js_error)
    }

    /// The version of the selector grammar this build parses (`"1.0"`).
    #[wasm_bindgen(js_name = selectorGrammarVersion)]
    pub fn selector_grammar_version_js() -> String {
        SELECTOR_GRAMMAR_VERSION.to_owned()
    }
}
