//! `IfcPresentationLayerWithStyle` visibility and identifier projection.

use crate::error::StyleResult;

use super::PresentationLayer;

impl<'m, 's> PresentationLayer<'m, 's> {
    pub fn layer_on(&self) -> StyleResult<Option<bool>> {
        self.record.optional_bool("LayerOn")
    }

    pub fn layer_frozen(&self) -> StyleResult<Option<bool>> {
        self.record.optional_bool("LayerFrozen")
    }

    pub fn layer_blocked(&self) -> StyleResult<Option<bool>> {
        self.record.optional_bool("LayerBlocked")
    }
}
