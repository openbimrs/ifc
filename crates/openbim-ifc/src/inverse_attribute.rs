//! `INVERSE` attributes by name (#462): `wall.IsDefinedBy`, resolved
//! through the reverse index.
//!
//! An `INVERSE` attribute occupies no Part 21 slot. The schema declares it
//! as `Name : SET [0:?] OF Entity FOR Attribute;`: its value is every
//! instance of `Entity` (or a subtype) whose explicit attribute `Attribute`
//! references this one, either directly or as a member of an aggregate
//! (`RelatedObjects : SET [1:?] OF IfcObjectDefinition`). The answer is
//! read from a [`ReverseIndex`], which records each reference with the
//! top-level slot it sits in: a referrer counts when its type is `Entity`
//! or a subtype in the declared release, and the slot is the one that
//! release gives `Attribute` (ISO 10303-11 §9.2.1.3).
//!
//! Everything is the declared release's own: IFC2X3's `IfcObject` has one
//! `IsDefinedBy` over `IfcRelDefines`, holding type and property
//! relationships alike, where IFC4 splits it into `IsTypedBy`
//! (`IfcRelDefinesByType`) and `IsDefinedBy` (`IfcRelDefinesByProperties`);
//! a type object's occurrences are `ObjectTypeOf` in IFC2X3 and `Types`
//! from IFC4; IFC4X3 moves `ReferencedInStructures` from `IfcElement` up to
//! `IfcProduct`. A name is never answered from another release.
//!
//! # Values and cardinality
//!
//! The value has the shape the schema declares:
//!
//! - a `SET` or `BAG` inverse, whatever its bounds, is
//!   [`InverseValue::Aggregate`]: every referrer, in ascending id order,
//!   empty when nothing references the entity. That includes the optional
//!   single inverses declared `SET [0:1]`, such as `ContainedInStructure`
//!   or `Decomposes`: zero or one element, never unwrapped, as IfcOpenShell
//!   returns a tuple. A count beyond the declared upper bound is returned
//!   as found; schema validation reports it.
//! - a non-aggregate inverse (`VoidsElements : IfcRelVoidsElement FOR
//!   RelatedOpeningElement`, IFC2X3's `IfcGroup.IsGroupedBy`) is
//!   [`InverseValue::Single`]: the one referrer, or `None` when nothing
//!   references the entity (the schema requires one; a reader treats it as
//!   it treats `$` in a mandatory slot). Two or more are refused with
//!   [`NamedAttributeError::InverseCardinality`], never answered with one.
//!
//! [`inverse_slots`] lists every `INVERSE` attribute of a type, inherited
//! ones first, with its `FOR` clause and bounds. [`total_inverses`] counts
//! the distinct entities referencing one, through any attribute, as
//! IfcOpenShell's `get_total_inverses` does; [`ReverseIndex::referrers`]
//! lists them with the slot each reference sits in, its `get_inverse(...,
//! with_attribute_indices=True)`.
//!
//! ```
//! # #[cfg(feature = "ifc4")] {
//! use ifc::{attribute_value_by_name, AttributeValue, Entity, EntityId, InverseValue,
//!           Model, ReverseIndex, Value};
//!
//! let mut model = Model::new();
//! let wall = model.push(Entity::new("IFCWALL", vec![Value::Null; 9]));
//! let rel = model.push(Entity::new("IFCRELDEFINESBYPROPERTIES", vec![
//!     Value::Null, Value::Null, Value::Null, Value::Null,
//!     Value::List(vec![Value::Ref(wall)]), Value::Null,
//! ]));
//! let reverse = ReverseIndex::build(&model);
//! let value = attribute_value_by_name(&model, ifc::schema::ifc4(), &reverse, wall, "isdefinedby");
//! assert_eq!(value.unwrap(), AttributeValue::Inverse(InverseValue::Aggregate(vec![rel])));
//! # }
//! ```
#![cfg(feature = "schema-api")]

use ifc_model::{EntityId, Model, ReverseIndex, Value};
use ifc_schema::{Aggregation, Schema};

use crate::named_attribute::{attribute_slot, attribute_slots, NamedAttributeError};

