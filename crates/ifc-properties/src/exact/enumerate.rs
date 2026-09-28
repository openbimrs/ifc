//! Exact enumeration of an object's properties (#78).
//!
//! A buildingSMART IDS property facet may name its set and property by
//! pattern (`Pset_.*Common`), and then every matching property must satisfy
//! the requirement. Deciding that needs every match, and an empty result
//! must be as much a proof as [`ExactResolution::Absent`]. So enumeration
//! runs the traversal of [`exact_property`] with two selectors in place of
//! its two names, and for one set name and one property name it answers
//! exactly what [`exact_property`] answers:
//!
//! - Model and assignment validation are the same.
//! - A set whose name the set selector rejects is skipped unread, as
//!   [`exact_property`] skips a set of another name. Every member of a
//!   selected set is validated before anything is matched.
//! - A selected predefined-set attribute that cannot be read exactly (an
//!   aggregate), and an unnamed predefined set outside the set selection
//!   one of whose own attributes is selected, are refused. A complex
//!   property or quantity resolves as `ExactValue::Complex` (#208). An
//!   unselected member must be well formed but need not have a supported
//!   value form.
//!
//! [`ExactResolution::Absent`]: super::ExactResolution::Absent
//! [`exact_property`]: super::exact_property

use std::{collections::BTreeMap, sync::Arc};

use ifc_model::{EntityId, Model};

use super::assignment::assigned_sets;
use super::release::{validate_model, Release};
use super::set::load_source_set;
use super::{ExactProperty, ExactPropertyError, ExactSource};

/// One property of an enumeration, with the name it was selected by.
#[derive(Debug, Clone, PartialEq)]
pub struct ExactPropertyEntry {
    /// `IfcProperty.Name` or `IfcPhysicalQuantity.Name`.
    pub name: Arc<str>,
    /// The resolved property with its provenance, as [`exact_property`]
    /// reports it.
    ///
    /// [`exact_property`]: super::exact_property
    pub property: ExactProperty,
}

/// Every property, simple quantity and predefined-set attribute of
/// `object`, resolved exactly.
///
/// Equivalent to [`exact_properties_where`] selecting every set and every
/// property, so any predefined set with an aggregate attribute is refused.
/// Every property and quantity kind resolves, complex ones included, as for
/// [`exact_property`](super::exact_property). Callers that need
/// only some properties (an IDS pattern facet) select them with
/// [`exact_properties_where`], so an unsupported member they do not ask
/// about cannot refuse their answer.
///
/// # Errors
///
/// Any [`ExactPropertyError`], as for [`exact_property`](super::exact_property).
pub fn exact_properties(
    model: &Model,
    object: EntityId,
) -> Result<Vec<ExactPropertyEntry>, ExactPropertyError> {
    exact_properties_where(model, object, |_| true, |_| true)
}

