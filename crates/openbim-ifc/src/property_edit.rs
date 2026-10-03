//! Checked edits of property and quantity values (#123).
//!
//! A property is addressed by its object, set name and property name, the
//! key the exact resolver (`ifc_properties::exact_properties`) reads it by.
//! [`apply_property_edits`] takes a batch of [`PropertyEdit`]s, plans every
//! one against the model as it stands plus the edits before it, stages the
//! whole plan on one [`Transaction`](ifc_model::Transaction) and commits it.
//! A refused edit, at any position, leaves the model untouched: nothing is
//! written until every edit has been planned and checked, and the commit
//! itself is all or nothing.
//!
//! # What an edit does
//!
//! - **Set** writes a value. An existing property of the object's own set
//!   is changed; a missing one is added to that set; a missing set is
//!   created with its `IfcRelDefinesByProperties`, or, for a type object,
//!   added to its `HasPropertySets`.
//! - **Remove** drops a property from the object's own set. A set left
//!   empty is removed with its relationship, since `HasProperties` and
//!   `Quantities` are `SET [1:?]`.
//!
//! # Occurrence and type
//!
//! A value an occurrence inherits from its type object is never changed
//! through the occurrence: a type set is shared by every occurrence of the
//! type. Setting it on the occurrence creates an occurrence-level override
//! in an occurrence set of the same name, in the inherited property's form
//! (entity, unit, enumeration). To change the type's own value, address the
//! type object. Removing an inherited property through the occurrence is
//! refused: IFC has no way to state that an inherited value is absent.
//!
//! Sharing is never mutated silently in either direction. A set the object
//! shares with other objects (one relationship relating several, or a set
//! two types hold), or a property entity two sets share, is copied for the
//! edited object first, and only the copy changes.
//!
//! # Validation
//!
//! Against the release the header declares, IFC2X3, IFC4 or IFC4X3 (any
//! other is refused, as the resolver refuses it): a value must be an
//! `IfcValue` member of that release written as a typed parameter whose
//! payload fits the member's underlying type; a quantity's must be its
//! declared measure; a property with a stated `Unit` keeps its measure; an
//! enumerated value must be in its `IfcPropertyEnumeration`. With the
//! `property-catalog` feature, a set named in the release's PSD/QTO catalog
//! (ADR 0017) is checked against its template too: set kind, value form,
//! data type, enumeration members, quantity kind, and that a new property
//! is one the template declares. With `property-catalog-runtime` instead,
//! the same check reads the catalog the host installed
//! (`property_catalog::runtime::install`), and a Set on a `Pset_` or `Qto_`
//! set is refused with [`PropertyEditFailure::CatalogNotLoaded`] until the
//! release's edition is installed. Without either feature such a set cannot
//! be checked and a Set on one is refused.
//!
//! # New records
//!
//! Sets and relationships are written by the release-bound `ifc-properties`
//! writers. They take the `OwnerHistory` of the edited object, which IFC2X3
//! requires; one is never invented. Their `GlobalId`s are name-based
//! (UUID version 8): derived from the object's `GlobalId`, the set name, the
//! record's role and the model's next free id, so the same edit of the same
//! file gives the same file, and no two new records share one.
#![cfg(feature = "properties")]

mod checks;
mod edit;
mod emit;
mod guid;
mod holder;
mod plan;
mod template;
mod value;

pub use edit::{
    apply_property_edits, stage_property_edits, PropertyEdit, PropertyEditError,
    PropertyEditFailure, PropertyEditOutcome, SetType, StagedPropertyEdits,
};
