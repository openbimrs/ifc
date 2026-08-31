//! Shared schema-resolved borrowed view primitives.

use ifc_model::{Entity, EntityId, Model, Value};
use ifc_schema::Schema;

use crate::error::{StyleError, StyleResult};
use crate::{
    Annotation, AnnotationFillArea, BlobTexture, ColourRgb, CurveStyle, CurveStyleFont,
    CurveStyleFontPattern, FillAreaStyle, FillAreaStyleHatching, FillAreaStyleTiles, ImageTexture,
    IndexedTextureMap, PixelTexture, PresentationLayer, PresentationStyleAssignment, StyledItem,
    SurfaceStyle, SurfaceStyleLighting, SurfaceStyleRefraction, SurfaceStyleRendering,
    SurfaceStyleShading, SurfaceStyleWithTextures, SurfaceTexture, TextLiteral,
    TextLiteralWithExtent, TextStyle, TextStyleFontModel, TextureCoordinate, TextureVertex,
    TextureVertexList,
};

/// Entry point for strict presentation and annotation projections.
#[derive(Debug, Clone, Copy)]
pub struct StyleView<'m, 's> {
    pub(crate) model: &'m Model,
    pub(crate) schema: &'s Schema,
}

impl<'m, 's> StyleView<'m, 's> {
    #[must_use]
    pub fn new(model: &'m Model, schema: &'s Schema) -> Self {
        Self { model, schema }
    }

    pub fn annotation(&self, id: EntityId) -> StyleResult<Annotation<'m, 's>> {
        Annotation::from_record(self.record(id, "IfcAnnotation")?)
    }

    pub fn text_literal(&self, id: EntityId) -> StyleResult<TextLiteral<'m, 's>> {
        TextLiteral::from_record(self.record(id, "IfcTextLiteral")?)
    }

    pub fn text_literal_with_extent(
        &self,
        id: EntityId,
    ) -> StyleResult<TextLiteralWithExtent<'m, 's>> {
        TextLiteralWithExtent::from_record(self.record(id, "IfcTextLiteralWithExtent")?)
    }

    pub fn annotation_fill_area(&self, id: EntityId) -> StyleResult<AnnotationFillArea<'m, 's>> {
        AnnotationFillArea::from_record(self.record(id, "IfcAnnotationFillArea")?)
    }

    pub fn colour_rgb(&self, id: EntityId) -> StyleResult<ColourRgb<'m, 's>> {
        Ok(ColourRgb::from_record(self.record(id, "IfcColourRgb")?))
    }

    pub fn surface_style_shading(&self, id: EntityId) -> StyleResult<SurfaceStyleShading<'m, 's>> {
        Ok(SurfaceStyleShading::from_record(
            self.record(id, "IfcSurfaceStyleShading")?,
        ))
    }

    pub fn surface_style(&self, id: EntityId) -> StyleResult<SurfaceStyle<'m, 's>> {
        Ok(SurfaceStyle::from_record(
            self.record(id, "IfcSurfaceStyle")?,
        ))
    }

    pub fn styled_item(&self, id: EntityId) -> StyleResult<StyledItem<'m, 's>> {
        Ok(StyledItem::from_record(self.record(id, "IfcStyledItem")?))
    }

    pub fn presentation_layer(&self, id: EntityId) -> StyleResult<PresentationLayer<'m, 's>> {
        Ok(PresentationLayer::from_record(
            self.record(id, "IfcPresentationLayerAssignment")?,
        ))
    }

    pub fn surface_style_rendering(
        &self,
        id: EntityId,
    ) -> StyleResult<SurfaceStyleRendering<'m, 's>> {
        Ok(SurfaceStyleRendering::from_record(
            self.record(id, "IfcSurfaceStyleRendering")?,
        ))
    }

    pub fn curve_style(&self, id: EntityId) -> StyleResult<CurveStyle<'m, 's>> {
        Ok(CurveStyle::from_record(self.record(id, "IfcCurveStyle")?))
    }

    pub fn fill_area_style(&self, id: EntityId) -> StyleResult<FillAreaStyle<'m, 's>> {
        Ok(FillAreaStyle::from_record(
            self.record(id, "IfcFillAreaStyle")?,
        ))
    }

    pub fn fill_area_style_hatching(
        &self,
        id: EntityId,
    ) -> StyleResult<FillAreaStyleHatching<'m, 's>> {
        Ok(FillAreaStyleHatching::from_record(
            self.record(id, "IfcFillAreaStyleHatching")?,
        ))
    }

