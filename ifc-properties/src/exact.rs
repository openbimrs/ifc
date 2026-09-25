//! Exact, fail-closed IFC2X3/IFC4 property resolution.
//!
//! Unlike the permissive views, this traversal rejects any incomplete or
//! malformed assignment data before it can claim an exact absence. Every
//! structural fact comes from the table of the one release the header
//! declares; nothing is aliased across releases.

mod refs;
mod release;
mod value;

use std::{collections::BTreeMap, fmt, sync::Arc};

use ifc_model::{EntityId, Model};
use ifc_schema::SchemaVersion;

use refs::{
    nonempty_refs_at, optional_refs_at, property_definition_refs_at, ref_at, refs_at, require_ref,
    text_at,
};
use release::{validate_model, Release};
use value::{exact_property_value, ResolvedValue};

/// Provenance of an exact result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ExactSource {
    /// Resolved from a property set assigned directly to the occurrence
    /// via `IfcRelDefinesByProperties`.
    Occurrence,
    /// Resolved from a property set inherited through the occurrence's
    /// `IfcTypeObject`, identified by that type's entity id.
    Type(EntityId),
}

/// Exact IFC logical value without collapsing unknown into a boolean.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExactLogical {
    /// `IfcLogical` value `.F.`.
    False,
    /// `IfcLogical` value `.U.` — genuinely undetermined, not absent.
    Unknown,
    /// `IfcLogical` value `.T.`.
    True,
}

/// Scalar values accepted by the exact resolver.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ExactValue {
    /// The property carried no value (`$`), distinct from an absent property.
    Null,
    /// An `IFCBOOLEAN` payload.
    Bool(bool),
    /// An `IFCLOGICAL` payload, keeping the tri-state distinction.
    Logical(ExactLogical),
    /// An `IFCBINARY` payload, stored as its literal encoded text.
    Binary(Arc<str>),
    /// An `IFCINTEGER` payload.
    Integer(i64),
    /// An `IFCREAL` (or compatible measure) payload; always finite.
    Real(f64),
    /// An `IFCTEXT`/`IFCLABEL`/`IFCIDENTIFIER`-family string payload.
    Text(Arc<str>),
}

/// A uniquely resolved property with IFC identity and provenance.
#[derive(Debug, Clone, PartialEq)]
pub struct ExactProperty {
    /// Whether the value came from the occurrence or was inherited from its type.
    pub source: ExactSource,
    /// The owning `IfcPropertySet.Name`.
    pub property_set: Arc<str>,
    /// Entity id of the `IfcPropertySet`.
    pub set_id: EntityId,
    /// Entity id of the `IfcPropertySingleValue`.
    pub property_id: EntityId,
    /// Declared IFC value type (for example `IFCINTEGER` or `IFCLENGTHMEASURE`).
    pub value_type: Option<Arc<str>>,
    /// Explicit `IfcPropertySingleValue.Unit`, if stated.
    pub unit_id: Option<EntityId>,
    /// The resolved `NominalValue`.
    pub value: ExactValue,
}

/// Exact lookup result.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ExactResolution {
    /// The property was found exactly once across occurrence and inherited sets.
    Present(ExactProperty),
    /// No occurrence or inherited property set carried a matching property;
    /// this is a proven absence, not a lookup failure.
    Absent,
}

