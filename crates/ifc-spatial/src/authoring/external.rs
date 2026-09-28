//! Staging the external spatial element and the project library.
//!
//! # Two records that are not spatial containers
//!
//! `IfcExternalSpatialElement` is a product: it carries a placement
//! and a representation, and describes space *outside* the building
//! envelope -- the air a facade radiates into, the earth a
//! foundation sits in. `IfcProjectLibrary` is an `IfcContext`,
//! sharing `IfcProject`'s tail rather than a container's.
//!
//! Neither is an `IfcSpatialStructureElement`, so neither can use
//! the container path: slot 5 onward means something different in
//! each of the three shapes and sharing one path would file a
//! placement as a long name.

use ifc_model::guid::Guid;
use ifc_model::{Entity, EntityId, Transaction, Value};

use super::{invalid, optional_text, SpatialAuthoringResult};

/// `IfcExternalSpatialElementTypeEnum`.
///
/// Closed: a token outside it names a kind of exterior space the
/// schema does not define.
const EXTERNAL_KIND: &[&str] = &[
    "EXTERNAL",
    "EXTERNAL_EARTH",
    "EXTERNAL_FIRE",
    "EXTERNAL_WATER",
    "USERDEFINED",
    "NOTDEFINED",
];

/// Attributes of an `IfcExternalSpatialElement`.
///
/// `#[non_exhaustive]`: build it with [`ExternalSpatialDraft::new`] and the
/// setters.
#[derive(Debug, Clone, Copy, Default)]
#[non_exhaustive]
pub struct ExternalSpatialDraft<'a> {
    /// `Name`.
    pub name: Option<&'a str>,
    /// `Description`.
    pub description: Option<&'a str>,
    /// `ObjectType`. Required when `predefined_type` is `USERDEFINED`.
    pub object_type: Option<&'a str>,
    /// `ObjectPlacement`.
    pub placement: Option<EntityId>,
    /// `Representation`.
    pub representation: Option<EntityId>,
    /// `LongName`.
    pub long_name: Option<&'a str>,
    /// `PredefinedType`, an `IfcExternalSpatialElementTypeEnum` token.
    pub predefined_type: Option<&'a str>,
}

impl<'a> ExternalSpatialDraft<'a> {
    /// An empty draft: every attribute unset.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set `Name`.
    #[must_use]
    pub fn name(mut self, value: &'a str) -> Self {
        self.name = Some(value);
        self
    }

    /// Set `Description`.
    #[must_use]
    pub fn description(mut self, value: &'a str) -> Self {
        self.description = Some(value);
        self
    }

    /// Set `ObjectType`; required when `predefined_type` is `USERDEFINED`.
    #[must_use]
    pub fn object_type(mut self, value: &'a str) -> Self {
        self.object_type = Some(value);
        self
    }

    /// Set `ObjectPlacement`.
    #[must_use]
    pub fn placement(mut self, value: EntityId) -> Self {
        self.placement = Some(value);
        self
    }

    /// Set `Representation`.
    #[must_use]
    pub fn representation(mut self, value: EntityId) -> Self {
        self.representation = Some(value);
        self
    }

    /// Set `LongName`.
    #[must_use]
    pub fn long_name(mut self, value: &'a str) -> Self {
        self.long_name = Some(value);
        self
    }

    /// Set `PredefinedType`, an `IfcExternalSpatialElementTypeEnum` token.
    #[must_use]
    pub fn predefined_type(mut self, value: &'a str) -> Self {
        self.predefined_type = Some(value);
        self
    }
}

/// Stage an `IfcExternalSpatialElement`.
///
/// IFC4 and IFC4X3 only: it writes their layout and leaves
/// `OwnerHistory` `$`, which IFC2X3 requires. In IFC2X3 use
/// [`create_external_spatial_element_with_owner_history`](super::create_external_spatial_element_with_owner_history), which binds the model's declared
/// release.
///
/// # Errors
///
/// Refuses a malformed GlobalId, a token outside
/// `IfcExternalSpatialElementTypeEnum`, and `USERDEFINED` without
/// `ObjectType`.
pub fn create_external_spatial_element(
    tx: &mut Transaction,
    global_id: &str,
    draft: ExternalSpatialDraft<'_>,
) -> SpatialAuthoringResult<EntityId> {
    check_external(global_id, &draft)?;
    let mut attributes = vec![Value::Null; 9];
    attributes[0] = Value::Text(global_id.into());
    attributes[2] = optional_text(draft.name);
    attributes[3] = optional_text(draft.description);
    attributes[4] = optional_text(draft.object_type);
    attributes[5] = draft.placement.map_or(Value::Null, Value::Ref);
    attributes[6] = draft.representation.map_or(Value::Null, Value::Ref);
    attributes[7] = optional_text(draft.long_name);
    attributes[8] = draft
        .predefined_type
        .map_or(Value::Null, |t| Value::Enum(t.into()));
    Ok(tx.create(Entity::new(EXTERNAL, attributes)))
}

