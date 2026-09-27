//! Which slots the schema derives for an entity.
//!
//! A subtype may redeclare an inherited explicit attribute in its `DERIVE`
//! block (`IfcSIUnit` derives `IfcNamedUnit.Dimensions`). The slot keeps its
//! position but Part 21 writes it as `*`. The redeclaration can sit on the
//! entity itself or on any supertype between it and the attribute's owner.

use ifc_schema::Schema;

/// Whether `attribute` is derived for instances of `entity`.
pub(crate) fn is_derived_slot(schema: &Schema, entity: &str, attribute: &str) -> bool {
    schema
        .entity(entity)
        .is_some_and(|definition| definition.is_derived(attribute))
        || schema
            .supertypes(entity)
            .iter()
            .filter_map(|name| schema.entity(name))
            .any(|definition| definition.is_derived(attribute))
}
