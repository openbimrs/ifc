//! Strict, codec-independent IFC presentation and annotation views.
//!
//! Appearance stays separate from geometry: this crate exposes colour, style,
//! texture, layer, styled-item, and annotation semantics over `ifc_model::Model`
//! without placing presentation state in the geometry kernel. Reads and writes
//! resolve attribute slots through the selected `ifc_schema::Schema`, so IFC2x3,
//! IFC4, and IFC4X3 layout drift is explicit rather than guessed.

mod annotation;
mod assignment;
mod authoring;
mod colour;
mod coverage;
mod curve_style;
mod error;
mod extent;
mod fill_style;
mod layer;
mod light;
mod surface_style;
mod text_style;
mod texture;
mod view;

pub use annotation::{
    Annotation, AnnotationFillArea, AnnotationType, BoxAlignment, TextLiteral,
    TextLiteralWithExtent, TextPath,
};
pub use assignment::{
    PresentationStyleAssignment, PresentationStyleMember, ResolvedStyle, StyleSource, StyledItem,
};
pub use authoring::{
    create_annotation, create_annotation_fill_area, create_colour_rgb, create_image_texture,
    create_light_source_ambient, create_light_source_directional, create_light_source_positional,
    create_light_source_spot, create_presentation_layer_with_style, create_styled_item,
    create_surface_style, create_surface_style_lighting, create_surface_style_refraction,
    create_surface_style_shading, create_text_literal, create_text_literal_with_extent,
    create_texture_coordinate_generator, create_texture_map, AnnotationDraft,
    AnnotationFillAreaDraft, Attenuation, ColourRgbDraft, ImageTextureDraft, LightSourceDraft,
    PointLight, PresentationLayerDraft, SpotCone, StyledItemDraft, SurfaceStyleDraft,
    SurfaceStyleShadingDraft, TextLiteralDraft, TextLiteralWithExtentDraft,
};
pub use colour::{ColourOrFactor, ColourRgb};
pub use coverage::{
    AppearanceDeclaration, AppearanceKind, AppearanceSupport, APPEARANCE_DECLARATIONS,
};
pub use curve_style::{CurveStyle, CurveStyleFont, CurveStyleFontPattern};
pub use error::{StyleError, StyleResult};
pub use extent::{PlanarBox, PlanarExtent};
pub use fill_style::{FillAreaStyle, FillAreaStyleHatching, FillAreaStyleTiles};
pub use layer::PresentationLayer;
pub use light::{
    LightDistributionData, LightIntensityDistribution, LightSource, LightSourceAmbient,
    LightSourceDirectional, LightSourceGoniometric, LightSourceKind, LightSourcePositional,
    LightSourceSpot,
};
pub use surface_style::{
    SurfaceSide, SurfaceStyle, SurfaceStyleLighting, SurfaceStyleRefraction, SurfaceStyleRendering,
    SurfaceStyleShading, SurfaceStyleWithTextures,
};
pub use text_style::{TextStyle, TextStyleFontModel};
pub use texture::{
    BlobTexture, ImageTexture, IndexedTextureMap, PixelTexture, SurfaceTexture, TextureCoordinate,
    TextureVertex, TextureVertexList,
};
pub use view::StyleView;
