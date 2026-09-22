//! Transactional authoring of the spatial structure.
//!
//! # Slot layouts are not repeated here
//!
//! IfcRelAggregates and IfcRelContainedInSpatialStructure disagree
//! about which slot holds the parent. The reader already encodes
//! that in relation::slots; authoring resolves the same constants
//! so a correction cannot update one side and leave the other.

use ifc_model::guid::Guid;
use ifc_model::{Entity, EntityId, Transaction, Value};

mod external;

pub use external::{
    create_external_spatial_element, create_project_library, ExternalSpatialDraft,
    ProjectLibraryDraft,
};

use crate::relation::slots::{RelSlots, AGGREGATES, CONTAINED_IN};

mod boundary;
mod relationships;

pub use boundary::{connect_path_elements, create_space_boundary, BoundaryDraft, BoundaryLevel};

use crate::tree::SpatialKind;
pub use relationships::{
    adhere_to_element, assign_to_actor, assign_to_group_by_factor, assign_to_process,
    assign_to_product, assign_to_resource, associate_profile_def, connect_elements,
    connect_with_realizing_elements, control_flow_element, cover_elements, cover_spaces, declare,
    define_by_object, fill_element, interfere_elements, position_products, project_element,
    serve_buildings, void_element,
};

/// Why a spatial record was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpatialAuthoringError {
    /// A value the schema constrains was not acceptable.
    Invalid {
        /// The entity being authored.
        entity: &'static str,
        /// The attribute at fault.
        attribute: &'static str,
        /// What was supplied.
        value: String,
    },
}

impl std::fmt::Display for SpatialAuthoringError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let Self::Invalid {
            entity,
            attribute,
            value,
        } = self;
        write!(f, "{entity}.{attribute}: {value}")
    }
}

impl std::error::Error for SpatialAuthoringError {}

/// Result of staging a spatial record.
pub type SpatialAuthoringResult<T> = Result<T, SpatialAuthoringError>;

pub(crate) fn invalid(
    entity: &'static str,
    attribute: &'static str,
    value: impl Into<String>,
) -> SpatialAuthoringError {
    SpatialAuthoringError::Invalid {
        entity,
        attribute,
        value: value.into(),
    }
}

/// Authored fields shared by the spatial containers.
#[derive(Debug, Clone, Copy, Default)]
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
}

/// Stage a spatial container.
///
/// `IfcSite`, `IfcBuilding`, `IfcBuildingStorey` and `IfcSpace` share
/// the `IfcSpatialStructureElement` prefix, so one staging path
/// serves all four and cannot drift between them.
///
/// # Errors
///
/// Refuses a malformed GlobalId. IfcRoot.GlobalId is required and
/// is how every relationship names this container.
pub fn create_spatial_element(
    tx: &mut Transaction,
    kind: SpatialKind,
    global_id: &str,
    draft: SpatialDraft<'_>,
) -> SpatialAuthoringResult<EntityId> {
    if Guid::parse(global_id).is_none() {
        return Err(invalid("IFCSPATIALSTRUCTUREELEMENT", "GlobalId", global_id));
    }
    // Each kind declares its own attribute count: writing Site width
    // onto a Storey would leave trailing slots the schema does not
    // define for it.
    let (type_name, width) = match kind {
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
    };
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

pub(crate) fn optional_text(value: Option<&str>) -> Value {
    value.map_or(Value::Null, |t| Value::Text(t.into()))
}

/// Stage an `IfcProject`.
///
/// Kept separate from the containers: IfcProject is an IfcContext,
/// not an IfcSpatialStructureElement, so slots 5 and up mean
/// different things and sharing the path would misplace them.
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
    let width = rel.relating.max(rel.related) + 1;
    let mut attributes = vec![Value::Null; width];
    attributes[0] = Value::Text(global_id.into());
    attributes[rel.relating] = Value::Ref(relating);
    attributes[rel.related] = Value::Ref(related);
    Ok(tx.create(Entity::new(rel.type_name, attributes)))
}

/// Stage an `IfcRelAggregates`: a container decomposed into parts.
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
