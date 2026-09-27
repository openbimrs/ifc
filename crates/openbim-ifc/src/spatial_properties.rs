//! Each spatial container's elements with their exactly resolved properties
//! (#121).
//!
//! # Why this lives in the facade
//!
//! The answer joins the spatial tree (`ifc-spatial`) with exact property
//! resolution (`ifc-properties`). ADR 0003 forbids those sibling crates from
//! depending on each other, so the join is an orchestration item, compiled
//! only with both `spatial` and `properties`.
//!
//! # What "an element in a container" means here
//!
//! From the IFC4 ADD2 TC1 documentation and the EXPRESS of IFC2X3 TC1,
//! IFC4 ADD2 TC1 and IFC4X3 ADD2:
//!
//! - **Contained** ([`SpatialMembership::Contained`]): named by an
//!   `IfcRelContainedInSpatialStructure` of the container. "Any element can
//!   only be assigned once to a certain level of the spatial structure";
//!   `IfcElement.ContainedInStructure` is `SET [0:1]` in all three releases.
//!   A file stating two homes keeps the first, as
//!   [`SpatialTree::anomalies`] reports.
//! - **Referenced** ([`SpatialMembership::Referenced`]): named by an
//!   `IfcRelReferencedInSpatialStructure` of the container. Such elements
//!   are "referenced, but not primarily contained" there and "can be
//!   referenced to zero, one or several levels"; the documented example is
//!   a curtain wall contained by the ground floor and referenced by the
//!   storeys above. So one element can appear under several containers,
//!   always with its membership stated.
//! - **Part** ([`SpatialMembership::Part`]): an `IfcRelAggregates` part,
//!   at any depth, of an element *contained* in the container. The Element
//!   Composition concept says "the part should not be contained in the
//!   spatial hierarchy ... The part is contained in the spatial structure by
//!   the spatial containment of its composite", and that parts "may have
//!   individual property sets". A part the file does contain itself is
//!   listed under its own container, once, as contained. Parts of a merely
//!   referenced element are not expanded: the documentation places parts by
//!   their composite's containment only.
//! - **Nested structure is its own entry.** A space aggregated into a
//!   storey is a container of its own; its elements are listed under the
//!   space, not folded into the storey. Use [`SpatialContainer::parent`] or
//!   [`SpatialTree::ancestors`] to roll them up.
//!
//! `IfcRelNests` (ports, nested components) and feature elements
//! (openings) do not place an element in a container and are not listed.
//!
//! # Honest answers per element
//!
//! Properties come from [`exact_properties`] (or [`exact_properties_where`]),
//! so an empty list is a proven absence and an error is the file failing to
//! prove its answer. A model-level refusal (diagnostics, missing or foreign
//! schema) is returned once by [`spatial_properties`], because no element
//! could be answered. Any other refusal is reported on the element it
//! concerns and never stops the others.

#![cfg(all(feature = "spatial", feature = "properties"))]

mod members;

use ifc_model::{EntityId, Model};
use ifc_properties::{
    exact_properties, exact_properties_where, exact_schema, ExactPropertyEntry, ExactPropertyError,
    SchemaVersion,
};
use ifc_spatial::{SpatialKind, SpatialTree};

/// How an element belongs to the container it is listed under.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum SpatialMembership {
    /// Named by an `IfcRelContainedInSpatialStructure` of this container.
    Contained,
    /// An `IfcRelAggregates` part, at any depth, of an element contained in
    /// this container.
    Part {
        /// The composite this part is directly aggregated into.
        whole: EntityId,
    },
    /// Named by an `IfcRelReferencedInSpatialStructure` of this container.
    Referenced,
}

/// `IfcRoot.Name` of a container, which is optional.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ContainerName<'v> {
    /// `$`, or a record too short to hold the attribute.
    Unset,
    /// The label.
    Text(&'v str),
    /// Something other than a string: not guessed at.
    Malformed,
}

/// A spatial container's identity and place in the tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct SpatialContainer<'v> {
    /// The container entity.
    pub id: EntityId,
    /// Its spatial role.
    pub kind: SpatialKind,
    /// Its STEP type name as stored, e.g. `IFCBUILDINGSTOREY`.
    pub type_name: &'v str,
    /// Its `Name` (slot 2, `IfcRoot`, in every release).
    pub name: ContainerName<'v>,
    /// The container aggregating it, if any.
    pub parent: Option<EntityId>,
}

/// One element listed under a container, before any property is resolved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct ElementMember<'v> {
    /// The element.
    pub element: EntityId,
    /// Its STEP type name as stored.
    pub type_name: &'v str,
    /// How it belongs to the container.
    pub membership: SpatialMembership,
}

/// One element with its exactly resolved properties.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct ElementProperties<'v> {
    /// The element.
    pub element: EntityId,
    /// Its STEP type name as stored.
    pub type_name: &'v str,
    /// How it belongs to the container.
    pub membership: SpatialMembership,
    /// Every property, quantity and predefined-set attribute, occurrence
    /// sets first, then inherited ones, as [`exact_properties`] orders them.
    /// Empty is a proven absence; an error is this element's alone.
    pub properties: Result<Vec<ExactPropertyEntry>, ExactPropertyError>,
}

