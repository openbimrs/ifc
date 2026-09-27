//! Transaction-staged authoring for presentation and annotation entities.

use std::sync::Arc;

use ifc_model::{Edit, Entity, EntityId, Model, Transaction, Value};
use ifc_schema::Schema;

use crate::annotation::{AnnotationType, BoxAlignment, TextPath};
use crate::error::{StyleError, StyleResult};

mod external;
mod fill;
mod light;
mod presentation;
mod surface;
mod texture;

pub use surface::{
    create_colour_rgb, create_presentation_layer_with_style, create_styled_item,
    create_surface_style, create_surface_style_rendering, create_surface_style_shading,
    ColourRgbDraft, PresentationLayerDraft, StyledItemDraft, SurfaceStyleDraft,
    SurfaceStyleRenderingDraft, SurfaceStyleShadingDraft,
};

pub use external::{
    create_blob_texture, create_externally_defined_style, create_indexed_polygonal_texture_map,
    create_indexed_triangle_texture_map, create_light_distribution_data,
    create_light_intensity_distribution, create_surface_style_with_textures, create_text_style,
    create_text_style_text_model, create_texture_coordinate_indices,
    create_texture_coordinate_indices_with_voids, BlobTextureDraft, ExternalStyleKind, SizeValue,
    TextModelDraft,
};
pub use fill::{create_fill_area_style_hatching, create_fill_area_style_tiles, HatchLineDistance};
pub use light::{
    create_light_source_ambient, create_light_source_directional, create_light_source_goniometric,
    create_light_source_positional, create_light_source_spot, create_surface_style_lighting,
    create_surface_style_refraction, Attenuation, GoniometricLight, LightSourceDraft, PointLight,
    SpotCone,
};
pub use presentation::{
    create_colour_rgb_list, create_curve_style, create_curve_style_font,
    create_curve_style_font_and_scaling, create_curve_style_font_pattern,
    create_draughting_predefined_colour, create_draughting_predefined_curve_font,
    create_fill_area_style, create_indexed_colour_map, create_pixel_texture,
    create_presentation_layer_assignment, create_text_style_font_model,
    create_text_style_for_defined_font, create_texture_vertex, create_texture_vertex_list,
    CurveStyleDraft, CurveWidth, FillStyleKind, PixelTextureDraft, PREDEFINED_COLOUR_NAMES,
    PREDEFINED_CURVE_FONT_NAMES,
};
pub use texture::{
    create_image_texture, create_texture_coordinate_generator, create_texture_map,
    ImageTextureDraft,
};

/// Draft input for [`create_annotation`]: the writable attributes of a new
/// `IfcAnnotation`.
#[derive(Debug, Clone, Default)]
pub struct AnnotationDraft<'a> {
    /// The `GlobalId` (IFC GUID); must parse as a valid base64-like GUID.
    pub global_id: &'a str,
    /// The `OwnerHistory` reference, when supplied.
    pub owner_history: Option<EntityId>,
    /// The `Name` attribute, when supplied.
    pub name: Option<&'a str>,
    /// The `Description` attribute, when supplied.
    pub description: Option<&'a str>,
    /// The `ObjectType` attribute. Required (non-empty) when `predefined_type`
    /// is `AnnotationType::UserDefined`.
    pub object_type: Option<&'a str>,
    /// The `ObjectPlacement` reference, when supplied.
    pub object_placement: Option<EntityId>,
    /// The `Representation` reference, when supplied.
    pub representation: Option<EntityId>,
    /// The IFC4X3 `PredefinedType`, when supplied.
    pub predefined_type: Option<AnnotationType>,
}

/// Draft input for [`create_text_literal`]: the writable attributes of a new
/// `IfcTextLiteral`.
#[derive(Debug, Clone, Copy)]
pub struct TextLiteralDraft<'a> {
    /// The `Literal` attribute; must be non-empty.
    pub literal: &'a str,
    /// The `Placement` reference to an `IfcPlacement`.
    pub placement: EntityId,
    /// The `Path` attribute.
    pub path: TextPath,
}

/// Draft input for [`create_text_literal_with_extent`]: the writable
/// attributes of a new `IfcTextLiteralWithExtent`.
#[derive(Debug, Clone, Copy)]
pub struct TextLiteralWithExtentDraft<'a> {
    /// The `Literal` attribute; must be non-empty.
    pub literal: &'a str,
    /// The `Placement` reference to an `IfcPlacement`.
    pub placement: EntityId,
    /// The `Path` attribute.
    pub path: TextPath,
    /// The `Extent` reference to an `IfcPlanarExtent`.
    pub extent: EntityId,
    /// The `BoxAlignment` attribute.
    pub box_alignment: BoxAlignment,
}

