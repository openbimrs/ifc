//! Removals that report (#478, #481): a product with what it solely owns,
//! the purge of what nothing uses, and a shape representation with what
//! only it uses. Each returns a `RemovalReport` record, and with `dryRun`
//! changes nothing.

use openbim_ifc_binding_core::{PurgeOptions, RemoveProductOptions, ToRecord};
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

use super::IfcModel;
use crate::error::js_error;
use crate::records;

#[wasm_bindgen]
impl IfcModel {
    /// Remove product (or type object) `id` with everything it solely owns
    /// -- its placement, representation, own property and quantity sets,
    /// material usage, openings and ports, and the relationships left
    /// without an end -- as one checked transaction, keeping what is shared
    /// (a property set another object uses, a type's representation maps).
    /// The report says what went, what stayed and why; with `dryRun`
    /// nothing changes. Refused with `still-referenced` while other
    /// placements are relative to its placement (unless
    /// `rebasePlacements`) or a type object types objects (unless
    /// `untypeOccurrences`), `wrong-entity-type` for neither a product nor
    /// a type object, `unsupported` for a placement that cannot be
    /// re-based.
    #[wasm_bindgen(js_name = removeProduct, unchecked_return_type = "RemovalReport")]
    pub fn remove_product_js(
        &mut self,
        id: u64,
        #[wasm_bindgen(unchecked_param_type = "RemoveProductOptions | undefined")]
        options: &JsValue,
    ) -> Result<JsValue, JsValue> {
        let mut parsed = RemoveProductOptions::new();
        parsed.rebase_placements = records::flag(options, "rebasePlacements").map_err(js_error)?;
        parsed.untype_occurrences =
            records::flag(options, "untypeOccurrences").map_err(js_error)?;
        parsed.dry_run = records::flag(options, "dryRun").map_err(js_error)?;
        let report = self.0.remove_product(id, parsed).map_err(js_error)?;
        Ok(records::record_to_js(&report.to_record()))
    }

    /// Remove unreferenced entities as one checked transaction; with
    /// `dryRun`, only report. Without `classes`, every resource entity no
    /// `IfcRoot` reaches (nor annotates something reached); with them, the
    /// entities of those classes nothing references, an `IfcRoot` only
    /// with `includeRoots`. The owner history stays unless
    /// `includeOwnerHistory`. Refused with `unsupported-schema` for a class
    /// the release does not declare.
    #[wasm_bindgen(js_name = purge, unchecked_return_type = "RemovalReport")]
    pub fn purge_js(
        &mut self,
        #[wasm_bindgen(unchecked_param_type = "PurgeOptions | undefined")] options: &JsValue,
    ) -> Result<JsValue, JsValue> {
        let mut parsed = PurgeOptions::new();
        parsed.classes = records::classes(options).map_err(js_error)?;
        parsed.include_roots = records::flag(options, "includeRoots").map_err(js_error)?;
        parsed.include_owner_history =
            records::flag(options, "includeOwnerHistory").map_err(js_error)?;
        parsed.dry_run = records::flag(options, "dryRun").map_err(js_error)?;
        let report = self.0.purge(parsed).map_err(js_error)?;
        Ok(records::record_to_js(&report.to_record()))
    }

    /// Remove shape representation `id` with what only it uses (#481), as
    /// one checked transaction: taken off its product definition shape
    /// (which goes when left empty), representation map or shape aspects,
    /// then removed with its items down to what nothing else uses. Shared
    /// items, profiles, points and styles stay, and the report says why;
    /// with `dryRun` nothing changes. Refused with `still-referenced` for a
    /// representation map an `IfcMappedItem` still maps,
    /// `wrong-entity-type` for an entity that is not an `IfcShapeModel`.
    #[wasm_bindgen(js_name = removeRepresentation, unchecked_return_type = "RemovalReport")]
    pub fn remove_representation_js(
        &mut self,
        id: u64,
        #[wasm_bindgen(unchecked_param_type = "RemoveRepresentationOptions | undefined")]
        options: &JsValue,
    ) -> Result<JsValue, JsValue> {
        let dry_run = records::flag(options, "dryRun").map_err(js_error)?;
        let report = self
            .0
            .remove_representation(id, dry_run)
            .map_err(js_error)?;
        Ok(records::record_to_js(&report.to_record()))
    }
}
