//! Texture-coordinate generators and mappings.

use ifc_model::{EntityId, Value};

use crate::error::StyleResult;
use crate::view::Record;

#[derive(Debug, Clone, Copy)]
pub struct TextureCoordinate<'m, 's> {
    record: Record<'m, 's>,
}

impl<'m, 's> TextureCoordinate<'m, 's> {
    pub(crate) fn from_record(record: Record<'m, 's>) -> Self {
        Self { record }
    }

    pub fn id(&self) -> EntityId {
        self.record.id
    }

    pub fn type_name(&self) -> &'m str {
        &self.record.entity.type_name
    }

    pub fn mode(&self) -> StyleResult<Option<&'m str>> {
        self.record.optional_text("Mode")
    }

    pub fn parameter(&self) -> StyleResult<Option<&'m Value>> {
        self.record.optional_raw("Parameter")
    }
}
