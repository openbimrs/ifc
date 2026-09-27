//! Reading a door's operation inputs exactly from the declared release.
//!
//! Every attribute is found by name in the bound release's table, never by a
//! restated slot: `IfcDoor.OperationType` exists only from IFC4 on, and
//! `IfcDoorStyle.OperationType` (IFC2X3) sits at a different position from
//! `IfcDoorType.OperationType`. Relationship structure is validated first by
//! `exact_predefined_sets`, which refuses malformed assignments, several
//! type objects, and relationship subtypes relating the door.

use std::sync::Arc;

use ifc_geometry::{product_world_transform, units, Transform};
use ifc_model::{Entity, EntityId, Model, Value};
use ifc_properties::{
    exact_predefined_sets, exact_schema, exact_unit, ExactPredefinedSet, ExactPropertyError,
    ExactSource, ExactValue, SchemaVersion,
};
use ifc_schema::{Schema, TypeKind};

use super::{layout, DoorOperationError, DoorOperationType, PanelPosition};

/// Everything the derivation needs, read and validated.
pub(super) struct DoorInputs {
    pub(super) operation: DoorOperationType,
    pub(super) operation_source: ExactSource,
    pub(super) panel_source: ExactSource,
    /// Metres.
    pub(super) overall_width: f64,
    /// World transform in metres.
    pub(super) world: Transform,
    pub(super) panels: Vec<Panel>,
}

/// One `IfcDoorPanelProperties`, as the layout needs it.
#[derive(Debug, Clone)]
pub(super) struct Panel {
    pub(super) set: EntityId,
    pub(super) position: PanelPosition,
    /// `PanelOperation` as written, e.g. `SWINGING`.
    pub(super) operation: Arc<str>,
    /// `PanelWidth`, or `None` for `$`.
    pub(super) width: Option<f64>,
}

pub(super) fn read_door(model: &Model, door: EntityId) -> Result<DoorInputs, DoorOperationError> {
    let version = exact_schema(model)?;
    let schema = ifc_schema::for_version(version).expect("exact releases are bundled");
    let entity = model
        .get(door)
        .ok_or(ExactPropertyError::MissingReference {
            from: door,
            to: door,
        })?;
    // Validates the door, every property and type relationship in the file,
    // and the type object, before anything below trusts their structure.
    let sets = exact_predefined_sets(model, door, "IfcDoorPanelProperties")?;
    if !schema.is_a(&entity.type_name, "IFCDOOR") {
        return Err(DoorOperationError::NotADoor {
            entity: door,
            type_name: entity.type_name.clone(),
        });
    }
    let (written, operation_source) = operation_type(model, schema, version, door, entity)?;
    let operation = layout::classify(&written)?;
    let overall_width = overall_width(model, schema, door, entity)?;
    let (panel_source, panels) = governing_panels(door, &sets)?;
    let panels = panels.iter().map(panel).collect::<Result<_, _>>()?;
    let mut scale = units::resolve(model);
    scale.length_to_metres = length_scale(model)?;
    let world =
        product_world_transform(model, &scale, door).map_err(DoorOperationError::Placement)?;
    Ok(DoorInputs {
        operation,
        operation_source,
        panel_source,
        overall_width,
        world,
        panels,
    })
}

/// The value of `entity`'s attribute `name` in the release, with its
/// declared type, or `None` when the release does not declare it.
fn attribute<'m>(
    schema: &'static Schema,
    entity: &'m Entity,
    name: &str,
) -> Option<(&'m Value, &'static str)> {
    let (slot, declaration) = schema
        .attributes(&entity.type_name)
        .into_iter()
        .enumerate()
        .find(|(_, attribute)| attribute.name.eq_ignore_ascii_case(name))?;
    // Arity was validated against the same table by the exact resolver.
    Some((&entity.attributes[slot], declaration.type_name.as_str()))
}

/// An enumeration value checked against its declared enumeration, or `None`
/// for `$`.
fn enumeration(
    schema: &'static Schema,
    id: EntityId,
    entity: &Entity,
    name: &'static str,
) -> Result<Option<Arc<str>>, DoorOperationError> {
    let malformed = DoorOperationError::MalformedAttribute {
        entity: id,
        attribute: name,
    };
    let Some((value, declared)) = attribute(schema, entity, name) else {
        return Ok(None);
    };
    let members = match schema.type_def(declared).map(|definition| &definition.kind) {
        Some(TypeKind::Enumeration(members)) => members,
        _ => return Err(malformed),
    };
    match value {
        Value::Null => Ok(None),
        Value::Enum(member) if members.iter().any(|m| m.eq_ignore_ascii_case(member)) => {
            Ok(Some(member.to_ascii_uppercase().into()))
        }
        _ => Err(malformed),
    }
}

