//! Transaction-staged authoring for presentation and annotation entities.

use std::sync::Arc;

use ifc_model::{Edit, Entity, EntityId, Model, Transaction, Value};
use ifc_schema::Schema;

use crate::annotation::{AnnotationType, BoxAlignment, TextPath};
use crate::error::{StyleError, StyleResult};
use crate::surface_style::{duplicate_surface_element_category, SURFACE_STYLE_ELEMENT_MEMBERS};

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

fn validate_optional_ref(
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

fn enumeration(value: &str) -> Value {
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

fn invalid_authoring(
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

/// Draft input for [`create_colour_rgb`]: the writable attributes of a new
/// `IfcColourRgb`.
#[derive(Debug, Clone, Copy)]
pub struct ColourRgbDraft<'a> {
    /// The `Name` attribute, when supplied.
    pub name: Option<&'a str>,
    /// The `Red` channel; must be a finite value in `[0, 1]`.
    pub red: f64,
    /// The `Green` channel; must be a finite value in `[0, 1]`.
    pub green: f64,
    /// The `Blue` channel; must be a finite value in `[0, 1]`.
    pub blue: f64,
}

/// Draft input for [`create_surface_style_shading`]: the writable attributes
/// of a new `IfcSurfaceStyleShading`.
#[derive(Debug, Clone, Copy)]
pub struct SurfaceStyleShadingDraft {
    /// The `SurfaceColour` reference to an `IfcColourRgb`.
    pub surface_colour: EntityId,
    /// The `Transparency` factor, when supplied; must be a finite value in
    /// `[0, 1]`.
    pub transparency: Option<f64>,
}

/// Draft input for [`create_surface_style`]: the writable attributes of a
/// new `IfcSurfaceStyle`.
#[derive(Debug, Clone)]
pub struct SurfaceStyleDraft<'a> {
    /// The `Name` attribute, when supplied.
    pub name: Option<&'a str>,
    /// The `Side` attribute.
    pub side: crate::SurfaceSide,
    /// The `Styles` elements: one to five references, no two from the same
    /// surface-style element category (shading, lighting, refraction,
    /// textures, externally defined).
    pub elements: Vec<EntityId>,
}

/// Draft input for [`create_styled_item`]: the writable attributes of a new
/// `IfcStyledItem`.
#[derive(Debug, Clone)]
pub struct StyledItemDraft<'a> {
    /// The `Item` reference to an `IfcRepresentationItem`, when supplied.
    pub item: Option<EntityId>,
    /// The `Styles` references; at least one is required. On IFC2x3 these
    /// are wrapped in a staged `IfcPresentationStyleAssignment`.
    pub styles: Vec<EntityId>,
    /// The `Name` attribute, when supplied.
    pub name: Option<&'a str>,
}

/// Draft input for [`create_presentation_layer_with_style`]: the writable
/// attributes of a new `IfcPresentationLayerWithStyle`.
#[derive(Debug, Clone)]
pub struct PresentationLayerDraft<'a> {
    /// The `Name` attribute; must be non-empty.
    pub name: &'a str,
    /// The `Description` attribute, when supplied.
    pub description: Option<&'a str>,
    /// The `AssignedItems` references; at least one is required, and each
    /// must resolve to an `IfcRepresentation` or `IfcRepresentationItem`.
    pub assigned_items: Vec<EntityId>,
    /// The `Identifier` attribute, when supplied.
    pub identifier: Option<&'a str>,
    /// The `LayerOn` attribute, when supplied.
    pub layer_on: Option<bool>,
    /// The `LayerFrozen` attribute, when supplied.
    pub layer_frozen: Option<bool>,
    /// The `LayerBlocked` attribute, when supplied.
    pub layer_blocked: Option<bool>,
    /// The `LayerStyles` references.
    pub layer_styles: Vec<EntityId>,
}