/// Every spatial container of a model with the elements it holds.
///
/// Built by [`spatial_properties`]. Borrows the model; it owns only the
/// spatial tree and the per-container member lists (ids and borrowed type
/// names). Properties are resolved lazily, element by element, as
/// [`ContainerElements::elements`] is iterated.
#[derive(Debug, Clone)]
pub struct SpatialProperties<'m> {
    model: &'m Model,
    schema: SchemaVersion,
    tree: SpatialTree,
    containers: Vec<members::Slot<'m>>,
    dangling_parts: Vec<(EntityId, EntityId)>,
}

/// One container and the elements listed under it.
#[derive(Debug, Clone, Copy)]
pub struct ContainerElements<'v> {
    /// The container.
    pub container: SpatialContainer<'v>,
    members: &'v [ElementMember<'v>],
    model: &'v Model,
}

/// Group `model`'s elements by spatial container, for exact property
/// resolution.
///
/// Containers come in spatial tree order: depth first from
/// [`SpatialTree::roots`], children in the tree's order, and any container
/// no root reaches (an aggregation cycle) afterwards by id. Within a
/// container, elements are ordered by id, then by membership.
///
/// # Errors
///
/// The model-level refusals of [`exact_schema`]: STEP diagnostics, a missing
/// or repeated `FILE_SCHEMA`, or a release other than IFC2X3, IFC4 or
/// IFC4X3. Nothing about any element can be resolved exactly then.
pub fn spatial_properties(model: &Model) -> Result<SpatialProperties<'_>, ExactPropertyError> {
    let schema = exact_schema(model)?;
    let tree = SpatialTree::build(model);
    let (containers, dangling_parts) = members::collect(model, &tree);
    Ok(SpatialProperties {
        model,
        schema,
        tree,
        containers,
        dangling_parts,
    })
}

impl<'m> SpatialProperties<'m> {
    /// The release every property was resolved against.
    #[must_use]
    pub fn schema(&self) -> SchemaVersion {
        self.schema
    }

    /// The spatial tree the grouping follows, with its anomalies, orphans
    /// and dangling references.
    #[must_use]
    pub fn tree(&self) -> &SpatialTree {
        &self.tree
    }

    /// `(relationship, part)` pairs: an `IfcRelAggregates` of a listed
    /// element naming a part the model does not contain. Such a part cannot
    /// be listed, so it is reported here instead.
    #[must_use]
    pub fn dangling_parts(&self) -> &[(EntityId, EntityId)] {
        &self.dangling_parts
    }

    /// Every container, in spatial tree order.
    pub fn containers(&self) -> impl Iterator<Item = ContainerElements<'_>> + '_ {
        self.containers.iter().map(|slot| self.view(slot))
    }

    /// The container `id`, if it is one.
    #[must_use]
    pub fn container(&self, id: EntityId) -> Option<ContainerElements<'_>> {
        let slot = self
            .containers
            .iter()
            .find(|slot| slot.container.id == id)?;
        Some(self.view(slot))
    }

    fn view<'v>(&'v self, slot: &'v members::Slot<'m>) -> ContainerElements<'v> {
        ContainerElements {
            container: slot.container,
            members: &slot.members,
            model: self.model,
        }
    }
}

impl<'v> ContainerElements<'v> {
    /// The elements listed here, without resolving anything.
    #[must_use]
    pub fn members(&self) -> &'v [ElementMember<'v>] {
        self.members
    }

    /// Each element with every property it carries, resolved on demand.
    pub fn elements(&self) -> impl Iterator<Item = ElementProperties<'v>> + 'v {
        let model = self.model;
        self.members
            .iter()
            .map(move |member| resolved(member, exact_properties(model, member.element)))
    }

    /// Each element with the properties the two selectors pick, as
    /// [`exact_properties_where`] selects them, resolved on demand.
    ///
    /// A rule that asks about some sets only is not refused by an
    /// unsupported member of a set it does not ask about.
    pub fn elements_where<S, P>(
        &self,
        mut select_set: S,
        mut select_property: P,
    ) -> impl Iterator<Item = ElementProperties<'v>> + 'v
    where
        S: FnMut(&str) -> bool + 'v,
        P: FnMut(&str) -> bool + 'v,
    {
        let model = self.model;
        self.members.iter().map(move |member| {
            let properties = exact_properties_where(
                model,
                member.element,
                &mut select_set,
                &mut select_property,
            );
            resolved(member, properties)
        })
    }
}

fn resolved<'v>(
    member: &ElementMember<'v>,
    properties: Result<Vec<ExactPropertyEntry>, ExactPropertyError>,
) -> ElementProperties<'v> {
    ElementProperties {
        element: member.element,
        type_name: member.type_name,
        membership: member.membership,
        properties,
    }
}
