//! The operations a batch carries.

use ifc_model::{EntityId, Value};

/// Attribute values by name, in the order the caller set them. Names match
/// ASCII case-insensitively, as EXPRESS identifiers do; a name set twice
/// is refused rather than silently overwritten.
pub type NamedValues = Vec<(String, Value)>;

/// One operation of a checked authoring batch ([`apply_authoring`]).
///
/// Every entity reference may name an entity of the model or, through
/// [`authoring_handle`], the entity an earlier operation of the same batch
/// produced. Each operation produces at most one entity: the one named in
/// its variant's documentation, reported in
/// [`AuthoringOutcome::ids`](super::AuthoringOutcome::ids).
///
/// [`apply_authoring`]: super::apply_authoring
/// [`authoring_handle`]: super::authoring_handle
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum AuthorOp {
    /// Create one entity of `type_name` from named attributes, checked
    /// against the declared release. An `IfcRoot` without a `GlobalId` gets
    /// a fresh one. Produces the entity.
    Create {
        /// The entity type, any case (`IfcWall`).
        type_name: String,
        /// Its attributes by name.
        attributes: NamedValues,
    },
    /// Replace named attributes of an existing entity; the whole entity is
    /// checked again afterwards. Produces the entity.
    Edit {
        /// The entity to edit.
        entity: EntityId,
        /// The attributes to replace, by name.
        attributes: NamedValues,
    },
    /// Remove an entity together with the relationships that reference it:
    /// it is taken out of every `IfcRelationship` aggregate holding it, and
    /// a relationship left without an end is removed too. Produces nothing.
    Remove {
        /// The entity to remove.
        entity: EntityId,
    },
    /// Create the model's `IfcProject`; a model holds one. Produces it.
    Project {
        /// Its attributes by name (`Name`, `UnitsInContext`,
        /// `RepresentationContexts`, ...).
        attributes: NamedValues,
        /// `OwnerHistory`, which IFC2X3 requires.
        owner_history: Option<EntityId>,
    },
    /// Create a spatial element (`IfcSite`, `IfcBuilding`,
    /// `IfcBuildingStorey`, `IfcSpace`, ...) and aggregate it under
    /// `parent` with an `IfcRelAggregates`. Produces the element.
    Spatial {
        /// The element type.
        type_name: String,
        /// The `IfcProject` or spatial element it decomposes.
        parent: EntityId,
        /// Its attributes by name.
        attributes: NamedValues,
        /// `ObjectPlacement`, an `IfcObjectPlacement`.
        placement: Option<EntityId>,
        /// `OwnerHistory` of the element and its relationship.
        owner_history: Option<EntityId>,
    },
    /// Create a product (`IfcWall`, ...), contain it in `container` with an
    /// `IfcRelContainedInSpatialStructure`, and type it by `type_object`
    /// with an `IfcRelDefinesByType`. Produces the product.
    Product {
        /// The product type; not a spatial element.
        type_name: String,
        /// The spatial structure element containing it.
        container: Option<EntityId>,
        /// Its attributes by name.
        attributes: NamedValues,
        /// `ObjectPlacement`, an `IfcObjectPlacement`.
        placement: Option<EntityId>,
        /// The `IfcTypeObject` it is typed by.
        type_object: Option<EntityId>,
        /// `OwnerHistory` of the product and its relationships.
        owner_history: Option<EntityId>,
    },
    /// Create a type object (`IfcWallType`, ...). Produces it.
    TypeObject {
        /// The type object's type.
        type_name: String,
        /// Its attributes by name.
        attributes: NamedValues,
        /// `OwnerHistory`, which IFC2X3 requires.
        owner_history: Option<EntityId>,
    },
    /// Type `objects` by `type_object` with one `IfcRelDefinesByType`; an
    /// object already typed is refused. Produces the relationship.
    AssignType {
        /// The `IfcTypeObject`.
        type_object: EntityId,
        /// The objects it types; at least one.
        objects: Vec<EntityId>,
        /// `OwnerHistory` of the relationship.
        owner_history: Option<EntityId>,
    },
    /// Contain `elements` in `structure` with one
    /// `IfcRelContainedInSpatialStructure`; an element already contained
    /// is refused. Produces the relationship.
    Contain {
        /// The spatial structure element.
        structure: EntityId,
        /// The elements; at least one.
        elements: Vec<EntityId>,
        /// `OwnerHistory` of the relationship.
        owner_history: Option<EntityId>,
    },
    /// Decompose `parent` into `parts` with one `IfcRelAggregates`; a part
    /// already aggregated is refused. Produces the relationship.
    Aggregate {
        /// The whole.
        parent: EntityId,
        /// The parts; at least one.
        parts: Vec<EntityId>,
        /// `OwnerHistory` of the relationship.
        owner_history: Option<EntityId>,
    },
    /// Create an `IfcLocalPlacement` with its `IfcAxis2Placement3D`,
    /// `IfcCartesianPoint` and, when given, `IfcDirection`s. Produces the
    /// local placement.
    Placement {
        /// `PlacementRelTo`, the `IfcObjectPlacement` it is relative to;
        /// absolute when `None`.
        relative_to: Option<EntityId>,
        /// The origin, three finite coordinates.
        location: [f64; 3],
        /// Local Z and local X; both or neither (`AxisAndRefDirProvision`),
        /// not parallel.
        axes: Option<([f64; 3], [f64; 3])>,
    },
    /// Create an `IfcOwnerHistory` with the person, organization and
    /// application it records, through `ifc-author`. Produces the owner
    /// history.
    OwnerHistory(OwnerHistoryOp),
}

