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
//!
//! # A queried type object (#193)
//!
//! An `IfcTypeObject` of the release (`is_a`, so every subtype: IFC2X3
//! `IfcDoorStyle` as much as IFC4 `IfcWallType`) may be queried too. Its sets
//! are its own `HasPropertySets`, read by the code that reads an occurrence's
//! inherited type sets, so the two cannot diverge. A type object has no other
//! route: IFC2X3 `RelatedObjects` is `SET OF IfcObject`, and IFC4 and IFC4X3
//! add the `NoRelatedTypeObject` rule, whose IFC4 documentation says "the
//! relationship between a IfcTypeObject and a IfcPropertySet is handled
//! through the direct relationship HasPropertySets at IfcTypeObject". A type
//! object in any `IfcRelDefinesByProperties` therefore stays refused with
//! [`ExactPropertyError::InvalidOccurrenceTarget`], also when it is the
//! queried object, rather than ignored. `IfcRelDefinesByType.RelatedObjects`
//! is `SET OF IfcObject` in every release, so a type object has no type.
//!
//! # One scan, many objects (#352)
//!
//! The relationship checks do not depend on the queried object, so they
//! are run once per scan by [`Relations::scan`], which also records which
//! objects each relationship relates. [`assigned_sets`] then answers one
//! object from a scan: the object's own checks, then, in the order the
//! per-object traversal met them, a refused relationship subtype relating
//! it, the first malformed relationship of each kind, and a second type
//! assignment. A scan stops at the first malformed relationship of a kind,
//! so anything it recorded for an object precedes that error in file order,
//! exactly as a traversal for that object alone would have met it.
//!
//! A scan for one object ([`Scope::One`]) records only that object and
//! costs what the per-object traversal cost; [`Scope::All`] records every
//! object once, which is what makes a loop over every object linear
//! ([`super::PropertyIndex`]).

use std::collections::HashMap;
use std::sync::Arc;

use ifc_model::{EntityId, Model};

use super::refs::{nonempty_refs_at, optional_refs_at, property_definition_refs_at, ref_at};
use super::refs::{refs_at, require_ref};
use super::release::Release;
use super::ExactPropertyError;

/// The property-set definitions assigned to one object.
pub(super) struct Assigned {
    /// Definitions related to the occurrence by `IfcRelDefinesByProperties`.
    pub(super) occurrence_sets: Vec<EntityId>,
    /// The object's `IfcTypeObject` and its `HasPropertySets`, if typed; for
    /// a queried type object, the object itself and its own
    /// `HasPropertySets`.
    pub(super) type_sets: Option<(EntityId, Vec<EntityId>)>,
}

/// Which objects a [`Relations::scan`] records assignments for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Scope {
    /// Every related object: one scan answers them all.
    All,
    /// One object, as a single query needs.
    One(EntityId),
}

impl Scope {
    fn wants(self, object: EntityId) -> bool {
        match self {
            Self::All => true,
            Self::One(wanted) => wanted == object,
        }
    }
}

/// Every `IfcRelDefinesByProperties` and `IfcRelDefinesByType` of a model,
/// validated once, and the objects they relate.
///
/// Owns ids only; it reads the model while scanning and borrows nothing
/// afterwards, so whoever holds it must keep it with the model it was
/// scanned from ([`super::PropertyIndex`] does, by borrowing that model).
#[derive(Debug, Clone)]
pub(super) struct Relations {
    scope: Scope,
    /// The first proper subtype of either relationship relating an object
    /// (IFC2X3 `IfcRelOverridesProperties`), recorded before
    /// `subtype_error`.
    subtype_relating: HashMap<EntityId, (EntityId, Arc<str>)>,
    /// The first such subtype whose `RelatedObjects` is malformed.
    subtype_error: Option<ExactPropertyError>,
    /// The first malformed `IfcRelDefinesByProperties`.
    definitions_error: Option<ExactPropertyError>,
    /// Each occurrence's definitions, in relationship and then definition
    /// order.
    occurrence_sets: HashMap<EntityId, Vec<EntityId>>,
    /// The first malformed `IfcRelDefinesByType`.
    typing_error: Option<ExactPropertyError>,
    /// Each typed object's first type and, when a second relationship
    /// relates it, that one's type, recorded before `typing_error`.
    types: HashMap<EntityId, (EntityId, Option<EntityId>)>,
}