    pub fn fill_area_style_tiles(&self, id: EntityId) -> StyleResult<FillAreaStyleTiles<'m, 's>> {
        Ok(FillAreaStyleTiles::from_record(
            self.record(id, "IfcFillAreaStyleTiles")?,
        ))
    }

    pub fn surface_texture(&self, id: EntityId) -> StyleResult<SurfaceTexture<'m, 's>> {
        Ok(SurfaceTexture::from_record(
            self.record(id, "IfcSurfaceTexture")?,
        ))
    }

    pub fn image_texture(&self, id: EntityId) -> StyleResult<ImageTexture<'m, 's>> {
        Ok(ImageTexture::from_record(
            self.record(id, "IfcImageTexture")?,
        ))
    }

    pub fn texture_coordinate(&self, id: EntityId) -> StyleResult<TextureCoordinate<'m, 's>> {
        Ok(TextureCoordinate::from_record(
            self.record(id, "IfcTextureCoordinate")?,
        ))
    }

    pub fn indexed_texture_map(&self, id: EntityId) -> StyleResult<IndexedTextureMap<'m, 's>> {
        Ok(IndexedTextureMap::from_record(
            self.record(id, "IfcIndexedTextureMap")?,
        ))
    }

    pub fn text_style(&self, id: EntityId) -> StyleResult<TextStyle<'m, 's>> {
        Ok(TextStyle::from_record(self.record(id, "IfcTextStyle")?))
    }

    pub fn text_style_font_model(&self, id: EntityId) -> StyleResult<TextStyleFontModel<'m, 's>> {
        Ok(TextStyleFontModel::from_record(
            self.record(id, "IfcTextStyleFontModel")?,
        ))
    }

    pub fn presentation_style_assignment(
        &self,
        id: EntityId,
    ) -> StyleResult<PresentationStyleAssignment<'m, 's>> {
        Ok(PresentationStyleAssignment::from_record(
            self.record(id, "IfcPresentationStyleAssignment")?,
        ))
    }

    pub fn surface_style_lighting(
        &self,
        id: EntityId,
    ) -> StyleResult<SurfaceStyleLighting<'m, 's>> {
        Ok(SurfaceStyleLighting::from_record(
            self.record(id, "IfcSurfaceStyleLighting")?,
        ))
    }

    pub fn surface_style_refraction(
        &self,
        id: EntityId,
    ) -> StyleResult<SurfaceStyleRefraction<'m, 's>> {
        Ok(SurfaceStyleRefraction::from_record(
            self.record(id, "IfcSurfaceStyleRefraction")?,
        ))
    }

    pub fn surface_style_with_textures(
        &self,
        id: EntityId,
    ) -> StyleResult<SurfaceStyleWithTextures<'m, 's>> {
        Ok(SurfaceStyleWithTextures::from_record(
            self.record(id, "IfcSurfaceStyleWithTextures")?,
        ))
    }

    pub fn curve_style_font(&self, id: EntityId) -> StyleResult<CurveStyleFont<'m, 's>> {
        Ok(CurveStyleFont::from_record(
            self.record(id, "IfcCurveStyleFont")?,
        ))
    }

    pub fn curve_style_font_pattern(
        &self,
        id: EntityId,
    ) -> StyleResult<CurveStyleFontPattern<'m, 's>> {
        Ok(CurveStyleFontPattern::from_record(
            self.record(id, "IfcCurveStyleFontPattern")?,
        ))
    }

    pub fn blob_texture(&self, id: EntityId) -> StyleResult<BlobTexture<'m, 's>> {
        Ok(BlobTexture::from_record(self.record(id, "IfcBlobTexture")?))
    }

    pub fn pixel_texture(&self, id: EntityId) -> StyleResult<PixelTexture<'m, 's>> {
        Ok(PixelTexture::from_record(
            self.record(id, "IfcPixelTexture")?,
        ))
    }

    pub fn texture_vertex(&self, id: EntityId) -> StyleResult<TextureVertex<'m, 's>> {
        Ok(TextureVertex::from_record(
            self.record(id, "IfcTextureVertex")?,
        ))
    }

