//! Texture vertices and coordinate lists.

use ifc_model::{EntityId, Value};

use crate::error::StyleResult;
use crate::view::Record;

/// Borrowed projection of `IfcTextureVertex`: a single 2D texture coordinate.
#[derive(Debug, Clone, Copy)]
pub struct TextureVertex<'m, 's> {
    record: Record<'m, 's>,
}

impl<'m, 's> TextureVertex<'m, 's> {
    pub(crate) fn from_record(record: Record<'m, 's>) -> Self {
        Self { record }
    }

    /// The entity id of this texture vertex.
    pub fn id(&self) -> EntityId {
        self.record.id
    }

    /// The `Coordinates` attribute: the raw `(s, t)` pair. Mandatory.
    pub fn coordinates(&self) -> StyleResult<&'m Value> {
        self.record.value("Coordinates")
    }
}

/// Borrowed projection of `IfcTextureVertexList`: an indexed list of texture
/// coordinates shared across faces via [`crate::IndexedTextureMap`].
#[derive(Debug, Clone, Copy)]
pub struct TextureVertexList<'m, 's> {
    record: Record<'m, 's>,
}

impl<'m, 's> TextureVertexList<'m, 's> {
    pub(crate) fn from_record(record: Record<'m, 's>) -> Self {
        Self { record }
    }

    /// The entity id of this texture vertex list.
    pub fn id(&self) -> EntityId {
        self.record.id
    }

    /// The `TexCoordsList` attribute: the raw list of `(s, t)` coordinate pairs. Mandatory.
    pub fn coordinates(&self) -> StyleResult<&'m Value> {
        self.record.value("TexCoordsList")
    }
}