/// One `INVERSE` attribute of an entity type, as a release declares it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct InverseSlot<'s> {
    /// The declared name, in the schema's spelling (`IsDefinedBy`).
    pub name: &'s str,
    /// The referencing entity type named before `FOR`
    /// (`IfcRelDefinesByProperties`); its subtypes count too.
    pub entity: &'s str,
    /// The explicit attribute of [`Self::entity`] named after `FOR`
    /// (`RelatedObjects`), which references this entity directly or as an
    /// aggregate member.
    pub for_attribute: &'s str,
    /// The `SET` or `BAG` with its bounds; `None` for a non-aggregate
    /// inverse, whose value is a single entity.
    pub aggregation: Option<&'s Aggregation>,
    /// The entity whose declaration introduces the inverse (`IfcObject`).
    pub declared_by: &'s str,
}

impl InverseSlot<'_> {
    /// Whether the inverse is a `SET` or `BAG`: its value is
    /// [`InverseValue::Aggregate`].
    #[must_use]
    pub const fn is_aggregate(&self) -> bool {
        self.aggregation.is_some()
    }

    /// The declared lower bound: `1` for a non-aggregate inverse.
    #[must_use]
    pub fn lower(&self) -> u64 {
        self.aggregation
            .map_or(Some(1), |aggregation| aggregation.lower.as_integer())
            .unwrap_or(0)
    }

    /// The declared upper bound: `Some(1)` for a non-aggregate inverse,
    /// `None` for `?`.
    #[must_use]
    pub fn upper(&self) -> Option<u64> {
        self.aggregation
            .map_or(Some(1), |aggregation| aggregation.upper.as_integer())
    }
}

/// The value of an `INVERSE` attribute; see the module docs for the shape.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum InverseValue {
    /// A non-aggregate inverse: its one referrer, or `None`.
    Single(Option<EntityId>),
    /// A `SET` or `BAG` inverse (`SET [0:1]` included): every referrer, in
    /// ascending id order.
    Aggregate(Vec<EntityId>),
}

impl InverseValue {
    /// Every referrer, in ascending id order, whatever the shape.
    #[must_use]
    pub fn ids(&self) -> &[EntityId] {
        match self {
            Self::Single(found) => found.as_slice(),
            Self::Aggregate(ids) => ids,
        }
    }
}

/// An attribute read by name: an explicit slot's stored value, or an
/// `INVERSE` attribute's referrers.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum AttributeValue<'m> {
    /// An explicit attribute, as [`attribute_by_name`](crate::attribute_by_name)
    /// reads it: `None` when the record stops before the slot.
    Explicit(Option<&'m Value>),
    /// An `INVERSE` attribute.
    Inverse(InverseValue),
}

/// Every `INVERSE` attribute of `type_name` in `schema`, inherited ones
/// first (root first, each entity's in declaration order). A subtype that
/// redeclares an inherited inverse (`SELF\X.Name`) replaces it in place;
/// no bundled IFC release does.
///
/// # Errors
///
/// [`NamedAttributeError::UnknownEntity`] when `schema` does not declare
/// `type_name`.
pub fn inverse_slots<'s>(
    schema: &'s Schema,
    type_name: &str,
) -> Result<Vec<InverseSlot<'s>>, NamedAttributeError> {
    let entity = schema
        .entity(type_name)
        .ok_or_else(|| NamedAttributeError::UnknownEntity {
            schema: schema.name().to_owned(),
            type_name: type_name.to_owned(),
        })?;
    let mut chain: Vec<_> = std::iter::once(entity)
        .chain(
            schema
                .supertypes(&entity.name)
                .into_iter()
                .filter_map(|name| schema.entity(name)),
        )
        .collect();
    chain.reverse();
    let mut slots: Vec<InverseSlot<'s>> = Vec::new();
    for owner in chain {
        for inverse in &owner.inverses {
            let slot = InverseSlot {
                name: &inverse.name,
                entity: &inverse.entity,
                for_attribute: &inverse.for_attribute,
                aggregation: inverse.aggregation.as_ref(),
                declared_by: &owner.name,
            };
            match slots
                .iter_mut()
                .find(|existing| existing.name.eq_ignore_ascii_case(&inverse.name))
            {
                Some(existing) => *existing = slot,
                None => slots.push(slot),
            }
        }
    }
    Ok(slots)
}

/// The `INVERSE` attribute named `name` (any case) of `type_name` in
/// `schema`, or `None` when the type has none of that name (an explicit
/// attribute's name included).
///
/// # Errors
///
/// [`NamedAttributeError::UnknownEntity`] for an undeclared type.
pub fn inverse_slot<'s>(
    schema: &'s Schema,
    type_name: &str,
    name: &str,
) -> Result<Option<InverseSlot<'s>>, NamedAttributeError> {
    Ok(inverse_slots(schema, type_name)?
        .into_iter()
        .find(|slot| slot.name.eq_ignore_ascii_case(name)))
}