    pub fn texture_vertex_list(&self, id: EntityId) -> StyleResult<TextureVertexList<'m, 's>> {
        Ok(TextureVertexList::from_record(
            self.record(id, "IfcTextureVertexList")?,
        ))
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

#[derive(Debug, Clone, Copy)]
pub(crate) struct Record<'m, 's> {
    pub(crate) id: EntityId,
    pub(crate) entity: &'m Entity,
    pub(crate) model: &'m Model,
    pub(crate) schema: &'s Schema,
}

impl<'m, 's> Record<'m, 's> {
    pub(crate) fn has_attribute(&self, name: &str) -> bool {
        self.slot(name).is_some()
    }

    fn slot(&self, name: &str) -> Option<usize> {
        self.schema
            .attributes(&self.entity.type_name)
            .iter()
            .position(|attribute| attribute.name.eq_ignore_ascii_case(name))
    }

    pub(crate) fn value(&self, attribute: &'static str) -> StyleResult<&'m Value> {
        let Some(slot) = self.slot(attribute) else {
            return Err(StyleError::UnsupportedAttribute {
                schema: self.schema.name().to_owned(),
                entity: "presentation entity",
                attribute,
            });
        };
        self.entity
            .attribute(slot)
            .ok_or_else(|| StyleError::MissingAttribute {
                entity: self.entity.type_name.to_string(),
                id: self.id,
                attribute,
            })
    }

    pub(crate) fn required_text(&self, attribute: &'static str) -> StyleResult<&'m str> {
        let value = self.value(attribute)?;
        match value.unwrap_typed() {
            Value::Text(text) => Ok(text),
            Value::Null | Value::Derived => Err(self.missing(attribute)),
            other => Err(self.invalid(attribute, other)),
        }
    }

    pub(crate) fn optional_text(&self, attribute: &'static str) -> StyleResult<Option<&'m str>> {
        if !self.has_attribute(attribute) {
            return Ok(None);
        }
        match self.value(attribute)?.unwrap_typed() {
            Value::Null | Value::Derived => Ok(None),
            Value::Text(text) => Ok(Some(text)),
            other => Err(self.invalid(attribute, other)),
        }
    }

    pub(crate) fn required_enum(&self, attribute: &'static str) -> StyleResult<&'m str> {
        match self.value(attribute)?.unwrap_typed() {
            Value::Enum(value) => Ok(value),
            Value::Null | Value::Derived => Err(self.missing(attribute)),
            other => Err(self.invalid(attribute, other)),
        }
    }

    pub(crate) fn optional_enum(&self, attribute: &'static str) -> StyleResult<Option<&'m str>> {
        if !self.has_attribute(attribute) {
            return Ok(None);
        }
        match self.value(attribute)?.unwrap_typed() {
            Value::Null | Value::Derived => Ok(None),
            Value::Enum(value) => Ok(Some(value)),
            other => Err(self.invalid(attribute, other)),
        }
    }

    pub(crate) fn required_number(&self, attribute: &'static str) -> StyleResult<f64> {
        match self.value(attribute)?.unwrap_typed() {
            Value::Real(value) => Ok(*value),
            Value::Integer(value) => Ok(*value as f64),
            Value::Null | Value::Derived => Err(self.missing(attribute)),
            value => Err(self.invalid(attribute, value)),
        }
    }

    pub(crate) fn optional_number(&self, attribute: &'static str) -> StyleResult<Option<f64>> {
        if !self.has_attribute(attribute) {
            return Ok(None);
        }
        match self.value(attribute)?.unwrap_typed() {
            Value::Null | Value::Derived => Ok(None),
            Value::Real(value) => Ok(Some(*value)),
            Value::Integer(value) => Ok(Some(*value as f64)),
            value => Err(self.invalid(attribute, value)),
        }
    }

    pub(crate) fn normalized(&self, attribute: &'static str) -> StyleResult<f64> {
        let value = self.required_number(attribute)?;
        if value.is_finite() && (0.0..=1.0).contains(&value) {
            Ok(value)
        } else {
            Err(StyleError::OutOfRange {
                entity: "IfcNormalisedRatioMeasure",
                id: self.id,
                attribute,
                value,
                minimum: 0.0,
                maximum: 1.0,
            })
        }
    }

    pub(crate) fn optional_normalized(&self, attribute: &'static str) -> StyleResult<Option<f64>> {
        let Some(value) = self.optional_number(attribute)? else {
            return Ok(None);
        };
        if value.is_finite() && (0.0..=1.0).contains(&value) {
            Ok(Some(value))
        } else {
            Err(StyleError::OutOfRange {
                entity: "IfcNormalisedRatioMeasure",
                id: self.id,
                attribute,
                value,
                minimum: 0.0,
                maximum: 1.0,
            })
        }
    }

    pub(crate) fn required_refs_any(&self, attribute: &'static str) -> StyleResult<Vec<EntityId>> {
        let value = self.value(attribute)?;
        let Value::List(items) = value.unwrap_typed() else {
            return if matches!(value, Value::Null | Value::Derived) {
                Err(self.missing(attribute))
            } else {
                Err(self.invalid(attribute, value))
            };
        };
        let mut out = Vec::with_capacity(items.len());
        for item in items {
            let Some(target) = item.unwrap_typed().as_ref_id() else {
                return Err(self.invalid(attribute, item));
            };
            if self.model.get(target).is_none() {
                return Err(StyleError::DanglingReference {
                    source_id: self.id,
                    target,
                });
            }
            out.push(target);
        }
        Ok(out)
    }

    pub(crate) fn required_refs(
        &self,
        attribute: &'static str,
        expected: &'static str,
        minimum: usize,
        maximum: Option<usize>,
    ) -> StyleResult<Vec<EntityId>> {
        let targets = self.required_refs_any(attribute)?;
        self.check_ref_count(attribute, &targets, minimum, maximum)?;
        for target in &targets {
            self.check_reference(*target, expected)?;
        }
        Ok(targets)
    }

    pub(crate) fn required_refs_select(
        &self,
        attribute: &'static str,
        expected: &'static str,
        members: &[&str],
        minimum: usize,
        maximum: Option<usize>,
    ) -> StyleResult<Vec<EntityId>> {
        let targets = self.required_refs_any(attribute)?;
        self.check_ref_count(attribute, &targets, minimum, maximum)?;
        for target in &targets {
            let target_entity = self
                .model
                .get(*target)
                .ok_or(StyleError::DanglingReference {
                    source_id: self.id,
                    target: *target,
                })?;
            if !members
                .iter()
                .any(|member| self.schema.is_a(&target_entity.type_name, member))
            {
                return Err(StyleError::ReferenceType {
                    target: *target,
                    expected,
                    actual: target_entity.type_name.to_string(),
                });
            }
        }
        Ok(targets)
    }

    pub(crate) fn check_ref_count(
        &self,
        attribute: &'static str,
        targets: &[EntityId],
        minimum: usize,
        maximum: Option<usize>,
    ) -> StyleResult<()> {
        if targets.len() < minimum || maximum.is_some_and(|maximum| targets.len() > maximum) {
            let expected = maximum.map_or_else(
                || format!("at least {minimum}"),
                |maximum| format!("{minimum}..={maximum}"),
            );
            return Err(StyleError::InvalidValue {
                entity: self.entity.type_name.to_string(),
                id: self.id,
                attribute,
                value: format!("{} reference(s), expected {expected}", targets.len()),
            });
        }
        Ok(())
    }

    pub(crate) fn optional_raw(&self, attribute: &'static str) -> StyleResult<Option<&'m Value>> {
        if !self.has_attribute(attribute) {
            return Ok(None);
        }
        let value = self.value(attribute)?;
        if matches!(value, Value::Null | Value::Derived) {
            Ok(None)
        } else {
            Ok(Some(value))
        }
    }

    pub(crate) fn required_integer(&self, attribute: &'static str) -> StyleResult<i64> {
        match self.value(attribute)?.unwrap_typed() {
            Value::Integer(value) => Ok(*value),
            Value::Null | Value::Derived => Err(self.missing(attribute)),
            value => Err(self.invalid(attribute, value)),
        }
    }

    pub(crate) fn required_bool(&self, attribute: &'static str) -> StyleResult<bool> {
        match self.value(attribute)?.unwrap_typed() {
            Value::Bool(value) => Ok(*value),
            Value::Null | Value::Derived => Err(self.missing(attribute)),
            value => Err(self.invalid(attribute, value)),
        }
    }

    pub(crate) fn optional_bool(&self, attribute: &'static str) -> StyleResult<Option<bool>> {
        if !self.has_attribute(attribute) {
            return Ok(None);
        }
        match self.value(attribute)?.unwrap_typed() {
            Value::Null | Value::Derived => Ok(None),
            Value::Bool(value) => Ok(Some(*value)),
            value => Err(self.invalid(attribute, value)),
        }
    }

    pub(crate) fn optional_ref_any(
        &self,
        attribute: &'static str,
    ) -> StyleResult<Option<EntityId>> {
        if !self.has_attribute(attribute) {
            return Ok(None);
        }
        match self.value(attribute)?.unwrap_typed() {
            Value::Null | Value::Derived => Ok(None),
            Value::Ref(target) => {
                if self.model.get(*target).is_none() {
                    Err(StyleError::DanglingReference {
                        source_id: self.id,
                        target: *target,
                    })
                } else {
                    Ok(Some(*target))
                }
            }
            value => Err(self.invalid(attribute, value)),
        }
    }

    pub(crate) fn required_ref(
        &self,
        attribute: &'static str,
        expected: &'static str,
    ) -> StyleResult<EntityId> {
        match self.value(attribute)?.unwrap_typed() {
            Value::Ref(id) => {
                self.check_reference(*id, expected)?;
                Ok(*id)
            }
            Value::Null | Value::Derived => Err(self.missing(attribute)),
            other => Err(self.invalid(attribute, other)),
        }
    }

    pub(crate) fn optional_ref(
        &self,
        attribute: &'static str,
        expected: &'static str,
    ) -> StyleResult<Option<EntityId>> {
        let target = self.optional_ref_any(attribute)?;
        if let Some(target) = target {
            self.check_reference(target, expected)?;
        }
        Ok(target)
    }

    pub(crate) fn optional_ref_select(
        &self,
        attribute: &'static str,
        expected: &'static str,
        members: &[&str],
    ) -> StyleResult<Option<EntityId>> {
        let target = self.optional_ref_any(attribute)?;
        if let Some(target) = target {
            self.check_reference_select(target, expected, members)?;
        }
        Ok(target)
    }

    pub(crate) fn required_ref_select(
        &self,
        attribute: &'static str,
        expected: &'static str,
        members: &[&str],
    ) -> StyleResult<EntityId> {
        match self.value(attribute)?.unwrap_typed() {
            Value::Ref(target) => {
                self.check_reference_select(*target, expected, members)?;
                Ok(*target)
            }
            Value::Null | Value::Derived => Err(self.missing(attribute)),
            other => Err(self.invalid(attribute, other)),
        }
    }

    pub(crate) fn optional_refs(
        &self,
        attribute: &'static str,
        expected: &'static str,
    ) -> StyleResult<Vec<EntityId>> {
        match self.value(attribute)?.unwrap_typed() {
            Value::Null | Value::Derived => Ok(Vec::new()),
            Value::List(values) => values
                .iter()
                .map(|value| match value.unwrap_typed() {
                    Value::Ref(id) => {
                        self.check_reference(*id, expected)?;
                        Ok(*id)
                    }
                    other => Err(self.invalid(attribute, other)),
                })
                .collect(),
            other => Err(self.invalid(attribute, other)),
        }
    }

    pub(crate) fn check_reference(
        &self,
        target: EntityId,
        expected: &'static str,
    ) -> StyleResult<()> {
        let target_entity = self
            .model
            .get(target)
            .ok_or(StyleError::DanglingReference {
                source_id: self.id,
                target,
            })?;
        if !self.schema.is_a(&target_entity.type_name, expected) {
            return Err(StyleError::ReferenceType {
                target,
                expected,
                actual: target_entity.type_name.to_string(),
            });
        }
        Ok(())
    }

    pub(crate) fn check_reference_select(
        &self,
        target: EntityId,
        expected: &'static str,
        members: &[&str],
    ) -> StyleResult<()> {
        let target_entity = self
            .model
            .get(target)
            .ok_or(StyleError::DanglingReference {
                source_id: self.id,
                target,
            })?;
        if !members
            .iter()
            .any(|member| self.schema.is_a(&target_entity.type_name, member))
        {
            return Err(StyleError::ReferenceType {
                target,
                expected,
                actual: target_entity.type_name.to_string(),
            });
        }
        Ok(())
    }

    fn missing(&self, attribute: &'static str) -> StyleError {
        StyleError::MissingAttribute {
            entity: self.entity.type_name.to_string(),
            id: self.id,
            attribute,
        }
    }

    pub(crate) fn invalid(&self, attribute: &'static str, value: &Value) -> StyleError {
        StyleError::InvalidValue {
            entity: self.entity.type_name.to_string(),
            id: self.id,
            attribute,
            value: format!("{value:?}"),
        }
    }
}
