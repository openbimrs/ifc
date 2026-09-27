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
//! - A selected `IfcComplexProperty`, a selected complex quantity, and a
//!   predefined set one of whose own
//!   attributes is selected are refused. An unselected member must be well
//!   formed but need not have a supported value form.
//!
//! [`ExactResolution::Absent`]: super::ExactResolution::Absent
//! [`exact_property`]: super::exact_property

use std::{collections::BTreeMap, sync::Arc};

use ifc_model::{EntityId, Model};

use super::assignment::assigned_sets;
use super::quantity::{predefined_attributes, predefined_name, quantity_members, quantity_value};
use super::refs::text_at;
use super::release::{validate_model, Release};
use super::set::{exact, load_set, property_members, property_value, SetKind};
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

/// Every property and simple quantity of `object`, resolved exactly.
///
/// Equivalent to [`exact_properties_where`] selecting every set and every
/// property, so any assigned `IfcComplexProperty`, any complex quantity and
/// any predefined property set with attributes of its own is refused. Every
/// `IfcSimpleProperty` kind resolves, as for
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
/// its values in attributes this resolver does not read. It is refused with
/// [`ExactPropertyError::UnsupportedDefinition`] when `select_property`
/// picks one of the attributes it declares itself and its `Name` is picked
/// by `select_set` or not stated; otherwise it is skipped.
///
/// The result is in assignment order: occurrence sets first, then the sets
/// inherited from the object's `IfcTypeObject`, each set's members in file
/// order. An inherited property is left out when an occurrence set of the
/// same name has a selected property of the same name: the occurrence
/// value overrides it at property level. An empty result is a proven
/// absence of every selected property.
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
struct Selector<'a> {
    set: &'a mut dyn FnMut(&str) -> bool,
    property: &'a mut dyn FnMut(&str) -> bool,
}

/// The selected properties of the selected sets of one source.
fn collect(
    model: &Model,
    release: Release,
    sets: &[EntityId],
    source: ExactSource,
    select: &mut Selector<'_>,
) -> Result<Vec<ExactPropertyEntry>, ExactPropertyError> {
    let mut entries = Vec::new();
    let mut selected_sets: BTreeMap<&str, EntityId> = BTreeMap::new();
    for &set_id in sets {
        let (set, kind) = load_set(model, release, set_id)?;
        if let SetKind::Predefined = kind {
            // An unnamed set cannot be ruled out by name, as in
            // `predefined_may_hold`.
            let named_out = predefined_name(set).is_some_and(|name| !(select.set)(name));
            if !named_out && predefined_attributes(release, set).any(|a| (select.property)(a)) {
                return Err(ExactPropertyError::UnsupportedDefinition {
                    entity: set_id,
                    type_name: set.type_name.clone(),
                });
            }
            continue;
        }
        let set_name = text_at(set_id, set.attributes.get(2), "Name")?;
        if !(select.set)(set_name) {
            continue;
        }
        if let Some(first) = selected_sets.insert(set_name, set_id) {
            return Err(ExactPropertyError::DuplicateMatchingSets {
                source,
                first,
                second: set_id,
            });
        }
        let members = match kind {
            SetKind::Quantities => quantity_members(model, release, set_id, set)?,
            _ => property_members(model, release, set_id, set)?,
        };
        let mut chosen: BTreeMap<&str, EntityId> = BTreeMap::new();
        let mut in_order = Vec::new();
        for (member_id, name) in members {
            if !(select.property)(name) {
                continue;
            }
            if let Some(first) = chosen.insert(name, member_id) {
                return Err(ExactPropertyError::DuplicateMatchingProperties {
                    set: set_id,
                    first,
                    second: member_id,
                });
            }
            in_order.push((member_id, name));
        }
        for (member_id, name) in in_order {
            let resolved = match kind {
                SetKind::Quantities => quantity_value(model, release, member_id)?,
                _ => property_value(model, release, member_id)?,
            };
            entries.push(ExactPropertyEntry {
                name: Arc::from(name),
                property: exact(source, set_name, set_id, member_id, resolved),
            });
        }
    }
    Ok(entries)
}
