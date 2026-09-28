//! `IfcRoot` authoring bound to the model's declared release (#191).
//!
//! `IfcRoot.OwnerHistory` is required in IFC2X3 TC1 and `OPTIONAL` from IFC4
//! on:
//!
//! ```text
//! IFC2X3_TC1   OwnerHistory : IfcOwnerHistory;
//! IFC4         OwnerHistory : OPTIONAL IfcOwnerHistory;
//! IFC4X3_ADD2  OwnerHistory : OPTIONAL IfcOwnerHistory;
//! ```
//!
//! The writers here bind the release through `quantity/release.rs` and lay
//! each record out by attribute name from its table. Those that leave
//! `OwnerHistory` unset refuse an IFC2X3 model with
//! [`PropertyError::AuthoringRequired`] instead of writing `$`. The
//! `*_with_owner_history` variants take a caller-supplied `IfcOwnerHistory`,
//! which must exist (in the model or staged on the transaction) and be an
//! `IfcOwnerHistory`; one is never invented here (build it with
//! `ifc-author`). IFC4 and IFC4X3 records are the same as before.
//!
//! [`add_property_set`](super::authoring::add_property_set) and
//! [`add_element_quantity`](super::authoring::add_element_quantity) take no
//! model and cannot refuse. So in a release that requires `OwnerHistory`,
//! [`attach_property_set_with_owner_history`] refuses to attach a
//! definition whose own `OwnerHistory` is unset.

use ifc_model::guid::Guid;
use ifc_model::{EntityId, Model, Transaction, Value};

use super::authoring::{check_element_quantity, check_property_set, optional_text};
use crate::quantity::release::{bind, projected, Layout};
use crate::{PropertyError, PropertyResult};

/// [`add_property_set`](crate::add_property_set) in the model's declared
/// release, with a caller-supplied `IfcOwnerHistory`, which IFC2X3 requires.
///
/// # Errors
///
/// Those of [`add_property_set`](crate::add_property_set), and:
/// [`PropertyError::MultipleSchemas`] or [`PropertyError::UnsupportedSchema`]
/// if the model binds no single known release;
/// [`PropertyError::MissingEntity`] if `owner_history` is neither in the
/// model nor staged; [`PropertyError::AuthoringInvalid`] if it is not an
/// `IfcOwnerHistory`. Nothing is staged on an error.
pub fn add_property_set_with_owner_history(
    tx: &mut Transaction,
    model: &Model,
    global_id: &str,
    name: &str,
    description: Option<&str>,
    properties: &[(&str, EntityId)],
    owner_history: EntityId,
) -> PropertyResult<EntityId> {
    const ENTITY: &str = "IFCPROPERTYSET";
    let layout = bind(model)?;
    check_property_set(global_id, name, properties)?;
    require_owner_history(tx, model, layout, ENTITY, owner_history)?;
    let record = layout.named_record(
        ENTITY,
        vec![
            ("GlobalId", Value::Text(global_id.into())),
            ("OwnerHistory", Value::Ref(owner_history)),
            ("Name", Value::Text(name.into())),
            ("Description", optional_text(description)),
            (
                "HasProperties",
                Value::List(properties.iter().map(|(_, id)| Value::Ref(*id)).collect()),
            ),
        ],
    )?;
    Ok(tx.create(record))
}

/// [`add_element_quantity`](crate::add_element_quantity) in the model's
/// declared release, with a caller-supplied `IfcOwnerHistory`, which IFC2X3
/// requires.
///
/// # Errors
///
/// Those of [`add_element_quantity`](crate::add_element_quantity), and the
/// release and owner-history refusals of
/// [`add_property_set_with_owner_history`]. Nothing is staged on an error.
pub fn add_element_quantity_with_owner_history(
    tx: &mut Transaction,
    model: &Model,
    global_id: &str,
    name: &str,
    method_of_measurement: Option<&str>,
    quantities: &[EntityId],
    owner_history: EntityId,
) -> PropertyResult<EntityId> {
    const ENTITY: &str = "IFCELEMENTQUANTITY";
    let layout = bind(model)?;
    check_element_quantity(global_id, name, quantities)?;
    require_owner_history(tx, model, layout, ENTITY, owner_history)?;
    let record = layout.named_record(
        ENTITY,
        vec![
            ("GlobalId", Value::Text(global_id.into())),
            ("OwnerHistory", Value::Ref(owner_history)),
            ("Name", Value::Text(name.into())),
            ("MethodOfMeasurement", optional_text(method_of_measurement)),
            ("Quantities", refs(quantities)),
        ],
    )?;
    Ok(tx.create(record))
}

