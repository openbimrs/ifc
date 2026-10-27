//! Undo and redo (#465): `setHistoryLimit`, `history`, `undo`, `redo` and
//! `clearHistory`. History is off until a limit is set; then every method
//! that changes the model is one undo entry.

use openbim_ifc_binding_core::{BindingError, ToRecord};
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

use super::IfcModel;
use crate::error::js_error;
use crate::records;

#[wasm_bindgen(typescript_custom_section)]
const IFC_HISTORY_TYPES: &'static str = r#"
/** What `IfcModel.history` reports. */
export interface HistoryState {
  /** The bound on undo entries; `0` while history is off. */
  limit: number;
  /** Entries `IfcModel.undo` can step back through. */
  undo: number;
  /** Entries `IfcModel.redo` can step forward through. */
  redo: number;
}

/** What `IfcModel.undo` or `IfcModel.redo` restored. */
export interface HistoryChange {
  /** Entities absent before, present after. */
  created: bigint[];
  /** Entities present before, absent after. */
  removed: bigint[];
  /** Entities present both times whose content changed. */
  modified: bigint[];
  /** Whether the file header changed. */
  header: boolean;
}
"#;

#[wasm_bindgen]
impl IfcModel {
    /// Keep at most `limit` undo entries; `0` (the default) turns history
    /// off and drops what is held. While it is on, every method that
    /// changes the model is one entry (`author` is one entry for its whole
    /// batch).
    /// Refused with `invalid-value` for a `limit` that is not a whole
    /// number of at least 0 that fits the platform.
    #[wasm_bindgen(js_name = setHistoryLimit)]
    pub fn set_history_limit_js(&mut self, limit: f64) -> Result<(), JsValue> {
        const MAX_SAFE: f64 = 9_007_199_254_740_991.0;
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let limit =
            if limit.is_finite() && (0.0..=MAX_SAFE).contains(&limit) && limit.fract() == 0.0 {
                usize::try_from(limit as u64).ok()
            } else {
                None
            };
        let limit = limit.ok_or_else(|| {
            js_error(BindingError::InvalidValue(
                "`limit` must be a whole number of at least 0".into(),
            ))
        })?;
        self.0.set_history_limit(limit);
        Ok(())
    }

    /// The history bound and how many entries each direction holds.
    #[wasm_bindgen(js_name = history, unchecked_return_type = "HistoryState")]
    pub fn history_js(&self) -> JsValue {
        records::record_to_js(&self.0.history().to_record())
    }

    /// Restore the model to the state before the last change: entities
    /// under their ids and at their file positions, the header included.
    /// Undoing a creation frees its id again. Refused with `history-empty`
    /// when there is nothing to undo (also while history is off) and
    /// `history-stale` when the model changed outside the history; the
    /// model is unchanged then.
    #[wasm_bindgen(js_name = undo, unchecked_return_type = "HistoryChange")]
    pub fn undo_js(&mut self) -> Result<JsValue, JsValue> {
        let change = self.0.undo().map_err(js_error)?;
        Ok(records::record_to_js(&change.to_record()))
    }

    /// Reverse the last `undo`. Refused with `history-empty` when there is
    /// nothing to redo and `history-stale` as `undo` is.
    #[wasm_bindgen(js_name = redo, unchecked_return_type = "HistoryChange")]
    pub fn redo_js(&mut self) -> Result<JsValue, JsValue> {
        let change = self.0.redo().map_err(js_error)?;
        Ok(records::record_to_js(&change.to_record()))
    }

    /// Drop every undo and redo entry, keeping the limit.
    #[wasm_bindgen(js_name = clearHistory)]
    pub fn clear_history_js(&mut self) {
        self.0.clear_history();
    }
}
