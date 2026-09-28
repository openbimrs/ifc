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
use ifc_model::{EntityId, Model, Transaction, Value};

use crate::authoring::invalid;
use crate::error::ElementTypeResult;
use crate::release::{bind, require_owner_history, Layout};

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

fn text(value: Option<&str>) -> Value {
    value.map_or(Value::Null, |v| Value::Text(v.into()))
}

/// Stage a type definition that carries no `PredefinedType`.
///
/// `name` is taken by value, not as an `Option`: `NameRequired` makes it
/// mandatory for every type object, so there is no legal way to omit it
/// and the signature says so.
///
/// # Release
///
/// Takes no model, so it writes the IFC4X3 layout with `OwnerHistory` `$`
/// and cannot refuse a model that declares another release; that record is
/// never valid IFC2X3, which requires `OwnerHistory`. Use
/// [`create_supertype_in`] or [`create_supertype_with_owner_history`] to
/// write the model's declared release.
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
    let request = Request {
        kind,
        global_id,
        name,
        draft,
    };
    author(tx, Layout::catalogue()?, request, None)
}

/// [`create_supertype`] in the model's declared release (#202).
///
/// The record is laid out by attribute name from that release's table.
/// `OwnerHistory` is left `$`, which IFC4 and IFC4X3 allow and IFC2X3 does
/// not, so an IFC2X3 model is refused with
/// [`AuthoringRequired`](crate::ElementTypeError::AuthoringRequired); use
/// [`create_supertype_with_owner_history`] there. A header without
/// `FILE_SCHEMA` binds IFC4.
///
/// # Errors
///
/// Those of [`create_supertype`], where "not declared" means not declared
/// by the bound release, and: [`MultipleSchemas`](crate::ElementTypeError::MultipleSchemas) or
/// [`UnsupportedSchema`](crate::ElementTypeError::UnsupportedSchema) for a model that binds no single
/// known release; [`EntityNotInSchema`](crate::ElementTypeError::EntityNotInSchema) for a type the
/// release does not declare or declares abstract (`IfcBuiltElementType` is
/// IFC4X3 only). Nothing is staged on an error.
pub fn create_supertype_in(
    tx: &mut Transaction,
    model: &Model,
    kind: SupertypeKind,
    global_id: &str,
    name: &str,
    draft: SupertypeDraft<'_>,
) -> ElementTypeResult<EntityId> {
    let request = Request {
        kind,
        global_id,
        name,
        draft,
    };
    author(tx, bind(model)?, request, None)
}

/// [`create_supertype_in`] with a caller-supplied `IfcOwnerHistory`, which
/// IFC2X3 requires on every `IfcRoot`.
///
/// `owner_history` must be in the model or staged earlier on `tx`, and must
/// be an `IfcOwnerHistory`; one is never invented here.
///
/// # Errors
///
/// Those of [`create_supertype_in`] except the IFC2X3 `OwnerHistory`
/// refusal, and [`MissingEntity`](crate::ElementTypeError::MissingEntity) or
/// [`Invalid`](crate::ElementTypeError::Invalid) on `OwnerHistory` for an `owner_history`
/// that does not resolve or is another entity. Nothing is staged on an
/// error.
pub fn create_supertype_with_owner_history(
    tx: &mut Transaction,
    model: &Model,
    kind: SupertypeKind,
    global_id: &str,
    name: &str,
    draft: SupertypeDraft<'_>,
    owner_history: EntityId,
) -> ElementTypeResult<EntityId> {
    let layout = bind(model)?;
    require_owner_history(tx, model, kind.type_name, owner_history)?;
    let request = Request {
        kind,
        global_id,
        name,
        draft,
    };
    author(tx, layout, request, Some(owner_history))
}

/// The caller's arguments, bundled.
struct Request<'a> {
    kind: SupertypeKind,
    global_id: &'a str,
    name: &'a str,
    draft: SupertypeDraft<'a>,
}

/// Stage a supertype in `layout`; `None` leaves `OwnerHistory` `$`.
fn author(
    tx: &mut Transaction,
    layout: Layout,
    request: Request<'_>,
    owner_history: Option<EntityId>,
) -> ElementTypeResult<EntityId> {
    let Request {
        kind,
        global_id,
        name,
        draft,
    } = request;
    let entity = kind.type_name;
    if Guid::parse(global_id).is_none() {
        return Err(invalid(entity, "GlobalId", global_id));
    }
    if name.trim().is_empty() {
        return Err(invalid(entity, "Name", "NameRequired"));
    }
    layout.require_entity(entity)?;

    let mut values = vec![
        ("GlobalId", Value::Text(global_id.into())),
        (
            "OwnerHistory",
            owner_history.map_or(Value::Null, Value::Ref),
        ),
        ("Name", Value::Text(name.into())),
        ("Description", text(draft.description)),
        ("ApplicableOccurrence", text(draft.applicable_occurrence)),
    ];

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
        let sets = draft.property_sets.iter().map(|(_, id)| Value::Ref(*id));
        values.push(("HasPropertySets", Value::List(sets.collect())));
    }

    // `RepresentationMaps`, `Tag` and `ElementType` exist only on the
    // deeper types. An attribute the entity does not declare is refused,
    // not dropped: silently discarding a caller's Tag writes a file
    // missing data they believe they supplied.
    let declared = |attribute| layout.attribute(entity, attribute).is_some();
    if !draft.representation_maps.is_empty() {
        if !declared("RepresentationMaps") {
            return Err(invalid(entity, "RepresentationMaps", "not declared"));
        }
        let maps = draft.representation_maps.iter().copied().map(Value::Ref);
        values.push(("RepresentationMaps", Value::List(maps.collect())));
    }
    if draft.tag.is_some() {
        if !declared("Tag") {
            return Err(invalid(entity, "Tag", "not declared"));
        }
        values.push(("Tag", text(draft.tag)));
    }
    if draft.element_type.is_some() {
        if !declared("ElementType") {
            return Err(invalid(entity, "ElementType", "not declared"));
        }
        values.push(("ElementType", text(draft.element_type)));
    }

    Ok(tx.create(layout.named_record(entity, values)?))
}