/// The properties and simple quantities of `object` in the sets
/// `select_set` picks, whose names `select_property` picks, resolved
/// exactly.
///
/// Both selectors see names only: `select_set` the `Name` of each assigned
/// property set and quantity set, `select_property` the `Name` of each
/// member of a selected set. With `|s| s == set` and `|p| p == name` the
/// result is `[x]` exactly when [`exact_property`] with that set name
/// answers `Present(x)`, empty exactly when it answers `Absent`, and the
/// same error otherwise.
///
/// A predefined property set (`IfcDoorLiningProperties` and the like) holds
/// its values in attributes of its own entity (#149). Its members are the
/// attributes it declares below `IfcPropertySetDefinition`, by schema name
/// (`LiningDepth`), and `select_set` sees its `Name`, or its entity name
/// (`IfcDoorLiningProperties`) when it states none. Because a set without a
/// `Name` cannot be ruled out by a name, such a set that `select_set` rejects
/// is still refused with [`ExactPropertyError::UnsupportedDefinition`] when
/// `select_property` picks one of its attributes (#66).
///
/// The result is in assignment order: occurrence sets first, then the sets
/// inherited from the object's `IfcTypeObject`, each set's members in file
/// order. An inherited property is left out when an occurrence set of the
/// same name has a selected property of the same name: the occurrence
/// value overrides it at property level. An empty result is a proven
/// absence of every selected property. For a queried `IfcTypeObject` the
/// result is its own `HasPropertySets`, each entry with
/// [`ExactSource::Type`] of `object` (#193), as for
/// [`exact_property`].
///
/// # Errors
///
/// Any [`ExactPropertyError`], as for [`exact_property`]. Ambiguity is
/// refused per source: two selected sets of the same name, a property set
/// and a quantity set included
/// ([`ExactPropertyError::DuplicateMatchingSets`]), and two selected members
/// of one set with the same name
/// ([`ExactPropertyError::DuplicateMatchingProperties`]).
///
/// [`exact_property`]: super::exact_property
pub fn exact_properties_where<S, P>(
    model: &Model,
    object: EntityId,
    mut select_set: S,
    mut select_property: P,
) -> Result<Vec<ExactPropertyEntry>, ExactPropertyError>
where
    S: FnMut(&str) -> bool,
    P: FnMut(&str) -> bool,
{
    let release = validate_model(model)?;
    let assigned = assigned_sets(model, release, object)?;
    let mut selector = Selector {
        set: &mut select_set,
        property: &mut select_property,
    };
    let mut entries = collect(
        model,
        release,
        &assigned.occurrence_sets,
        ExactSource::Occurrence,
        &mut selector,
    )?;
    if let Some((type_id, sets)) = &assigned.type_sets {
        let inherited = collect(
            model,
            release,
            sets,
            ExactSource::Type(*type_id),
            &mut selector,
        )?;
        let overridden: Vec<(Arc<str>, Arc<str>)> = entries
            .iter()
            .map(|entry| (entry.property.property_set.clone(), entry.name.clone()))
            .collect();
        entries.extend(inherited.into_iter().filter(|entry| {
            !overridden
                .iter()
                .any(|(set, name)| *set == entry.property.property_set && *name == entry.name)
        }));
    }
    Ok(entries)
}

/// The caller's two selectors.
pub(super) struct Selector<'a> {
    pub(super) set: &'a mut dyn FnMut(&str) -> bool,
    pub(super) property: &'a mut dyn FnMut(&str) -> bool,
}

/// The selected properties of the selected sets of one source.
pub(super) fn collect(
    model: &Model,
    release: Release,
    sets: &[EntityId],
    source: ExactSource,
    select: &mut Selector<'_>,
) -> Result<Vec<ExactPropertyEntry>, ExactPropertyError> {
    let mut entries = Vec::new();
    let mut selected_sets: BTreeMap<&str, EntityId> = BTreeMap::new();
    for &set_id in sets {
        let set = load_source_set(model, release, source, set_id)?;
        if !(select.set)(set.name) {
            set.refuse_unselected(release, &mut *select.property)?;
            continue;
        }
        if let Some(first) = set.shares_name(&mut selected_sets) {
            return Err(ExactPropertyError::DuplicateMatchingSets {
                source,
                first,
                second: set_id,
            });
        }
        let mut chosen: BTreeMap<&str, EntityId> = BTreeMap::new();
        let mut in_order = Vec::new();
        for (member, name) in set.members(model, release)? {
            if !(select.property)(name) {
                continue;
            }
            if let Some(first) = chosen.insert(name, set.member_id(member)) {
                return Err(ExactPropertyError::DuplicateMatchingProperties {
                    set: set_id,
                    first,
                    second: set.member_id(member),
                });
            }
            in_order.push((member, name));
        }
        for (member, name) in in_order {
            let resolved = set.value(model, release, member)?;
            // A predefined set without a `Name` shares its entity name with
            // any other such set: a member both hold is ambiguous, as
            // `exact_property` finds it.
            let earlier = entries.iter().find(|entry: &&ExactPropertyEntry| {
                entry.property.property_set.as_ref() == set.name && entry.name.as_ref() == name
            });
            if let Some(earlier) = earlier {
                return Err(ExactPropertyError::DuplicateMatchingSets {
                    source,
                    first: earlier.property.set_id,
                    second: set_id,
                });
            }
            entries.push(ExactPropertyEntry {
                name: Arc::from(name),
                property: set.exact(source, member, resolved),
            });
        }
    }
    Ok(entries)
}

