//! An edited object's own sets, the values it inherits, and who else holds
//! a set or a property.
//!
//! What an object states and inherits is read once per object with the
//! exact resolver, so an edit addresses exactly what the read side reports:
//! the resolver's `occurrence` sets are an occurrence's own, a queried type
//! object's sets are its own, and an occurrence's `type` entries are what it
//! inherits. Everything is read from the model as it was before the batch;
//! the planner tracks the batch's own changes on top.

use std::collections::BTreeMap;

use ifc_model::{EntityId, Model, ReverseIndex, Value};
use ifc_properties::{ExactSource, PropertyIndex, SchemaVersion};
use ifc_schema::Schema;

use super::edit::PropertyEditFailure;

/// The release every edit of a batch is checked against.
#[derive(Debug, Clone, Copy)]
pub(super) struct Release {
    pub(super) version: SchemaVersion,
    pub(super) schema: &'static Schema,
}

impl Release {
    /// The slot of `attribute` in `entity`'s record, by name.
    pub(super) fn slot(&self, entity: &str, attribute: &str) -> Option<usize> {
        self.schema
            .attribute_names(entity)
            .iter()
            .position(|name| name.eq_ignore_ascii_case(attribute))
    }

    /// Attribute `attribute` of `id`, by name; `None` when the entity or the
    /// attribute is missing.
    pub(super) fn attribute<'m>(
        &self,
        model: &'m Model,
        id: EntityId,
        attribute: &str,
    ) -> Option<&'m Value> {
        let entity = model.get(id)?;
        let slot = self.slot(&entity.type_name, attribute)?;
        entity.attributes.get(slot)
    }

    /// The text of attribute `attribute` of `id`, a typed wrapper read
    /// through.
    pub(super) fn text<'m>(
        &self,
        model: &'m Model,
        id: EntityId,
        attribute: &str,
    ) -> Option<&'m str> {
        match self.attribute(model, id, attribute)?.unwrap_typed() {
            Value::Text(text) => Some(text),
            _ => None,
        }
    }
}

/// One of the object's own sets.
#[derive(Debug, Clone)]
pub(super) struct OwnSet {
    pub(super) id: EntityId,
    /// Its entity, upper-case.
    pub(super) type_name: String,
}

/// A value the occurrence inherits from its type object.
#[derive(Debug, Clone, Copy)]
pub(super) struct Inherited {
    pub(super) type_object: EntityId,
    pub(super) property: EntityId,
}

/// What one edited object states and inherits.
#[derive(Debug, Clone)]
pub(super) struct Holder {
    pub(super) id: EntityId,
    /// An `IfcTypeObject`, whose sets are its `HasPropertySets`.
    pub(super) is_type: bool,
    /// Its `GlobalId`, which new records derive theirs from.
    pub(super) global_id: Option<String>,
    /// Its `OwnerHistory`, which new records take.
    pub(super) owner_history: Option<EntityId>,
    /// Its own sets, by name.
    pub(super) own: BTreeMap<String, OwnSet>,
    /// The sets its type object holds, by name: the type object and the
    /// set's entity, upper-case.
    pub(super) inherited_sets: BTreeMap<String, (EntityId, String)>,
    /// Inherited values by set and property name.
    pub(super) inherited: BTreeMap<(String, String), Inherited>,
}

impl Holder {
    /// Read `id`'s own and inherited sets.
    ///
    /// # Errors
    ///
    /// [`PropertyEditFailure::MissingEntity`] for an id not in the model,
    /// [`PropertyEditFailure::Resolve`] for whatever the resolver refuses,
    /// and [`PropertyEditFailure::InvalidModel`] for two own sets of one
    /// name.
    pub(super) fn read(
        model: &Model,
        release: Release,
        properties: &PropertyIndex<'_>,
        id: EntityId,
    ) -> Result<Self, PropertyEditFailure> {
        let entity = model
            .get(id)
            .ok_or(PropertyEditFailure::MissingEntity(id))?;
        // `properties` was built from `model`: the same answer as
        // `exact_properties(model, id)`, without rescanning every
        // relationship for each object of a batch (#352).
        let entries = properties
            .exact_properties(id)
            .map_err(PropertyEditFailure::Resolve)?;
        let is_type = release.schema.is_a(&entity.type_name, "IFCTYPEOBJECT");
        let mut holder = Self {
            id,
            is_type,
            global_id: release.text(model, id, "GlobalId").map(str::to_owned),
            owner_history: release
                .attribute(model, id, "OwnerHistory")
                .and_then(Value::as_ref_id),
            own: BTreeMap::new(),
            inherited_sets: BTreeMap::new(),
            inherited: BTreeMap::new(),
        };
        for entry in &entries {
            let property = &entry.property;
            let set_name = property.property_set.to_string();
            let set_type = model
                .get(property.set_id)
                .map(|set| set.type_name.to_ascii_uppercase())
                .unwrap_or_default();
            let own = match property.source {
                ExactSource::Occurrence => !is_type,
                ExactSource::Type(type_object) => {
                    if is_type {
                        type_object == id
                    } else {
                        holder
                            .inherited_sets
                            .entry(set_name.clone())
                            .or_insert((type_object, set_type.clone()));
                        holder.inherited.insert(
                            (set_name.clone(), entry.name.to_string()),
                            Inherited {
                                type_object,
                                property: property.property_id,
                            },
                        );
                        false
                    }
                }
                // A material's sets are no object's to edit here.
                _ => false,
            };
            if !own {
                continue;
            }
            match holder.own.get(&set_name) {
                Some(existing) if existing.id != property.set_id => {
                    return Err(PropertyEditFailure::InvalidModel(format!(
                        "{id} states two sets named {set_name}: {} and {}",
                        existing.id, property.set_id
                    )));
                }
                Some(_) => {}
                None => {
                    holder.own.insert(
                        set_name,
                        OwnSet {
                            id: property.set_id,
                            type_name: set_type,
                        },
                    );
                }
            }
        }
        Ok(holder)
    }
}