pub(super) const EXTERNAL: &str = "IFCEXTERNALSPATIALELEMENT";

/// The checks of [`create_external_spatial_element`].
pub(super) fn check_external(
    global_id: &str,
    draft: &ExternalSpatialDraft<'_>,
) -> SpatialAuthoringResult<()> {
    if Guid::parse(global_id).is_none() {
        return Err(invalid(EXTERNAL, "GlobalId", global_id));
    }
    if let Some(token) = draft.predefined_type {
        if !EXTERNAL_KIND.contains(&token) {
            return Err(invalid(EXTERNAL, "PredefinedType", token));
        }
        if token == "USERDEFINED"
            && draft
                .object_type
                .is_none_or(|value| value.trim().is_empty())
        {
            return Err(invalid(EXTERNAL, "ObjectType", "required by USERDEFINED"));
        }
    }
    Ok(())
}

/// Attributes of an `IfcProjectLibrary`.
///
/// `#[non_exhaustive]`: build it with [`ProjectLibraryDraft::new`] and the
/// setters.
#[derive(Debug, Clone, Copy, Default)]
#[non_exhaustive]
pub struct ProjectLibraryDraft<'a> {
    /// `Name`.
    pub name: Option<&'a str>,
    /// `Description`.
    pub description: Option<&'a str>,
    /// `ObjectType`.
    pub object_type: Option<&'a str>,
    /// `LongName`.
    pub long_name: Option<&'a str>,
    /// `Phase`.
    pub phase: Option<&'a str>,
    /// `UnitsInContext`, an `IfcUnitAssignment`.
    pub units: Option<EntityId>,
}

impl<'a> ProjectLibraryDraft<'a> {
    /// An empty draft: every attribute unset.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set `Name`.
    #[must_use]
    pub fn name(mut self, value: &'a str) -> Self {
        self.name = Some(value);
        self
    }

    /// Set `Description`.
    #[must_use]
    pub fn description(mut self, value: &'a str) -> Self {
        self.description = Some(value);
        self
    }

    /// Set `ObjectType`.
    #[must_use]
    pub fn object_type(mut self, value: &'a str) -> Self {
        self.object_type = Some(value);
        self
    }

    /// Set `LongName`.
    #[must_use]
    pub fn long_name(mut self, value: &'a str) -> Self {
        self.long_name = Some(value);
        self
    }

    /// Set `Phase`.
    #[must_use]
    pub fn phase(mut self, value: &'a str) -> Self {
        self.phase = Some(value);
        self
    }

    /// Set `UnitsInContext`, an `IfcUnitAssignment`.
    #[must_use]
    pub fn units(mut self, value: EntityId) -> Self {
        self.units = Some(value);
        self
    }
}

/// Stage an `IfcProjectLibrary`.
///
/// A library holds shared definitions -- types, properties,
/// materials -- for reuse across projects. It is an `IfcContext`
/// like `IfcProject`, not a container, so `RepresentationContexts`
/// and `UnitsInContext` sit where a container keeps its placement.
///
/// `RepresentationContexts` is `SET [1:?]`: absent is legal,
/// present and empty is not.
///
/// IFC4 and IFC4X3 only: it writes their layout and leaves
/// `OwnerHistory` `$`, which IFC2X3 requires. In IFC2X3 use
/// [`create_project_library_with_owner_history`](super::create_project_library_with_owner_history), which binds the model's declared
/// release.
///
/// # Errors
///
/// Refuses a malformed GlobalId and an empty representation-context set.
pub fn create_project_library(
    tx: &mut Transaction,
    global_id: &str,
    draft: ProjectLibraryDraft<'_>,
    representation_contexts: &[EntityId],
) -> SpatialAuthoringResult<EntityId> {
    const ENTITY: &str = "IFCPROJECTLIBRARY";
    if Guid::parse(global_id).is_none() {
        return Err(invalid(ENTITY, "GlobalId", global_id));
    }
    let mut attributes = vec![Value::Null; 9];
    attributes[0] = Value::Text(global_id.into());
    attributes[2] = optional_text(draft.name);
    attributes[3] = optional_text(draft.description);
    attributes[4] = optional_text(draft.object_type);
    attributes[5] = optional_text(draft.long_name);
    attributes[6] = optional_text(draft.phase);
    if !representation_contexts.is_empty() {
        attributes[7] = Value::List(
            representation_contexts
                .iter()
                .copied()
                .map(Value::Ref)
                .collect(),
        );
    }
    attributes[8] = draft.units.map_or(Value::Null, Value::Ref);
    Ok(tx.create(Entity::new(ENTITY, attributes)))
}