/// One assigned property set or quantity set that a set selector picked.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct ExactPropertySetEntry {
    /// The name the selector saw: `Name`, or for a predefined set that states
    /// none, its entity name (`IfcDoorLiningProperties`).
    pub name: Arc<str>,
    /// The set entity.
    pub set_id: EntityId,
    /// Whether the object carries the set itself or inherits it from its
    /// type; a queried type object's own sets are `Type` of that object.
    pub source: ExactSource,
    /// How many members the set holds, each validated; `0` for an empty set.
    pub members: usize,
}

/// The property sets and quantity sets of `object` whose names `select_set`
/// picks, empty ones included (#186).
///
/// [`exact_properties_where`] lists properties, so a selected set with no
/// members leaves no trace in its answer. This lists the sets themselves:
/// an IDS property facet must fail on a matching set that is empty, and it
/// can tell that case from "no such set" only here. An empty result is a
/// proven absence of every selected set.
///
/// The traversal, model and assignment validation and refusals are those of
/// [`exact_properties_where`]; every member of a selected set is validated,
/// though none is resolved. One difference: a set whose `HasProperties` or
/// `Quantities` is `()` or `$` is listed with `members == 0`. The schema
/// declares both `SET [1:?]`, so [`exact_properties_where`] refuses such a
/// set as [`ExactPropertyError::MalformedAggregate`]; here the question is
/// only whether the set exists, and it does. Occurrence sets come first, then the sets
/// inherited from the object's `IfcTypeObject`, each in assignment order. A
/// type set is listed even when an occurrence set has the same name:
/// overriding works per property, so both sets exist for the object. A
/// queried `IfcTypeObject` lists its own `HasPropertySets` with
/// [`ExactSource::Type`] of `object` (#193).
///
/// # Errors
///
/// Any [`ExactPropertyError`], as for [`exact_properties_where`]; in
/// particular two selected sets of one name on one source are
/// [`ExactPropertyError::DuplicateMatchingSets`].
pub fn exact_property_sets_where<S>(
    model: &Model,
    object: EntityId,
    mut select_set: S,
) -> Result<Vec<ExactPropertySetEntry>, ExactPropertyError>
where
    S: FnMut(&str) -> bool,
{
    let release = validate_model(model)?;
    let assigned = assigned_sets(model, release, object)?;
    let mut entries = list_sets(
        model,
        release,
        &assigned.occurrence_sets,
        ExactSource::Occurrence,
        &mut select_set,
    )?;
    if let Some((type_id, sets)) = &assigned.type_sets {
        entries.extend(list_sets(
            model,
            release,
            sets,
            ExactSource::Type(*type_id),
            &mut select_set,
        )?);
    }
    Ok(entries)
}

/// The selected sets of one source.
pub(super) fn list_sets(
    model: &Model,
    release: Release,
    sets: &[EntityId],
    source: ExactSource,
    select_set: &mut dyn FnMut(&str) -> bool,
) -> Result<Vec<ExactPropertySetEntry>, ExactPropertyError> {
    let mut entries = Vec::new();
    let mut selected: BTreeMap<&str, EntityId> = BTreeMap::new();
    for &set_id in sets {
        let set = load_source_set(model, release, source, set_id)?;
        if !select_set(set.name) {
            continue;
        }
        if let Some(first) = set.shares_name(&mut selected) {
            return Err(ExactPropertyError::DuplicateMatchingSets {
                source,
                first,
                second: set_id,
            });
        }
        // An empty or unset member list violates `SET [1:?]`, but the set
        // exists and holds nothing, which is what an IDS facet must see.
        let members = if set.member_list_is_empty(release) {
            0
        } else {
            set.members(model, release)?.len()
        };
        entries.push(ExactPropertySetEntry {
            name: Arc::from(set.name),
            set_id,
            source,
            members,
        });
    }
    Ok(entries)
}
