//! An occurrence's type, a type's occurrences, an element's openings and
//! an opening's fillings (#463).
//!
//! Each question is answered from the objectified relationship that states
//! it, read forward, never from an inverse attribute: IFC2X3 files list a
//! type under `IsDefinedBy` together with the property sets, IFC4 and
//! IFC4X3 under `IsTypedBy`, and both are the same `IfcRelDefinesByType`
//! (`RelatedObjects`, `RelatingType`). Slots are looked up by attribute
//! name in the table the caller passes, so the answer follows the release
//! the file declares.
//!
//! | Question | Relationship | Answer |
//! | --- | --- | --- |
//! | [`type_of`] an `IfcObject` | `IfcRelDefinesByType` | at most one type |
//! | [`occurrences_of`] an `IfcTypeObject` | `IfcRelDefinesByType` | its objects |
//! | [`openings_of`] an `IfcElement` | `IfcRelVoidsElement` | its openings |
//! | [`fillings_of`] an `IfcOpeningElement` | `IfcRelFillsElement` | its doors, windows, ... |
//! | [`voided_element_of`] an `IfcFeatureElementSubtraction` | `IfcRelVoidsElement` | at most one host |
//!
//! # Refusals
//!
//! A query refuses an id the model lacks or of a type it does not accept,
//! a relationship whose ends are not the references the schema declares,
//! an answer that names an entity the model lacks, and, where the schema
//! allows one answer (`IsTypedBy`, IFC2X3's `IfcObject` rule WR1, and
//! `VoidsElements`), several different answers. Nothing is guessed.
//!
//! # Cost
//!
//! Each call reads the relationships of its one kind through the type
//! index, so it is proportional to their number, not to the model's size.
//!
//! ```
//! # #[cfg(all(feature = "step", feature = "ifc4"))] {
//! use ifc::{Codec, EntityId, StepCodec};
//!
//! let model = StepCodec.read_bytes(b"ISO-10303-21;HEADER;FILE_DESCRIPTION((''),'2;1');\
//!     FILE_NAME('','',(''),(''),'','','');FILE_SCHEMA(('IFC4'));ENDSEC;DATA;\
//!     #1=IFCWALLTYPE('0dGA15OQf9_P5UdNj5qcto',$,'WT',$,$,$,$,$,$,.SOLIDWALL.);\
//!     #2=IFCWALL('31t64Uzj98DhdKplMxBpF5',$,'Wall',$,$,$,$,$,$);\
//!     #3=IFCRELDEFINESBYTYPE('0xtMgJ4Tb6LQGWVe9NZXe6',$,$,$,(#2),#1);\
//!     #4=IFCOPENINGELEMENT('1kTvXnbbzCWw8lcMd1dR4o',$,$,$,$,$,$,$,$);\
//!     #5=IFCRELVOIDSELEMENT('2O2Fr$t4X7Zf8NOew3FLOH',$,$,$,#2,#4);\
//!     ENDSEC;END-ISO-10303-21;").unwrap();
//! let schema = ifc::schema::ifc4();
//!
//! assert_eq!(ifc::type_of(&model, schema, EntityId(2)), Ok(Some(EntityId(1))));
//! assert_eq!(ifc::occurrences_of(&model, schema, EntityId(1)), Ok(vec![EntityId(2)]));
//! assert_eq!(ifc::openings_of(&model, schema, EntityId(2)), Ok(vec![EntityId(4)]));
//! assert_eq!(ifc::voided_element_of(&model, schema, EntityId(4)), Ok(Some(EntityId(2))));
//! assert_eq!(ifc::fillings_of(&model, schema, EntityId(4)), Ok(vec![]));
//! # }
//! ```
#![cfg(feature = "schema-api")]

use ifc_model::{EntityId, Model, Value};
use ifc_schema::Schema;

/// Why an object-relation query refused.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ObjectRelationError {
    /// The queried id is not in the model.
    MissingEntity(EntityId),
    /// The queried entity is not of a type the query accepts, in the
    /// release the caller passed.
    WrongEntityType {
        /// The queried entity.
        entity: EntityId,
        /// Its type, as the file writes it.
        found: String,
        /// The type the query accepts, subtypes included.
        expected: &'static str,
    },
    /// A relationship's end is not the reference (or list of references)
    /// the schema declares.
    Malformed {
        /// The relationship.
        relationship: EntityId,
        /// The attribute that is malformed.
        attribute: &'static str,
    },
    /// A relationship names an entity the model lacks.
    MissingReference {
        /// The relationship.
        relationship: EntityId,
        /// The id it names.
        target: EntityId,
    },
    /// The schema allows one answer and the file states several different
    /// ones: two types for one object, or two hosts for one opening.
    Ambiguous {
        /// The queried entity.
        entity: EntityId,
        /// The different answers, in file order.
        candidates: Vec<EntityId>,
        /// The relationships stating them, in file order.
        relationships: Vec<EntityId>,
    },
}