/// [`attach_property_set`](crate::attach_property_set) with a
/// caller-supplied `IfcOwnerHistory`, which IFC2X3 requires.
///
/// # Errors
///
/// Those of [`attach_property_set`](crate::attach_property_set) except the
/// IFC2X3 refusal, the owner-history refusals of
/// [`add_property_set_with_owner_history`], and
/// [`PropertyError::AuthoringInvalid`] on `RelatingPropertyDefinition` when
/// the release requires `OwnerHistory` and the definition leaves it unset,
/// as [`add_property_set`](crate::add_property_set) does. Nothing is staged
/// on an error.
pub fn attach_property_set_with_owner_history(
    tx: &mut Transaction,
    model: &Model,
    global_id: &str,
    objects: &[EntityId],
    property_set: EntityId,
    owner_history: EntityId,
) -> PropertyResult<EntityId> {
    attach_definition(
        tx,
        model,
        global_id,
        objects,
        property_set,
        Some(owner_history),
    )
}

/// [`attach_type`](crate::attach_type) with a caller-supplied
/// `IfcOwnerHistory`, which IFC2X3 requires.
///
/// # Errors
///
/// Those of [`attach_type`](crate::attach_type) except the IFC2X3 refusal,
/// and the owner-history refusals of
/// [`add_property_set_with_owner_history`]. Nothing is staged on an error.
pub fn attach_type_with_owner_history(
    tx: &mut Transaction,
    model: &Model,
    global_id: &str,
    objects: &[EntityId],
    relating_type: EntityId,
    owner_history: EntityId,
) -> PropertyResult<EntityId> {
    attach_type_record(
        tx,
        model,
        global_id,
        objects,
        relating_type,
        Some(owner_history),
    )
}

/// Stage an `IfcRelDefinesByProperties`; `None` leaves `OwnerHistory` `$`.
pub(super) fn attach_definition(
    tx: &mut Transaction,
    model: &Model,
    global_id: &str,
    objects: &[EntityId],
    property_set: EntityId,
    owner_history: Option<EntityId>,
) -> PropertyResult<EntityId> {
    const ENTITY: &str = "IFCRELDEFINESBYPROPERTIES";
    let layout = bind(model)?;
    require_guid(ENTITY, global_id)?;
    if objects.is_empty() {
        return Err(invalid(ENTITY, "RelatedObjects", "an empty set"));
    }
    for object in objects {
        let Some(entity) = model.get(*object) else {
            return Err(PropertyError::MissingEntity { id: *object });
        };
        // NoRelatedTypeObject: a type carries its sets in HasPropertySets.
        if layout.schema().is_a(&entity.type_name, "IFCTYPEOBJECT") {
            return Err(invalid(
                ENTITY,
                "RelatedObjects",
                entity.type_name.to_string(),
            ));
        }
    }
    if let Some(owner_history) = owner_history {
        require_owner_history(tx, model, layout, ENTITY, owner_history)?;
    }
    let record = layout.named_record(
        ENTITY,
        vec![
            ("GlobalId", Value::Text(global_id.into())),
            (
                "OwnerHistory",
                owner_history.map_or(Value::Null, Value::Ref),
            ),
            ("RelatedObjects", refs(objects)),
            ("RelatingPropertyDefinition", Value::Ref(property_set)),
        ],
    )?;
    require_owned_definition(tx, model, layout, property_set)?;
    Ok(tx.create(record))
}

