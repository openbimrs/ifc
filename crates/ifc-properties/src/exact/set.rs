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
//!
//! A set's members are its properties, its quantities, or, for a predefined
//! set, the attributes its entity declares itself (`predefined.rs`). Both
//! [`find_property`] and the enumeration read them through [`LoadedSet`].

use std::{collections::BTreeMap, sync::Arc};

use ifc_model::{Entity, EntityId, Model, Value};

use super::complex::{complex_value, is_complex};
use super::composite::composite_value;
use super::predefined::{attribute_value, own_attributes, predefined_key};
use super::quantity::{quantity_members, quantity_value, slot};
use super::refs::{nonempty_refs_at, text_at};
use super::release::Release;
use super::value::{exact_property_value, ResolvedValue};
use super::{ExactProperty, ExactPropertyError, ExactSource};

/// The kind of an assigned definition, once it is known to be readable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SetKind {
    /// An `IfcPropertySet`.
    Properties,
    /// An `IfcElementQuantity`.
    Quantities,
    /// Any other `IfcPropertySetDefinition`: a predefined set, whose values
    /// are attributes of its own entity.
    Predefined,
}

/// One member of a set: a property or quantity entity, or the slot of a
/// predefined set's own attribute.
#[derive(Debug, Clone, Copy)]
pub(super) enum Member {
    /// An `IfcProperty` or `IfcPhysicalQuantity`.
    Entity(EntityId),
    /// A slot of the predefined set itself.
    Attribute(usize),
}

/// An assigned definition, loaded, classified and named.
pub(super) struct LoadedSet<'m> {
    pub(super) id: EntityId,
    pub(super) entity: &'m Entity,
    pub(super) kind: SetKind,
    /// The name a set selector sees: `Name`, or for a predefined set that
    /// states none, its entity name in the release's spelling.
    pub(super) name: &'m str,
    /// Whether `name` is a stated `Name`.
    pub(super) named: bool,
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

/// [`load_set`], then the set's name.
///
/// # Errors
///
/// Those of [`load_set`], or a `Name` that is not text (`$` too, except on
/// a predefined set).
pub(super) fn load_named(
    model: &Model,
    release: Release,
    id: EntityId,
) -> Result<LoadedSet<'_>, ExactPropertyError> {
    let (entity, kind) = load_set(model, release, id)?;
    let (name, named) = match kind {
        SetKind::Predefined => predefined_key(release, id, entity)?,
        _ => (text_at(id, entity.attributes.get(2), "Name")?, true),
    };
    Ok(LoadedSet {
        id,
        entity,
        kind,
        name,
        named,
    })
}

impl<'m> LoadedSet<'m> {
    /// Refuse a set the set selector did not pick when that cannot rule it
    /// out: a predefined set that states no `Name`, one of whose own
    /// attributes `select_property` picks (#66).
    ///
    /// # Errors
    ///
    /// [`ExactPropertyError::UnsupportedDefinition`] in that case.
    pub(super) fn refuse_unselected(
        &self,
        release: Release,
        mut select_property: impl FnMut(&str) -> bool,
    ) -> Result<(), ExactPropertyError> {
        let unnamed_predefined = self.kind == SetKind::Predefined && !self.named;
        if unnamed_predefined
            && own_attributes(release, self.entity).any(|(_, name)| select_property(name))
        {
            return Err(ExactPropertyError::UnsupportedDefinition {
                entity: self.id,
                type_name: self.entity.type_name.clone(),
            });
        }
        Ok(())
    }

    /// Record this selected set's name among `selected`, returning the set
    /// that already stated it.
    ///
    /// Two sets that state one `Name` on one source are ambiguous whatever
    /// is asked. Predefined sets that state none are not: a door carries
    /// one `IfcDoorPanelProperties` per leaf. Those are ambiguous only for a
    /// member both hold, which the callers detect per member.
    pub(super) fn shares_name(
        &self,
        selected: &mut BTreeMap<&'m str, EntityId>,
    ) -> Option<EntityId> {
        if self.kind == SetKind::Predefined && !self.named {
            return None;
        }
        selected.insert(self.name, self.id)
    }

    /// Whether the set's member list is empty or unset (`()` or `$`).
    ///
    /// `HasProperties` and `Quantities` are `SET [1:?]` in every release, so
    /// such a set is invalid and [`Self::members`] refuses it. Enumerating
    /// sets (#186) still needs to see it: the set exists and holds nothing.
    /// A predefined set has no member list.
    pub(super) fn member_list_is_empty(&self, release: Release) -> bool {
        let slot = match self.kind {
            SetKind::Properties => slot(release, "IFCPROPERTYSET", "HasProperties"),
            SetKind::Quantities => slot(release, "IFCELEMENTQUANTITY", "Quantities"),
            SetKind::Predefined => return false,
        };
        match self.entity.attributes.get(slot) {
            Some(Value::Null) => true,
            Some(value) => value.as_list().is_some_and(<[Value]>::is_empty),
            None => false,
        }
    }

