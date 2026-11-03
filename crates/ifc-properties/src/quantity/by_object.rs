//! Quantity sets by the object that carries them.
//!
//! The same two routes as property sets (`IfcRelDefinesByProperties` for an
//! occurrence, `IfcTypeObject.HasPropertySets` for a type), followed by the
//! traversal in `pset/attach.rs`. A quantity set is an `IfcElementQuantity`,
//! an `IfcPropertySetDefinition` like `IfcPropertySet`, so a type holds both
//! kinds in one attribute; each view reads only its own kind.

use std::collections::BTreeMap;

use ifc_model::{EntityId, Model};

use crate::error::PropertyAnomaly;
use crate::pset::attach::sets_by_object;
use crate::pset::Attachment;
use crate::quantity::set::{quantity_set, QuantitySet};

/// Quantity sets found on each object, with how each one was attached.
pub type AttachedQuantitySets = BTreeMap<EntityId, Vec<(Attachment, QuantitySet)>>;

/// Every quantity set in the file, keyed by the object carrying it.
///
/// Both routes are followed. A set reachable by both appears once per object,
/// with [`Attachment`] recording how it arrived: precedence is a caller
/// decision, so the reader must not collapse the distinction here. Sets of
/// one object are ordered by id. Property sets are never returned here, and
/// quantity sets are never returned by
/// [`property_sets_by_object`](crate::property_sets_by_object).
///
/// Anomalies report a type object attached by the forbidden relationship,
/// relationships whose targets are missing from the file, the members
/// [`quantity_set`] could not resolve (once per set), and, once per set, each
/// top-level quantity whose name an earlier one in the same set already has
/// ([`PropertyAnomaly::DuplicatePropertyName`]).
pub fn quantity_sets_by_object(model: &Model) -> (AttachedQuantitySets, Vec<PropertyAnomaly>) {
    let (out, mut anomalies) = sets_by_object(model, quantity_set, |set: &QuantitySet| set.id);
    // A set shared by many objects is checked once.
    let mut unique: BTreeMap<EntityId, &QuantitySet> = BTreeMap::new();
    for (_, set) in out.values().flatten() {
        unique.entry(set.id).or_insert(set);
    }
    for set in unique.values() {
        duplicate_quantity_names(set, &mut anomalies);
    }
    (out, anomalies)
}

/// `UniqueQuantityNames` (IFC4 and later): report each quantity whose name an
/// earlier one in the same set already has.
fn duplicate_quantity_names(set: &QuantitySet, anomalies: &mut Vec<PropertyAnomaly>) {
    let mut first: BTreeMap<&str, EntityId> = BTreeMap::new();
    for quantity in &set.quantities {
        let Some(name) = quantity.name() else {
            continue;
        };
        match first.get(name) {
            Some(&kept) => anomalies.push(PropertyAnomaly::DuplicatePropertyName {
                set: set.id,
                kept,
                rejected: quantity.id(),
            }),
            None => {
                first.insert(name, quantity.id());
            }
        }
    }
}
