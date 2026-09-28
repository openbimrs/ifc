//! Authoring for property templates and type-driven definition.
//!
//! Split from the parent module, which stages property instances.
//! These entities sit one level up: a template declares what a set
//! should contain before any instance exists, and
//! `IfcRelDefinesByType` carries a type's properties to its
//! occurrences -- the route `IfcRelDefinesByProperties` refuses.
//!
//! The template writers take no model and write `OwnerHistory` as `$`.
//! That is valid in IFC4 and IFC4X3, the only releases that declare
//! templates; IFC2X3 declares none of these entities. Their
//! `*_with_owner_history` variants take the model and refuse an IFC2X3
//! one with [`EntityNotInSchema`](crate::PropertyError::EntityNotInSchema)
//! (#202).

use ifc_model::{EntityId, Model, Transaction, Value};

use super::authoring::{optional_text, require_name};
use super::owned::{stage_ifc4, stage_owned, Rooted};
use super::predefined::{invalid, require_guid, token};
use crate::error::PropertyResult;

/// `IfcRelDefinesByType` slots.
pub mod defines_by_type_slot {
    /// `GlobalId`. Required.
    pub const GLOBAL_ID: usize = 0;
    /// `RelatedObjects`. Required.
    pub const RELATED_OBJECTS: usize = 4;
    /// `RelatingType`. Required.
    pub const RELATING_TYPE: usize = 5;
}

/// Attach a type object to its occurrences with `IfcRelDefinesByType`.
///
/// The counterpart of [`super::authoring::attach_property_set`]:
/// properties carried by a
/// type reach its occurrences through this relationship, which is why
/// `IfcRelDefinesByProperties` refuses a type object outright.
///
/// # Release
///
/// Bound to the model's declared release like
/// [`super::authoring::attach_property_set`]: laid out by attribute name,
/// type checks against that release's inheritance, and an IFC2X3 model
/// refused with [`AuthoringRequired`](crate::PropertyError::AuthoringRequired) because its
/// `OwnerHistory` would be `$`. Use
/// [`attach_type_with_owner_history`](crate::attach_type_with_owner_history)
/// there.
///
/// # Errors
///
/// Refuses a malformed GUID, an empty occurrence list (`SET [1:?]`),
/// a relating type that is not an `IfcTypeObject` subtype, and any
/// occurrence that is itself a type object; a model that binds no single
/// known release, and an IFC2X3 model. Nothing is staged on an error.
pub fn attach_type(
    tx: &mut Transaction,
    model: &Model,
    global_id: &str,
    objects: &[EntityId],
    relating_type: EntityId,
) -> PropertyResult<EntityId> {
    super::root_authoring::attach_type_record(tx, model, global_id, objects, relating_type, None)
}

/// `IfcPropertySetTemplate` slots.
pub mod pset_template_slot {
    /// `GlobalId`. Required.
    pub const GLOBAL_ID: usize = 0;
    /// `Name`.
    pub const NAME: usize = 2;
    /// `Description`.
    pub const DESCRIPTION: usize = 3;
    /// `TemplateType`.
    pub const TEMPLATE_TYPE: usize = 4;
    /// `ApplicableEntity`.
    pub const APPLICABLE_ENTITY: usize = 5;
    /// `HasPropertyTemplates`. Required.
    pub const HAS_PROPERTY_TEMPLATES: usize = 6;
}

/// `IfcRelDefinesByTemplate` slots.
pub mod defines_by_template_slot {
    /// `GlobalId`. Required.
    pub const GLOBAL_ID: usize = 0;
    /// `RelatedPropertySets`. Required.
    pub const RELATED_PROPERTY_SETS: usize = 4;
    /// `RelatingTemplate`. Required.
    pub const RELATING_TEMPLATE: usize = 5;
}