impl Relations {
    /// Validate every relationship of both kinds and record the objects
    /// `scope` asks for.
    pub(super) fn scan(model: &Model, release: Release, scope: Scope) -> Self {
        let mut relations = Self {
            scope,
            subtype_relating: HashMap::new(),
            subtype_error: None,
            definitions_error: None,
            occurrence_sets: HashMap::new(),
            typing_error: None,
            types: HashMap::new(),
        };
        relations.subtype_error = relations.scan_subtypes(model, release).err();
        relations.definitions_error = relations.scan_definitions(model, release).err();
        relations.typing_error = relations.scan_types(model, release).err();
        relations
    }

    /// How many objects the scan recorded an assignment for.
    pub(super) fn objects(&self) -> usize {
        let mut objects: Vec<EntityId> = self
            .occurrence_sets
            .keys()
            .chain(self.types.keys())
            .copied()
            .collect();
        objects.sort_unstable();
        objects.dedup();
        objects.len()
    }

    /// Record any proper subtype of the two traversed relationships.
    ///
    /// `Model::ids_of_type` is exact-type, so a subtype instance such as
    /// IFC2X3 `IfcRelOverridesProperties` would otherwise be skipped
    /// silently and its object answered as if the relationship were absent.
    /// Subtypes that relate other objects do not affect that answer.
    fn scan_subtypes(&mut self, model: &Model, release: Release) -> Result<(), ExactPropertyError> {
        for parent in ["IFCRELDEFINESBYPROPERTIES", "IFCRELDEFINESBYTYPE"] {
            for subtype in release.schema.subtypes(parent) {
                for relation_id in model.ids_of_type(subtype) {
                    let r = model.get(*relation_id).expect("type index is current");
                    let related = refs_at(*relation_id, r.attributes.get(4), "RelatedObjects")?;
                    for object in related {
                        if self.scope.wants(object) {
                            self.subtype_relating
                                .entry(object)
                                .or_insert_with(|| (*relation_id, r.type_name.clone()));
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn scan_definitions(
        &mut self,
        model: &Model,
        release: Release,
    ) -> Result<(), ExactPropertyError> {
        let schema = release.schema;
        for relation_id in model.ids_of_type("IFCRELDEFINESBYPROPERTIES") {
            let r = model.get(*relation_id).expect("type index is current");
            release.require_exact_slots(*relation_id, r)?;
            let related = nonempty_refs_at(*relation_id, r.attributes.get(4), "RelatedObjects")?;
            for related_id in &related {
                require_ref(model, *relation_id, *related_id)?;
                let related_object = model.get(*related_id).expect("checked reference");
                if schema.entity(related_object.type_name.as_ref()).is_none() {
                    return Err(
                        release.not_in_schema(*related_id, related_object.type_name.clone())
                    );
                }
                if !occurrence_domain(release, related_object.type_name.as_ref()) {
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
            for &definition in &definitions {
                require_ref(model, *relation_id, definition)?;
                let definition_entity = model.get(definition).expect("checked reference");
                if schema
                    .entity(definition_entity.type_name.as_ref())
                    .is_none()
                {
                    return Err(
                        release.not_in_schema(definition, definition_entity.type_name.clone())
                    );
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
            }
            // `RelatedObjects` holds no duplicates (`refs_at` refuses them),
            // so each object receives the definitions once.
            for related_id in related {
                if self.scope.wants(related_id) {
                    self.occurrence_sets
                        .entry(related_id)
                        .or_default()
                        .extend_from_slice(&definitions);
                }
            }
        }
        Ok(())
    }

    fn scan_types(&mut self, model: &Model, release: Release) -> Result<(), ExactPropertyError> {
        let schema = release.schema;
        for relation_id in model.ids_of_type("IFCRELDEFINESBYTYPE") {
            let r = model.get(*relation_id).expect("type index is current");
            release.require_exact_slots(*relation_id, r)?;
            let related = nonempty_refs_at(*relation_id, r.attributes.get(4), "RelatedObjects")?;
            for related_id in &related {
                require_ref(model, *relation_id, *related_id)?;
                let related_object = model.get(*related_id).expect("checked reference");
                if schema.entity(related_object.type_name.as_ref()).is_none() {
                    return Err(
                        release.not_in_schema(*related_id, related_object.type_name.clone())
                    );
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
            for related_id in related {
                if !self.scope.wants(related_id) {
                    continue;
                }
                match self.types.get_mut(&related_id) {
                    None => {
                        self.types.insert(related_id, (type_id, None));
                    }
                    Some((_, second @ None)) => *second = Some(type_id),
                    Some(_) => {}
                }
            }
        }
        Ok(())
    }

    /// The definitions assigned to `object`, which [`query_target`] has
    /// accepted, from this scan.
    fn assigned(
        &self,
        model: &Model,
        object: EntityId,
        queried_type: bool,
    ) -> Result<Assigned, ExactPropertyError> {
        debug_assert!(self.scope.wants(object), "the scan recorded this object");
        if let Some((relationship, type_name)) = self.subtype_relating.get(&object) {
            return Err(ExactPropertyError::UnsupportedRelationship {
                relationship: *relationship,
                type_name: type_name.clone(),
            });
        }
        if let Some(error) = &self.subtype_error {
            return Err(error.clone());
        }
        if let Some(error) = &self.definitions_error {
            return Err(error.clone());
        }
        let assigned_type = match self.types.get(&object) {
            Some(&(first, Some(second))) => {
                return Err(ExactPropertyError::MultipleTypeAssignments {
                    object,
                    first,
                    second,
                })
            }
            Some(&(first, None)) => Some(first),
            None => None,
        };
        if let Some(error) = &self.typing_error {
            return Err(error.clone());
        }
        let occurrence_sets = self
            .occurrence_sets
            .get(&object)
            .cloned()
            .unwrap_or_default();
        // A queried type object met in `RelatedObjects` of either
        // relationship was refused by the scan, so it has no occurrence sets
        // and no type.
        debug_assert!(!queried_type || (occurrence_sets.is_empty() && assigned_type.is_none()));
        let holder = if queried_type {
            Some(object)
        } else {
            assigned_type
        };
        let type_sets = match holder {
            Some(type_id) => {
                // Checked already, for the assigned type and the queried
                // object alike: a known `IfcTypeObject` with the release's
                // arity, so `HasPropertySets` is slot 5 in every bundled
                // release.
                let type_object = model.get(type_id).expect("checked reference");
                let sets =
                    optional_refs_at(type_id, type_object.attributes.get(5), "HasPropertySets")?;
                Some((type_id, sets))
            }
            None => None,
        };
        Ok(Assigned {
            occurrence_sets,
            type_sets,
        })
    }
}

/// The occurrence domain is the release's own `RelatedObjects` type:
/// `IfcObject` in IFC2X3, `IfcObjectDefinition` in IFC4 and IFC4X3. Type
/// objects are never occurrences, whatever the release.
fn occurrence_domain(release: Release, type_name: &str) -> bool {
    release.slot_accepts("IFCRELDEFINESBYPROPERTIES", 4, type_name)
        && !release.schema.is_a(type_name, "IFCTYPEOBJECT")
}

/// Validate `object` as a query target: whether it is a type object, which
/// is queried for its own `HasPropertySets` (#193).
fn query_target(
    model: &Model,
    release: Release,
    object: EntityId,
) -> Result<bool, ExactPropertyError> {
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
    let queried_type = schema.is_a(query_entity.type_name.as_ref(), "IFCTYPEOBJECT");
    if !queried_type && !occurrence_domain(release, query_entity.type_name.as_ref()) {
        return Err(ExactPropertyError::InvalidQueryObject {
            object,
            type_name: query_entity.type_name.clone(),
        });
    }
    release.require_exact_slots(object, query_entity)?;
    Ok(queried_type)
}

/// Validate `object` as a query target and collect every definition
/// assigned to it, directly or through its type.
///
/// `relations` is a scan of the whole model ([`Scope::All`]); without one,
/// the relationships are scanned for `object` alone.
///
/// # Errors
///
/// Any structural [`ExactPropertyError`] of the object, of any
/// `IfcRelDefinesByProperties`/`IfcRelDefinesByType` in the file, or of the
/// assigned type object.
pub(super) fn assigned_sets(
    model: &Model,
    release: Release,
    relations: Option<&Relations>,
    object: EntityId,
) -> Result<Assigned, ExactPropertyError> {
    let queried_type = query_target(model, release, object)?;
    match relations {
        Some(relations) => relations.assigned(model, object, queried_type),
        None => Relations::scan(model, release, Scope::One(object)).assigned(
            model,
            object,
            queried_type,
        ),
    }
}