/// Draft input for [`create_annotation_fill_area`]: the writable attributes
/// of a new `IfcAnnotationFillArea`.
#[derive(Debug, Clone)]
pub struct AnnotationFillAreaDraft {
    /// The `OuterBoundary` reference to an `IfcCurve`.
    pub outer_boundary: EntityId,
    /// The `InnerBoundaries` references, if any; an empty list is written
    /// as the IFC null value.
    pub inner_boundaries: Vec<EntityId>,
}

/// Stage a new `IfcAnnotation` in `tx`. Fails if `GlobalId` does not parse as
/// a GUID, if `PredefinedType` is `USERDEFINED` with an empty `ObjectType`,
/// or if any referenced entity does not resolve to its expected IFC type.
pub fn create_annotation(
    tx: &mut Transaction,
    model: &Model,
    schema: &Schema,
    draft: AnnotationDraft<'_>,
) -> StyleResult<EntityId> {
    if ifc_model::guid::Guid::parse(draft.global_id).is_none() {
        return Err(invalid_authoring(
            "IfcAnnotation",
            "GlobalId",
            draft.global_id,
        ));
    }
    if draft.predefined_type == Some(AnnotationType::UserDefined)
        && draft
            .object_type
            .is_none_or(|value| value.trim().is_empty())
    {
        return Err(invalid_authoring(
            "IfcAnnotation",
            "ObjectType",
            "USERDEFINED requires a non-empty ObjectType",
        ));
    }
    validate_optional_ref(tx, model, schema, draft.owner_history, "IfcOwnerHistory")?;
    validate_optional_ref(
        tx,
        model,
        schema,
        draft.object_placement,
        "IfcObjectPlacement",
    )?;
    validate_optional_ref(
        tx,
        model,
        schema,
        draft.representation,
        "IfcProductRepresentation",
    )?;

    let mut values = vec![("GlobalId", text(draft.global_id))];
    optional_reference(&mut values, "OwnerHistory", draft.owner_history);
    optional_text(&mut values, "Name", draft.name);
    optional_text(&mut values, "Description", draft.description);
    optional_text(&mut values, "ObjectType", draft.object_type);
    optional_reference(&mut values, "ObjectPlacement", draft.object_placement);
    optional_reference(&mut values, "Representation", draft.representation);
    if let Some(value) = draft.predefined_type {
        values.push(("PredefinedType", enumeration(value.as_ifc())));
    }
    Ok(tx.create(build_named(schema, "IfcAnnotation", values)?))
}

/// Stage a new `IfcTextLiteral` in `tx`. Fails if `literal` is empty or
/// `placement` does not resolve to an `IfcPlacement`.
pub fn create_text_literal(
    tx: &mut Transaction,
    model: &Model,
    schema: &Schema,
    draft: TextLiteralDraft<'_>,
) -> StyleResult<EntityId> {
    if draft.literal.is_empty() {
        return Err(invalid_authoring("IfcTextLiteral", "Literal", "empty"));
    }
    validate_ref(tx, model, schema, draft.placement, "IfcPlacement")?;
    Ok(tx.create(build_named(
        schema,
        "IfcTextLiteral",
        vec![
            ("Literal", text(draft.literal)),
            ("Placement", Value::Ref(draft.placement)),
            ("Path", enumeration(draft.path.as_ifc())),
        ],
    )?))
}

/// Stage a new `IfcTextLiteralWithExtent` in `tx`. Fails if `literal` is
/// empty, or if `placement`/`extent` do not resolve to their expected types.
pub fn create_text_literal_with_extent(
    tx: &mut Transaction,
    model: &Model,
    schema: &Schema,
    draft: TextLiteralWithExtentDraft<'_>,
) -> StyleResult<EntityId> {
    if draft.literal.is_empty() {
        return Err(invalid_authoring(
            "IfcTextLiteralWithExtent",
            "Literal",
            "empty",
        ));
    }

    validate_ref(tx, model, schema, draft.placement, "IfcPlacement")?;
    validate_ref(tx, model, schema, draft.extent, "IfcPlanarExtent")?;
    Ok(tx.create(build_named(
        schema,
        "IfcTextLiteralWithExtent",
        vec![
            ("Literal", text(draft.literal)),
            ("Placement", Value::Ref(draft.placement)),
            ("Path", enumeration(draft.path.as_ifc())),
            ("Extent", Value::Ref(draft.extent)),
            ("BoxAlignment", text(draft.box_alignment.as_ifc())),
        ],
    )?))
}

