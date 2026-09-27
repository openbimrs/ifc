//! `IfcPresentationLayerWithStyle` visibility and identifier projection.

use crate::error::StyleResult;

use super::PresentationLayer;

impl<'m, 's> PresentationLayer<'m, 's> {
    /// The `LayerOn` visibility flag, when authored.
    pub fn layer_on(&self) -> StyleResult<Option<bool>> {
        self.record.optional_bool("LayerOn")
    }

    /// The `LayerFrozen` flag, when authored.
    pub fn layer_frozen(&self) -> StyleResult<Option<bool>> {
        self.record.optional_bool("LayerFrozen")
    }

    /// The `LayerBlocked` flag, when authored.
    pub fn layer_blocked(&self) -> StyleResult<Option<bool>> {
        self.record.optional_bool("LayerBlocked")
    }
}
