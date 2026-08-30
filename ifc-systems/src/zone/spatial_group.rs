//! Where an element sits spatially, and where it is merely referenced.
//!
//! # Two relationships that are not interchangeable
//!
//! ```text
//! IfcRelContainedInSpatialStructure   4 = RelatedElements  5 = RelatingStructure
//! IfcRelReferencedInSpatialStructure  4 = RelatedElements  5 = RelatingStructure
//! ```
//!
//! Same slot layout, different cardinality and different meaning:
//!
//! - `ContainedInStructure` is `SET [0:1]`: an element has AT MOST ONE
//!   containing structure. This is its home.
//! - `ReferencedInStructures` is `SET [0:?]`: an element may be referenced by
//!   MANY structures. A duct passing through five rooms is referenced by all
//!   five and contained by none of them.
//!
//! Merging the two loses the distinction that makes the second useful. A
//! caller asking "which storey owns this pump" wants containment; a caller
//! asking "which rooms does this duct serve" wants references. They are kept
//! apart.

use std::collections::{BTreeMap, BTreeSet};

use ifc_model::{EntityId, Model, Value};

use crate::error::SystemAnomaly;

fn refs(value: Option<&Value>) -> Vec<EntityId> {
    match value {
        Some(Value::List(items)) => items
            .iter()
            .filter_map(|v| match v {
                Value::Ref(id) => Some(*id),
                _ => None,
            })
            .collect(),
        Some(Value::Ref(id)) => vec![*id],
        _ => Vec::new(),
    }
}

mod slot {
    /// Both spatial relationships put elements at 4 and the structure at 5.
    pub const RELATED_ELEMENTS: usize = 4;
    pub const RELATING_STRUCTURE: usize = 5;
}

/// Where one element sits in the spatial structure.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SpatialPlacement {
    /// The single containing structure, if the file states one.
    ///
    /// `Option`, not `Vec`, because the schema says `SET [0:1]`. A file
    /// stating two is malformed and reports `ContainedTwice`.
    pub contained_in: Option<EntityId>,
    /// Structures that reference this element, ascending by id.
    ///
    /// Unbounded by the schema, so a `Vec` is the honest shape.
    pub referenced_in: Vec<EntityId>,
}

impl SpatialPlacement {
    /// Every structure this element relates to, containment first.
    ///
    /// Convenience for callers that genuinely do not care which mechanism
    /// stated the link -- but they must opt in, rather than the reader
    /// flattening the distinction for everyone.
    pub fn all_structures(&self) -> Vec<EntityId> {
        let mut out: Vec<_> = self.contained_in.into_iter().collect();
        out.extend(self.referenced_in.iter().copied());
        out.dedup();
        out
    }
}

/// Read spatial containment and referencing for every element that has either.
///
/// Elements with no spatial relationship are absent from the map rather than
/// present-and-empty: absence is the file's actual statement.
pub fn spatial_placements(
    model: &Model,
) -> (BTreeMap<EntityId, SpatialPlacement>, Vec<SystemAnomaly>) {
    let mut out: BTreeMap<EntityId, SpatialPlacement> = BTreeMap::new();
    let mut anomalies = Vec::new();

    // Containment: at most one per element, so a second is an anomaly.
    for &relation in model.ids_of_type("IFCRELCONTAINEDINSPATIALSTRUCTURE") {
        let Some(entity) = model.get(relation) else {
            continue;
        };
        let structure = match entity.attributes.get(slot::RELATING_STRUCTURE) {
            Some(Value::Ref(id)) => *id,
            _ => continue,
        };
        for element in refs(entity.attributes.get(slot::RELATED_ELEMENTS)) {
            let slot_entry = out.entry(element).or_default();
            match slot_entry.contained_in {
                Some(existing) if existing != structure => {
                    // IfcElement.ContainedInStructure is SET [0:1]; two
                    // different homes cannot both be true. Keep the first by
                    // id order so the result is deterministic, and say so.
                    anomalies.push(SystemAnomaly::ContainedTwice {
                        element,
                        first: existing,
                        second: structure,
                    });
                }
                Some(_) => {}
                None => slot_entry.contained_in = Some(structure),
            }
        }
    }

    // Referencing: many per element is legal and expected.
    let mut references: BTreeMap<EntityId, BTreeSet<EntityId>> = BTreeMap::new();
    for &relation in model.ids_of_type("IFCRELREFERENCEDINSPATIALSTRUCTURE") {
        let Some(entity) = model.get(relation) else {
            continue;
        };
        let structure = match entity.attributes.get(slot::RELATING_STRUCTURE) {
            Some(Value::Ref(id)) => *id,
            _ => continue,
        };
        for element in refs(entity.attributes.get(slot::RELATED_ELEMENTS)) {
            references.entry(element).or_default().insert(structure);
        }
    }
    for (element, structures) in references {
        out.entry(element).or_default().referenced_in = structures.into_iter().collect();
    }

    (out, anomalies)
}