/// Stage an `IfcPropertySetTemplate`.
///
/// Declares what a property set should contain before any instance
/// exists: which properties, of what measure, on which entities.
///
/// Takes no model, so it cannot refuse an IFC2X3 model, which declares no
/// templates. Use [`add_property_set_template_with_owner_history`] to
/// write against the model's declared release.
///
/// # Errors
///
/// Refuses a malformed GUID and an empty template list, which the
/// schema types as `SET [1:?]`.
pub fn add_property_set_template(
    tx: &mut Transaction,
    global_id: &str,
    name: &str,
    applicable_entity: Option<&str>,
    templates: &[EntityId],
) -> PropertyResult<EntityId> {
    stage_ifc4(
        tx,
        set_template(global_id, name, applicable_entity, templates)?,
    )
}

/// [`add_property_set_template`] in the model's declared release, with a
/// caller-supplied `IfcOwnerHistory` (#202).
///
/// # Errors
///
/// Those of [`add_property_set_template`];
/// [`EntityNotInSchema`](crate::PropertyError::EntityNotInSchema) in an
/// IFC2X3 model, which declares no property templates; and the release and
/// owner-history refusals of
/// [`add_door_lining_properties_with_owner_history`](crate::add_door_lining_properties_with_owner_history).
/// Nothing is staged on an error.
pub fn add_property_set_template_with_owner_history(
    tx: &mut Transaction,
    model: &Model,
    global_id: &str,
    name: &str,
    applicable_entity: Option<&str>,
    templates: &[EntityId],
    owner_history: EntityId,
) -> PropertyResult<EntityId> {
    let rooted = set_template(global_id, name, applicable_entity, templates)?;
    stage_owned(tx, model, rooted, owner_history)
}

fn set_template<'a>(
    global_id: &'a str,
    name: &'a str,
    applicable_entity: Option<&str>,
    templates: &[EntityId],
) -> PropertyResult<Rooted<'a>> {
    const ENTITY: &str = "IFCPROPERTYSETTEMPLATE";
    require_guid(ENTITY, global_id)?;
    require_name(ENTITY, name)?;
    if templates.is_empty() {
        return Err(invalid(ENTITY, "HasPropertyTemplates", "empty"));
    }
    let values = vec![
        ("ApplicableEntity", optional_text(applicable_entity)),
        (
            "HasPropertyTemplates",
            Value::List(templates.iter().copied().map(Value::Ref).collect()),
        ),
    ];
    Ok(Rooted {
        entity: ENTITY,
        global_id,
        name: Some(name),
        description: None,
        values,
    })
}

/// Stage an `IfcRelDefinesByTemplate`.
///
/// Binds authored property sets to the template they follow, which is
/// how a checker knows an instance was meant to conform.
///
/// Takes no model, so it cannot refuse an IFC2X3 model, which declares no
/// templates. Use [`attach_template_with_owner_history`] to write against
/// the model's declared release.
///
/// # Errors
///
/// Refuses a malformed GUID and an empty property-set list
/// (`SET [1:?]`).
pub fn attach_template(
    tx: &mut Transaction,
    global_id: &str,
    property_sets: &[EntityId],
    template: EntityId,
) -> PropertyResult<EntityId> {
    stage_ifc4(tx, defines_by_template(global_id, property_sets, template)?)
}

/// [`attach_template`] in the model's declared release, with a
/// caller-supplied `IfcOwnerHistory` (#202).
///
/// # Errors
///
/// Those of [`attach_template`], and those of
/// [`add_property_set_template_with_owner_history`]. Nothing is staged on
/// an error.
pub fn attach_template_with_owner_history(
    tx: &mut Transaction,
    model: &Model,
    global_id: &str,
    property_sets: &[EntityId],
    template: EntityId,
    owner_history: EntityId,
) -> PropertyResult<EntityId> {
    let rooted = defines_by_template(global_id, property_sets, template)?;
    stage_owned(tx, model, rooted, owner_history)
}

