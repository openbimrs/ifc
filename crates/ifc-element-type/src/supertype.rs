//! Type definitions that carry no `PredefinedType`.
//!
//! The generated catalogue in [`crate::table`] is keyed on a predefined
//! type enum, so the seven structural type definitions that declare no
//! such enum have no row there and cannot be staged by
//! [`crate::create_type`].
//!
//! They are still concrete. `SUPERTYPE OF (ONEOF ...)` constrains which
//! subtype an instance may additionally be; it does not make the
//! supertype uninstantiable, and the schema marks none of these
//! ABSTRACT. The occurrence side already works this way: `IfcBuiltElement`
//! is authored on exactly the same footing as `IfcBuiltElementType` is
//! here.
//!
//! # The rule this module exists to enforce
//!
//! `IfcTypeObject` states `NameRequired`:
//!
//! ```text
//! NameRequired : EXISTS(SELF\IfcRoot.Name);
//! ```
//!
//! `Name` is OPTIONAL in the attribute list and mandatory by rule. A
//! writer reading only the slot table files a nameless type, which
//! parses and then cannot be referred to by anything.

use ifc_model::guid::Guid;
use ifc_model::{Entity, EntityId, Transaction, Value};

use crate::authoring::{ElementTypeError, ElementTypeResult};

/// A type definition with no `PredefinedType` enum.
///
/// The three arities are the three inheritance depths: `IfcTypeObject`
/// stops at `HasPropertySets`, `IfcTypeProduct` adds
/// `RepresentationMaps` and `Tag`, and the element types add
/// `ElementType`. Writing all seven at one arity would leave trailing
/// slots on the shallow ones and truncate the deep ones.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SupertypeKind {
    /// STEP type name, upper-case as stored.
    ///
    /// Upper-case because `Entity::new` does not normalise and the model
    /// indexes by the stored string: a mixed-case record is invisible to
    /// `of_type` lookups that use the catalogue's own spelling.
    pub type_name: &'static str,
    /// Total attribute count, including inherited.
    pub arity: usize,
}

/// `IfcTypeObject`: the root of every type definition.
pub const TYPE_OBJECT: SupertypeKind = SupertypeKind {
    type_name: "IFCTYPEOBJECT",
    arity: 6,
};

/// `IfcTypeProduct`: adds `RepresentationMaps` and `Tag`.
pub const TYPE_PRODUCT: SupertypeKind = SupertypeKind {
    type_name: "IFCTYPEPRODUCT",
    arity: 8,
};

/// `IfcBuiltElementType`: the built-element branch.
pub const BUILT_ELEMENT_TYPE: SupertypeKind = SupertypeKind {
    type_name: "IFCBUILTELEMENTTYPE",
    arity: 9,
};

/// `IfcCivilElementType`: civil works with no more specific class.
pub const CIVIL_ELEMENT_TYPE: SupertypeKind = SupertypeKind {
    type_name: "IFCCIVILELEMENTTYPE",
    arity: 9,
};

/// `IfcDeepFoundationType`: piles and caisson foundations.
pub const DEEP_FOUNDATION_TYPE: SupertypeKind = SupertypeKind {
    type_name: "IFCDEEPFOUNDATIONTYPE",
    arity: 9,
};

/// `IfcDistributionElementType`: the distribution branch.
pub const DISTRIBUTION_ELEMENT_TYPE: SupertypeKind = SupertypeKind {
    type_name: "IFCDISTRIBUTIONELEMENTTYPE",
    arity: 9,
};

/// `IfcFurnishingElementType`: furniture and system furniture.
pub const FURNISHING_ELEMENT_TYPE: SupertypeKind = SupertypeKind {
    type_name: "IFCFURNISHINGELEMENTTYPE",
    arity: 9,
};

/// Every supertype this module can stage.
pub const ALL_SUPERTYPES: &[SupertypeKind] = &[
    TYPE_OBJECT,
    TYPE_PRODUCT,
    BUILT_ELEMENT_TYPE,
    CIVIL_ELEMENT_TYPE,
    DEEP_FOUNDATION_TYPE,
    DISTRIBUTION_ELEMENT_TYPE,
    FURNISHING_ELEMENT_TYPE,
];

