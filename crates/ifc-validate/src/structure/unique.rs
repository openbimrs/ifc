//! Duplicate `GlobalId`s.
//!
//! # Why this is a real defect and not pedantry
//!
//! An IFC `GlobalId` is a 128-bit UUID in a compressed base64 spelling. It is
//! the only stable cross-file identity an object has: change tracking, issue
//! references, and federation all key on it. Two objects sharing one is not a
//! cosmetic problem -- it makes "which wall did the clash report mean?"
//! unanswerable.
//!
//! IFC4 states this as a global rule (`IfcSingleProjectInstance` is the other
//! one). It is checked here rather than in `where_rule` because it needs a
//! whole-file index, not a per-entity predicate.

use std::collections::BTreeMap;

use ifc_model::{EntityId, Model};
use ifc_schema::Schema;

use crate::report::{Finding, Path, Report};

/// Reports every `GlobalId` held by more than one entity.
///
/// Only entities the schema says are `IfcRoot` subtypes carry a `GlobalId`,
/// and it is always slot 0. Both facts come from the schema rather than from
/// the entity name, so a file using an unknown subtype of `IfcRoot` is still
/// checked.
pub fn duplicate_global_ids(model: &Model, schema: &Schema, report: &mut Report) {
    let mut seen: BTreeMap<&str, Vec<EntityId>> = BTreeMap::new();
    for (id, entity) in model.iter() {
        if !schema.is_a(&entity.type_name, "IfcRoot") {
            continue;
        }
        let Some(guid) = entity.text(0) else { continue };
        seen.entry(guid).or_default().push(id);
    }
    for (guid, mut holders) in seen {
        if holders.len() < 2 {
            continue;
        }
        holders.sort_unstable();
        // Report against every holder after the first: the first occurrence is
        // not itself wrong, and blaming it would make the finding order depend
        // on which entity happened to be read first.
        for duplicate in &holders[1..] {
            report.push(Finding::error(
                "structure.unique.duplicate_global_id",
                Path::Entity(*duplicate),
                format!("GlobalId {guid} is already used by {}", holders[0]),
            ));
        }
    }
}
