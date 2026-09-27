//! Which property-set definitions reach a queried object, read exactly.
//!
//! ```text
//! IfcRelDefinesByProperties  4 = RelatedObjects  5 = RelatingPropertyDefinition
//! IfcRelDefinesByType        4 = RelatedObjects  5 = RelatingType
//! IfcTypeObject              5 = HasPropertySets
//! ```
//!
//! The positions are the same in IFC2X3 TC1, IFC4 ADD2 TC1 and IFC4X3 ADD2
//! (`IfcRoot` contributes four attributes; `IfcTypeObject` adds
//! `ApplicableOccurrence` before `HasPropertySets`). The domains are not:
//! IFC2X3 inherits `RelatedObjects : SET OF IfcObject` from `IfcRelDefines`,
//! while IFC4 and IFC4X3 declare `SET [1:?] OF IfcObjectDefinition` on
//! `IfcRelDefinesByProperties` with the `NoRelatedTypeObject` rule. They are
//! read from the bound release's table, never assumed.
//!
//! Every relationship in the file is validated, not only those relating the
//! queried object: a malformed one could otherwise hide an assignment.

use ifc_model::{EntityId, Model};

use super::refs::{nonempty_refs_at, optional_refs_at, property_definition_refs_at, ref_at};
use super::refs::{refs_at, require_ref};
use super::release::Release;
use super::ExactPropertyError;

/// The property-set definitions assigned to one object.
pub(super) struct Assigned {
    /// Definitions related to the occurrence by `IfcRelDefinesByProperties`.
    pub(super) occurrence_sets: Vec<EntityId>,
    /// The object's `IfcTypeObject` and its `HasPropertySets`, if typed.
    pub(super) type_sets: Option<(EntityId, Vec<EntityId>)>,
}

