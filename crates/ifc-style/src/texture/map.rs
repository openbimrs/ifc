//! Indexed texture-map projection.

use ifc_model::{EntityId, Value};

use crate::error::StyleResult;
use crate::view::Record;

mod triangle;

pub use triangle::TriangleTextureCoordinates;

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

    /// The `Maps` attribute: the textures this map applies. Mandatory.
    ///
    /// `LIST [1:?] OF IfcSurfaceTexture` in both IFC4 and IFC4X3. It is a
    /// list even when, as is usual, it holds a single texture.
    pub fn maps(&self) -> StyleResult<Vec<EntityId>> {
        self.record
            .required_refs("Maps", "IfcSurfaceTexture", 1, None)
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

    /// Per-corner texture coordinates for an `IfcIndexedTriangleTextureMap`.
    ///
    /// `triangle_count` is the `CoordIndex` length of the mapped face set.
    /// Returns `Ok(None)` for the polygonal variant, which indexes through
    /// `IfcTextureCoordinateIndices` instead, and when `TexCoordIndex` is
    /// omitted, whose meaning the schema leaves undefined.
    ///
    /// # Errors
    ///
    /// A malformed `TexCoordsList`, a `TexCoordIndex` entry that addresses a
    /// missing texture vertex, or more mapped triangles than the face set has.
    pub fn triangle_coordinates(
        &self,
        triangle_count: usize,
    ) -> StyleResult<Option<TriangleTextureCoordinates>> {
        let record = &self.record;
        if !record
            .schema
            .is_a(&record.entity.type_name, "IfcIndexedTriangleTextureMap")
        {
            return Ok(None);
        }
        let Some(index) = self.tex_coord_index()? else {
            return Ok(None);
        };
        let list = crate::StyleView::new(record.model, record.schema)
            .texture_vertex_list(self.tex_coords()?)?;
        triangle::resolve(
            self.id(),
            self.type_name(),
            index,
            list.coordinates()?,
            triangle_count,
        )
        .map(Some)
    }
}
