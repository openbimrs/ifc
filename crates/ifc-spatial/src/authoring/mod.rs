//! Transactional authoring of the spatial structure.
//!
//! # Owner history and the declared release (#202)
//!
//! `IfcRoot.OwnerHistory` is required in IFC2X3 and optional from IFC4 on.
//! The writers that take no model cannot see the release: they write the
//! IFC4/IFC4X3 layout with `OwnerHistory` `$` and are IFC4/IFC4X3 only.
//! Every one has a `*_with_owner_history` variant that binds the model's
//! declared release, lays the record out by attribute name from its table,
//! and takes a caller-supplied `IfcOwnerHistory`; none is ever invented.
//! [`create_space_boundary`], which takes the model, binds the release too
//! and refuses IFC2X3 without an owner history.
//!
//! # Slot layouts are not repeated here
//!
//! IfcRelAggregates and IfcRelContainedInSpatialStructure disagree
//! about which slot holds the parent. The reader already encodes
//! that in relation::slots; authoring resolves the same constants
//! so a correction cannot update one side and leave the other.

use ifc_model::guid::Guid;
use ifc_model::{Entity, EntityId, Transaction, Value};
mod connections;
mod error;
mod external;
mod owned;
mod owned_relationships;
pub(crate) mod release;

pub(crate) use error::invalid;
pub use error::{SpatialAuthoringError, SpatialAuthoringResult};

pub use external::{
    create_external_spatial_element, create_project_library, ExternalSpatialDraft,
    ProjectLibraryDraft,
};

use crate::relation::slots::{RelSlots, AGGREGATES, CONTAINED_IN};

mod boundary;
mod relationships;

pub use boundary::{
    connect_path_elements, connect_path_elements_with_owner_history, create_space_boundary,
    create_space_boundary_with_owner_history, BoundaryDraft, BoundaryLevel,
};
pub use owned::{
    aggregate_with_owner_history, contain_with_owner_history,
    create_external_spatial_element_with_owner_history, create_project_library_with_owner_history,
    create_project_with_owner_history, create_spatial_element_with_owner_history,
};
pub use owned_relationships::{
    adhere_to_element_with_owner_history, assign_to_actor_with_owner_history,
    assign_to_group_by_factor_with_owner_history, assign_to_process_with_owner_history,
    assign_to_product_with_owner_history, assign_to_resource_with_owner_history,
    associate_profile_def_with_owner_history, connect_elements_with_owner_history,
    connect_with_realizing_elements_with_owner_history, control_flow_element_with_owner_history,
    cover_elements_with_owner_history, cover_spaces_with_owner_history, declare_with_owner_history,
    define_by_object_with_owner_history, fill_element_with_owner_history,
    interfere_elements_with_owner_history, position_products_with_owner_history,
    project_element_with_owner_history, serve_buildings_with_owner_history,
    void_element_with_owner_history,
};

use crate::tree::SpatialKind;
pub use connections::{connect_elements, connect_with_realizing_elements, interfere_elements};
pub use relationships::{
    adhere_to_element, assign_to_actor, assign_to_group_by_factor, assign_to_process,
    assign_to_product, assign_to_resource, associate_profile_def, control_flow_element,
    cover_elements, cover_spaces, declare, define_by_object, fill_element, position_products,
    project_element, serve_buildings, void_element,
};

/// Authored fields shared by the spatial containers.
///
/// `#[non_exhaustive]`: build it with [`SpatialDraft::new`] and the
/// setters, so a field a later release needs can be added without breaking
/// callers.
#[derive(Debug, Clone, Copy, Default)]
#[non_exhaustive]
pub struct SpatialDraft<'a> {
    /// `IfcRoot.Name`.
    pub name: Option<&'a str>,
    /// `IfcRoot.Description`.
    pub description: Option<&'a str>,
    /// `IfcSpatialStructureElement.LongName`.
    pub long_name: Option<&'a str>,
    /// `CompositionType`, an `IfcElementCompositionEnum` token.
    pub composition: Option<&'a str>,
    /// `ObjectPlacement`, when the container is placed.
    pub placement: Option<EntityId>,
    /// IFC2X3 `IfcSpace.InteriorOrExteriorSpace`, an
    /// `IfcInternalOrExternalEnum` token that release requires on a space
    /// (#214). IFC4 and IFC4X3 do not declare it, so a value there, or on a
    /// container other than a space, is refused rather than dropped.
    pub interior_or_exterior: Option<&'a str>,
}

impl<'a> SpatialDraft<'a> {
    /// An empty draft: every attribute unset.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set `IfcRoot.Name`.
    #[must_use]
    pub fn name(mut self, value: &'a str) -> Self {
        self.name = Some(value);
        self
    }

    /// Set `IfcRoot.Description`.
    #[must_use]
    pub fn description(mut self, value: &'a str) -> Self {
        self.description = Some(value);
        self
    }