/// Why a property cannot be resolved exactly.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ExactPropertyError {
    /// The model carries STEP-level diagnostics, so exactness cannot be
    /// guaranteed; count of diagnostics is reported for context.
    IncompleteModel {
        /// Number of diagnostics recorded against the model.
        diagnostics: usize,
    },
    /// The model header declares no `FILE_SCHEMA`.
    MissingSchema,
    /// The model header declares more than one schema.
    MultipleSchemas {
        /// Number of schemas declared in the header.
        schemas: usize,
    },
    /// The model header declares a schema other than IFC2X3 or IFC4.
    UnsupportedSchema {
        /// The declared schema token.
        schema: String,
    },
    /// An entity reference points at an id absent from the model.
    MissingReference {
        /// The entity holding the reference.
        from: EntityId,
        /// The entity id that could not be resolved.
        to: EntityId,
    },
    /// An aggregate attribute (list/set) was `$`, empty when required, or
    /// not encoded as a list at all.
    MalformedAggregate {
        /// The entity whose attribute was malformed.
        entity: EntityId,
        /// Name of the offending attribute.
        attribute: &'static str,
    },
    /// The occurrence is assigned to more than one `IfcTypeObject` via
    /// `IfcRelDefinesByType`, which IFC forbids.
    MultipleTypeAssignments {
        /// The occurrence with conflicting type assignments.
        object: EntityId,
        /// The first `IfcTypeObject` found.
        first: EntityId,
        /// The second, conflicting `IfcTypeObject` found.
        second: EntityId,
    },
    /// `IfcRelDefinesByProperties.RelatedObjects` names an object that is
    /// not a non-type `IfcObjectDefinition`.
    InvalidOccurrenceTarget {
        /// The `IfcRelDefinesByProperties` relationship.
        relationship: EntityId,
        /// The invalid related object.
        object: EntityId,
    },
    /// `IfcRelDefinesByType.RelatedObjects` names an object that is not an
    /// `IfcObject`.
    InvalidTypeTarget {
        /// The `IfcRelDefinesByType` relationship.
        relationship: EntityId,
        /// The invalid related object.
        object: EntityId,
    },
    /// The queried entity is not a non-type `IfcObjectDefinition` and
    /// therefore cannot carry properties.
    InvalidQueryObject {
        /// The rejected query object.
        object: EntityId,
        /// The object's actual IFC type name.
        type_name: Arc<str>,
    },
    /// The same entity id appears more than once in an aggregate attribute
    /// that must have unique members.
    DuplicateAggregateMember {
        /// The entity holding the aggregate.
        entity: EntityId,
        /// Name of the offending attribute.
        attribute: &'static str,
        /// The entity id that appeared more than once.
        member: EntityId,
    },
    /// Two property sets with the same name matched the query for the same
    /// source (occurrence or type), making the result ambiguous.
    DuplicateMatchingSets {
        /// Whether the ambiguity arose among occurrence or type sets.
        source: ExactSource,
        /// The first matching property set.
        first: EntityId,
        /// The second, conflicting matching property set.
        second: EntityId,
    },
    /// Two properties with the same name matched within the same set.
    DuplicateMatchingProperties {
        /// The `IfcPropertySet` containing the ambiguous properties.
        set: EntityId,
        /// The first matching property.
        first: EntityId,
        /// The second, conflicting matching property.
        second: EntityId,
    },
    /// A `Name` attribute expected to be a string was `$`, a reference, or
    /// otherwise not text.
    MalformedName {
        /// The entity whose name attribute was malformed.
        entity: EntityId,
        /// Name of the offending attribute (normally `"Name"`).
        attribute: &'static str,
    },
    /// A `RelatingPropertyDefinition` reference resolves to an entity that
    /// is not an `IfcPropertySetDefinition`.
    UnsupportedDefinition {
        /// The rejected entity.
        entity: EntityId,
        /// The entity's actual IFC type name.
        type_name: Arc<str>,
    },
    /// A member of `IfcPropertySet.HasProperties` is not an `IfcProperty`,
    /// or is a property kind the exact resolver does not yet support.
    UnsupportedProperty {
        /// The rejected entity.
        entity: EntityId,
        /// The entity's actual IFC type name.
        type_name: Arc<str>,
    },
    /// `IfcPropertySingleValue.NominalValue` was `$` where a value was required.
    MissingValueSlot {
        /// The property with the missing value.
        property: EntityId,
    },
    /// An entity's attribute count does not match what the declared
    /// release's schema declares for its type (a malformed or truncated STEP record).
    MalformedEntitySlots {
        /// The malformed entity.
        entity: EntityId,
        /// The entity's IFC type name.
        type_name: Arc<str>,
        /// Attribute count the schema declares for this type.
        expected: usize,
        /// Attribute count actually present on the entity.
        actual: usize,
    },
    /// `IfcPropertySingleValue.Unit` references an entity that is not a
    /// member of the `IfcUnit` select.
    UnsupportedUnit {
        /// The property with the invalid unit reference.
        property: EntityId,
    },
    /// `IfcPropertySingleValue.NominalValue` carries a typed value whose
    /// declared type is not accepted by `IFCVALUE`, or whose payload does
    /// not match its declared type.
    UnsupportedValue {
        /// The property with the invalid value.
        property: EntityId,
    },
    /// `IfcPropertySingleValue.NominalValue` is an `IFCREAL` that is NaN or
    /// infinite, which IFC does not permit.
    NonFiniteReal {
        /// The property with the non-finite real value.
        property: EntityId,
    },
    /// A traversed record, typed value, or select member names a construct
    /// that the release declared in `FILE_SCHEMA` does not define, for
    /// example an `IfcDoorType` or an `IfcPropertySetDefinitionSet` in an
    /// IFC2X3 file. The file mixes releases, so nothing it says about the
    /// property is trusted.
    NotInSchema {
        /// The entity holding or being the foreign construct.
        entity: EntityId,
        /// The construct's name as written in the file.
        name: Arc<str>,
        /// The release the header declares.
        schema: SchemaVersion,
    },
    /// A proper subtype of `IfcRelDefinesByProperties` or
    /// `IfcRelDefinesByType` relates the queried object, such as IFC2X3
    /// `IfcRelOverridesProperties`. Its semantics change which value applies,
    /// and the exact resolver does not interpret them, so it refuses rather
    /// than answer as if the relationship were absent.
    UnsupportedRelationship {
        /// The relationship instance.
        relationship: EntityId,
        /// Its IFC type name.
        type_name: Arc<str>,
    },
}
impl fmt::Display for ExactPropertyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "exact IFC property resolution failed: {self:?}")
    }
}

