//! Copies that report (#479): a product with what it solely owns, and
//! entities appended from another model. Each returns a `CopyReport`
//! record, and with `dryRun` changes nothing.

use openbim_ifc_binding_core::{AppendOptions, BindingError, CopyProductOptions, ToRecord};
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

use super::IfcModel;
use crate::error::js_error;
use crate::{geometry, records};

#[wasm_bindgen]
impl IfcModel {
    /// Copy product (or type object) `id` with everything it solely owns,
    /// as one checked transaction: what `removeProduct` would remove with
    /// it is copied (its placement, representation, own property sets,
    /// material usage, ports and, unless `openings` or `fillings` is
    /// `false`, its openings and the doors and windows filling them), what
    /// it would keep is shared, the copy joining the relationship (its
    /// type, shared property sets, classification and, unless `container`
    /// or `aggregate` is `false`, its container and whole). `offset` moves
    /// the copy in its parent placement's frame; `placement` places it by
    /// that `IfcObjectPlacement` instead. The report maps every original
    /// to its copy; with `dryRun` nothing changes. Refused with
    /// `wrong-entity-type` for neither a product nor a type object,
    /// `invalid-value` for both `offset` and `placement`, `unsupported`
    /// for an offset onto a placement that is not a local placement with a
    /// Cartesian location, or a moved copy whose openings are not placed
    /// relative to it.
    #[wasm_bindgen(js_name = copyProduct, unchecked_return_type = "CopyReport")]
    pub fn copy_product_js(
        &mut self,
        id: u64,
        #[wasm_bindgen(unchecked_param_type = "CopyProductOptions | undefined")] options: &JsValue,
    ) -> Result<JsValue, JsValue> {
        let parsed = copy_options(options).map_err(js_error)?;
        let report = self.0.copy_product(id, parsed).map_err(js_error)?;
        Ok(records::record_to_js(&report.to_record()))
    }

    /// Copy entities `ids` of `source` into this model with what belongs
    /// to them, as one checked transaction: what they reference, their
    /// property sets, type, material and classification, their openings,
    /// fillings and ports, and what attaches to their geometry. Units,
    /// contexts, the owner history, materials and styles equal to this
    /// model's are reused (`reuse` names the kinds; all by default).
    /// Relationships to objects not copied (a spatial container) are
    /// skipped and reported. `globalIds`: `keep` (the default; one this
    /// model holds is refused), `merge` (an `IfcRoot` this model holds by
    /// `GlobalId` and type is reused, its relationships joined),
    /// `regenerate` or `regenerate-colliding`. Refused with `unsupported`
    /// when the models' length or plane angle units differ,
    /// `unsupported-schema` for an entity this model's release cannot
    /// hold, `invalid-value` for a kept `GlobalId` this model holds.
    #[wasm_bindgen(js_name = append, unchecked_return_type = "CopyReport")]
    pub fn append_js(
        &mut self,
        source: &IfcModel,
        #[wasm_bindgen(unchecked_param_type = "bigint[] | BigUint64Array")] ids: &JsValue,
        #[wasm_bindgen(unchecked_param_type = "AppendOptions | undefined")] options: &JsValue,
    ) -> Result<JsValue, JsValue> {
        let ids = geometry::ids(ids)
            .map_err(js_error)?
            .ok_or_else(|| js_error(BindingError::InvalidValue("append needs ids".into())))?;
        let parsed = append_options(options).map_err(js_error)?;
        let report = self.0.append(&source.0, &ids, &parsed).map_err(js_error)?;
        Ok(records::record_to_js(&report.to_record()))
    }
}

fn copy_options(options: &JsValue) -> Result<CopyProductOptions, BindingError> {
    let mut parsed = CopyProductOptions::new();
    let offset = records::option(options, "offset")?;
    if !offset.is_undefined() && !offset.is_null() {
        let numbers: Option<Vec<f64>> = js_sys::Array::is_array(&offset)
            .then(|| {
                js_sys::Array::from(&offset)
                    .iter()
                    .map(|n| n.as_f64().filter(|n| n.is_finite()))
                    .collect()
            })
            .flatten();
        parsed.offset = Some(
            numbers
                .and_then(|numbers| <[f64; 3]>::try_from(numbers).ok())
                .ok_or_else(|| {
                    BindingError::InvalidValue("`offset` must be three finite numbers".into())
                })?,
        );
    }
    let placement = records::option(options, "placement")?;
    if !placement.is_undefined() && !placement.is_null() {
        let id = geometry::ids(&js_sys::Array::of1(&placement).into())?
            .and_then(|ids| ids.first().copied())
            .ok_or_else(|| BindingError::InvalidValue("`placement` must be an id".into()))?;
        parsed.placement = Some(id);
    }
    parsed.openings = records::flag_or(options, "openings", true)?;
    parsed.fillings = records::flag_or(options, "fillings", true)?;
    parsed.container = records::flag_or(options, "container", true)?;
    parsed.aggregate = records::flag_or(options, "aggregate", true)?;
    parsed.dry_run = records::flag(options, "dryRun")?;
    Ok(parsed)
}

fn append_options(options: &JsValue) -> Result<AppendOptions, BindingError> {
    let mut parsed = AppendOptions::new();
    let mode = records::option(options, "globalIds")?;
    if !mode.is_undefined() && !mode.is_null() {
        parsed.global_ids = mode
            .as_string()
            .ok_or_else(|| BindingError::InvalidValue("`globalIds` must be a string".into()))?;
    }
    if !records::option(options, "reuse")?.is_undefined() {
        let kinds = records::string_list(options, "reuse")?;
        let known = ["units", "contexts", "ownerHistory", "materials", "styles"];
        if let Some(unknown) = kinds.iter().find(|kind| !known.contains(&kind.as_str())) {
            return Err(BindingError::InvalidValue(format!(
                "reuse kind `{unknown}` is not one of {}",
                known.join(", ")
            )));
        }
        let reused = |kind: &str| kinds.iter().any(|k| k == kind);
        parsed.reuse_units = reused("units");
        parsed.reuse_contexts = reused("contexts");
        parsed.reuse_owner_history = reused("ownerHistory");
        parsed.reuse_materials = reused("materials");
        parsed.reuse_styles = reused("styles");
    }
    parsed.dry_run = records::flag(options, "dryRun")?;
    Ok(parsed)
}