    /// Set `LongName`.
    #[must_use]
    pub fn long_name(mut self, value: &'a str) -> Self {
        self.long_name = Some(value);
        self
    }

    /// Set `CompositionType`, an `IfcElementCompositionEnum` token.
    #[must_use]
    pub fn composition(mut self, value: &'a str) -> Self {
        self.composition = Some(value);
        self
    }

    /// Set `ObjectPlacement`.
    #[must_use]
    pub fn placement(mut self, value: EntityId) -> Self {
        self.placement = Some(value);
        self
    }

    /// Set the IFC2X3 `IfcSpace.InteriorOrExteriorSpace` token.
    #[must_use]
    pub fn interior_or_exterior(mut self, value: &'a str) -> Self {
        self.interior_or_exterior = Some(value);
        self
    }
}

/// The IFC2X3 `IfcSpace` attribute [`SpatialDraft::interior_or_exterior`]
/// fills.
pub(crate) const INTERIOR_OR_EXTERIOR: &str = "InteriorOrExteriorSpace";

/// Stage a spatial container.
///
/// `IfcSite`, `IfcBuilding`, `IfcBuildingStorey` and `IfcSpace` share
/// the `IfcSpatialStructureElement` prefix, so one staging path
/// serves all four and cannot drift between them.
///
/// IFC4 and IFC4X3 only: it writes their layout and leaves
/// `OwnerHistory` `$`, which IFC2X3 requires. In IFC2X3 use
/// [`create_spatial_element_with_owner_history`], which binds the model's declared
/// release.
///
/// # Errors
///
/// Refuses a malformed GlobalId. IfcRoot.GlobalId is required and
/// is how every relationship names this container. Refuses
/// `interior_or_exterior` with
/// [`AuthoringNotInSchema`](SpatialAuthoringError::AuthoringNotInSchema):
/// only IFC2X3 declares it, and this writer cannot write IFC2X3.
pub fn create_spatial_element(
    tx: &mut Transaction,
    kind: SpatialKind,
    global_id: &str,
    draft: SpatialDraft<'_>,
) -> SpatialAuthoringResult<EntityId> {
    let (type_name, width) = container(kind, global_id)?;
    // IFC4 and IFC4X3 declare no `InteriorOrExteriorSpace`; dropping the
    // value would lose what the caller stated.
    if draft.interior_or_exterior.is_some() {
        return Err(SpatialAuthoringError::AuthoringNotInSchema {
            entity: type_name,
            attribute: INTERIOR_OR_EXTERIOR,
            schema: ifc_schema::SchemaVersion::Ifc4,
        });
    }
    let mut attributes = vec![Value::Null; width];
    attributes[0] = Value::Text(global_id.into());
    attributes[2] = optional_text(draft.name);
    attributes[3] = optional_text(draft.description);
    attributes[5] = draft.placement.map_or(Value::Null, Value::Ref);
    attributes[7] = optional_text(draft.long_name);
    attributes[8] = draft
        .composition
        .map_or(Value::Null, |t| Value::Enum(t.into()));
    Ok(tx.create(Entity::new(type_name, attributes)))
}

/// The entity and IFC4 attribute count `kind` stages, after checking the
/// GlobalId.
fn container(kind: SpatialKind, global_id: &str) -> SpatialAuthoringResult<(&'static str, usize)> {
    if Guid::parse(global_id).is_none() {
        return Err(invalid("IFCSPATIALSTRUCTUREELEMENT", "GlobalId", global_id));
    }
    // Each kind declares its own attribute count: writing Site width
    // onto a Storey would leave trailing slots the schema does not
    // define for it.
    Ok(match kind {
        SpatialKind::Site => ("IFCSITE", 14),
        SpatialKind::Building => ("IFCBUILDING", 12),
        SpatialKind::Storey => ("IFCBUILDINGSTOREY", 10),
        SpatialKind::Space => ("IFCSPACE", 11),
        // Project is an IfcContext with a different layout; the other
        // variants the reader uses to classify are not containers this
        // function can author.
        other => {
            return Err(invalid(
                "IFCSPATIALSTRUCTUREELEMENT",
                "kind",
                format!("{other:?}"),
            ))
        }
    })
}

pub(crate) fn optional_text(value: Option<&str>) -> Value {
    value.map_or(Value::Null, |t| Value::Text(t.into()))
}

/// Stage an `IfcProject`.
///
/// Kept separate from the containers: IfcProject is an IfcContext,
/// not an IfcSpatialStructureElement, so slots 5 and up mean
/// different things and sharing the path would misplace them.
///
/// IFC4 and IFC4X3 only: it writes their layout and leaves
/// `OwnerHistory` `$`, which IFC2X3 requires. In IFC2X3 use
/// [`create_project_with_owner_history`], which binds the model's declared
/// release.
///
/// # Errors
///
/// Refuses a malformed GlobalId.
pub fn create_project(
    tx: &mut Transaction,
    global_id: &str,
    name: Option<&str>,
    units: Option<EntityId>,
) -> SpatialAuthoringResult<EntityId> {
    if Guid::parse(global_id).is_none() {
        return Err(invalid("IFCPROJECT", "GlobalId", global_id));
    }
    let mut attributes = vec![Value::Null; 9];
    attributes[0] = Value::Text(global_id.into());
    attributes[2] = optional_text(name);
    attributes[8] = units.map_or(Value::Null, Value::Ref);
    Ok(tx.create(Entity::new("IFCPROJECT", attributes)))
}