/// The fields of [`AuthorOp::OwnerHistory`].
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct OwnerHistoryOp {
    /// `IfcPerson.Identification` (IFC2X3 `Id`).
    pub person_identification: Option<String>,
    /// `IfcPerson.FamilyName`.
    pub family_name: Option<String>,
    /// `IfcPerson.GivenName`.
    pub given_name: Option<String>,
    /// `IfcOrganization.Name`: the person's organization and the
    /// application's developer.
    pub organization: String,
    /// `IfcApplication.ApplicationFullName`.
    pub application_name: String,
    /// `IfcApplication.Version`.
    pub application_version: String,
    /// `IfcApplication.ApplicationIdentifier`.
    pub application_identifier: String,
    /// `IfcOwnerHistory.ChangeAction`, an `IfcChangeActionEnum` constant;
    /// IFC2X3 requires it.
    pub change_action: Option<String>,
    /// `IfcOwnerHistory.CreationDate`, seconds since 1970.
    pub creation_date: i64,
    /// `IfcOwnerHistory.LastModifiedDate`.
    pub last_modified_date: Option<i64>,
}

impl OwnerHistoryOp {
    /// An owner history with its required fields; the rest unset.
    #[must_use]
    pub fn new(
        organization: impl Into<String>,
        application_name: impl Into<String>,
        application_version: impl Into<String>,
        application_identifier: impl Into<String>,
        creation_date: i64,
    ) -> Self {
        Self {
            organization: organization.into(),
            application_name: application_name.into(),
            application_version: application_version.into(),
            application_identifier: application_identifier.into(),
            creation_date,
            ..Self::default()
        }
    }
}

impl AuthorOp {
    /// Every entity id the operation names outside its attribute values,
    /// mutably, so a handle can be resolved in place.
    pub(super) fn ids_mut(&mut self) -> Vec<&mut EntityId> {
        let mut ids: Vec<&mut EntityId> = Vec::new();
        match self {
            Self::Create { .. } | Self::OwnerHistory(_) => {}
            Self::Edit { entity, .. } | Self::Remove { entity } => ids.push(entity),
            Self::Project { owner_history, .. } | Self::TypeObject { owner_history, .. } => {
                ids.extend(owner_history.as_mut());
            }
            Self::Spatial {
                parent,
                placement,
                owner_history,
                ..
            } => {
                ids.push(parent);
                ids.extend(placement.as_mut());
                ids.extend(owner_history.as_mut());
            }
            Self::Product {
                container,
                placement,
                type_object,
                owner_history,
                ..
            } => {
                ids.extend(container.as_mut());
                ids.extend(placement.as_mut());
                ids.extend(type_object.as_mut());
                ids.extend(owner_history.as_mut());
            }
            Self::AssignType {
                type_object: one,
                objects: many,
                owner_history,
            }
            | Self::Contain {
                structure: one,
                elements: many,
                owner_history,
            }
            | Self::Aggregate {
                parent: one,
                parts: many,
                owner_history,
            } => {
                ids.push(one);
                ids.extend(many.iter_mut());
                ids.extend(owner_history.as_mut());
            }
            Self::Placement { relative_to, .. } => ids.extend(relative_to.as_mut()),
        }
        ids
    }

    /// The attribute values the operation carries, mutably.
    pub(super) fn values_mut(&mut self) -> Option<&mut NamedValues> {
        match self {
            Self::Create { attributes, .. }
            | Self::Edit { attributes, .. }
            | Self::Project { attributes, .. }
            | Self::Spatial { attributes, .. }
            | Self::Product { attributes, .. }
            | Self::TypeObject { attributes, .. } => Some(attributes),
            _ => None,
        }
    }
}