/// Stage a new `IfcAnnotationFillArea` in `tx`. Fails if `outer_boundary` or
/// any of `inner_boundaries` does not resolve to an `IfcCurve`.
pub fn create_annotation_fill_area(
    tx: &mut Transaction,
    model: &Model,
    schema: &Schema,
    draft: AnnotationFillAreaDraft,
) -> StyleResult<EntityId> {
    validate_ref(tx, model, schema, draft.outer_boundary, "IfcCurve")?;
    for inner in &draft.inner_boundaries {
        validate_ref(tx, model, schema, *inner, "IfcCurve")?;
    }
    let inner = if draft.inner_boundaries.is_empty() {
        Value::Null
    } else {
        Value::List(draft.inner_boundaries.into_iter().map(Value::Ref).collect())
    };
    Ok(tx.create(build_named(
        schema,
        "IfcAnnotationFillArea",
        vec![
            ("OuterBoundary", Value::Ref(draft.outer_boundary)),
            ("InnerBoundaries", inner),
        ],
    )?))
}

/// Pick whichever spelling of an attribute the target schema declares.
///
/// A handful of attributes were renamed between IFC4 and IFC4X3 without
/// moving slot or changing type: `IfcFillAreaStyle.ModelorDraughting`
/// became `ModelOrDraughting`, and
/// `IfcCurveStyleFontAndScaling.CurveFont` became `CurveStyleFont`.
/// Writing a hardcoded spelling makes the writer refuse its own output
/// under the other schema, so the candidates are resolved against the
/// schema in use. Falls back to the first candidate so an unknown entity
/// still produces the usual `build_named` error rather than a silent miss.
/// Stage an `IfcPlanarExtent`, or its `IfcPlanarBox` subtype.
///
/// The extent is a rectangular presentation area: two sizes and, for
/// the box form, the placement that positions it. Sizes are
/// `IfcLengthMeasure`, which admits negatives in the type system;
/// a negative or non-finite extent describes no area, so it is
/// refused here rather than written for a reader to puzzle over.
///
/// Passing a placement selects `IfcPlanarBox`; omitting it stages the
/// plain extent. The two share slots 0 and 1, so the subtype only ever
/// adds.
///
/// # Errors
///
/// Refuses a non-finite or non-positive `SizeInX`/`SizeInY`, and a
/// `placement` that is not an `IfcAxis2Placement2D`/`3D`.
pub fn create_planar_extent(
    tx: &mut Transaction,
    model: &Model,
    schema: &Schema,
    size_in_x: f64,
    size_in_y: f64,
    placement: Option<EntityId>,
) -> StyleResult<EntityId> {
    let entity = if placement.is_some() {
        "IfcPlanarBox"
    } else {
        "IfcPlanarExtent"
    };
    for (attribute, value) in [("SizeInX", size_in_x), ("SizeInY", size_in_y)] {
        if !value.is_finite() || value <= 0.0 {
            return Err(invalid_authoring(entity, attribute, format!("{value}")));
        }
    }
    let mut values = vec![
        ("SizeInX", Value::Real(size_in_x)),
        ("SizeInY", Value::Real(size_in_y)),
    ];
    if let Some(placement) = placement {
        // `IfcAxis2Placement` is a SELECT over the 2D and 3D forms, not
        // a supertype, so `validate_ref`'s `is_a` check rejects both
        // members. Each concrete form is checked instead.
        let placement_type = schema_placement_type(tx, model, placement)?;
        if !matches!(
            placement_type.as_str(),
            "IFCAXIS2PLACEMENT2D" | "IFCAXIS2PLACEMENT3D"
        ) {
            return Err(StyleError::ReferenceType {
                target: placement,
                expected: "IfcAxis2Placement",
                actual: placement_type,
            });
        }
        values.push(("Placement", Value::Ref(placement)));
    }
    Ok(tx.create(build_named(schema, entity, values)?))
}

