//! Entities of a type *including its subtypes*.
//!
//! [`Model::ids_of_type`] is exact-type only: asking for `IfcElement` returns
//! nothing, because no instance is ever declared as the abstract supertype.
//! The inheritance tree lives in the schema, and ADR 0003 forbids
//! `ifc-model` and `ifc-schema` from depending on each other, so the join is
//! made here in the orchestration layer.
//!
//! # The caller supplies the schema
//!
//! The subtype tree differs between versions (`IfcBuiltElement` exists only
//! in IFC4X3), so the answer depends on which schema the file uses. This
//! module does not guess it from the header. A file whose header is missing
//! or wrong would otherwise be queried against a silently substituted tree.
//! Resolve the schema once, for example with
//! [`crate::SchemaVersion::from_header_token`] and `ifc_schema::for_version`,
//! and pass it in.

use ifc_model::{EntityId, Model};
use ifc_schema::Schema;

/// Ids of every entity whose type is `type_name` or any subtype of it.
///
/// Results are in the file's original order, the same order
/// [`Model::ids`] yields, so the output is deterministic and diffable.
///
/// Returns an empty list when `schema` does not declare `type_name`. An
/// undeclared name is not treated as its own exact type: a misspelled query
/// such as `IfcWal` should find nothing rather than silently match nothing
/// for a different reason.
///
/// # Cost
///
/// One exact-type index lookup per type in the subtree, then a sort of the
/// matched ids by file position. The subtree for `IfcElement` in IFC4X3 is
/// about 120 types, nearly all of which are absent from a typical file and
/// cost a hash miss.
#[must_use]
pub fn ids_of_type_including_subtypes(
    model: &Model,
    schema: &Schema,
    type_name: &str,
) -> Vec<EntityId> {
    if schema.entity(type_name).is_none() {
        return Vec::new();
    }
    let mut ids: Vec<EntityId> = std::iter::once(type_name)
        .chain(schema.subtypes(type_name))
        .flat_map(|name| model.ids_of_type(name).iter().copied())
        .collect();
    if ids.len() > 1 {
        let position: std::collections::HashMap<EntityId, usize> = model
            .ids()
            .enumerate()
            .map(|(index, id)| (id, index))
            .collect();
        ids.sort_unstable_by_key(|id| position.get(id).copied().unwrap_or(usize::MAX));
    }
    ids
}

#[cfg(test)]
mod tests;