/// How a holder holds one of its existing sets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Link {
    /// Through this `IfcRelDefinesByProperties`.
    Rel(EntityId),
    /// In its own `HasPropertySets`.
    Type,
}

/// Whether `holder` alone holds `set`, and how it holds it.
///
/// Only the references that make a set some object's count: an
/// `IfcRelDefinesByProperties` and a type object's `HasPropertySets`. A set
/// is the holder's alone when exactly one of them names it, and that is a
/// relationship relating only the holder, or the holder's own list. Any
/// other referrer (an `IfcRelDefinesByTemplate`) shares no values.
///
/// # Errors
///
/// [`PropertyEditFailure::InvalidModel`] when no relationship relates the
/// holder to the set, or two do; [`PropertyEditFailure::Unsupported`] for a
/// relationship whose definition is an `IfcPropertySetDefinitionSet`.
pub(super) fn holding(
    model: &Model,
    release: Release,
    reverse: &ReverseIndex,
    holder: &Holder,
    set: EntityId,
) -> Result<(Link, bool), PropertyEditFailure> {
    let schema = release.schema;
    let holders: Vec<EntityId> = reverse
        .referrers(set)
        .iter()
        .map(|referrer| referrer.from)
        .filter(|from| {
            model.get(*from).is_some_and(|entity| {
                schema.is_a(&entity.type_name, "IFCRELDEFINESBYPROPERTIES")
                    || schema.is_a(&entity.type_name, "IFCTYPEOBJECT")
            })
        })
        .collect();
    if holder.is_type {
        return Ok((Link::Type, holders == [holder.id]));
    }
    let related = |rel: EntityId| -> Vec<EntityId> {
        release
            .attribute(model, rel, "RelatedObjects")
            .and_then(Value::as_list)
            .map(|items| items.iter().filter_map(Value::as_ref_id).collect())
            .unwrap_or_default()
    };
    let rels: Vec<EntityId> = holders
        .iter()
        .copied()
        .filter(|rel| related(*rel).contains(&holder.id))
        .collect();
    let [rel] = rels.as_slice() else {
        return Err(PropertyEditFailure::InvalidModel(format!(
            "{} is related to set {set} by {} relationships, not one",
            holder.id,
            rels.len()
        )));
    };
    match release.attribute(model, *rel, "RelatingPropertyDefinition") {
        Some(Value::Ref(definition)) if *definition == set => {}
        _ => {
            return Err(PropertyEditFailure::Unsupported(format!(
                "{rel} relates set {set} through an IfcPropertySetDefinitionSet, which this writer does not split"
            )))
        }
    }
    let exclusive = holders == [*rel] && related(*rel) == [holder.id];
    Ok((Link::Rel(*rel), exclusive))
}

/// Whether `property` is referenced by `set` alone, and once there.
pub(super) fn sole_member(
    model: &Model,
    reverse: &ReverseIndex,
    set: EntityId,
    property: EntityId,
) -> bool {
    let only_set = reverse
        .referrers(property)
        .iter()
        .all(|referrer| referrer.from == set);
    let once = model
        .get(set)
        .map(|entity| {
            entity
                .attributes
                .iter()
                .filter_map(Value::as_list)
                .flatten()
                .filter(|item| item.as_ref_id() == Some(property))
                .count()
                == 1
        })
        .unwrap_or(false);
    only_set && once
}