fn defines_by_template<'a>(
    global_id: &'a str,
    property_sets: &[EntityId],
    template: EntityId,
) -> PropertyResult<Rooted<'a>> {
    const ENTITY: &str = "IFCRELDEFINESBYTEMPLATE";
    require_guid(ENTITY, global_id)?;
    if property_sets.is_empty() {
        return Err(invalid(ENTITY, "RelatedPropertySets", "empty"));
    }
    let values = vec![
        (
            "RelatedPropertySets",
            Value::List(property_sets.iter().copied().map(Value::Ref).collect()),
        ),
        ("RelatingTemplate", Value::Ref(template)),
    ];
    Ok(Rooted {
        entity: ENTITY,
        global_id,
        name: None,
        description: None,
        values,
    })
}

const COMPLEX_TEMPLATE_TYPE: &[&str] = &["P_COMPLEX", "Q_COMPLEX"];

/// Stage an `IfcComplexPropertyTemplate`.
///
/// Takes no model, so it cannot refuse an IFC2X3 model, which declares no
/// templates. Use [`add_complex_property_template_with_owner_history`] to
/// write against the model's declared release.
///
/// # Errors
///
/// Refuses a malformed GlobalId, an empty template set (the attribute
/// is `SET [1:?]` when present), a duplicate template reference, and a
/// `TemplateType` outside its enumeration.
///
/// `NoSelfReference` needs no check: [`Transaction::create`] allocates
/// the id as it stages the entity, so a caller cannot hold that id in
/// order to pass it as one of its own children.
///
/// `UniquePropertyNames` is stated over the templates' names, which a
/// staged entity cannot be read back to supply. Callers pass
/// `(name, id)` pairs, matching `add_property_set`.
pub fn add_complex_property_template(
    tx: &mut Transaction,
    global_id: &str,
    name: Option<&str>,
    usage: (Option<&str>, Option<&str>),
    templates: &[(&str, EntityId)],
) -> PropertyResult<EntityId> {
    stage_ifc4(tx, complex_template(global_id, name, usage, templates)?)
}

/// [`add_complex_property_template`] in the model's declared release,
/// with a caller-supplied `IfcOwnerHistory` (#202).
///
/// # Errors
///
/// Those of [`add_complex_property_template`], and those of
/// [`add_property_set_template_with_owner_history`]. Nothing is staged on
/// an error.
pub fn add_complex_property_template_with_owner_history(
    tx: &mut Transaction,
    model: &Model,
    global_id: &str,
    name: Option<&str>,
    usage: (Option<&str>, Option<&str>),
    templates: &[(&str, EntityId)],
    owner_history: EntityId,
) -> PropertyResult<EntityId> {
    let rooted = complex_template(global_id, name, usage, templates)?;
    stage_owned(tx, model, rooted, owner_history)
}

fn complex_template<'a>(
    global_id: &'a str,
    name: Option<&'a str>,
    (usage_name, template_type): (Option<&str>, Option<&str>),
    templates: &[(&str, EntityId)],
) -> PropertyResult<Rooted<'a>> {
    const ENTITY: &str = "IFCCOMPLEXPROPERTYTEMPLATE";
    require_guid(ENTITY, global_id)?;
    let mut values = vec![("UsageName", optional_text(usage_name))];
    if let Some(kind) = template_type {
        values.push((
            "TemplateType",
            token(ENTITY, "TemplateType", kind, COMPLEX_TEMPLATE_TYPE)?,
        ));
    }
    if !templates.is_empty() {
        for (index, (child, _)) in templates.iter().enumerate() {
            if templates[..index].iter().any(|(seen, _)| seen == child) {
                return Err(invalid(ENTITY, "HasPropertyTemplates", (*child).to_owned()));
            }
        }
        let children = templates.iter().map(|(_, id)| Value::Ref(*id));
        values.push(("HasPropertyTemplates", Value::List(children.collect())));
    }
    Ok(Rooted {
        entity: ENTITY,
        global_id,
        name,
        description: None,
        values,
    })
}
