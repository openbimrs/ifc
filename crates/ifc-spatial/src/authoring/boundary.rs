//! Authoring space boundaries and path connections.
//!
//! A space boundary says which element bounds a space and how.
//! The three levels refine each other: the base form states the
//! boundary, the 1st level adds a parent, the 2nd level adds the
//! boundary on the other side of the same element.
//!
//! `CorrectPhysOrVirt` ties the physical/virtual flag to the type
//! of the bounding element, so the flag cannot be set independently
//! of what it describes. This module reads the type name from the
//! transaction or model rather than the schema tables: the crate
//! deliberately has no `ifc-schema` dependency.

use ifc_model::guid::Guid;
use ifc_model::{Edit, Entity, EntityId, Model, Transaction, Value};

use super::{invalid, SpatialAuthoringResult};

/// Which space-boundary level to stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundaryLevel {
    /// `IfcRelSpaceBoundary`: the boundary alone.
    Base,
    /// `IfcRelSpaceBoundary1stLevel`: adds `ParentBoundary`.
    First,
    /// `IfcRelSpaceBoundary2ndLevel`: adds `CorrespondingBoundary`.
    Second,
}

impl BoundaryLevel {
    /// The entity type name for this level.
    fn type_name(self) -> &'static str {
        match self {
            Self::Base => "IFCRELSPACEBOUNDARY",
            Self::First => "IFCRELSPACEBOUNDARY1STLEVEL",
            Self::Second => "IFCRELSPACEBOUNDARY2NDLEVEL",
        }
    }

    /// Total attribute count, including inherited.
    fn arity(self) -> usize {
        match self {
            Self::Base => 9,
            Self::First => 10,
            Self::Second => 11,
        }
    }
}

/// Authored fields for a space boundary.
#[derive(Debug, Clone, Copy)]
pub struct BoundaryDraft<'a> {
    /// `IfcRoot.Name`.
    pub name: Option<&'a str>,
    /// `IfcRoot.Description`.
    pub description: Option<&'a str>,
    /// `RelatingSpace`, an `IfcSpaceBoundarySelect`.
    pub space: EntityId,
    /// `RelatedBuildingElement`, an `IfcElement`.
    pub element: EntityId,
    /// `ConnectionGeometry`, if the boundary has a shape.
    pub connection_geometry: Option<EntityId>,
    /// `PhysicalOrVirtualBoundary`. Constrained by `CorrectPhysOrVirt`.
    pub physical_or_virtual: &'a str,
    /// `InternalOrExternalBoundary`.
    pub internal_or_external: &'a str,
    /// `ParentBoundary`. First and second levels only.
    pub parent: Option<EntityId>,
    /// `CorrespondingBoundary`. Second level only.
    pub corresponding: Option<EntityId>,
}

const PHYS_OR_VIRT: &[&str] = &["PHYSICAL", "VIRTUAL", "NOTDEFINED"];
const INT_OR_EXT: &[&str] = &[
    "INTERNAL",
    "EXTERNAL",
    "EXTERNAL_EARTH",
    "EXTERNAL_WATER",
    "EXTERNAL_FIRE",
    "NOTDEFINED",
];

/// Resolve the type name of a staged or committed entity.
fn type_name<'a>(tx: &'a Transaction, model: &'a Model, id: EntityId) -> Option<&'a str> {
    tx.edits()
        .iter()
        .rev()
        .find_map(|edit| match edit {
            Edit::Create { id: staged, entity } if *staged == id => Some(entity.type_name.as_ref()),
            _ => None,
        })
        .or_else(|| model.get(id).map(|entity| entity.type_name.as_ref()))
}