/// Resolve the type name of a staged or committed entity.
///
/// A `Transaction` cannot be read back, so a reference to an
/// entity created earlier in the same transaction is only visible
/// in its edit list.
fn schema_placement_type(tx: &Transaction, model: &Model, target: EntityId) -> StyleResult<String> {
    tx.edits()
        .iter()
        .rev()
        .find_map(|edit| match edit {
            Edit::Create { id, entity } if *id == target => Some(entity.type_name.to_string()),
            _ => None,
        })
        .or_else(|| model.get(target).map(|e| e.type_name.to_string()))
        .ok_or(StyleError::DanglingReference {
            source_id: EntityId(0),
            target,
        })
}

pub(crate) fn schema_attribute(
    schema: &Schema,
    entity: &str,
    candidates: [&'static str; 2],
) -> &'static str {
    candidates
        .into_iter()
        .find(|candidate| {
            schema
                .attributes(entity)
                .iter()
                .any(|attribute| attribute.name.eq_ignore_ascii_case(candidate))
        })
        .unwrap_or(candidates[0])
}

pub(crate) fn build_named(
    schema: &Schema,
    entity: &'static str,
    values: Vec<(&'static str, Value)>,
) -> StyleResult<Entity> {
    let declared = schema.attributes(entity);
    if declared.is_empty() && schema.entity(entity).is_none() {
        return Err(StyleError::UnsupportedEntity {
            schema: schema.name().to_owned(),
            entity,
        });
    }
    let mut slots = vec![Value::Null; declared.len()];
    let mut filled = vec![false; declared.len()];
    for (name, value) in values {
        let Some(index) = declared
            .iter()
            .position(|attribute| attribute.name.eq_ignore_ascii_case(name))
        else {
            return Err(StyleError::UnsupportedAttribute {
                schema: schema.name().to_owned(),
                entity,
                attribute: name,
            });
        };
        slots[index] = value;
        filled[index] = true;
    }
    for (index, attribute) in declared.iter().enumerate() {
        if !filled[index] && !attribute.optional {
            return Err(StyleError::AuthoringInvalid {
                entity,
                attribute: "required attribute",
                value: attribute.name.clone(),
            });
        }
    }
    Ok(Entity::new(entity.to_ascii_uppercase(), slots))
}

pub(crate) fn validate_ref(
    tx: &Transaction,
    model: &Model,
    schema: &Schema,
    target: EntityId,
    expected: &'static str,
) -> StyleResult<()> {
    let type_name = tx
        .edits()
        .iter()
        .rev()
        .find_map(|edit| match edit {
            Edit::Create { id, entity } if *id == target => Some(entity.type_name.as_ref()),
            _ => None,
        })
        .or_else(|| model.get(target).map(|entity| entity.type_name.as_ref()))
        .ok_or(StyleError::DanglingReference {
            source_id: EntityId(0),
            target,
        })?;
    if !schema.is_a(type_name, expected) {
        return Err(StyleError::ReferenceType {
            target,
            expected,
            actual: type_name.to_owned(),
        });
    }
    Ok(())
}

pub(crate) fn validate_optional_ref(
    tx: &Transaction,
    model: &Model,
    schema: &Schema,
    target: Option<EntityId>,
    expected: &'static str,
) -> StyleResult<()> {
    if let Some(target) = target {
        validate_ref(tx, model, schema, target, expected)?;
    }
    Ok(())
}

fn text(value: &str) -> Value {
    Value::Text(Arc::from(value))
}

pub(crate) fn enumeration(value: &str) -> Value {
    Value::Enum(Arc::from(value.to_ascii_uppercase()))
}

fn optional_text(values: &mut Vec<(&'static str, Value)>, name: &'static str, value: Option<&str>) {
    if let Some(value) = value {
        values.push((name, text(value)));
    }
}

fn optional_reference(
    values: &mut Vec<(&'static str, Value)>,
    name: &'static str,
    value: Option<EntityId>,
) {
    if let Some(value) = value {
        values.push((name, Value::Ref(value)));
    }
}

pub(crate) fn invalid_authoring(
    entity: &'static str,
    attribute: &'static str,
    value: impl ToString,
) -> StyleError {
    StyleError::AuthoringInvalid {
        entity,
        attribute,
        value: value.to_string(),
    }
}

pub(crate) fn validate_ratio(
    entity: &'static str,
    attribute: &'static str,
    value: f64,
) -> StyleResult<()> {
    if value.is_finite() && (0.0..=1.0).contains(&value) {
        Ok(())
    } else {
        Err(invalid_authoring(entity, attribute, value))
    }
}