    /// Every member with its name, in file or schema order. Each is
    /// validated; none is resolved.
    ///
    /// # Errors
    ///
    /// A malformed member list, or a missing, foreign, mis-sized or unnamed
    /// member.
    pub(super) fn members(
        &self,
        model: &'m Model,
        release: Release,
    ) -> Result<Vec<(Member, &'m str)>, ExactPropertyError> {
        let entities = |members: Vec<(EntityId, &'m str)>| {
            members
                .into_iter()
                .map(|(id, name)| (Member::Entity(id), name))
                .collect()
        };
        Ok(match self.kind {
            SetKind::Properties => {
                entities(property_members(model, release, self.id, self.entity)?)
            }
            SetKind::Quantities => {
                entities(quantity_members(model, release, self.id, self.entity)?)
            }
            SetKind::Predefined => own_attributes(release, self.entity)
                .map(|(slot, name)| (Member::Attribute(slot), name))
                .collect(),
        })
    }

    /// The id a member is reported under: the property or quantity, or the
    /// predefined set itself.
    pub(super) fn member_id(&self, member: Member) -> EntityId {
        match member {
            Member::Entity(id) => id,
            Member::Attribute(_) => self.id,
        }
    }

    /// Resolve one member.
    ///
    /// # Errors
    ///
    /// An attribute that cannot be read exactly, or any value, unit, rule
    /// or nesting error.
    pub(super) fn value(
        &self,
        model: &Model,
        release: Release,
        member: Member,
    ) -> Result<ResolvedValue, ExactPropertyError> {
        match (self.kind, member) {
            (SetKind::Quantities, Member::Entity(id)) => quantity_value(model, release, id),
            (_, Member::Entity(id)) => property_value(model, release, id),
            (_, Member::Attribute(slot)) => {
                attribute_value(model, release, self.id, self.entity, slot)
            }
        }
    }

    /// Assemble a resolved member with its provenance.
    pub(super) fn exact(
        &self,
        source: ExactSource,
        member: Member,
        resolved: ResolvedValue,
    ) -> ExactProperty {
        ExactProperty {
            source,
            property_set: Arc::from(self.name),
            set_id: self.id,
            property_id: self.member_id(member),
            value_type: resolved.value_type,
            unit_id: resolved.unit_id,
            value: resolved.value,
        }
    }
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

/// The value of property `property_id`: a single, enumerated, list,
/// bounded, table or reference value, or an `IfcComplexProperty` as
/// [`ExactValue::Complex`](super::ExactValue::Complex) (#208).
///
/// # Errors
///
/// Any value, unit, rule or nesting error of the property.
pub(super) fn property_value(
    model: &Model,
    release: Release,
    property_id: EntityId,
) -> Result<ResolvedValue, ExactPropertyError> {
    let property = model.get(property_id).expect("checked reference");
    if is_complex(release, property) {
        return complex_value(model, release, property_id, property);
    }
    simple_property_value(model, release, property_id, property)
}

/// The value of `IfcSimpleProperty` `property`, whose arity the caller
/// confirmed.
///
/// # Errors
///
/// [`ExactPropertyError::UnsupportedProperty`] for any other property kind,
/// or any value, unit or rule error of the property.
pub(super) fn simple_property_value(
    model: &Model,
    release: Release,
    property_id: EntityId,
    property: &Entity,
) -> Result<ResolvedValue, ExactPropertyError> {
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

/// The one property, quantity or predefined attribute named
/// `wanted_property` among `sets` of one source, optionally restricted to
/// sets named `wanted_set`.
pub(super) fn find_property(
    model: &Model,
    release: Release,
    sets: &[EntityId],
    source: ExactSource,
    wanted_set: Option<&str>,
    wanted_property: &str,
) -> Result<Option<ExactProperty>, ExactPropertyError> {
    let mut result: Option<ExactProperty> = None;
    let mut matching_sets = BTreeMap::new();
    for &set_id in sets {
        let set = load_named(model, release, set_id)?;
        if wanted_set.is_some_and(|name| set.name != name) {
            set.refuse_unselected(release, |name| name == wanted_property)?;
            continue;
        }
        if let Some(first) = set.shares_name(&mut matching_sets) {
            return Err(ExactPropertyError::DuplicateMatchingSets {
                source,
                first,
                second: set_id,
            });
        }
        let mut matching: Option<Member> = None;
        for (member, name) in set.members(model, release)? {
            if name != wanted_property {
                continue;
            }
            if let Some(first) = matching.replace(member) {
                return Err(ExactPropertyError::DuplicateMatchingProperties {
                    set: set_id,
                    first: set.member_id(first),
                    second: set.member_id(member),
                });
            }
        }
        if let Some(member) = matching {
            let candidate = set.exact(source, member, set.value(model, release, member)?);
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
