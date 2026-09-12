//! Texture-coordinate generators and mappings.

use ifc_model::{EntityId, Value};

use crate::error::StyleResult;
use crate::view::Record;

/// Borrowed projection of an `IfcTextureCoordinate` (and its generator/map subtypes):
/// procedural or explicit texture-coordinate assignment for a mapped item.
#[derive(Debug, Clone, Copy)]
pub struct TextureCoordinate<'m, 's> {
    record: Record<'m, 's>,
}

impl<'m, 's> TextureCoordinate<'m, 's> {
    pub(crate) fn from_record(record: Record<'m, 's>) -> Self {
        Self { record }
    }

    /// The entity id of this texture coordinate entity.
    pub fn id(&self) -> EntityId {
        self.record.id
    }

    /// The concrete IFC entity type name (e.g. `"IFCTEXTUREMAP"`,
    /// `"IFCTEXTURECOORDINATEGENERATOR"`).
    pub fn type_name(&self) -> &'m str {
        &self.record.entity.type_name
    }

    /// The `Mode` attribute: the generator/mapping mode name, when authored.
    pub fn mode(&self) -> StyleResult<Option<&'m str>> {
        self.record.optional_text("Mode")
    }

    /// The `Parameter` attribute: mode-specific numeric parameters, when authored.
    pub fn parameter(&self) -> StyleResult<Option<&'m Value>> {
        self.record.optional_raw("Parameter")
    }
}