/// Stage an `IfcRelDefinesByType`; `None` leaves `OwnerHistory` `$`.
pub(super) fn attach_type_record(
    tx: &mut Transaction,
    model: &Model,
    global_id: &str,
    objects: &[EntityId],
    relating_type: EntityId,
    owner_history: Option<EntityId>,
) -> PropertyResult<EntityId> {
    const ENTITY: &str = "IFCRELDEFINESBYTYPE";
    let layout = bind(model)?;
    require_guid(ENTITY, global_id)?;
    if objects.is_empty() {
        return Err(invalid(ENTITY, "RelatedObjects", "empty"));
    }
    let is_type = |id: EntityId| {
        model
            .get(id)
            .is_some_and(|entity| layout.schema().is_a(&entity.type_name, "IFCTYPEOBJECT"))
    };
    if !is_type(relating_type) {
        return Err(invalid(ENTITY, "RelatingType", "not an IfcTypeObject"));
    }
    if objects.iter().any(|object| is_type(*object)) {
        return Err(invalid(
            ENTITY,
            "RelatedObjects",
            "a type object cannot be an occurrence",
        ));
    }
    if let Some(owner_history) = owner_history {
        require_owner_history(tx, model, layout, ENTITY, owner_history)?;
    }
    let record = layout.named_record(
        ENTITY,
        vec![
            ("GlobalId", Value::Text(global_id.into())),
            (
                "OwnerHistory",
                owner_history.map_or(Value::Null, Value::Ref),
            ),
            ("RelatedObjects", refs(objects)),
            ("RelatingType", Value::Ref(relating_type)),
        ],
    )?;
    Ok(tx.create(record))
}

/// Fail unless `id` is an `IfcOwnerHistory` in the model or staged on `tx`.
fn require_owner_history(
    tx: &Transaction,
    model: &Model,
    layout: Layout,
    entity: &'static str,
    id: EntityId,
) -> PropertyResult<()> {
    let found = projected(tx, model, id).ok_or(PropertyError::MissingEntity { id })?;
    if found.type_name.eq_ignore_ascii_case("IFCOWNERHISTORY")
        && layout.schema().entity("IFCOWNERHISTORY").is_some()
    {
        return Ok(());
    }
    Err(invalid(
        entity,
        "OwnerHistory",
        format!("#{} is a {}, not an IfcOwnerHistory", id.0, found.type_name),
    ))
}

/// In a release that requires `IfcRoot.OwnerHistory`, refuse to attach a
/// definition that leaves it unset: the relationship would be valid and
/// the file it lands in would not.
///
/// A definition that cannot be found is left to the caller, as before.
fn require_owned_definition(
    tx: &Transaction,
    model: &Model,
    layout: Layout,
    definition: EntityId,
) -> PropertyResult<()> {
    let Some(found) = projected(tx, model, definition) else {
        return Ok(());
    };
    let declared = layout.schema().attributes(&found.type_name);
    let unset = declared
        .iter()
        .position(|attribute| attribute.name.eq_ignore_ascii_case("OwnerHistory"))
        .filter(|slot| !declared[*slot].optional)
        .is_some_and(|slot| matches!(found.attributes.get(slot), None | Some(Value::Null)));
    if !unset {
        return Ok(());
    }
    Err(invalid(
        "IFCRELDEFINESBYPROPERTIES",
        "RelatingPropertyDefinition",
        format!(
            "#{} {} has no OwnerHistory, which {:?} requires",
            definition.0,
            found.type_name,
            layout.version()
        ),
    ))
}

fn require_guid(entity: &'static str, global_id: &str) -> PropertyResult<()> {
    if Guid::parse(global_id).is_none() {
        return Err(invalid(entity, "GlobalId", global_id));
    }
    Ok(())
}

fn invalid(
    entity: &'static str,
    attribute: &'static str,
    value: impl Into<String>,
) -> PropertyError {
    PropertyError::AuthoringInvalid {
        entity,
        attribute,
        value: value.into(),
    }
}

fn refs(ids: &[EntityId]) -> Value {
    Value::List(ids.iter().copied().map(Value::Ref).collect())
}
