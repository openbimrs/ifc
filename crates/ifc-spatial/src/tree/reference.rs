//! Elements a spatial structure references without containing them.
//!
//! # Containment is not the only link
//!
//! `IfcRelContainedInSpatialStructure` places an element in exactly one
//! structure: `IfcElement.ContainedInStructure` is `SET [0:1]` in IFC2X3 TC1,
//! IFC4 ADD2 TC1 and IFC4X3 ADD2. `IfcRelReferencedInSpatialStructure`
//! (IFC2x3 onwards) additionally assigns an element to "those levels of the
//! project spatial structure, in which they are referenced, but not
//! primarily contained"; an element "can be referenced to zero, one or
//! several levels" (IFC4 ADD2 TC1 documentation). The IFC4 example is a
//! curtain wall contained by the ground floor and referenced by the storeys
//! above it.
//!
//! The two are kept apart: [`SpatialTree::elements_of`] answers containment
//! only, and [`SpatialTree::referenced_elements`] answers references only.
//! Merging them would make a multi-storey element look contained twice,
//! which the schema forbids.

use std::collections::BTreeMap;

use ifc_model::{EntityId, Model};

use super::build::SpatialTree;
use crate::relation::{link, slots};

impl SpatialTree {
    /// Record every `IfcRelReferencedInSpatialStructure`.
    ///
    /// A relationship naming an absent entity is reported through
    /// `dangling`, and one whose structure is not a spatial container
    /// through `anomalies`, as containment reports both.
    pub(super) fn apply_references(&mut self, model: &Model) {
        let rel = slots::REFERENCED_IN;
        for &relationship in model.ids_of_type(rel.type_name) {
            let Some(structure) = link::refs_in_slot(model, relationship, rel.relating)
                .into_iter()
                .next()
            else {
                continue;
            };
            if model.get(structure).is_none() {
                self.dangling.push((relationship, structure));
                continue;
            }
            if self.node(structure).is_none() {
                self.anomalies
                    .push(super::SpatialAnomaly::ReferencedInNonContainer {
                        relation: relationship,
                        structure,
                    });
                continue;
            }
            for element in link::refs_in_slot(model, relationship, rel.related) {
                if model.get(element).is_none() {
                    self.dangling.push((relationship, element));
                    continue;
                }
                push_unique(&mut self.referenced, structure, element);
                push_unique(&mut self.referenced_in, element, structure);
            }
        }
    }

    /// Elements `IfcRelReferencedInSpatialStructure` references in
    /// `container`, in file order, each once.
    ///
    /// Disjoint in meaning from [`elements_of`](Self::elements_of): a
    /// referenced element is contained elsewhere (or nowhere). An element
    /// the file both contains and references in one container appears in
    /// both lists, because the file states both.
    #[must_use]
    pub fn referenced_elements(&self, container: EntityId) -> &[EntityId] {
        self.referenced.get(&container).map_or(&[], Vec::as_slice)
    }

    /// Containers that reference `element`, in file order, each once.
    ///
    /// Empty for an element that is only contained; its container is
    /// [`container_of`](Self::container_of).
    #[must_use]
    pub fn referencing_structures(&self, element: EntityId) -> &[EntityId] {
        self.referenced_in.get(&element).map_or(&[], Vec::as_slice)
    }
}

/// Append `value` to `key`'s list unless it is already there.
fn push_unique(map: &mut BTreeMap<EntityId, Vec<EntityId>>, key: EntityId, value: EntityId) {
    let list = map.entry(key).or_default();
    if !list.contains(&value) {
        list.push(value);
    }
}