/// Attributes of a supertype definition.
///
/// `property_sets` carries `(name, id)` pairs rather than bare ids:
/// `UniquePropertySetNames` is stated over the sets' names, and a
/// staged entity cannot be read back out of a `Transaction` to supply
/// them. Matches the convention `add_property_set` already uses.
#[derive(Debug, Clone, Copy, Default)]
pub struct SupertypeDraft<'a> {
    /// `Description`.
    pub description: Option<&'a str>,
    /// `ApplicableOccurrence`, slot 4.
    pub applicable_occurrence: Option<&'a str>,
    /// `HasPropertySets`, slot 5: `(Name, id)` per set.
    pub property_sets: &'a [(&'a str, EntityId)],
    /// `RepresentationMaps`, slot 6. Rejected below arity 8.
    pub representation_maps: &'a [EntityId],
    /// `Tag`, slot 7. Rejected below arity 8.
    pub tag: Option<&'a str>,
    /// `ElementType`, slot 8. Rejected below arity 9.
    pub element_type: Option<&'a str>,
}

fn invalid(
    entity: &'static str,
    attribute: &'static str,
    value: impl Into<String>,
) -> ElementTypeError {
    ElementTypeError::Invalid {
        entity,
        attribute,
        value: value.into(),
    }
}

fn text(value: Option<&str>) -> Value {
    value.map_or(Value::Null, |v| Value::Text(v.into()))
}

/// Stage a type definition that carries no `PredefinedType`.
///
/// `name` is taken by value, not as an `Option`: `NameRequired` makes it
/// mandatory for every type object, so there is no legal way to omit it
/// and the signature says so.
///
/// # Errors
///
/// Refuses a malformed GlobalId, a blank name (NameRequired), an empty
/// but present property-set list, duplicate property-set names
/// (UniquePropertySetNames), an empty representation-map list, and any
/// attribute the entity's arity does not reach.
pub fn create_supertype(
    tx: &mut Transaction,
    kind: SupertypeKind,
    global_id: &str,
    name: &str,
    draft: SupertypeDraft<'_>,
) -> ElementTypeResult<EntityId> {
    let entity = kind.type_name;
    if Guid::parse(global_id).is_none() {
        return Err(invalid(entity, "GlobalId", global_id));
    }
    if name.trim().is_empty() {
        return Err(invalid(entity, "Name", "NameRequired"));
    }

    let mut attrs = vec![Value::Null; kind.arity];
    attrs[0] = Value::Text(global_id.into());
    attrs[2] = Value::Text(name.into());
    attrs[3] = text(draft.description);
    attrs[4] = text(draft.applicable_occurrence);

    if !draft.property_sets.is_empty() {
        for (index, (set_name, _)) in draft.property_sets.iter().enumerate() {
            // A blank name defeats the uniqueness rule rather than
            // satisfying it: two unnamed sets are not distinguishable
            // by the name the rule compares.
            if set_name.trim().is_empty() {
                return Err(invalid(entity, "HasPropertySets", "blank name"));
            }
            if draft.property_sets[..index]
                .iter()
                .any(|(seen, _)| seen == set_name)
            {
                return Err(invalid(entity, "HasPropertySets", (*set_name).to_owned()));
            }
        }
        attrs[5] = Value::List(
            draft
                .property_sets
                .iter()
                .map(|(_, id)| Value::Ref(*id))
                .collect(),
        );
    }

    // Slots 6-8 exist only at the deeper arities. An attribute the
    // entity does not declare is refused, not dropped: silently
    // discarding a caller's Tag writes a file missing data they
    // believe they supplied.
    if !draft.representation_maps.is_empty() {
        if kind.arity < 8 {
            return Err(invalid(entity, "RepresentationMaps", "not declared"));
        }
        attrs[6] = Value::List(
            draft
                .representation_maps
                .iter()
                .copied()
                .map(Value::Ref)
                .collect(),
        );
    }
    if draft.tag.is_some() {
        if kind.arity < 8 {
            return Err(invalid(entity, "Tag", "not declared"));
        }
        attrs[7] = text(draft.tag);
    }
    if draft.element_type.is_some() {
        if kind.arity < 9 {
            return Err(invalid(entity, "ElementType", "not declared"));
        }
        attrs[8] = text(draft.element_type);
    }

    Ok(tx.create(Entity::new(entity, attrs)))
}
