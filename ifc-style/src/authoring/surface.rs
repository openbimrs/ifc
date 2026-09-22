//! Authoring colours, surface styles, and presentation layers.
//!
//! Split from the module root: these writers form one concern --
//! how a surface looks -- and the root was over the monolith limit.

use ifc_model::{Edit, EntityId, Model, Transaction, Value};
use ifc_schema::Schema;

use super::{
    build_named, enumeration, invalid_authoring, optional_reference, optional_text, text,
    validate_optional_ref, validate_ratio, validate_ref,
};
use crate::colour::ColourOrFactor;
use crate::error::{StyleError, StyleResult};
use crate::surface_style::{duplicate_surface_element_category, SURFACE_STYLE_ELEMENT_MEMBERS};

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

/// Authored fields for an `IfcSurfaceStyleRendering`.
///
/// The rendering form extends the shading form with the parameters a
/// renderer needs. Every colour slot is an `IfcColourOrFactor`, so a
/// caller states either an explicit colour or a factor of the surface
/// colour -- the two are not interchangeable and the schema keeps both.
#[derive(Debug, Clone, Copy)]
pub struct SurfaceStyleRenderingDraft {
    /// `SurfaceColour`, an `IfcColourRgb`.
    pub surface_colour: EntityId,
    /// `Transparency`, a normalised ratio.
    pub transparency: Option<f64>,
    /// `DiffuseColour`.
    pub diffuse: Option<ColourOrFactor>,
    /// `TransmissionColour`.
    pub transmission: Option<ColourOrFactor>,
    /// `DiffuseTransmissionColour`.
    pub diffuse_transmission: Option<ColourOrFactor>,
    /// `ReflectionColour`.
    pub reflection: Option<ColourOrFactor>,
    /// `SpecularColour`.
    pub specular: Option<ColourOrFactor>,
    /// `ReflectanceMethod`, an `IfcReflectanceMethodEnum` token.
    pub reflectance_method: &'static str,
}

/// Stage an `IfcSurfaceStyleRendering`.
///
/// # Errors
///
/// Refuses a `surface_colour` or colour member that is not an
/// `IfcColourRgb`, a factor or transparency outside `[0, 1]`, and a
/// `reflectance_method` the schema does not declare.
pub fn create_surface_style_rendering(
    tx: &mut Transaction,
    model: &Model,
    schema: &Schema,
    draft: SurfaceStyleRenderingDraft,
) -> StyleResult<EntityId> {
    const ENTITY: &str = "IfcSurfaceStyleRendering";
    validate_ref(tx, model, schema, draft.surface_colour, "IfcColourRgb")?;
    if let Some(value) = draft.transparency {
        validate_ratio(ENTITY, "Transparency", value)?;
    }
    let mut values = vec![("SurfaceColour", Value::Ref(draft.surface_colour))];
    if let Some(value) = draft.transparency {
        values.push(("Transparency", Value::Real(value)));
    }
    for (attribute, member) in [
        ("DiffuseColour", draft.diffuse),
        ("TransmissionColour", draft.transmission),
        ("DiffuseTransmissionColour", draft.diffuse_transmission),
        ("ReflectionColour", draft.reflection),
        ("SpecularColour", draft.specular),
    ] {
        let Some(member) = member else { continue };
        let value = match member {
            ColourOrFactor::Colour(id) => {
                validate_ref(tx, model, schema, id, "IfcColourRgb")?;
                Value::Ref(id)
            }
            ColourOrFactor::Factor(factor) => {
                validate_ratio(ENTITY, attribute, factor)?;
                Value::Real(factor)
            }
        };
        values.push((attribute, value));
    }
    // A reflectance method the schema does not declare names a
    // shading model the renderer cannot resolve.
    let declared = schema
        .attributes(ENTITY)
        .iter()
        .find(|a| a.name.eq_ignore_ascii_case("ReflectanceMethod"))
        .and_then(|a| schema.type_def(&a.type_name))
        .is_some_and(|def| match &def.kind {
            ifc_schema::TypeKind::Enumeration(values) => values
                .iter()
                .any(|v| v.eq_ignore_ascii_case(draft.reflectance_method)),
            _ => false,
        });
    if !declared {
        return Err(invalid_authoring(
            ENTITY,
            "ReflectanceMethod",
            draft.reflectance_method,
        ));
    }
    values.push((
        "ReflectanceMethod",
        Value::Enum(draft.reflectance_method.into()),
    ));
    Ok(tx.create(build_named(schema, ENTITY, values)?))
}
