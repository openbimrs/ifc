//! Base surface texture projection across IFC2x3, IFC4, and IFC4X3.

use ifc_model::{EntityId, Value};

use crate::error::StyleResult;
use crate::view::Record;

/// Borrowed projection of `IfcSurfaceTexture`, the abstract base attributes
/// shared by `IfcBlobTexture`, `IfcPixelTexture`, and `IfcImageTexture`.
#[derive(Debug, Clone, Copy)]
pub struct SurfaceTexture<'m, 's> {
    record: Record<'m, 's>,
}

impl<'m, 's> SurfaceTexture<'m, 's> {
    pub(crate) fn from_record(record: Record<'m, 's>) -> Self {
        Self { record }
    }

    /// The entity id of this surface texture.
    pub fn id(&self) -> EntityId {
        self.record.id
    }

    /// The concrete IFC entity type name (e.g. `"IFCIMAGETEXTURE"`).
    pub fn type_name(&self) -> &'m str {
        &self.record.entity.type_name
    }

    /// The `RepeatS` flag: whether the texture tiles along its S axis. Mandatory.
    pub fn repeat_s(&self) -> StyleResult<bool> {
        self.record.required_bool("RepeatS")
    }

    /// The `RepeatT` flag: whether the texture tiles along its T axis. Mandatory.
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

    /// The `TextureTransform` attribute, when authored.
    pub fn texture_transform(&self) -> StyleResult<Option<EntityId>> {
        self.record
            .optional_ref("TextureTransform", "IfcCartesianTransformationOperator2D")
    }

    /// The `Parameter` attribute (a list of texture-specific parameter
    /// strings), when authored.
    pub fn parameters(&self) -> StyleResult<Option<&'m Value>> {
        self.record.optional_raw("Parameter")
    }
}

/// Borrowed projection of `IfcBlobTexture`: a raster image embedded inline as binary data.
#[derive(Debug, Clone, Copy)]
pub struct BlobTexture<'m, 's> {
    record: Record<'m, 's>,
}

impl<'m, 's> BlobTexture<'m, 's> {
    pub(crate) fn from_record(record: Record<'m, 's>) -> Self {
        Self { record }
    }

    /// The base `IfcSurfaceTexture` attributes shared with other texture kinds.
    pub fn surface_texture(&self) -> SurfaceTexture<'m, 's> {
        SurfaceTexture::from_record(self.record)
    }

    /// The `RasterFormat` attribute (e.g. `"PNG"`, `"JPEG"`). Mandatory.
    pub fn raster_format(&self) -> StyleResult<&'m str> {
        self.record.required_text("RasterFormat")
    }

    /// The `RasterCode` attribute: the raw binary image data. Mandatory.
    pub fn raster_code(&self) -> StyleResult<&'m Value> {
        self.record.value("RasterCode")
    }
}

/// Borrowed projection of `IfcPixelTexture`: an explicit width/height/colour raster.
#[derive(Debug, Clone, Copy)]
pub struct PixelTexture<'m, 's> {
    record: Record<'m, 's>,
}

impl<'m, 's> PixelTexture<'m, 's> {
    pub(crate) fn from_record(record: Record<'m, 's>) -> Self {
        Self { record }
    }

    /// The `Width` attribute in pixels. Mandatory.
    pub fn width(&self) -> StyleResult<i64> {
        self.record.required_integer("Width")
    }

    /// The `Height` attribute in pixels. Mandatory.
    pub fn height(&self) -> StyleResult<i64> {
        self.record.required_integer("Height")
    }

    /// The `ColourComponents` attribute: the number of colour channels per pixel. Mandatory.
    pub fn colour_components(&self) -> StyleResult<i64> {
        self.record.required_integer("ColourComponents")
    }

    /// The `Pixel` attribute: the raw pixel byte list. Mandatory.
    pub fn pixel(&self) -> StyleResult<&'m Value> {
        self.record.value("Pixel")
    }
}
