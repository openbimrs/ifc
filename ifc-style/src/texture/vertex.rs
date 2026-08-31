//! Texture vertices and coordinate lists.

use ifc_model::{EntityId, Value};

use crate::error::StyleResult;
use crate::view::Record;

#[derive(Debug, Clone, Copy)]
pub struct TextureVertex<'m, 's> {
    record: Record<'m, 's>,
}

impl<'m, 's> TextureVertex<'m, 's> {
    pub(crate) fn from_record(record: Record<'m, 's>) -> Self {
        Self { record }
    }

    pub fn id(&self) -> EntityId {
        self.record.id
    }

    pub fn coordinates(&self) -> StyleResult<&'m Value> {
        self.record.value("Coordinates")
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TextureVertexList<'m, 's> {
    record: Record<'m, 's>,
}

impl<'m, 's> TextureVertexList<'m, 's> {
    pub(crate) fn from_record(record: Record<'m, 's>) -> Self {
        Self { record }
    }

    pub fn id(&self) -> EntityId {
        self.record.id
    }

    pub fn coordinates(&self) -> StyleResult<&'m Value> {
        self.record.value("TexCoordsList")
    }
}