/// Stage a relationship using the slot layout the reader resolves.
///
/// `parent` always goes to the slot the reader treats as the
/// containing end, whichever index that is for this relationship.
fn relate(
    tx: &mut Transaction,
    rel: RelSlots,
    global_id: &str,
    parent: EntityId,
    children: &[EntityId],
) -> SpatialAuthoringResult<EntityId> {
    check_relate(rel, global_id, parent, children)?;
    let width = rel.relating.max(rel.related) + 1;
    let mut attributes = vec![Value::Null; width];
    attributes[0] = Value::Text(global_id.into());
    attributes[rel.relating] = Value::Ref(parent);
    attributes[rel.related] = Value::List(children.iter().copied().map(Value::Ref).collect());
    Ok(tx.create(Entity::new(rel.type_name, attributes)))
}

/// Stage a relationship whose related end is a single reference.
///
/// `relate` writes a `SET` to the related slot. Three of the feature
/// relationships take exactly one element there, and a one-element list
/// is not the same value: a reader resolving `RelatedOpeningElement`
/// expects a reference, not a list holding one.
fn relate_one(
    tx: &mut Transaction,
    rel: RelSlots,
    global_id: &str,
    relating: EntityId,
    related: EntityId,
) -> SpatialAuthoringResult<EntityId> {
    check_relate_one(rel, global_id, relating, related)?;
    let width = rel.relating.max(rel.related) + 1;
    let mut attributes = vec![Value::Null; width];
    attributes[0] = Value::Text(global_id.into());
    attributes[rel.relating] = Value::Ref(relating);
    attributes[rel.related] = Value::Ref(related);
    Ok(tx.create(Entity::new(rel.type_name, attributes)))
}

/// The checks of [`relate`]: a GlobalId, a non-empty child set, and no
/// parent among its own children.
fn check_relate(
    rel: RelSlots,
    global_id: &str,
    parent: EntityId,
    children: &[EntityId],
) -> SpatialAuthoringResult<()> {
    if Guid::parse(global_id).is_none() {
        return Err(invalid(rel.type_name, "GlobalId", global_id));
    }
    if children.is_empty() {
        return Err(invalid(rel.type_name, "RelatedObjects", "empty"));
    }
    if children.contains(&parent) {
        return Err(invalid(
            rel.type_name,
            "RelatedObjects",
            "contains the parent",
        ));
    }
    Ok(())
}

/// The checks of [`relate_one`]: a GlobalId and two distinct ends.
fn check_relate_one(
    rel: RelSlots,
    global_id: &str,
    relating: EntityId,
    related: EntityId,
) -> SpatialAuthoringResult<()> {
    if Guid::parse(global_id).is_none() {
        return Err(invalid(rel.type_name, "GlobalId", global_id));
    }
    if relating == related {
        return Err(invalid(
            rel.type_name,
            "RelatedElement",
            "is the relating element",
        ));
    }
    Ok(())
}

/// Stage an `IfcRelAggregates`: a container decomposed into parts.
///
/// IFC4 and IFC4X3 only: it writes their layout and leaves
/// `OwnerHistory` `$`, which IFC2X3 requires. In IFC2X3 use
/// [`aggregate_with_owner_history`], which binds the model's declared
/// release.
///
/// # Errors
///
/// Refuses a malformed GlobalId, an empty child list, and a parent
/// listed among its own children.
pub fn aggregate(
    tx: &mut Transaction,
    global_id: &str,
    parent: EntityId,
    children: &[EntityId],
) -> SpatialAuthoringResult<EntityId> {
    relate(tx, AGGREGATES, global_id, parent, children)
}

/// Stage an `IfcRelContainedInSpatialStructure`.
///
/// Note the slot inversion against IfcRelAggregates: here the
/// structure is slot 5 and the elements slot 4. Passing `structure`
/// as the parent keeps callers from having to know that.
///
/// IFC4 and IFC4X3 only: it writes their layout and leaves
/// `OwnerHistory` `$`, which IFC2X3 requires. In IFC2X3 use
/// [`contain_with_owner_history`], which binds the model's declared
/// release.
///
/// # Errors
///
/// Refuses a malformed GlobalId, an empty element list, and a
/// structure listed among its own contents.
pub fn contain(
    tx: &mut Transaction,
    global_id: &str,
    structure: EntityId,
    elements: &[EntityId],
) -> SpatialAuthoringResult<EntityId> {
    relate(tx, CONTAINED_IN, global_id, structure, elements)
}
