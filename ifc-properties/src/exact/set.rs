//! Reading one assigned property-set definition exactly.
//!
//! ```text
//! IfcPropertySet      2 = Name   4 = HasProperties  (SET [1:?] OF IfcProperty)
//! IfcProperty         0 = Name
//! ```
//!
//! Both positions hold in IFC2X3 TC1, IFC4 ADD2 TC1 and IFC4X3 ADD2 (IFC4X3
//! renames `IfcProperty.Description` to `Specification`; `Name` stays
//! first). Every member is validated before any is matched, so a malformed
//! member is refused even when another member carries the asked-for name.

use std::{collections::BTreeMap, sync::Arc};

use ifc_model::{Entity, EntityId, Model};

use super::composite::composite_value;
use super::quantity::{find_quantity, predefined_may_hold};
use super::refs::{nonempty_refs_at, text_at};
use super::release::Release;
use super::value::{exact_property_value, ResolvedValue};
use super::{ExactProperty, ExactPropertyError, ExactSource};

/// The kind of an assigned definition, once it is known to be readable.
pub(super) enum SetKind {
    /// An `IfcPropertySet`.
    Properties,
    /// An `IfcElementQuantity`.
    Quantities,
    /// Any other `IfcPropertySetDefinition`: a predefined set, whose values
    /// are entity attributes this resolver does not read.
    Predefined,
}

/// Load assigned definition `set_id`, check its arity and classify it.
///
/// # Errors
///
/// A missing entity, a slot-count mismatch, or an entity that is no
/// `IfcPropertySetDefinition` at all.
pub(super) fn load_set(
    model: &Model,
    release: Release,
    set_id: EntityId,
) -> Result<(&Entity, SetKind), ExactPropertyError> {
    let schema = release.schema;
    let set = model
        .get(set_id)
        .ok_or(ExactPropertyError::MissingReference {
            from: set_id,
            to: set_id,
        })?;
    release.require_exact_slots(set_id, set)?;
    let kind = if set.is_type("IFCPROPERTYSET") {
        SetKind::Properties
    } else if schema.is_a(&set.type_name, "IFCELEMENTQUANTITY") {
        SetKind::Quantities
    } else if schema.is_a(&set.type_name, "IFCPROPERTYSETDEFINITION") {
        SetKind::Predefined
    } else {
        return Err(ExactPropertyError::UnsupportedDefinition {
            entity: set_id,
            type_name: set.type_name.clone(),
        });
    };
    Ok((set, kind))
}

/// Every member of an `IfcPropertySet` with its `Name`, in file order.
///
/// # Errors
///
/// A malformed or empty `HasProperties`, a missing, foreign or non-property
/// member, a member with the wrong arity, or a member without a text name.
pub(super) fn property_members<'m>(
    model: &'m Model,
    release: Release,
    set_id: EntityId,
    set: &Entity,
) -> Result<Vec<(EntityId, &'m str)>, ExactPropertyError> {
    let schema = release.schema;
    let mut members = Vec::new();
    for property_id in nonempty_refs_at(set_id, set.attributes.get(4), "HasProperties")? {
        let property = model
            .get(property_id)
            .ok_or(ExactPropertyError::MissingReference {
                from: set_id,
                to: property_id,
            })?;
        if schema.entity(property.type_name.as_ref()).is_none() {
            return Err(release.not_in_schema(property_id, property.type_name.clone()));
        }
        if !schema.is_a(property.type_name.as_ref(), "IFCPROPERTY") {
            return Err(ExactPropertyError::UnsupportedProperty {
                entity: property_id,
                type_name: property.type_name.clone(),
            });
        }
        release.require_exact_slots(property_id, property)?;
        let name = text_at(property_id, property.attributes.first(), "Name")?;
        members.push((property_id, name));
    }
    Ok(members)
}

/// The value of property `property_id`, which must be an
/// `IfcSimpleProperty`: a single, enumerated, list, bounded, table or
/// reference value.
///
/// # Errors
///
/// [`ExactPropertyError::UnsupportedProperty`] for an `IfcComplexProperty`,
/// or any value, unit or rule error of the property.
pub(super) fn property_value(
    model: &Model,
    release: Release,
    property_id: EntityId,
) -> Result<ResolvedValue, ExactPropertyError> {
    let property = model.get(property_id).expect("checked reference");
    if property.is_type("IFCPROPERTYSINGLEVALUE") {
        return exact_property_value(model, release, property_id, property);
    }
    composite_value(model, release, property_id, property)?.ok_or_else(|| {
        ExactPropertyError::UnsupportedProperty {
            entity: property_id,
            type_name: property.type_name.clone(),
        }
    })
}

/// The one property or quantity named `wanted_property` among `sets` of
/// one source, optionally restricted to sets named `wanted_set`.
pub(super) fn find_property(
    model: &Model,
    release: Release,
    sets: &[EntityId],
    source: ExactSource,
    wanted_set: Option<&str>,
    wanted_property: &str,
) -> Result<Option<ExactProperty>, ExactPropertyError> {
    let mut result = None;
    let mut matching_sets = BTreeMap::new();
    for &set_id in sets {
        let (set, kind) = load_set(model, release, set_id)?;
        if let SetKind::Predefined = kind {
            // A predefined set keeps its values in attributes, which this
            // resolver does not read. Skipping it is only honest when none
            // of those attributes could be the property asked for.
            if !predefined_may_hold(release, set, wanted_set, wanted_property) {
                continue;
            }
            return Err(ExactPropertyError::UnsupportedDefinition {
                entity: set_id,
                type_name: set.type_name.clone(),
            });
        }
        let set_name = text_at(set_id, set.attributes.get(2), "Name")?;
        if let Some(name) = wanted_set {
            if set_name != name {
                continue;
            }
        }
        if let Some(first) = matching_sets.insert(set_name.to_owned(), set_id) {
            return Err(ExactPropertyError::DuplicateMatchingSets {
                source,
                first,
                second: set_id,
            });
        }
        let found = if let SetKind::Quantities = kind {
            find_quantity(model, release, set_id, set, wanted_property)?
        } else {
            let mut matching = None;
            for (property_id, name) in property_members(model, release, set_id, set)? {
                if name != wanted_property {
                    continue;
                }
                if let Some(first) = matching.replace(property_id) {
                    return Err(ExactPropertyError::DuplicateMatchingProperties {
                        set: set_id,
                        first,
                        second: property_id,
                    });
                }
            }
            match matching {
                Some(property_id) => {
                    Some((property_id, property_value(model, release, property_id)?))
                }
                None => None,
            }
        };
        if let Some((property_id, resolved)) = found {
            let candidate = exact(source, set_name, set_id, property_id, resolved);
            if let Some(first) = result.replace(candidate) {
                return Err(ExactPropertyError::DuplicateMatchingSets {
                    source,
                    first: first.set_id,
                    second: set_id,
                });
            }
        }
    }
    Ok(result)
}

/// Assemble a resolved property with its provenance.
pub(super) fn exact(
    source: ExactSource,
    set_name: &str,
    set_id: EntityId,
    property_id: EntityId,
    resolved: ResolvedValue,
) -> ExactProperty {
    ExactProperty {
        source,
        property_set: Arc::from(set_name),
        set_id,
        property_id,
        value_type: resolved.value_type,
        unit_id: resolved.unit_id,
        value: resolved.value,
    }
}
