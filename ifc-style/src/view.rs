//! Shared schema-resolved borrowed view primitives.

mod record;

pub(crate) use record::Record;

use ifc_model::{EntityId, Model};
use ifc_schema::Schema;

use crate::error::{StyleError, StyleResult};
use crate::{
    Annotation, AnnotationFillArea, BlobTexture, ColourRgb, CurveStyle, CurveStyleFont,
    CurveStyleFontPattern, FillAreaStyle, FillAreaStyleHatching, FillAreaStyleTiles, ImageTexture,
    IndexedTextureMap, LightDistributionData, LightIntensityDistribution, LightSource,
    LightSourceAmbient, LightSourceDirectional, LightSourceGoniometric, LightSourcePositional,
    LightSourceSpot, PixelTexture, PlanarBox, PlanarExtent, PresentationLayer,
    PresentationStyleAssignment, StyledItem, SurfaceStyle, SurfaceStyleLighting,
    SurfaceStyleRefraction, SurfaceStyleRendering, SurfaceStyleShading, SurfaceStyleWithTextures,
    SurfaceTexture, TextLiteral, TextLiteralWithExtent, TextStyle, TextStyleFontModel,
    TextureCoordinate, TextureVertex, TextureVertexList,
};

/// Entry point for strict presentation and annotation projections.
#[derive(Debug, Clone, Copy)]
pub struct StyleView<'m, 's> {
    pub(crate) model: &'m Model,
    pub(crate) schema: &'s Schema,
}

impl<'m, 's> StyleView<'m, 's> {
    /// Bind a schema-resolved view over `model` using `schema`'s attribute
    /// layout and type hierarchy.
    #[must_use]
    pub fn new(model: &'m Model, schema: &'s Schema) -> Self {
        Self { model, schema }
    }

    /// Project entity `id` as an `IfcAnnotation`.
    pub fn annotation(&self, id: EntityId) -> StyleResult<Annotation<'m, 's>> {
        Annotation::from_record(self.record(id, "IfcAnnotation")?)
    }

    /// Project entity `id` as an `IfcTextLiteral`.
    pub fn text_literal(&self, id: EntityId) -> StyleResult<TextLiteral<'m, 's>> {
        TextLiteral::from_record(self.record(id, "IfcTextLiteral")?)
    }

    /// Project entity `id` as an `IfcTextLiteralWithExtent`.
    pub fn text_literal_with_extent(
        &self,
        id: EntityId,
    ) -> StyleResult<TextLiteralWithExtent<'m, 's>> {
        TextLiteralWithExtent::from_record(self.record(id, "IfcTextLiteralWithExtent")?)
    }

    /// Project entity `id` as an `IfcAnnotationFillArea`.
    pub fn annotation_fill_area(&self, id: EntityId) -> StyleResult<AnnotationFillArea<'m, 's>> {
        AnnotationFillArea::from_record(self.record(id, "IfcAnnotationFillArea")?)
    }

    /// Project entity `id` as an `IfcColourRgb`.
    pub fn colour_rgb(&self, id: EntityId) -> StyleResult<ColourRgb<'m, 's>> {
        Ok(ColourRgb::from_record(self.record(id, "IfcColourRgb")?))
    }

    /// Project entity `id` as an `IfcSurfaceStyleShading`.
    pub fn surface_style_shading(&self, id: EntityId) -> StyleResult<SurfaceStyleShading<'m, 's>> {
        Ok(SurfaceStyleShading::from_record(
            self.record(id, "IfcSurfaceStyleShading")?,
        ))
    }

    /// Project entity `id` as an `IfcSurfaceStyle`.
    pub fn surface_style(&self, id: EntityId) -> StyleResult<SurfaceStyle<'m, 's>> {
        Ok(SurfaceStyle::from_record(
            self.record(id, "IfcSurfaceStyle")?,
        ))
    }

