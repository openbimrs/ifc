//! Exact, fail-closed IFC4 property resolution.
//!
//! Unlike the permissive views, this traversal rejects any incomplete or
//! malformed assignment data before it can claim an exact absence.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    sync::Arc,
};

use ifc_model::{Entity, EntityId, Model, Value};
use ifc_schema::{ifc4, Schema, SchemaVersion, TypeKind};

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
    /// The model header declares a schema other than IFC4.
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
    /// `IfcRelDefinesByType`, which IFC4 forbids.
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
    /// An entity's attribute count does not match what the IFC4 schema
    /// declares for its type — a malformed or truncated STEP record.
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
    /// infinite, which IFC4 does not permit.
    NonFiniteReal {
        /// The property with the non-finite real value.
        property: EntityId,
    },
}
impl fmt::Display for ExactPropertyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "exact IFC4 property resolution failed: {self:?}")
    }
}
impl std::error::Error for ExactPropertyError {}

/// Resolve an IFC4 `IfcPropertySingleValue` by exact set/property name.
///
/// With `set_name == None`, all assigned sets are searched. Occurrence values
/// override matching inherited values at property level.
pub fn exact_property(
    model: &Model,
    object: EntityId,
    set_name: Option<&str>,
    property_name: &str,
) -> Result<ExactResolution, ExactPropertyError> {
    validate_model(model)?;
    let schema = ifc4();
    let query_entity = model
        .get(object)
        .ok_or(ExactPropertyError::MissingReference {
            from: object,
            to: object,
        })?;
    if !schema.is_a(query_entity.type_name.as_ref(), "IFCOBJECTDEFINITION")
        || schema.is_a(query_entity.type_name.as_ref(), "IFCTYPEOBJECT")
    {
        return Err(ExactPropertyError::InvalidQueryObject {
            object,
            type_name: query_entity.type_name.clone(),
        });
    }
    require_exact_slots(schema, object, query_entity)?;
    let mut occurrence_sets = Vec::new();
    let mut assigned_type = None;
    for relation_id in model.ids_of_type("IFCRELDEFINESBYPROPERTIES") {
        let r = model.get(*relation_id).expect("type index is current");
        require_exact_slots(schema, *relation_id, r)?;
        let related = nonempty_refs_at(*relation_id, r.attributes.get(4), "RelatedObjects")?;
        for related_id in &related {
            require_ref(model, *relation_id, *related_id)?;
            let related_object = model.get(*related_id).expect("checked reference");
            if !schema.is_a(related_object.type_name.as_ref(), "IFCOBJECTDEFINITION")
                || schema.is_a(related_object.type_name.as_ref(), "IFCTYPEOBJECT")
            {
                return Err(ExactPropertyError::InvalidOccurrenceTarget {
                    relationship: *relation_id,
                    object: *related_id,
                });
            }
            require_exact_slots(schema, *related_id, related_object)?;
        }
        let definitions = property_definition_refs_at(
            *relation_id,
            r.attributes.get(5),
            "RelatingPropertyDefinition",
        )?;
        for definition in definitions {
            require_ref(model, *relation_id, definition)?;
            let definition_entity = model.get(definition).expect("checked reference");
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
        require_exact_slots(schema, *relation_id, r)?;
        let related = nonempty_refs_at(*relation_id, r.attributes.get(4), "RelatedObjects")?;
        for related_id in &related {
            require_ref(model, *relation_id, *related_id)?;
            let related_object = model.get(*related_id).expect("checked reference");
            if !schema.is_a(related_object.type_name.as_ref(), "IFCOBJECT") {
                return Err(ExactPropertyError::InvalidTypeTarget {
                    relationship: *relation_id,
                    object: *related_id,
                });
            }
            require_exact_slots(schema, *related_id, related_object)?;
        }
        let type_id = ref_at(*relation_id, r.attributes.get(5), "RelatingType")?;
        require_ref(model, *relation_id, type_id)?;
        let type_object = model.get(type_id).expect("checked reference");
        if !schema.is_a(type_object.type_name.as_ref(), "IFCTYPEOBJECT") {
            return Err(ExactPropertyError::UnsupportedDefinition {
                entity: type_id,
                type_name: type_object.type_name.clone(),
            });
        }
        require_exact_slots(schema, type_id, type_object)?;
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
        &occurrence_sets,
        ExactSource::Occurrence,
        set_name,
        property_name,
    )?;
    let inherited = match assigned_type {
        Some(type_id) => {
            let type_object = model.get(type_id).expect("checked reference");
            if !ifc4().is_a(&type_object.type_name, "IFCTYPEOBJECT") {
                return Err(ExactPropertyError::UnsupportedDefinition {
                    entity: type_id,
                    type_name: type_object.type_name.clone(),
                });
            }
            let sets = optional_refs_at(type_id, type_object.attributes.get(5), "HasPropertySets")?;
            find_property(
                model,
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

fn validate_model(model: &Model) -> Result<(), ExactPropertyError> {
    if !model.diagnostics().is_empty() {
        return Err(ExactPropertyError::IncompleteModel {
            diagnostics: model.diagnostics().len(),
        });
    }
    match model.header().schema.as_slice() {
        [] => Err(ExactPropertyError::MissingSchema),
        [schema] if SchemaVersion::from_header_token(schema) == Some(SchemaVersion::Ifc4) => Ok(()),
        [schema] => Err(ExactPropertyError::UnsupportedSchema {
            schema: schema.clone(),
        }),
        schemas => Err(ExactPropertyError::MultipleSchemas {
            schemas: schemas.len(),
        }),
    }
}

fn find_property(
    model: &Model,
    sets: &[EntityId],
    source: ExactSource,
    wanted_set: Option<&str>,
    wanted_property: &str,
) -> Result<Option<ExactProperty>, ExactPropertyError> {
    let schema = ifc4();
    let mut result = None;
    let mut matching_sets = BTreeMap::new();
    for &set_id in sets {
        let set = model
            .get(set_id)
            .ok_or(ExactPropertyError::MissingReference {
                from: set_id,
                to: set_id,
            })?;
        require_exact_slots(schema, set_id, set)?;
        if !set.is_type("IFCPROPERTYSET") && ifc4().is_a(&set.type_name, "IFCPROPERTYSETDEFINITION")
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
            if !schema.is_a(property.type_name.as_ref(), "IFCPROPERTY") {
                return Err(ExactPropertyError::UnsupportedProperty {
                    entity: property_id,
                    type_name: property.type_name.clone(),
                });
            }
            require_exact_slots(schema, property_id, property)?;
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
            } = exact_property_value(model, property_id, property)?;
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
fn select_accepts_type(schema: &Schema, select: &str, candidate: &str) -> bool {
    select_accepts(schema, select, candidate, false, &mut BTreeSet::new())
}

fn select_accepts_entity(schema: &Schema, select: &str, candidate: &str) -> bool {
    select_accepts(schema, select, candidate, true, &mut BTreeSet::new())
}

fn select_accepts(
    schema: &Schema,
    select: &str,
    candidate: &str,
    entity: bool,
    visited: &mut BTreeSet<String>,
) -> bool {
    let key = select.to_ascii_uppercase();
    if !visited.insert(key.clone()) {
        return false;
    }
    let accepted = schema.type_def(select).is_some_and(|definition| {
        let TypeKind::Select(members) = &definition.kind else {
            return false;
        };
        members.iter().any(|member| {
            member.eq_ignore_ascii_case(candidate)
                || (entity && schema.entity(member).is_some() && schema.is_a(candidate, member))
                || (schema.type_def(member).is_some()
                    && select_accepts(schema, member, candidate, entity, visited))
        })
    });
    visited.remove(&key);
    accepted
}

fn typed_payload_matches(
    schema: &Schema,
    type_name: &str,
    value: &Value,
    visited: &mut BTreeSet<String>,
) -> bool {
    let key = type_name.to_ascii_uppercase();
    if !visited.insert(key.clone()) {
        return false;
    }
    let matches = schema
        .type_def(type_name)
        .is_some_and(|definition| match &definition.kind {
            TypeKind::Defined(rhs) => {
                let base = rhs
                    .split(|character: char| {
                        !(character.is_ascii_alphanumeric() || character == '_')
                    })
                    .find(|part| !part.is_empty())
                    .unwrap_or("");
                if schema.type_def(base).is_some() {
                    typed_payload_matches(schema, base, value, visited)
                } else {
                    match base.to_ascii_uppercase().as_str() {
                        "INTEGER" => matches!(value, Value::Integer(_)),
                        "REAL" => matches!(value, Value::Real(_)),
                        "NUMBER" => matches!(value, Value::Integer(_) | Value::Real(_)),
                        "STRING" => matches!(value, Value::Text(_)),
                        "BINARY" => matches!(value, Value::Binary(_)),
                        "BOOLEAN" => matches!(value, Value::Bool(_)),
                        "LOGICAL" => matches!(value, Value::Bool(_) | Value::LogicalUnknown),
                        _ => false,
                    }
                }
            }
            TypeKind::Enumeration(members) => match value {
                Value::Enum(member) => members
                    .iter()
                    .any(|candidate| candidate.eq_ignore_ascii_case(member)),
                _ => false,
            },
            TypeKind::Select(_) => match value {
                Value::Typed {
                    type_name: member,
                    value: payload,
                } if select_accepts_type(schema, type_name, member) => {
                    typed_payload_matches(schema, member, payload, visited)
                }
                _ => false,
            },
        });
    visited.remove(&key);
    matches
}

#[derive(Debug)]
struct ResolvedValue {
    value: ExactValue,
    value_type: Option<Arc<str>>,
    unit_id: Option<EntityId>,
}

fn exact_property_value(
    model: &Model,
    property: EntityId,
    entity: &Entity,
) -> Result<ResolvedValue, ExactPropertyError> {
    let unit_id = match &entity.attributes[3] {
        Value::Null => None,
        Value::Ref(unit_id) => {
            let unit = model
                .get(*unit_id)
                .ok_or(ExactPropertyError::MissingReference {
                    from: property,
                    to: *unit_id,
                })?;
            if !select_accepts_entity(ifc4(), "IFCUNIT", unit.type_name.as_ref()) {
                return Err(ExactPropertyError::UnsupportedUnit { property });
            }
            require_exact_slots(ifc4(), *unit_id, unit)?;
            Some(*unit_id)
        }
        _ => return Err(ExactPropertyError::UnsupportedUnit { property }),
    };
    match &entity.attributes[2] {
        Value::Null => Ok(ResolvedValue {
            value: ExactValue::Null,
            value_type: None,
            unit_id,
        }),
        Value::Typed { type_name, value }
            if select_accepts_type(ifc4(), "IFCVALUE", type_name.as_ref())
                && typed_payload_matches(
                    ifc4(),
                    type_name.as_ref(),
                    value.as_ref(),
                    &mut BTreeSet::new(),
                ) =>
        {
            exact_value(property, entity.attributes.get(2)).map(|value| ResolvedValue {
                value,
                value_type: Some(type_name.clone()),
                unit_id,
            })
        }
        _ => Err(ExactPropertyError::UnsupportedValue { property }),
    }
}

fn exact_value(
    property: EntityId,
    value: Option<&Value>,
) -> Result<ExactValue, ExactPropertyError> {
    let Some(value) = value else {
        return Err(ExactPropertyError::MissingValueSlot { property });
    };
    match value {
        Value::Typed { type_name, value } if type_name.eq_ignore_ascii_case("IFCLOGICAL") => {
            match value.as_ref() {
                Value::Bool(false) => Ok(ExactValue::Logical(ExactLogical::False)),
                Value::LogicalUnknown => Ok(ExactValue::Logical(ExactLogical::Unknown)),
                Value::Bool(true) => Ok(ExactValue::Logical(ExactLogical::True)),
                _ => Err(ExactPropertyError::UnsupportedValue { property }),
            }
        }
        Value::Typed { value, .. } => exact_value(property, Some(value.as_ref())),
        Value::Null => Ok(ExactValue::Null),
        Value::Bool(v) => Ok(ExactValue::Bool(*v)),
        Value::LogicalUnknown => Ok(ExactValue::Logical(ExactLogical::Unknown)),
        Value::Binary(v) => Ok(ExactValue::Binary(v.clone())),
        Value::Integer(v) => Ok(ExactValue::Integer(*v)),
        Value::Real(v) if v.is_finite() => Ok(ExactValue::Real(*v)),
        Value::Real(_) => Err(ExactPropertyError::NonFiniteReal { property }),
        Value::Text(v) => Ok(ExactValue::Text(v.clone())),
        _ => Err(ExactPropertyError::UnsupportedValue { property }),
    }
}
fn require_exact_slots(
    schema: &Schema,
    entity_id: EntityId,
    entity: &Entity,
) -> Result<(), ExactPropertyError> {
    let expected = schema.attributes(entity.type_name.as_ref()).len();
    let actual = entity.attributes.len();
    if actual != expected {
        return Err(ExactPropertyError::MalformedEntitySlots {
            entity: entity_id,
            type_name: entity.type_name.clone(),
            expected,
            actual,
        });
    }
    Ok(())
}

fn require_ref(model: &Model, from: EntityId, to: EntityId) -> Result<(), ExactPropertyError> {
    if model.get(to).is_some() {
        Ok(())
    } else {
        Err(ExactPropertyError::MissingReference { from, to })
    }
}
fn property_definition_refs_at(
    entity: EntityId,
    value: Option<&Value>,
    attribute: &'static str,
) -> Result<Vec<EntityId>, ExactPropertyError> {
    let Some(value) = value else {
        return Err(ExactPropertyError::MalformedAggregate { entity, attribute });
    };
    match value {
        Value::Ref(id) => Ok(vec![*id]),
        Value::Typed { type_name, value }
            if type_name.eq_ignore_ascii_case("IFCPROPERTYSETDEFINITIONSET") =>
        {
            nonempty_refs_at(entity, Some(value.as_ref()), attribute)
        }
        Value::List(_) => nonempty_refs_at(entity, Some(value), attribute),
        _ => Err(ExactPropertyError::MalformedAggregate { entity, attribute }),
    }
}

fn nonempty_refs_at(
    entity: EntityId,
    value: Option<&Value>,
    attribute: &'static str,
) -> Result<Vec<EntityId>, ExactPropertyError> {
    let refs = refs_at(entity, value, attribute)?;
    if refs.is_empty() {
        Err(ExactPropertyError::MalformedAggregate { entity, attribute })
    } else {
        Ok(refs)
    }
}

fn optional_refs_at(
    entity: EntityId,
    value: Option<&Value>,
    attribute: &'static str,
) -> Result<Vec<EntityId>, ExactPropertyError> {
    match value {
        None => Err(ExactPropertyError::MalformedAggregate { entity, attribute }),
        Some(Value::Null) => Ok(Vec::new()),
        value => nonempty_refs_at(entity, value, attribute),
    }
}
fn refs_at(
    entity: EntityId,
    value: Option<&Value>,
    attribute: &'static str,
) -> Result<Vec<EntityId>, ExactPropertyError> {
    let Some(value) = value else {
        return Err(ExactPropertyError::MalformedAggregate { entity, attribute });
    };
    let Value::List(values) = value else {
        return Err(ExactPropertyError::MalformedAggregate { entity, attribute });
    };
    let mut seen = BTreeSet::new();
    values
        .iter()
        .map(|v| {
            let member = v
                .as_ref_id()
                .ok_or(ExactPropertyError::MalformedAggregate { entity, attribute })?;
            if !seen.insert(member) {
                return Err(ExactPropertyError::DuplicateAggregateMember {
                    entity,
                    attribute,
                    member,
                });
            }
            Ok(member)
        })
        .collect()
}
fn ref_at(
    entity: EntityId,
    value: Option<&Value>,
    attribute: &'static str,
) -> Result<EntityId, ExactPropertyError> {
    value
        .and_then(Value::as_ref_id)
        .ok_or(ExactPropertyError::MalformedAggregate { entity, attribute })
}
fn text_at<'a>(
    entity: EntityId,
    value: Option<&'a Value>,
    attribute: &'static str,
) -> Result<&'a str, ExactPropertyError> {
    value
        .and_then(|v| v.unwrap_typed().as_text())
        .ok_or(ExactPropertyError::MalformedName { entity, attribute })
}
