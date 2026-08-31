//! Base surface texture projection across IFC2x3, IFC4, and IFC4X3.

use ifc_model::{EntityId, Value};

use crate::error::StyleResult;
use crate::view::Record;

#[derive(Debug, Clone, Copy)]
pub struct SurfaceTexture<'m, 's> {
    record: Record<'m, 's>,
}

impl<'m, 's> SurfaceTexture<'m, 's> {
    pub(crate) fn from_record(record: Record<'m, 's>) -> Self {
        Self { record }
    }

    pub fn id(&self) -> EntityId {
        self.record.id
    }

    pub fn type_name(&self) -> &'m str {
        &self.record.entity.type_name
    }

    pub fn repeat_s(&self) -> StyleResult<bool> {
        self.record.required_bool("RepeatS")
    }

    pub fn repeat_t(&self) -> StyleResult<bool> {
        self.record.required_bool("RepeatT")
    }

    /// IFC4/IFC4X3 texture mode; absent by schema in IFC2x3.
    pub fn mode(&self) -> StyleResult<Option<&'m str>> {
        self.record.optional_text("Mode")
    }

    /// IFC2x3 texture type; absent by schema in IFC4 and IFC4X3.
    pub fn texture_type(&self) -> StyleResult<Option<&'m str>> {
        self.record.optional_enum("TextureType")
    }

    pub fn texture_transform(&self) -> StyleResult<Option<EntityId>> {
        self.record
            .optional_ref("TextureTransform", "IfcCartesianTransformationOperator2D")
    }

    pub fn parameters(&self) -> StyleResult<Option<&'m Value>> {
        self.record.optional_raw("Parameter")
    }
}

#[derive(Debug, Clone, Copy)]
pub struct BlobTexture<'m, 's> {
    record: Record<'m, 's>,
}

impl<'m, 's> BlobTexture<'m, 's> {
    pub(crate) fn from_record(record: Record<'m, 's>) -> Self {
        Self { record }
    }

    pub fn surface_texture(&self) -> SurfaceTexture<'m, 's> {
        SurfaceTexture::from_record(self.record)
    }

    pub fn raster_format(&self) -> StyleResult<&'m str> {
        self.record.required_text("RasterFormat")
    }

    pub fn raster_code(&self) -> StyleResult<&'m Value> {
        self.record.value("RasterCode")
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PixelTexture<'m, 's> {
    record: Record<'m, 's>,
}

impl<'m, 's> PixelTexture<'m, 's> {
    pub(crate) fn from_record(record: Record<'m, 's>) -> Self {
        Self { record }
    }

    pub fn width(&self) -> StyleResult<i64> {
        self.record.required_integer("Width")
    }

    pub fn height(&self) -> StyleResult<i64> {
        self.record.required_integer("Height")
    }

    pub fn colour_components(&self) -> StyleResult<i64> {
        self.record.required_integer("ColourComponents")
    }

    pub fn pixel(&self) -> StyleResult<&'m Value> {
        self.record.value("Pixel")
    }
}