    /// Project entity `id` as an `IfcStyledItem`.
    pub fn styled_item(&self, id: EntityId) -> StyleResult<StyledItem<'m, 's>> {
        Ok(StyledItem::from_record(self.record(id, "IfcStyledItem")?))
    }

    /// Project entity `id` as an `IfcPresentationLayerAssignment`.
    pub fn presentation_layer(&self, id: EntityId) -> StyleResult<PresentationLayer<'m, 's>> {
        Ok(PresentationLayer::from_record(
            self.record(id, "IfcPresentationLayerAssignment")?,
        ))
    }

    /// Project entity `id` as an `IfcSurfaceStyleRendering`.
    pub fn surface_style_rendering(
        &self,
        id: EntityId,
    ) -> StyleResult<SurfaceStyleRendering<'m, 's>> {
        Ok(SurfaceStyleRendering::from_record(
            self.record(id, "IfcSurfaceStyleRendering")?,
        ))
    }

    /// Project entity `id` as an `IfcCurveStyle`.
    pub fn curve_style(&self, id: EntityId) -> StyleResult<CurveStyle<'m, 's>> {
        Ok(CurveStyle::from_record(self.record(id, "IfcCurveStyle")?))
    }

    /// Project entity `id` as an `IfcFillAreaStyle`.
    pub fn fill_area_style(&self, id: EntityId) -> StyleResult<FillAreaStyle<'m, 's>> {
        Ok(FillAreaStyle::from_record(
            self.record(id, "IfcFillAreaStyle")?,
        ))
    }

    /// Project entity `id` as an `IfcFillAreaStyleHatching`.
    pub fn fill_area_style_hatching(
        &self,
        id: EntityId,
    ) -> StyleResult<FillAreaStyleHatching<'m, 's>> {
        Ok(FillAreaStyleHatching::from_record(
            self.record(id, "IfcFillAreaStyleHatching")?,
        ))
    }

    /// Project entity `id` as an `IfcFillAreaStyleTiles`.
    pub fn fill_area_style_tiles(&self, id: EntityId) -> StyleResult<FillAreaStyleTiles<'m, 's>> {
        Ok(FillAreaStyleTiles::from_record(
            self.record(id, "IfcFillAreaStyleTiles")?,
        ))
    }

    /// Project entity `id` as an `IfcSurfaceTexture`.
    pub fn surface_texture(&self, id: EntityId) -> StyleResult<SurfaceTexture<'m, 's>> {
        Ok(SurfaceTexture::from_record(
            self.record(id, "IfcSurfaceTexture")?,
        ))
    }

    /// Project entity `id` as an `IfcImageTexture`.
    pub fn image_texture(&self, id: EntityId) -> StyleResult<ImageTexture<'m, 's>> {
        Ok(ImageTexture::from_record(
            self.record(id, "IfcImageTexture")?,
        ))
    }

    /// Project entity `id` as an `IfcTextureCoordinate`.
    pub fn texture_coordinate(&self, id: EntityId) -> StyleResult<TextureCoordinate<'m, 's>> {
        Ok(TextureCoordinate::from_record(
            self.record(id, "IfcTextureCoordinate")?,
        ))
    }

    /// Project entity `id` as an `IfcIndexedTextureMap`.
    pub fn indexed_texture_map(&self, id: EntityId) -> StyleResult<IndexedTextureMap<'m, 's>> {
        Ok(IndexedTextureMap::from_record(
            self.record(id, "IfcIndexedTextureMap")?,
        ))
    }

    /// Project entity `id` as an `IfcTextStyle`.
    pub fn text_style(&self, id: EntityId) -> StyleResult<TextStyle<'m, 's>> {
        Ok(TextStyle::from_record(self.record(id, "IfcTextStyle")?))
    }

    /// Project entity `id` as an `IfcTextStyleFontModel`.
    pub fn text_style_font_model(&self, id: EntityId) -> StyleResult<TextStyleFontModel<'m, 's>> {
        Ok(TextStyleFontModel::from_record(
            self.record(id, "IfcTextStyleFontModel")?,
        ))
    }