/// Attribute `name` (any case) of entity `id`, resolved against `schema`:
/// an explicit attribute's stored value, or an `INVERSE` attribute's
/// referrers read from `reverse`, which must be built from `model` as it
/// is now ([`ReverseIndex::build`]; build it once for many reads).
///
/// # Errors
///
/// [`NamedAttributeError::MissingEntity`],
/// [`NamedAttributeError::UnknownEntity`] for a type `schema` does not
/// declare, [`NamedAttributeError::UnknownAttribute`] for a name that is
/// neither, and [`NamedAttributeError::InverseCardinality`] for a
/// non-aggregate inverse more than one entity references.
pub fn attribute_value_by_name<'m>(
    model: &'m Model,
    schema: &Schema,
    reverse: &ReverseIndex,
    id: EntityId,
    name: &str,
) -> Result<AttributeValue<'m>, NamedAttributeError> {
    let entity = model
        .get(id)
        .ok_or(NamedAttributeError::MissingEntity(id))?;
    match attribute_slot(schema, &entity.type_name, name) {
        Ok(slot) => Ok(AttributeValue::Explicit(entity.attribute(slot.index))),
        Err(NamedAttributeError::InverseAttribute { .. }) => {
            let inverse =
                inverse_slot(schema, &entity.type_name, name)?.expect("attribute_slot found it");
            inverse_value(model, schema, reverse, id, &entity.type_name, &inverse)
                .map(AttributeValue::Inverse)
        }
        Err(error) => Err(error),
    }
}

/// How many distinct entities reference `id`, through any attribute; `0`
/// for an id nothing references or the model lacks. IfcOpenShell's
/// `get_total_inverses`.
#[must_use]
pub fn total_inverses(reverse: &ReverseIndex, id: EntityId) -> usize {
    let referrers = reverse.referrers(id);
    // Sorted by (from, slot): one entity's references are adjacent.
    referrers.len()
        - referrers
            .windows(2)
            .filter(|pair| pair[0].from == pair[1].from)
            .count()
}

fn inverse_value(
    model: &Model,
    schema: &Schema,
    reverse: &ReverseIndex,
    id: EntityId,
    type_name: &str,
    inverse: &InverseSlot<'_>,
) -> Result<InverseValue, NamedAttributeError> {
    // Inherited attributes keep their slot in every subtype, so the slot
    // the declaring entity gives the FOR attribute is every referrer's.
    let for_slot = attribute_slots(schema, inverse.entity)?
        .into_iter()
        .find(|slot| slot.name.eq_ignore_ascii_case(inverse.for_attribute))
        .ok_or_else(|| NamedAttributeError::UnknownAttribute {
            schema: schema.name().to_owned(),
            type_name: inverse.entity.to_owned(),
            name: inverse.for_attribute.to_owned(),
        })?
        .index;
    // Referrer types repeat (every IfcRelDefinesByProperties), so the
    // subtype answer is asked once per type.
    let mut accepted: Vec<(&str, bool)> = Vec::new();
    let mut found: Vec<EntityId> = Vec::new();
    for referrer in reverse.referrers(id) {
        if referrer.slot != for_slot || found.last() == Some(&referrer.from) {
            continue;
        }
        let Some(source) = model.get(referrer.from) else {
            continue;
        };
        let source_type: &str = &source.type_name;
        let is_a = match accepted.iter().find(|(name, _)| *name == source_type) {
            Some(&(_, is_a)) => is_a,
            None => {
                let is_a = schema.is_a(source_type, inverse.entity);
                accepted.push((source_type, is_a));
                is_a
            }
        };
        if is_a {
            found.push(referrer.from);
        }
    }
    if inverse.is_aggregate() {
        return Ok(InverseValue::Aggregate(found));
    }
    match found.as_slice() {
        [] => Ok(InverseValue::Single(None)),
        [one] => Ok(InverseValue::Single(Some(*one))),
        _ => Err(NamedAttributeError::InverseCardinality {
            type_name: schema
                .entity(type_name)
                .map_or(type_name, |entity| entity.name.as_str())
                .to_owned(),
            name: inverse.name.to_owned(),
            referrers: found,
        }),
    }
}

#[cfg(test)]
mod tests;
