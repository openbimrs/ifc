//! Resolving which quantity sets apply to an object.
//!
//! The quantity counterpart of `properties_of` / `resolved_properties`, with
//! the same precedence rule (`assignment.rs`): an occurrence inherits the
//! `IfcElementQuantity` sets of its type, and a same-named set on the
//! occurrence wins, the type's being kept as shadowed.
//!
//! # Schema facts
//!
//! `IfcTypeObject.HasPropertySets` holds `IfcPropertySetDefinition`s, and
//! `IfcElementQuantity` is one in every release, so a type states quantity
//! sets in the same attribute as property sets. An occurrence takes them
//! through `IfcRelDefinesByProperties`. No release forbids two same-named
//! quantity sets on one owner (`UniquePropertySetNames` counts only
//! `IfcPropertySet`), so the view is keyed by name and reports the repeat as
//! [`PropertyAnomaly::DuplicateSetName`].
//!
//! # Precedence is per set
//!
//! The winner is chosen per set by name, not per quantity. An occurrence set
//! `Qto_WallBaseQuantities` hides the type's set of that name completely: a
//! quantity only the type states is not inherited through it. The exact path
//! (`exact_properties_where`) resolves per property instead. Property sets
//! and quantity sets are separate namespaces here: a property set and a
//! quantity set of one name never shadow each other (#546).
//!
//! # Units
//!
//! The view does not resolve units. A [`Quantity`] carries the unit it
//! states, if any; the effective unit is
//! `exact_unit(model, kind.measure_type(), unit)`, which falls back to the
//! project unit when none is stated. [`stated_unit`](crate::stated_unit)
//! answers only for what the quantity states.
//!
//! # Cost
//!
//! [`quantities_of`] scans the whole model for one object. Use
//! [`resolved_quantities`] when asking about many.

use std::collections::BTreeMap;

use ifc_model::{EntityId, Model};

use crate::error::PropertyAnomaly;
use crate::quantity::{quantity_sets_by_object, Quantity, QuantitySet};
use crate::query::assignment::{resolve, NamedSet, Source};

impl NamedSet for QuantitySet {
    fn set_id(&self) -> EntityId {
        self.id
    }

    fn set_name(&self) -> Option<&str> {
        self.name.as_deref()
    }
}

/// A quantity set that applies to an object, with its provenance.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct ResolvedQuantitySet {
    /// Where it came from.
    pub source: Source,
    /// The set itself.
    pub set: QuantitySet,
    /// A same-named set this one overrode, if any.
    ///
    /// Populated when an occurrence set shadows a type set, so a caller can
    /// explain why a quantity differs from the type default.
    pub shadowed: Option<Box<ResolvedQuantitySet>>,
}

/// Resolved quantity sets per object, after precedence is applied.
pub type ResolvedQuantities = BTreeMap<EntityId, Vec<ResolvedQuantitySet>>;

/// Every quantity set applying to every object, precedence applied.
///
/// Sets are ordered by name (an unnamed one by `#<id>`), so output is
/// deterministic. An object appears only with at least one set. A type
/// object gets only the sets attached to it by the forbidden relationship
/// (reported as [`PropertyAnomaly::TypeAttachedByRelationship`]); its own
/// sets are in [`quantity_sets_by_object`](crate::quantity_sets_by_object).
///
/// Anomalies are those of `quantity_sets_by_object`, those of resolving the
/// type assignments, and [`PropertyAnomaly::DuplicateSetName`] for two
/// same-named quantity sets on one owner. `MissingDefinition`, `TypedTwice`
/// and the `IfcRelDefinesByType` anomalies appear in the lists of both this
/// view and the property view.
pub fn resolved_quantities(model: &Model) -> (ResolvedQuantities, Vec<PropertyAnomaly>) {
    let (direct, mut anomalies) = quantity_sets_by_object(model);
    let out = resolve(model, &direct, &mut anomalies)
        .into_iter()
        .map(|(object, sets)| {
            let sets = sets
                .into_iter()
                .map(|p| ResolvedQuantitySet {
                    source: p.source,
                    set: p.set,
                    shadowed: p.shadowed.map(|(source, set)| {
                        Box::new(ResolvedQuantitySet {
                            source,
                            set,
                            shadowed: None,
                        })
                    }),
                })
                .collect();
            (object, sets)
        })
        .collect();
    (out, anomalies)
}

/// Quantity sets applying to one object.
///
/// Scans the whole model; prefer [`resolved_quantities`] for many objects.
pub fn quantities_of(model: &Model, object: EntityId) -> Vec<ResolvedQuantitySet> {
    resolved_quantities(model)
        .0
        .remove(&object)
        .unwrap_or_default()
}

/// Find a single quantity by set name and quantity name.
///
/// Returns the quantity from the occurrence's set of that name when the
/// occurrence states one, else from the type's. A quantity absent from the
/// winning set is `None` even if the shadowed type set has it.
pub fn quantity_value<'a>(
    resolved: &'a [ResolvedQuantitySet],
    set_name: &str,
    quantity_name: &str,
) -> Option<&'a Quantity> {
    resolved
        .iter()
        .find(|r| r.set.name.as_deref() == Some(set_name))
        .and_then(|r| r.set.quantity(quantity_name))
}