    /// Project entity `id` as an IFC2x3 `IfcPresentationStyleAssignment`.
    pub fn presentation_style_assignment(
        &self,
        id: EntityId,
    ) -> StyleResult<PresentationStyleAssignment<'m, 's>> {
        Ok(PresentationStyleAssignment::from_record(
            self.record(id, "IfcPresentationStyleAssignment")?,
        ))
    }

    /// Project entity `id` as an `IfcSurfaceStyleLighting`.
    pub fn surface_style_lighting(
        &self,
        id: EntityId,
    ) -> StyleResult<SurfaceStyleLighting<'m, 's>> {
        Ok(SurfaceStyleLighting::from_record(
            self.record(id, "IfcSurfaceStyleLighting")?,
        ))
    }

    /// Project entity `id` as an `IfcSurfaceStyleRefraction`.
    pub fn surface_style_refraction(
        &self,
        id: EntityId,
    ) -> StyleResult<SurfaceStyleRefraction<'m, 's>> {
        Ok(SurfaceStyleRefraction::from_record(
            self.record(id, "IfcSurfaceStyleRefraction")?,
        ))
    }

    /// Project entity `id` as an `IfcSurfaceStyleWithTextures`.
    pub fn surface_style_with_textures(
        &self,
        id: EntityId,
    ) -> StyleResult<SurfaceStyleWithTextures<'m, 's>> {
        Ok(SurfaceStyleWithTextures::from_record(
            self.record(id, "IfcSurfaceStyleWithTextures")?,
        ))
    }

    /// Project entity `id` as an `IfcCurveStyleFont`.
    pub fn curve_style_font(&self, id: EntityId) -> StyleResult<CurveStyleFont<'m, 's>> {
        Ok(CurveStyleFont::from_record(
            self.record(id, "IfcCurveStyleFont")?,
        ))
    }

    /// Project entity `id` as an `IfcCurveStyleFontPattern`.
    pub fn curve_style_font_pattern(
        &self,
        id: EntityId,
    ) -> StyleResult<CurveStyleFontPattern<'m, 's>> {
        Ok(CurveStyleFontPattern::from_record(
            self.record(id, "IfcCurveStyleFontPattern")?,
        ))
    }

    /// Project entity `id` as an `IfcBlobTexture`.
    pub fn blob_texture(&self, id: EntityId) -> StyleResult<BlobTexture<'m, 's>> {
        Ok(BlobTexture::from_record(self.record(id, "IfcBlobTexture")?))
    }

    /// Project entity `id` as an `IfcPixelTexture`.
    pub fn pixel_texture(&self, id: EntityId) -> StyleResult<PixelTexture<'m, 's>> {
        Ok(PixelTexture::from_record(
            self.record(id, "IfcPixelTexture")?,
        ))
    }

    /// Project entity `id` as an `IfcTextureVertex`.
    pub fn texture_vertex(&self, id: EntityId) -> StyleResult<TextureVertex<'m, 's>> {
        Ok(TextureVertex::from_record(
            self.record(id, "IfcTextureVertex")?,
        ))
    }

    /// Project entity `id` as an `IfcTextureVertexList`.
    pub fn texture_vertex_list(&self, id: EntityId) -> StyleResult<TextureVertexList<'m, 's>> {
        Ok(TextureVertexList::from_record(
            self.record(id, "IfcTextureVertexList")?,
        ))
    }

    /// Project entity `id` as an `IfcLightSource`, whatever its subtype.
    ///
    /// Use this when only the shared colour/intensity attributes are needed;
    /// call [`LightSource::kind`] to decide which concrete projection below
    /// to reach for.
    pub fn light_source(&self, id: EntityId) -> StyleResult<LightSource<'m, 's>> {
        Ok(LightSource::from_record(self.record(id, "IfcLightSource")?))
    }