/// The operation type and where it was stated.
///
/// IFC4 `IfcDoor.OperationType`: "shall only be used, if no type object
/// IfcDoorType is assigned, providing its own IfcDoorType.OperationType".
/// A value on only one of them governs; two values must agree, since the
/// specification leaves no rule to prefer either.
fn operation_type(
    model: &Model,
    schema: &'static Schema,
    version: SchemaVersion,
    door: EntityId,
    entity: &Entity,
) -> Result<(Arc<str>, ExactSource), DoorOperationError> {
    let occurrence = enumeration(schema, door, entity, "OperationType")?;
    let typed = match type_object(model, schema, door)? {
        Some(type_id) => {
            let type_entity = model.get(type_id).expect("validated reference");
            // IFC4 `CorrectStyleAssigned` and IFC4X3 `CorrectTypeAssigned`
            // require an `IfcDoorType`; IFC2X3 has only `IfcDoorStyle`.
            let expected = match version {
                SchemaVersion::Ifc2x3 => "IFCDOORSTYLE",
                _ => "IFCDOORTYPE",
            };
            if !schema.is_a(&type_entity.type_name, expected) {
                return Err(DoorOperationError::UnsupportedTypeObject {
                    type_object: type_id,
                    type_name: type_entity.type_name.clone(),
                });
            }
            let value = enumeration(schema, type_id, type_entity, "OperationType")?.ok_or(
                DoorOperationError::MalformedAttribute {
                    entity: type_id,
                    attribute: "OperationType",
                },
            )?;
            Some((type_id, value))
        }
        None => None,
    };
    match (occurrence, typed) {
        (Some(occurrence), Some((type_object, type_value))) if occurrence != type_value => {
            Err(DoorOperationError::ConflictingOperationType {
                occurrence,
                type_object,
                type_value,
            })
        }
        (Some(occurrence), _) => Ok((occurrence, ExactSource::Occurrence)),
        (None, Some((type_object, value))) => Ok((value, ExactSource::Type(type_object))),
        (None, None) => Err(DoorOperationError::MissingOperationType { door }),
    }
}

/// The door's type object, through `IfcRelDefinesByType`.
///
/// Uniqueness, targets and relationship subtypes were validated by the
/// exact resolver, so the first match is the only one.
fn type_object(
    model: &Model,
    schema: &'static Schema,
    door: EntityId,
) -> Result<Option<EntityId>, DoorOperationError> {
    for id in model.ids_of_type("IFCRELDEFINESBYTYPE") {
        let relation = model.get(*id).expect("type index is current");
        let related = attribute(schema, relation, "RelatedObjects");
        let relating = attribute(schema, relation, "RelatingType");
        let (Some((Value::List(related), _)), Some((Value::Ref(type_id), _))) = (related, relating)
        else {
            return Err(DoorOperationError::MalformedAttribute {
                entity: *id,
                attribute: "RelatingType",
            });
        };
        if related.contains(&Value::Ref(door)) {
            return Ok(Some(*type_id));
        }
    }
    Ok(None)
}

/// `IfcDoor.OverallWidth` in metres.
fn overall_width(
    model: &Model,
    schema: &'static Schema,
    door: EntityId,
    entity: &Entity,
) -> Result<f64, DoorOperationError> {
    match attribute(schema, entity, "OverallWidth") {
        Some((Value::Null, _)) => Err(DoorOperationError::MissingOverallWidth { door }),
        // `IfcPositiveLengthMeasure`: a finite length greater than zero.
        Some((Value::Real(width), _)) if width.is_finite() && *width > 0.0 => {
            Ok(width * length_scale(model)?)
        }
        _ => Err(DoorOperationError::MalformedAttribute {
            entity: door,
            attribute: "OverallWidth",
        }),
    }
}

/// Metres per project length unit, resolved exactly.
fn length_scale(model: &Model) -> Result<f64, DoorOperationError> {
    Ok(exact_unit(model, "IFCPOSITIVELENGTHMEASURE", None)?.scale)
}

/// The occurrence's panel sets if it has any, else its type's.
fn governing_panels(
    door: EntityId,
    sets: &[ExactPredefinedSet],
) -> Result<(ExactSource, Vec<&ExactPredefinedSet>), DoorOperationError> {
    let source = sets
        .iter()
        .find(|set| set.source == ExactSource::Occurrence)
        .or_else(|| sets.first())
        .map(|set| set.source)
        .ok_or(DoorOperationError::NoPanelProperties { door })?;
    let governing = sets.iter().filter(|set| set.source == source).collect();
    Ok((source, governing))
}

/// One panel set's operation, position and width.
fn panel(set: &&ExactPredefinedSet) -> Result<Panel, DoorOperationError> {
    let malformed = |attribute| DoorOperationError::MalformedAttribute {
        entity: set.set_id,
        attribute,
    };
    let text = |name: &'static str| match set.attribute(name).map(|a| &a.value) {
        Some(ExactValue::Enum(value)) => Ok(value.clone()),
        _ => Err(malformed(name)),
    };
    let position = match &*text("PanelPosition")? {
        "LEFT" => PanelPosition::Left,
        "MIDDLE" => PanelPosition::Middle,
        "RIGHT" => PanelPosition::Right,
        "NOTDEFINED" => PanelPosition::NotDefined,
        _ => return Err(malformed("PanelPosition")),
    };
    let width = match set.attribute("PanelWidth").map(|a| &a.value) {
        Some(ExactValue::Null) => None,
        Some(ExactValue::Real(width)) => Some(*width),
        _ => return Err(malformed("PanelWidth")),
    };
    Ok(Panel {
        set: set.set_id,
        position,
        operation: text("PanelOperation")?,
        width,
    })
}