/// Stage a new `IfcColourRgb` in `tx`. Fails if any channel is not a finite
/// value in `[0, 1]`.
pub fn create_colour_rgb(
    tx: &mut Transaction,
    schema: &Schema,
    draft: ColourRgbDraft<'_>,
) -> StyleResult<EntityId> {
    validate_ratio("IfcColourRgb", "Red", draft.red)?;
    validate_ratio("IfcColourRgb", "Green", draft.green)?;
    validate_ratio("IfcColourRgb", "Blue", draft.blue)?;
    let mut values = Vec::new();
    optional_text(&mut values, "Name", draft.name);
    values.extend([
        ("Red", Value::Real(draft.red)),
        ("Green", Value::Real(draft.green)),
        ("Blue", Value::Real(draft.blue)),
    ]);
    Ok(tx.create(build_named(schema, "IfcColourRgb", values)?))
}

/// Stage a new `IfcSurfaceStyleShading` in `tx`. Fails if `surface_colour`
/// does not resolve to an `IfcColourRgb`, or `transparency` is not a finite
/// value in `[0, 1]`.
pub fn create_surface_style_shading(
    tx: &mut Transaction,
    model: &Model,
    schema: &Schema,
    draft: SurfaceStyleShadingDraft,
) -> StyleResult<EntityId> {
    validate_ref(tx, model, schema, draft.surface_colour, "IfcColourRgb")?;
    if let Some(value) = draft.transparency {
        validate_ratio("IfcSurfaceStyleShading", "Transparency", value)?;
    }
    let mut values = vec![("SurfaceColour", Value::Ref(draft.surface_colour))];
    if let Some(value) = draft.transparency {
        values.push(("Transparency", Value::Real(value)));
    }
    Ok(tx.create(build_named(schema, "IfcSurfaceStyleShading", values)?))
}

/// Stage a new `IfcSurfaceStyle` in `tx`. Fails if `elements` is empty,
/// exceeds five members, contains two members from the same surface-style
/// element category, or any member does not resolve to its expected type.
pub fn create_surface_style(
    tx: &mut Transaction,
    model: &Model,
    schema: &Schema,
    draft: SurfaceStyleDraft<'_>,
) -> StyleResult<EntityId> {
    if draft.elements.is_empty() || draft.elements.len() > 5 {
        return Err(invalid_authoring(
            "IfcSurfaceStyle",
            "Styles",
            format!("expected 1..=5 elements, found {}", draft.elements.len()),
        ));
    }
    let mut element_types = Vec::with_capacity(draft.elements.len());
    for element in &draft.elements {
        element_types.push(validate_surface_element(tx, model, schema, *element)?);
    }
    if let Some(category) =
        duplicate_surface_element_category(schema, element_types.iter().map(String::as_str))
    {
        return Err(invalid_authoring(
            "IfcSurfaceStyle",
            "Styles",
            format!("duplicate {category} category"),
        ));
    }
    let mut values = Vec::new();
    optional_text(&mut values, "Name", draft.name);
    values.push(("Side", enumeration(draft.side.as_ifc())));
    values.push((
        "Styles",
        Value::List(draft.elements.into_iter().map(Value::Ref).collect()),
    ));
    Ok(tx.create(build_named(schema, "IfcSurfaceStyle", values)?))
}

/// Stage a new `IfcStyledItem` in `tx`. Fails if `styles` is empty, `item`
/// does not resolve to an `IfcRepresentationItem`, or a style does not
/// resolve to its expected type for the target schema version.
pub fn create_styled_item(
    tx: &mut Transaction,
    model: &Model,
    schema: &Schema,
    draft: StyledItemDraft<'_>,
) -> StyleResult<EntityId> {
    validate_optional_ref(tx, model, schema, draft.item, "IfcRepresentationItem")?;
    if draft.styles.is_empty() {
        return Err(invalid_authoring(
            "IfcStyledItem",
            "Styles",
            "at least one style is required",
        ));
    }
    for style in &draft.styles {
        validate_ref(tx, model, schema, *style, "IfcPresentationStyle")?;
    }

    let mut staged = tx.clone();
    let style_values = if schema.version() == Some(ifc_schema::SchemaVersion::Ifc2x3) {
        let wrapper = build_named(
            schema,
            "IfcPresentationStyleAssignment",
            vec![(
                "Styles",
                Value::List(draft.styles.into_iter().map(Value::Ref).collect()),
            )],
        )?;
        vec![Value::Ref(staged.create(wrapper))]
    } else {
        draft.styles.into_iter().map(Value::Ref).collect()
    };
    let mut values = vec![("Styles", Value::List(style_values))];
    optional_reference(&mut values, "Item", draft.item);
    optional_text(&mut values, "Name", draft.name);
    let id = staged.create(build_named(schema, "IfcStyledItem", values)?);
    *tx = staged;
    Ok(id)
}