    /// Project entity `id` as an `IfcLightSourceAmbient`.
    pub fn light_source_ambient(&self, id: EntityId) -> StyleResult<LightSourceAmbient<'m, 's>> {
        Ok(LightSourceAmbient::from_record(
            self.record(id, "IfcLightSourceAmbient")?,
        ))
    }

    /// Project entity `id` as an `IfcLightSourceDirectional`.
    pub fn light_source_directional(
        &self,
        id: EntityId,
    ) -> StyleResult<LightSourceDirectional<'m, 's>> {
        Ok(LightSourceDirectional::from_record(
            self.record(id, "IfcLightSourceDirectional")?,
        ))
    }

    /// Project entity `id` as an `IfcLightSourceGoniometric`.
    pub fn light_source_goniometric(
        &self,
        id: EntityId,
    ) -> StyleResult<LightSourceGoniometric<'m, 's>> {
        Ok(LightSourceGoniometric::from_record(
            self.record(id, "IfcLightSourceGoniometric")?,
        ))
    }

    /// Project entity `id` as an `IfcLightSourcePositional`.
    ///
    /// Accepts an `IfcLightSourceSpot` too, since a spot *is a* positional
    /// light; use [`Self::light_source_spot`] to reach the cone attributes.
    pub fn light_source_positional(
        &self,
        id: EntityId,
    ) -> StyleResult<LightSourcePositional<'m, 's>> {
        Ok(LightSourcePositional::from_record(
            self.record(id, "IfcLightSourcePositional")?,
        ))
    }

    /// Project entity `id` as an `IfcLightSourceSpot`.
    pub fn light_source_spot(&self, id: EntityId) -> StyleResult<LightSourceSpot<'m, 's>> {
        Ok(LightSourceSpot::from_record(
            self.record(id, "IfcLightSourceSpot")?,
        ))
    }

    /// Project entity `id` as an `IfcLightIntensityDistribution`.
    pub fn light_intensity_distribution(
        &self,
        id: EntityId,
    ) -> StyleResult<LightIntensityDistribution<'m, 's>> {
        Ok(LightIntensityDistribution::from_record(
            self.record(id, "IfcLightIntensityDistribution")?,
        ))
    }

    /// Project entity `id` as an `IfcLightDistributionData`.
    pub fn light_distribution_data(
        &self,
        id: EntityId,
    ) -> StyleResult<LightDistributionData<'m, 's>> {
        Ok(LightDistributionData::from_record(
            self.record(id, "IfcLightDistributionData")?,
        ))
    }

    /// Project entity `id` as an `IfcPlanarExtent`.
    ///
    /// Accepts an `IfcPlanarBox` too, since a box *is a* planar extent; use
    /// [`Self::planar_box`] to reach its placement.
    pub fn planar_extent(&self, id: EntityId) -> StyleResult<PlanarExtent<'m, 's>> {
        Ok(PlanarExtent::from_record(
            self.record(id, "IfcPlanarExtent")?,
        ))
    }

    /// Project entity `id` as an `IfcPlanarBox`.
    pub fn planar_box(&self, id: EntityId) -> StyleResult<PlanarBox<'m, 's>> {
        Ok(PlanarBox::from_record(self.record(id, "IfcPlanarBox")?))
    }

    pub(crate) fn record(
        &self,
        id: EntityId,
        expected: &'static str,
    ) -> StyleResult<Record<'m, 's>> {
        if self.schema.entity(expected).is_none() {
            return Err(StyleError::UnsupportedEntity {
                schema: self.schema.name().to_owned(),
                entity: expected,
            });
        }
        let entity = self.model.get(id).ok_or(StyleError::UnknownEntity { id })?;
        if !self.schema.is_a(&entity.type_name, expected) {
            return Err(StyleError::WrongEntityType {
                id,
                expected,
                actual: entity.type_name.to_string(),
            });
        }
        Ok(Record {
            id,
            entity,
            model: self.model,
            schema: self.schema,
        })
    }
}
