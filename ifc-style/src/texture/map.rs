//! Indexed texture-map projection.

use ifc_model::{EntityId, Value};

use crate::error::StyleResult;
use crate::view::Record;

/// Borrowed projection of an indexed texture map (`IfcIndexedTriangleTextureMap`
/// or `IfcIndexedPolygonalTextureMap`): binds a texture to a tessellated
/// face set via per-vertex texture-coordinate indices.
#[derive(Debug, Clone, Copy)]
pub struct IndexedTextureMap<'m, 's> {
    record: Record<'m, 's>,
}

impl<'m, 's> IndexedTextureMap<'m, 's> {
    pub(crate) fn from_record(record: Record<'m, 's>) -> Self {
        Self { record }
    }

    /// The entity id of this texture map.
    pub fn id(&self) -> EntityId {
        self.record.id
    }

    /// The concrete IFC entity type name.
    pub fn type_name(&self) -> &'m str {
        &self.record.entity.type_name
    }

    /// `Maps` precedes `MappedTo` in both IFC4 and IFC4X3.
    pub fn maps(&self) -> StyleResult<EntityId> {
        self.record.required_ref("Maps", "IfcSurfaceTexture")
    }

    /// The `MappedTo` attribute: the tessellated face set this map applies to. Mandatory.
    pub fn mapped_to(&self) -> StyleResult<EntityId> {
        self.record
            .required_ref("MappedTo", "IfcTessellatedFaceSet")
    }

    /// The `TexCoordIndex` attribute: per-face indices into `TexCoords`, when authored.
    pub fn tex_coord_index(&self) -> StyleResult<Option<&'m Value>> {
        self.record.optional_raw("TexCoordIndex")
    }

    /// The `TexCoords` attribute: the indexed texture-vertex list. Mandatory.
    pub fn tex_coords(&self) -> StyleResult<EntityId> {
        self.record
            .required_ref("TexCoords", "IfcTextureVertexList")
    }
}