/// The IFC release a model's properties are resolved against.
///
/// This is the release `exact_property` binds to: the single `FILE_SCHEMA`
/// token, if it names a release the exact resolver supports (IFC2X3 or
/// IFC4). A consumer binds its vocabulary per release with this answer
/// instead of re-parsing the header. It fails exactly as `exact_property`
/// fails at model level: diagnostics, no schema, several schemas, or an
/// unsupported one.
///
/// # Errors
///
/// [`ExactPropertyError::IncompleteModel`], [`ExactPropertyError::MissingSchema`],
/// [`ExactPropertyError::MultipleSchemas`], or
/// [`ExactPropertyError::UnsupportedSchema`].
pub fn exact_schema(model: &Model) -> Result<SchemaVersion, ExactPropertyError> {
    validate_model(model).map(|release| release.version)
}

impl std::error::Error for ExactPropertyError {}

/// Resolve an `IfcPropertySingleValue` by exact set/property name.
///
/// The model is resolved against the single release its `FILE_SCHEMA`
/// declares, IFC2X3 or IFC4 (see [`exact_schema`]); every domain, select and
/// slot count is that release's. With `set_name == None`, all assigned sets
/// are searched. Occurrence values override matching inherited values at
/// property level.
///
/// # Errors
///
/// Any [`ExactPropertyError`]: the answer is refused rather than guessed
/// whenever the evidence is incomplete, ambiguous, or foreign to the
/// declared release.
pub fn exact_property(
    model: &Model,
    object: EntityId,
    set_name: Option<&str>,
    property_name: &str,
) -> Result<ExactResolution, ExactPropertyError> {
    let release = validate_model(model)?;
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
    // `IfcObject` in IFC2X3, `IfcObjectDefinition` in IFC4. Type objects are
    // never occurrences, whatever the release.
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
    let occurrence = find_property(
        model,
        release,
        &occurrence_sets,
        ExactSource::Occurrence,
        set_name,
        property_name,
    )?;
    let inherited = match assigned_type {
        Some(type_id) => {
            // Checked above: a known `IfcTypeObject` with the release's arity,
            // so `HasPropertySets` is slot 5 in both IFC2X3 and IFC4.
            let type_object = model.get(type_id).expect("checked reference");
            let sets = optional_refs_at(type_id, type_object.attributes.get(5), "HasPropertySets")?;
            find_property(
                model,
                release,
                &sets,
                ExactSource::Type(type_id),
                set_name,
                property_name,
            )?
        }
        None => None,
    };
    Ok(occurrence
        .or(inherited)
        .map_or(ExactResolution::Absent, ExactResolution::Present))
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

fn find_property(
    model: &Model,
    release: Release,
    sets: &[EntityId],
    source: ExactSource,
    wanted_set: Option<&str>,
    wanted_property: &str,
) -> Result<Option<ExactProperty>, ExactPropertyError> {
    let schema = release.schema;
    let mut result = None;
    let mut matching_sets = BTreeMap::new();
    for &set_id in sets {
        let set = model
            .get(set_id)
            .ok_or(ExactPropertyError::MissingReference {
                from: set_id,
                to: set_id,
            })?;
        release.require_exact_slots(set_id, set)?;
        if !set.is_type("IFCPROPERTYSET") && schema.is_a(&set.type_name, "IFCPROPERTYSETDEFINITION")
        {
            continue;
        }
        if !set.is_type("IFCPROPERTYSET") {
            return Err(ExactPropertyError::UnsupportedDefinition {
                entity: set_id,
                type_name: set.type_name.clone(),
            });
        }
        let set_name = text_at(set_id, set.attributes.get(2), "Name")?;
        if let Some(name) = wanted_set {
            if set_name != name {
                continue;
            }
        }
        if let Some(first) = matching_sets.insert(set_name.to_owned(), set_id) {
            return Err(ExactPropertyError::DuplicateMatchingSets {
                source,
                first,
                second: set_id,
            });
        }
        let mut matching = None;
        for property_id in nonempty_refs_at(set_id, set.attributes.get(4), "HasProperties")? {
            let property = model
                .get(property_id)
                .ok_or(ExactPropertyError::MissingReference {
                    from: set_id,
                    to: property_id,
                })?;
            if schema.entity(property.type_name.as_ref()).is_none() {
                return Err(release.not_in_schema(property_id, property.type_name.clone()));
            }
            if !schema.is_a(property.type_name.as_ref(), "IFCPROPERTY") {
                return Err(ExactPropertyError::UnsupportedProperty {
                    entity: property_id,
                    type_name: property.type_name.clone(),
                });
            }
            release.require_exact_slots(property_id, property)?;
            if text_at(property_id, property.attributes.first(), "Name")? != wanted_property {
                continue;
            }
            if let Some(first) = matching.replace(property_id) {
                return Err(ExactPropertyError::DuplicateMatchingProperties {
                    set: set_id,
                    first,
                    second: property_id,
                });
            }
        }
        if let Some(property_id) = matching {
            let property = model.get(property_id).expect("checked reference");
            if !property.is_type("IFCPROPERTYSINGLEVALUE") {
                return Err(ExactPropertyError::UnsupportedProperty {
                    entity: property_id,
                    type_name: property.type_name.clone(),
                });
            }
            let ResolvedValue {
                value,
                value_type,
                unit_id,
            } = exact_property_value(model, release, property_id, property)?;
            let candidate = ExactProperty {
                source,
                property_set: Arc::from(set_name),
                set_id,
                property_id,
                value_type,
                unit_id,
                value,
            };
            if let Some(first) = result.replace(candidate) {
                return Err(ExactPropertyError::DuplicateMatchingSets {
                    source,
                    first: first.set_id,
                    second: set_id,
                });
            }
        }
    }
    Ok(result)
}
