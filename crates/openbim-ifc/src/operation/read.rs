//! Exact reads both operation joins make, from the declared release.
//!
//! Every attribute is found by name in the bound release's table, never by a
//! restated slot: `IfcDoor.OperationType` and `IfcWindow.PartitioningType`
//! exist only from IFC4 on, and IFC2X3's `IfcDoorStyle`/`IfcWindowStyle`
//! hold their `OperationType` at other positions than the IFC4 type
//! objects. Relationship structure is validated first by the caller's
//! `exact_predefined_sets`, which refuses malformed assignments, several
//! type objects, and relationship subtypes relating the product.

use std::sync::Arc;

use ifc_geometry::{product_world_transform, units, GeometryError, Transform};
use ifc_model::{Entity, EntityId, Model, Value};
use ifc_properties::{exact_unit, ExactPredefinedSet, ExactSource, ExactUnitError};
use ifc_schema::{Schema, TypeKind};

/// Why an attribute could not be read exactly.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ReadError {
    /// The value is not one its declaration accepts.
    Malformed {
        entity: EntityId,
        attribute: &'static str,
    },
    /// The project length unit could not be resolved exactly.
    Unit(ExactUnitError),
    /// The placement could not be resolved.
    Placement(GeometryError),
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
/// for `$` and for an attribute the release does not declare.
pub(crate) fn enumeration(
    schema: &'static Schema,
    id: EntityId,
    entity: &Entity,
    name: &'static str,
) -> Result<Option<Arc<str>>, ReadError> {
    let malformed = ReadError::Malformed {
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

/// The product's type object, through `IfcRelDefinesByType`.
///
/// Uniqueness, targets and relationship subtypes were validated by the
/// exact resolver, so the first match is the only one.
pub(crate) fn type_object(
    model: &Model,
    schema: &'static Schema,
    product: EntityId,
) -> Result<Option<EntityId>, ReadError> {
    for id in model.ids_of_type("IFCRELDEFINESBYTYPE") {
        let relation = model.get(*id).expect("type index is current");
        let related = attribute(schema, relation, "RelatedObjects");
        let relating = attribute(schema, relation, "RelatingType");
        let (Some((Value::List(related), _)), Some((Value::Ref(type_id), _))) = (related, relating)
        else {
            return Err(ReadError::Malformed {
                entity: *id,
                attribute: "RelatingType",
            });
        };
        if related.contains(&Value::Ref(product)) {
            return Ok(Some(*type_id));
        }
    }
    Ok(None)
}

/// An `IfcPositiveLengthMeasure` attribute in metres, or `None` for `$`.
pub(crate) fn positive_length(
    model: &Model,
    schema: &'static Schema,
    id: EntityId,
    entity: &Entity,
    name: &'static str,
) -> Result<Option<f64>, ReadError> {
    match attribute(schema, entity, name) {
        Some((Value::Null, _)) => Ok(None),
        // `IfcPositiveLengthMeasure`: a finite length greater than zero.
        Some((Value::Real(length), _)) if length.is_finite() && *length > 0.0 => {
            Ok(Some(length * length_scale(model)?))
        }
        _ => Err(ReadError::Malformed {
            entity: id,
            attribute: name,
        }),
    }
}

/// Metres per project length unit, resolved exactly.
fn length_scale(model: &Model) -> Result<f64, ReadError> {
    exact_unit(model, "IFCPOSITIVELENGTHMEASURE", None)
        .map(|unit| unit.scale)
        .map_err(ReadError::Unit)
}

/// The product's world transform in metres, scaled by the exact length
/// unit rather than the permissive one.
pub(crate) fn world_transform(model: &Model, product: EntityId) -> Result<Transform, ReadError> {
    let mut scale = units::resolve(model);
    scale.length_to_metres = length_scale(model)?;
    product_world_transform(model, &scale, product).map_err(ReadError::Placement)
}

/// The occurrence's sets if it has any, else its type's; never a mix.
/// `None` when neither carries one.
pub(crate) fn governing(
    sets: &[ExactPredefinedSet],
) -> Option<(ExactSource, Vec<&ExactPredefinedSet>)> {
    let source = sets
        .iter()
        .find(|set| set.source == ExactSource::Occurrence)
        .or_else(|| sets.first())
        .map(|set| set.source)?;
    let governing = sets.iter().filter(|set| set.source == source).collect();
    Some((source, governing))
}