/// Validate `object` as a query target and collect every definition
/// assigned to it, directly or through its type.
///
/// # Errors
///
/// Any structural [`ExactPropertyError`] of the object, of any
/// `IfcRelDefinesByProperties`/`IfcRelDefinesByType` in the file, or of the
/// assigned type object.
pub(super) fn assigned_sets(
    model: &Model,
    release: Release,
    object: EntityId,
) -> Result<Assigned, ExactPropertyError> {
    let schema = release.schema;
    let query_entity = model
        .get(object)
        .ok_or(ExactPropertyError::MissingReference {
            from: object,
            to: object,
        })?;
    if schema.entity(query_entity.type_name.as_ref()).is_none() {
        return Err(release.not_in_schema(object, query_entity.type_name.clone()));
    }
    // The occurrence domain is the release's own `RelatedObjects` type:
    // `IfcObject` in IFC2X3, `IfcObjectDefinition` in IFC4 and IFC4X3. Type
    // objects are never occurrences, whatever the release.
    let occurrence_domain = |type_name: &str| {
        release.slot_accepts("IFCRELDEFINESBYPROPERTIES", 4, type_name)
            && !schema.is_a(type_name, "IFCTYPEOBJECT")
    };
    if !occurrence_domain(query_entity.type_name.as_ref()) {
        return Err(ExactPropertyError::InvalidQueryObject {
            object,
            type_name: query_entity.type_name.clone(),
        });
    }
    release.require_exact_slots(object, query_entity)?;
    refuse_unsupported_relationships(model, release, object)?;
    let mut occurrence_sets = Vec::new();
    let mut assigned_type = None;
    for relation_id in model.ids_of_type("IFCRELDEFINESBYPROPERTIES") {
        let r = model.get(*relation_id).expect("type index is current");
        release.require_exact_slots(*relation_id, r)?;
        let related = nonempty_refs_at(*relation_id, r.attributes.get(4), "RelatedObjects")?;
        for related_id in &related {
            require_ref(model, *relation_id, *related_id)?;
            let related_object = model.get(*related_id).expect("checked reference");
            if schema.entity(related_object.type_name.as_ref()).is_none() {
                return Err(release.not_in_schema(*related_id, related_object.type_name.clone()));
            }
            if !occurrence_domain(related_object.type_name.as_ref()) {
                return Err(ExactPropertyError::InvalidOccurrenceTarget {
                    relationship: *relation_id,
                    object: *related_id,
                });
            }
            release.require_exact_slots(*related_id, related_object)?;
        }
        let definitions = property_definition_refs_at(
            release,
            *relation_id,
            r.attributes.get(5),
            "RelatingPropertyDefinition",
        )?;
        for definition in definitions {
            require_ref(model, *relation_id, definition)?;
            let definition_entity = model.get(definition).expect("checked reference");
            if schema
                .entity(definition_entity.type_name.as_ref())
                .is_none()
            {
                return Err(release.not_in_schema(definition, definition_entity.type_name.clone()));
            }
            if !schema.is_a(
                definition_entity.type_name.as_ref(),
                "IFCPROPERTYSETDEFINITION",
            ) {
                return Err(ExactPropertyError::UnsupportedDefinition {
                    entity: definition,
                    type_name: definition_entity.type_name.clone(),
                });
            }
            if related.contains(&object) {
                occurrence_sets.push(definition);
            }
        }
    }
    for relation_id in model.ids_of_type("IFCRELDEFINESBYTYPE") {
        let r = model.get(*relation_id).expect("type index is current");
        release.require_exact_slots(*relation_id, r)?;
        let related = nonempty_refs_at(*relation_id, r.attributes.get(4), "RelatedObjects")?;
        for related_id in &related {
            require_ref(model, *relation_id, *related_id)?;
            let related_object = model.get(*related_id).expect("checked reference");
            if schema.entity(related_object.type_name.as_ref()).is_none() {
                return Err(release.not_in_schema(*related_id, related_object.type_name.clone()));
            }
            if !schema.is_a(related_object.type_name.as_ref(), "IFCOBJECT") {
                return Err(ExactPropertyError::InvalidTypeTarget {
                    relationship: *relation_id,
                    object: *related_id,
                });
            }
            release.require_exact_slots(*related_id, related_object)?;
        }
        let type_id = ref_at(*relation_id, r.attributes.get(5), "RelatingType")?;
        require_ref(model, *relation_id, type_id)?;
        let type_object = model.get(type_id).expect("checked reference");
        if schema.entity(type_object.type_name.as_ref()).is_none() {
            return Err(release.not_in_schema(type_id, type_object.type_name.clone()));
        }
        if !schema.is_a(type_object.type_name.as_ref(), "IFCTYPEOBJECT") {
            return Err(ExactPropertyError::UnsupportedDefinition {
                entity: type_id,
                type_name: type_object.type_name.clone(),
            });
        }
        release.require_exact_slots(type_id, type_object)?;
        if related.contains(&object) {
            if let Some(first) = assigned_type.replace(type_id) {
                return Err(ExactPropertyError::MultipleTypeAssignments {
                    object,
                    first,
                    second: type_id,
                });
            }
        }
    }
    let type_sets = match assigned_type {
        Some(type_id) => {
            // Checked above: a known `IfcTypeObject` with the release's
            // arity, so `HasPropertySets` is slot 5 in every bundled release.
            let type_object = model.get(type_id).expect("checked reference");
            let sets = optional_refs_at(type_id, type_object.attributes.get(5), "HasPropertySets")?;
            Some((type_id, sets))
        }
        None => None,
    };
    Ok(Assigned {
        occurrence_sets,
        type_sets,
    })
}

/// Refuse any proper subtype of the two traversed relationships that
/// relates `object`.
///
/// `Model::ids_of_type` is exact-type, so a subtype instance such as IFC2X3
/// `IfcRelOverridesProperties` would otherwise be skipped silently and its
/// object answered as if the relationship were absent. Subtypes that relate
/// other objects do not affect this answer and are left alone.
fn refuse_unsupported_relationships(
    model: &Model,
    release: Release,
    object: EntityId,
) -> Result<(), ExactPropertyError> {
    for parent in ["IFCRELDEFINESBYPROPERTIES", "IFCRELDEFINESBYTYPE"] {
        for subtype in release.schema.subtypes(parent) {
            for relation_id in model.ids_of_type(subtype) {
                let r = model.get(*relation_id).expect("type index is current");
                let related = refs_at(*relation_id, r.attributes.get(4), "RelatedObjects")?;
                if related.contains(&object) {
                    return Err(ExactPropertyError::UnsupportedRelationship {
                        relationship: *relation_id,
                        type_name: r.type_name.clone(),
                    });
                }
            }
        }
    }
    Ok(())
}
