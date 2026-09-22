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
    create_annotation, create_annotation_fill_area, create_blob_texture, create_colour_rgb,
    create_colour_rgb_list, create_curve_style, create_curve_style_font,
    create_curve_style_font_and_scaling, create_curve_style_font_pattern,
    create_draughting_predefined_colour, create_draughting_predefined_curve_font,
    create_externally_defined_style, create_fill_area_style, create_image_texture,
    create_indexed_colour_map, create_indexed_polygonal_texture_map, create_light_source_ambient,
    create_light_source_directional, create_light_source_positional, create_light_source_spot,
    create_pixel_texture, create_planar_extent, create_presentation_layer_assignment,
    create_presentation_layer_with_style, create_styled_item, create_surface_style,
    create_surface_style_lighting, create_surface_style_refraction, create_surface_style_shading,
    create_surface_style_with_textures, create_text_literal, create_text_literal_with_extent,
    create_text_style, create_text_style_font_model, create_text_style_for_defined_font,
    create_text_style_text_model, create_texture_coordinate_generator,
    create_texture_coordinate_indices, create_texture_coordinate_indices_with_voids,
    create_texture_map, create_texture_vertex, create_texture_vertex_list, AnnotationDraft,
    AnnotationFillAreaDraft, Attenuation, BlobTextureDraft, ColourRgbDraft, CurveStyleDraft,
    CurveWidth, ExternalStyleKind, FillStyleKind, ImageTextureDraft, LightSourceDraft,
    PixelTextureDraft, PointLight, PresentationLayerDraft, SizeValue, SpotCone, StyledItemDraft,
    SurfaceStyleDraft, SurfaceStyleShadingDraft, TextLiteralDraft, TextLiteralWithExtentDraft,
    TextModelDraft, PREDEFINED_COLOUR_NAMES, PREDEFINED_CURVE_FONT_NAMES,
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
