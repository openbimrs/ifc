//! Authoring for property templates and type-driven definition.
//!
//! Split from the parent module, which stages property instances.
//! These entities sit one level up: a template declares what a set
//! should contain before any instance exists, and
//! `IfcRelDefinesByType` carries a type's properties to its
//! occurrences -- the route `IfcRelDefinesByProperties` refuses.

use ifc_model::guid::Guid;
use ifc_model::{Entity, EntityId, Model, Transaction, Value};
use ifc_schema::ifc4;

use super::authoring::{optional_text, require_name};
use crate::error::{PropertyError, PropertyResult};

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
/// # Errors
///
/// Refuses a malformed GUID, an empty occurrence list (`SET [1:?]`),
/// a relating type that is not an `IfcTypeObject` subtype, and any
/// occurrence that is itself a type object.
pub fn attach_type(
    tx: &mut Transaction,
    model: &Model,
    global_id: &str,
    objects: &[EntityId],
    relating_type: EntityId,
) -> PropertyResult<EntityId> {
    if Guid::parse(global_id).is_none() {
        return Err(PropertyError::AuthoringInvalid {
            entity: "IFCRELDEFINESBYTYPE",
            attribute: "GlobalId",
            value: global_id.to_owned(),
        });
    }
    if objects.is_empty() {
        return Err(PropertyError::AuthoringInvalid {
            entity: "IFCRELDEFINESBYTYPE",
            attribute: "RelatedObjects",
            value: "empty".to_owned(),
        });
    }
    let schema = ifc4();
    let is_type = |id: EntityId| {
        model
            .get(id)
            .is_some_and(|entity| schema.is_a(&entity.type_name, "IFCTYPEOBJECT"))
    };
    if !is_type(relating_type) {
        return Err(PropertyError::AuthoringInvalid {
            entity: "IFCRELDEFINESBYTYPE",
            attribute: "RelatingType",
            value: "not an IfcTypeObject".to_owned(),
        });
    }
    for object in objects {
        if is_type(*object) {
            return Err(PropertyError::AuthoringInvalid {
                entity: "IFCRELDEFINESBYTYPE",
                attribute: "RelatedObjects",
                value: "a type object cannot be an occurrence".to_owned(),
            });
        }
    }
    let mut attributes = vec![Value::Null; defines_by_type_slot::RELATING_TYPE + 1];
    attributes[defines_by_type_slot::GLOBAL_ID] = Value::Text(global_id.into());
    attributes[defines_by_type_slot::RELATED_OBJECTS] =
        Value::List(objects.iter().copied().map(Value::Ref).collect());
    attributes[defines_by_type_slot::RELATING_TYPE] = Value::Ref(relating_type);
    Ok(tx.create(Entity::new("IFCRELDEFINESBYTYPE", attributes)))
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
    if Guid::parse(global_id).is_none() {
        return Err(PropertyError::AuthoringInvalid {
            entity: "IFCPROPERTYSETTEMPLATE",
            attribute: "GlobalId",
            value: global_id.to_owned(),
        });
    }
    require_name("IFCPROPERTYSETTEMPLATE", name)?;
    if templates.is_empty() {
        return Err(PropertyError::AuthoringInvalid {
            entity: "IFCPROPERTYSETTEMPLATE",
            attribute: "HasPropertyTemplates",
            value: "empty".to_owned(),
        });
    }
    let mut attributes = vec![Value::Null; pset_template_slot::HAS_PROPERTY_TEMPLATES + 1];
    attributes[pset_template_slot::GLOBAL_ID] = Value::Text(global_id.into());
    attributes[pset_template_slot::NAME] = Value::Text(name.into());
    attributes[pset_template_slot::APPLICABLE_ENTITY] = optional_text(applicable_entity);
    attributes[pset_template_slot::HAS_PROPERTY_TEMPLATES] =
        Value::List(templates.iter().copied().map(Value::Ref).collect());
    Ok(tx.create(Entity::new("IFCPROPERTYSETTEMPLATE", attributes)))
}

/// Stage an `IfcRelDefinesByTemplate`.
///
/// Binds authored property sets to the template they follow, which is
/// how a checker knows an instance was meant to conform.
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
    if Guid::parse(global_id).is_none() {
        return Err(PropertyError::AuthoringInvalid {
            entity: "IFCRELDEFINESBYTEMPLATE",
            attribute: "GlobalId",
            value: global_id.to_owned(),
        });
    }
    if property_sets.is_empty() {
        return Err(PropertyError::AuthoringInvalid {
            entity: "IFCRELDEFINESBYTEMPLATE",
            attribute: "RelatedPropertySets",
            value: "empty".to_owned(),
        });
    }
    let mut attributes = vec![Value::Null; defines_by_template_slot::RELATING_TEMPLATE + 1];
    attributes[defines_by_template_slot::GLOBAL_ID] = Value::Text(global_id.into());
    attributes[defines_by_template_slot::RELATED_PROPERTY_SETS] =
        Value::List(property_sets.iter().copied().map(Value::Ref).collect());
    attributes[defines_by_template_slot::RELATING_TEMPLATE] = Value::Ref(template);
    Ok(tx.create(Entity::new("IFCRELDEFINESBYTEMPLATE", attributes)))
}
