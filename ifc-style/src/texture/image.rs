//! `IfcImageTexture` URL projection.

use ifc_model::EntityId;

use crate::error::StyleResult;
use crate::texture::SurfaceTexture;
use crate::view::Record;

#[derive(Debug, Clone, Copy)]
pub struct ImageTexture<'m, 's> {
    record: Record<'m, 's>,
}

impl<'m, 's> ImageTexture<'m, 's> {
    pub(crate) fn from_record(record: Record<'m, 's>) -> Self {
        Self { record }
    }

    pub fn id(&self) -> EntityId {
        self.record.id
    }

    pub fn surface_texture(&self) -> SurfaceTexture<'m, 's> {
        SurfaceTexture::from_record(self.record)
    }

    pub fn url_reference(&self) -> StyleResult<&'m str> {
        self.record.required_text("URLReference")
    }
}