impl std::fmt::Display for ObjectRelationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingEntity(EntityId(id)) => write!(f, "no entity #{id}"),
            Self::WrongEntityType {
                entity,
                found,
                expected,
            } => write!(f, "#{} is an {found}, not an {expected}", entity.0),
            Self::Malformed {
                relationship,
                attribute,
            } => write!(
                f,
                "#{}'s {attribute} is not the reference the schema declares",
                relationship.0
            ),
            Self::MissingReference {
                relationship,
                target,
            } => write!(
                f,
                "#{} names #{}, which is not in the model",
                relationship.0, target.0
            ),
            Self::Ambiguous {
                entity,
                candidates,
                relationships,
            } => write!(
                f,
                "#{} has {} answers where the schema allows one: {} (stated by {})",
                entity.0,
                candidates.len(),
                hashes(candidates),
                hashes(relationships)
            ),
        }
    }
}

impl std::error::Error for ObjectRelationError {}

fn hashes(ids: &[EntityId]) -> String {
    ids.iter()
        .map(|EntityId(id)| format!("#{id}"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// One objectified relationship kind: its type and the names of its ends.
struct RelKind {
    type_name: &'static str,
    /// The end the query starts from.
    from: End,
    /// The end the query answers with.
    to: End,
}

/// One end of a relationship: its attribute name, and whether the schema
/// declares a list of references or a single one.
#[derive(Clone, Copy)]
struct End {
    name: &'static str,
    many: bool,
}

const DEFINES_BY_TYPE: &str = "IfcRelDefinesByType";
const VOIDS_ELEMENT: &str = "IfcRelVoidsElement";
const FILLS_ELEMENT: &str = "IfcRelFillsElement";

const RELATED_OBJECTS: End = End {
    name: "RelatedObjects",
    many: true,
};
const RELATING_TYPE: End = End {
    name: "RelatingType",
    many: false,
};
const RELATING_BUILDING_ELEMENT: End = End {
    name: "RelatingBuildingElement",
    many: false,
};
const RELATED_OPENING_ELEMENT: End = End {
    name: "RelatedOpeningElement",
    many: false,
};
const RELATING_OPENING_ELEMENT: End = End {
    name: "RelatingOpeningElement",
    many: false,
};
const RELATED_BUILDING_ELEMENT: End = End {
    name: "RelatedBuildingElement",
    many: false,
};

/// The type object of `occurrence`, through `IfcRelDefinesByType`, or
/// `None` when it has none.
///
/// # Errors
///
/// `occurrence` missing or not an `IfcObject`; a malformed relationship;
/// a type the model lacks; two different types for one object (the schema
/// allows one: `IsTypedBy` is `SET [0:1]`, and IFC2X3 states the same with
/// `IfcObject` rule WR1).
pub fn type_of(
    model: &Model,
    schema: &Schema,
    occurrence: EntityId,
) -> Result<Option<EntityId>, ObjectRelationError> {
    single(
        model,
        schema,
        occurrence,
        "IfcObject",
        &RelKind {
            type_name: DEFINES_BY_TYPE,
            from: RELATED_OBJECTS,
            to: RELATING_TYPE,
        },
    )
}

/// Every object `type_object` types, through `IfcRelDefinesByType`, in
/// file order, each once.
///
/// # Errors
///
/// `type_object` missing or not an `IfcTypeObject`; a malformed
/// relationship; an object the model lacks.
pub fn occurrences_of(
    model: &Model,
    schema: &Schema,
    type_object: EntityId,
) -> Result<Vec<EntityId>, ObjectRelationError> {
    many(
        model,
        schema,
        type_object,
        "IfcTypeObject",
        &RelKind {
            type_name: DEFINES_BY_TYPE,
            from: RELATING_TYPE,
            to: RELATED_OBJECTS,
        },
    )
}

/// Every opening (or other subtraction) voiding `element`, through
/// `IfcRelVoidsElement`, in file order, each once.
///
/// # Errors
///
/// `element` missing or not an `IfcElement`; a malformed relationship; an
/// opening the model lacks.
pub fn openings_of(
    model: &Model,
    schema: &Schema,
    element: EntityId,
) -> Result<Vec<EntityId>, ObjectRelationError> {
    many(
        model,
        schema,
        element,
        "IfcElement",
        &RelKind {
            type_name: VOIDS_ELEMENT,
            from: RELATING_BUILDING_ELEMENT,
            to: RELATED_OPENING_ELEMENT,
        },
    )
}

/// Every element filling `opening` (a door, a window), through
/// `IfcRelFillsElement`, in file order, each once. An opening may hold
/// several (`HasFillings` is `SET [0:?]`).
///
/// # Errors
///
/// `opening` missing or not an `IfcOpeningElement`; a malformed
/// relationship; an element the model lacks.
pub fn fillings_of(
    model: &Model,
    schema: &Schema,
    opening: EntityId,
) -> Result<Vec<EntityId>, ObjectRelationError> {
    many(
        model,
        schema,
        opening,
        "IfcOpeningElement",
        &RelKind {
            type_name: FILLS_ELEMENT,
            from: RELATING_OPENING_ELEMENT,
            to: RELATED_BUILDING_ELEMENT,
        },
    )
}

/// The element `opening` voids, through `IfcRelVoidsElement`, or `None`
/// when it voids none.
///
/// # Errors
///
/// `opening` missing or not an `IfcFeatureElementSubtraction`; a malformed
/// relationship; a host the model lacks; two different hosts (the schema
/// allows one: `VoidsElements` is a single inverse).
pub fn voided_element_of(
    model: &Model,
    schema: &Schema,
    opening: EntityId,
) -> Result<Option<EntityId>, ObjectRelationError> {
    single(
        model,
        schema,
        opening,
        "IfcFeatureElementSubtraction",
        &RelKind {
            type_name: VOIDS_ELEMENT,
            from: RELATED_OPENING_ELEMENT,
            to: RELATING_BUILDING_ELEMENT,
        },
    )
}

/// The one answer [`answers`] may give, refused when there are several.
fn single(
    model: &Model,
    schema: &Schema,
    entity: EntityId,
    expected: &'static str,
    kind: &RelKind,
) -> Result<Option<EntityId>, ObjectRelationError> {
    let found = answers(model, schema, entity, expected, kind)?;
    let mut candidates: Vec<EntityId> = Vec::new();
    for (_, answer) in &found {
        if !candidates.contains(answer) {
            candidates.push(*answer);
        }
    }
    match candidates.as_slice() {
        [] => Ok(None),
        [one] => Ok(Some(*one)),
        _ => {
            let mut relationships: Vec<EntityId> = Vec::new();
            for (relationship, _) in &found {
                if !relationships.contains(relationship) {
                    relationships.push(*relationship);
                }
            }
            Err(ObjectRelationError::Ambiguous {
                entity,
                candidates,
                relationships,
            })
        }
    }
}

/// Every answer [`answers`] gives, each once, in file order.
fn many(
    model: &Model,
    schema: &Schema,
    entity: EntityId,
    expected: &'static str,
    kind: &RelKind,
) -> Result<Vec<EntityId>, ObjectRelationError> {
    let mut out: Vec<EntityId> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for (_, answer) in answers(model, schema, entity, expected, kind)? {
        if seen.insert(answer) {
            out.push(answer);
        }
    }
    Ok(out)
}

/// `(relationship, answer)` for every relationship of `kind` whose `from`
/// end names `entity`, after checking `entity` is an `expected`.
fn answers(
    model: &Model,
    schema: &Schema,
    entity: EntityId,
    expected: &'static str,
    kind: &RelKind,
) -> Result<Vec<(EntityId, EntityId)>, ObjectRelationError> {
    let queried = model
        .get(entity)
        .ok_or(ObjectRelationError::MissingEntity(entity))?;
    if !schema.is_a(&queried.type_name, expected) {
        return Err(ObjectRelationError::WrongEntityType {
            entity,
            found: queried.type_name.to_string(),
            expected,
        });
    }
    let mut out = Vec::new();
    // The type index per type, not `ids_of_type_including_subtypes`, whose
    // sort into file order costs a pass over the whole model per call. None
    // of the three relationships has a subtype in any bundled release, so
    // this is file order too.
    let types = std::iter::once(kind.type_name).chain(schema.subtypes(kind.type_name));
    for &relationship in types.flat_map(|name| model.ids_of_type(name)) {
        let Some(record) = model.get(relationship) else {
            continue;
        };
        let names = schema.attribute_names(&record.type_name);
        let end = |end: End| -> Result<Vec<EntityId>, ObjectRelationError> {
            let malformed = ObjectRelationError::Malformed {
                relationship,
                attribute: end.name,
            };
            let slot = names
                .iter()
                .position(|name| name.eq_ignore_ascii_case(end.name))
                .ok_or(malformed.clone())?;
            match (record.attributes.get(slot), end.many) {
                (Some(Value::Ref(id)), false) => Ok(vec![*id]),
                (Some(Value::List(items)), true) => items
                    .iter()
                    .map(|item| match item {
                        Value::Ref(id) => Ok(*id),
                        _ => Err(malformed.clone()),
                    })
                    .collect(),
                _ => Err(malformed),
            }
        };
        if !end(kind.from)?.contains(&entity) {
            continue;
        }
        for answer in end(kind.to)? {
            if !model.contains(answer) {
                return Err(ObjectRelationError::MissingReference {
                    relationship,
                    target: answer,
                });
            }
            out.push((relationship, answer));
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests;
