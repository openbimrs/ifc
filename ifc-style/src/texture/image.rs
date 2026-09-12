//! `IfcImageTexture` URL projection.

use ifc_model::EntityId;

use crate::error::StyleResult;
use crate::texture::SurfaceTexture;
use crate::view::Record;

/// Borrowed projection of `IfcImageTexture`: a raster image referenced by URL
/// rather than embedded inline.
#[derive(Debug, Clone, Copy)]
pub struct ImageTexture<'m, 's> {
    record: Record<'m, 's>,
}

impl<'m, 's> ImageTexture<'m, 's> {
    pub(crate) fn from_record(record: Record<'m, 's>) -> Self {
        Self { record }
    }

    /// The entity id of this `IfcImageTexture`.
    pub fn id(&self) -> EntityId {
        self.record.id
    }

    /// The base `IfcSurfaceTexture` attributes shared with other texture kinds.
    pub fn surface_texture(&self) -> SurfaceTexture<'m, 's> {
        SurfaceTexture::from_record(self.record)
    }

    /// The `URLReference` attribute: the image's location. Mandatory.
    pub fn url_reference(&self) -> StyleResult<&'m str> {
        self.record.required_text("URLReference")
    }
}
