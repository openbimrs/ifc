//! Indexed texture-map projection.

use ifc_model::{EntityId, Value};

use crate::error::StyleResult;
use crate::view::Record;

#[derive(Debug, Clone, Copy)]
pub struct IndexedTextureMap<'m, 's> {
    record: Record<'m, 's>,
}

impl<'m, 's> IndexedTextureMap<'m, 's> {
    pub(crate) fn from_record(record: Record<'m, 's>) -> Self {
        Self { record }
    }

    pub fn id(&self) -> EntityId {
        self.record.id
    }

    pub fn type_name(&self) -> &'m str {
        &self.record.entity.type_name
    }

    /// `Maps` precedes `MappedTo` in both IFC4 and IFC4X3.
    pub fn maps(&self) -> StyleResult<EntityId> {
        self.record.required_ref("Maps", "IfcSurfaceTexture")
    }

    pub fn mapped_to(&self) -> StyleResult<EntityId> {
        self.record
            .required_ref("MappedTo", "IfcTessellatedFaceSet")
    }

    pub fn tex_coord_index(&self) -> StyleResult<Option<&'m Value>> {
        self.record.optional_raw("TexCoordIndex")
    }

    pub fn tex_coords(&self) -> StyleResult<EntityId> {
        self.record
            .required_ref("TexCoords", "IfcTextureVertexList")
    }
}