/// Stage a new `IfcPresentationLayerWithStyle` in `tx`. Fails if `name` is
/// empty, `assigned_items` is empty, or any assigned item or layer style
/// does not resolve to its expected type.
pub fn create_presentation_layer_with_style(
    tx: &mut Transaction,
    model: &Model,
    schema: &Schema,
    draft: PresentationLayerDraft<'_>,
) -> StyleResult<EntityId> {
    if draft.name.is_empty() {
        return Err(invalid_authoring(
            "IfcPresentationLayerWithStyle",
            "Name",
            "must not be empty",
        ));
    }
    if draft.assigned_items.is_empty() {
        return Err(invalid_authoring(
            "IfcPresentationLayerWithStyle",
            "AssignedItems",
            "at least one item is required",
        ));
    }
    for item in &draft.assigned_items {
        validate_layered_item(tx, model, schema, *item)?;
    }
    for style in &draft.layer_styles {
        validate_ref(tx, model, schema, *style, "IfcPresentationStyle")?;
    }
    let mut values = vec![
        ("Name", text(draft.name)),
        (
            "AssignedItems",
            Value::List(draft.assigned_items.into_iter().map(Value::Ref).collect()),
        ),
        (
            "LayerStyles",
            Value::List(draft.layer_styles.into_iter().map(Value::Ref).collect()),
        ),
    ];
    optional_text(&mut values, "Description", draft.description);
    optional_text(&mut values, "Identifier", draft.identifier);
    if let Some(value) = draft.layer_on {
        values.push(("LayerOn", Value::Bool(value)));
    }
    if let Some(value) = draft.layer_frozen {
        values.push(("LayerFrozen", Value::Bool(value)));
    }
    if let Some(value) = draft.layer_blocked {
        values.push(("LayerBlocked", Value::Bool(value)));
    }
    Ok(tx.create(build_named(
        schema,
        "IfcPresentationLayerWithStyle",
        values,
    )?))
}

fn validate_ratio(entity: &'static str, attribute: &'static str, value: f64) -> StyleResult<()> {
    if value.is_finite() && (0.0..=1.0).contains(&value) {
        Ok(())
    } else {
        Err(invalid_authoring(entity, attribute, value))
    }
}

fn staged_type(tx: &Transaction, model: &Model, target: EntityId) -> Option<String> {
    tx.edits()
        .iter()
        .rev()
        .find_map(|edit| match edit {
            Edit::Create { id, entity } if *id == target => Some(entity.type_name.to_string()),
            _ => None,
        })
        .or_else(|| model.get(target).map(|entity| entity.type_name.to_string()))
}

fn validate_surface_element(
    tx: &Transaction,
    model: &Model,
    schema: &Schema,
    target: EntityId,
) -> StyleResult<String> {
    let actual = staged_type(tx, model, target).ok_or(StyleError::DanglingReference {
        source_id: EntityId(0),
        target,
    })?;
    if SURFACE_STYLE_ELEMENT_MEMBERS
        .iter()
        .any(|member| schema.is_a(&actual, member))
    {
        Ok(actual)
    } else {
        Err(StyleError::ReferenceType {
            target,
            expected: "IfcSurfaceStyleElementSelect",
            actual,
        })
    }
}

fn validate_layered_item(
    tx: &Transaction,
    model: &Model,
    schema: &Schema,
    target: EntityId,
) -> StyleResult<()> {
    let actual = staged_type(tx, model, target).ok_or(StyleError::DanglingReference {
        source_id: EntityId(0),
        target,
    })?;
    if schema.is_a(&actual, "IfcRepresentationItem") || schema.is_a(&actual, "IfcRepresentation") {
        Ok(())
    } else {
        Err(StyleError::ReferenceType {
            target,
            expected: "IfcLayeredItem",
            actual,
        })
    }
}