/// Stage an `IfcRelSpaceBoundary` at the requested level.
///
/// # Errors
///
/// Refuses a malformed GlobalId, an unknown enum token, a `parent`
/// or `corresponding` reference on a level that does not declare the
/// slot, and a physical/virtual flag that contradicts the bounding
/// element (`CorrectPhysOrVirt`).
pub fn create_space_boundary(
    tx: &mut Transaction,
    model: &Model,
    level: BoundaryLevel,
    global_id: &str,
    draft: BoundaryDraft<'_>,
) -> SpatialAuthoringResult<EntityId> {
    let entity = level.type_name();
    if Guid::parse(global_id).is_none() {
        return Err(invalid(entity, "GlobalId", global_id));
    }
    let physical = draft.physical_or_virtual.to_ascii_uppercase();
    if !PHYS_OR_VIRT.contains(&physical.as_str()) {
        return Err(invalid(
            entity,
            "PhysicalOrVirtualBoundary",
            draft.physical_or_virtual,
        ));
    }
    let internal = draft.internal_or_external.to_ascii_uppercase();
    if !INT_OR_EXT.contains(&internal.as_str()) {
        return Err(invalid(
            entity,
            "InternalOrExternalBoundary",
            draft.internal_or_external,
        ));
    }

    // CorrectPhysOrVirt: the flag and the element must agree. A
    // physical boundary cannot be bounded by a virtual element, and a
    // virtual one only by a virtual element or an opening. NOTDEFINED
    // is unconstrained.
    let bounded_by = type_name(tx, model, draft.element)
        .map(str::to_ascii_uppercase)
        .unwrap_or_default();
    let is_virtual = bounded_by == "IFCVIRTUALELEMENT";
    let is_opening = bounded_by == "IFCOPENINGELEMENT";
    let agrees = match physical.as_str() {
        "PHYSICAL" => !is_virtual,
        "VIRTUAL" => is_virtual || is_opening,
        _ => true,
    };
    if !agrees {
        return Err(invalid(
            entity,
            "PhysicalOrVirtualBoundary",
            format!("{physical} does not agree with {bounded_by}"),
        ));
    }

    // A slot the level does not declare cannot be filled: writing it
    // would land past the end of the record.
    if draft.parent.is_some() && level == BoundaryLevel::Base {
        return Err(invalid(
            entity,
            "ParentBoundary",
            "not declared at this level",
        ));
    }
    if draft.corresponding.is_some() && level != BoundaryLevel::Second {
        return Err(invalid(
            entity,
            "CorrespondingBoundary",
            "not declared at this level",
        ));
    }

    let mut attributes = vec![Value::Null; level.arity()];
    attributes[0] = Value::Text(global_id.into());
    attributes[2] = draft.name.map_or(Value::Null, |t| Value::Text(t.into()));
    attributes[3] = draft
        .description
        .map_or(Value::Null, |t| Value::Text(t.into()));
    attributes[4] = Value::Ref(draft.space);
    attributes[5] = Value::Ref(draft.element);
    attributes[6] = draft.connection_geometry.map_or(Value::Null, Value::Ref);
    attributes[7] = Value::Enum(physical.into());
    attributes[8] = Value::Enum(internal.into());
    if let Some(parent) = draft.parent {
        attributes[9] = Value::Ref(parent);
    }
    if let Some(corresponding) = draft.corresponding {
        attributes[10] = Value::Ref(corresponding);
    }
    Ok(tx.create(Entity::new(entity, attributes)))
}

const CONNECTION_TYPE: &[&str] = &["ATPATH", "ATSTART", "ATEND", "NOTDEFINED"];

/// Stage an `IfcRelConnectsPathElements`.
///
/// Priorities rank which element wins where two path elements meet.
/// `NormalizedRelatingPriorities` and its related twin bound every
/// entry to 0..=100; an out-of-range entry is a ranking the schema
/// cannot express, so it is refused rather than clamped.
///
/// # Errors
///
/// Refuses a malformed GlobalId, an element connected to itself, an
/// unknown connection-type token, and a priority outside 0..=100.
pub fn connect_path_elements(
    tx: &mut Transaction,
    global_id: &str,
    relating: EntityId,
    related: EntityId,
    priorities: (&[i64], &[i64]),
    connection_types: (&str, &str),
) -> SpatialAuthoringResult<EntityId> {
    const ENTITY: &str = "IFCRELCONNECTSPATHELEMENTS";
    if Guid::parse(global_id).is_none() {
        return Err(invalid(ENTITY, "GlobalId", global_id));
    }
    if relating == related {
        return Err(invalid(ENTITY, "RelatedElement", "is the relating element"));
    }
    let (relating_priorities, related_priorities) = priorities;
    for (values, attribute) in [
        (relating_priorities, "RelatingPriorities"),
        (related_priorities, "RelatedPriorities"),
    ] {
        if let Some(out) = values.iter().find(|value| !(0..=100).contains(*value)) {
            return Err(invalid(ENTITY, attribute, out.to_string()));
        }
    }
    let (relating_type, related_type) = connection_types;
    let relating_token = relating_type.to_ascii_uppercase();
    let related_token = related_type.to_ascii_uppercase();
    for (token, attribute) in [
        (&relating_token, "RelatingConnectionType"),
        (&related_token, "RelatedConnectionType"),
    ] {
        if !CONNECTION_TYPE.contains(&token.as_str()) {
            return Err(invalid(ENTITY, attribute, token.clone()));
        }
    }

    let integers =
        |values: &[i64]| Value::List(values.iter().copied().map(Value::Integer).collect());
    let mut attributes = vec![Value::Null; 11];
    attributes[0] = Value::Text(global_id.into());
    attributes[5] = Value::Ref(relating);
    attributes[6] = Value::Ref(related);
    attributes[7] = integers(relating_priorities);
    attributes[8] = integers(related_priorities);
    attributes[9] = Value::Enum(related_token.into());
    attributes[10] = Value::Enum(relating_token.into());
    Ok(tx.create(Entity::new(ENTITY, attributes)))
}
