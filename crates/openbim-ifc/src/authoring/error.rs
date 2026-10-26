//! Why a batch was refused.

use std::fmt;

use ifc_model::{Conflict, EntityId};

/// A refused batch: which operation failed, and why. Nothing was written.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct AuthoringError {
    /// The operation's position in the batch; `None` for a failure of the
    /// batch as a whole (the declared release, the commit).
    pub op: Option<usize>,
    /// What was refused; boxed, as the variants carry their diagnostics.
    pub failure: Box<AuthoringFailure>,
}

/// What an [`AuthoringError`] refused.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum AuthoringFailure {
    /// The declared release has no entity of this name.
    UnknownEntity {
        /// The release, e.g. `IFC4`.
        schema: String,
        /// The type as asked for.
        type_name: String,
    },
    /// The entity type is `ABSTRACT`: no instance of it can exist.
    AbstractEntity {
        /// The type, in the schema's spelling.
        type_name: String,
    },
    /// An operation's type argument is not of the kind it builds, such as
    /// a `Product` of `IfcBuildingStorey`.
    NotA {
        /// The type as asked for.
        type_name: String,
        /// The kind the operation builds, e.g. `IfcProduct`.
        expected: String,
    },
    /// `ifc-author` refused the record: an unknown, duplicate, derived or
    /// missing attribute, or a value of the wrong type or form.
    Author(ifc_author::AuthorError),
    /// An aggregate value breaks its declared bounds or uniqueness.
    Cardinality {
        /// The entity being written.
        entity: String,
        /// The attribute.
        attribute: String,
        /// What is wrong, e.g. `0 items; at least 1`.
        detail: String,
    },
    /// The entity an operation edits or removes does not exist.
    MissingEntity(EntityId),
    /// A value references an entity that does not exist.
    MissingReference {
        /// The entity being written.
        entity: String,
        /// The attribute holding the reference.
        attribute: String,
        /// The missing entity.
        target: EntityId,
    },
    /// A value references an entity of a type the attribute does not
    /// accept.
    WrongReferenceType {
        /// The entity being written.
        entity: String,
        /// The attribute holding the reference.
        attribute: String,
        /// The referenced entity.
        target: EntityId,
        /// Its type.
        actual: String,
        /// The declared type.
        expected: String,
    },
    /// A `GlobalId` another entity already holds.
    DuplicateGlobalId {
        /// The `GlobalId`.
        global_id: String,
        /// The entity holding it.
        holder: EntityId,
    },
    /// A `GlobalId` derived from the batch seed that an entity of the model
    /// already holds: the seed named an earlier batch (#548). Draw one seed
    /// per batch ([`fresh_seed`](crate::fresh_seed)).
    SeedReused {
        /// The derived `GlobalId`.
        global_id: String,
        /// The entity holding it.
        holder: EntityId,
    },
    /// The operation would give an object a second relationship the
    /// schema allows once: a second containment (`ContainedInStructure`),
    /// decomposition (`Decomposes`) or type (`IsTypedBy`), or a second
    /// `IfcProject`.
    AlreadyRelated {
        /// The object.
        id: EntityId,
        /// The relationship type, e.g. `IFCRELCONTAINEDINSPATIALSTRUCTURE`.
        relationship: &'static str,
        /// The relationship (or project) it already has.
        existing: EntityId,
    },
    /// A relationship would relate an entity to itself, or to nothing.
    InvalidRelationship {
        /// The relationship type.
        relationship: &'static str,
        /// What is wrong.
        detail: &'static str,
    },
    /// A removal is blocked: an entity that is not a relationship still
    /// references the target. Remove or edit the referrer first.
    StillReferenced {
        /// The entity being removed.
        id: EntityId,
        /// The entity referencing it.
        by: EntityId,
    },
    /// A product's placement cannot go: other placements, which stay, are
    /// relative to it. Re-base them
    /// ([`RemoveProductOptions::rebase_placements`](super::RemoveProductOptions::rebase_placements))
    /// or remove their products first.
    PlacementHasDependents {
        /// The placement.
        placement: EntityId,
        /// The placements relative to it that stay.
        dependents: Vec<EntityId>,
    },
    /// A type object still types objects. Untype them
    /// ([`RemoveProductOptions::untype_occurrences`](super::RemoveProductOptions::untype_occurrences))
    /// or remove them first.
    TypeHasOccurrences {
        /// The type object.
        type_object: EntityId,
        /// The objects it types.
        occurrences: Vec<EntityId>,
    },
    /// A dependent placement cannot be re-based: a placement on the way is
    /// not an `IfcLocalPlacement` over an `IfcAxis2Placement3D` with
    /// Cartesian points and directions.
    UnsupportedRebase {
        /// The placement that could not be read or composed.
        placement: EntityId,
        /// What it is instead.
        detail: String,
    },
    /// Mapping a type's representations would clash with one the object
    /// already has: the same `RepresentationIdentifier` (or, both unnamed,
    /// the same context), or a product definition shape another product
    /// shares (#480).
    RepresentationConflict {
        /// The object.
        object: EntityId,
        /// The representation, or shared product definition shape, in the
        /// way.
        existing: EntityId,
        /// What clashes.
        detail: String,
    },
    /// A type's `IfcRepresentationMap` cannot be mapped onto an object:
    /// its `MappedRepresentation` is not an `IfcShapeRepresentation`, or
    /// its `MappingOrigin` is neither an `IfcAxis2Placement3D` nor an
    /// `IfcAxis2Placement2D` (#480).
    UnsupportedRepresentationMap {
        /// The representation map.
        map: EntityId,
        /// What it holds instead.
        detail: String,
    },
    /// A material set usage cannot be derived from the type with
    /// documented defaults: the type has several material associations
    /// or carries a usage instead of a set, or the object's class has no
    /// documented `LayerSetDirection` and none was given (#480).
    MaterialUsageUndetermined {
        /// The object, or the type when the type's material is at fault.
        object: EntityId,
        /// Why.
        detail: String,
    },
    /// An object's own `PredefinedType` contradicts the one its type
    /// provides (#480).
    PredefinedTypeConflict {
        /// The object.
        object: EntityId,
        /// Its value (with `ObjectType` for `USERDEFINED`).
        object_value: String,
        /// The type's value (with `ElementType` for `USERDEFINED`).
        type_value: String,
    },
    /// A handle names an operation that has not run yet or produced no
    /// entity.
    InvalidHandle {
        /// The handle as passed.
        handle: u64,
        /// What is wrong.
        detail: String,
    },
    /// Placement coordinates or directions the schema cannot hold.
    InvalidPlacement {
        /// What is wrong.
        detail: String,
    },
    /// The model's ids reach the handle range, so a reference could not
    /// be told from a handle.
    IdRangeExhausted {
        /// The model's next free id.
        next_id: u64,
    },
    /// The planned transaction failed its preflight.
    Conflict(Vec<Conflict>),
    /// Items a shape representation's `RepresentationType` does not admit,
    /// by the declared release's `IfcShapeRepresentationTypes`
    /// (`CorrectItemsForType`, IFC2X3 `WR24`).
    RepresentationTypeMismatch {
        /// The type as given.
        representation_type: String,
        /// The items it does not admit; every item for a `BoundingBox`
        /// of more than one.
        items: Vec<EntityId>,
        /// What the type admits.
        expected: String,
    },
    /// No `RepresentationType` the declared release lists admits every
    /// item, so none can be inferred; name one, or split the items.
    NoRepresentationType {
        /// The items.
        items: Vec<EntityId>,
    },
    /// Checking the items against the `RepresentationType` needs a
    /// dimension (`IfcCurve.Dim`) the release derives in a way the batch
    /// does not read; refused rather than assumed.
    UndecidableRepresentationType {
        /// The type as given.
        representation_type: String,
        /// The item whose dimension is not known.
        item: EntityId,
    },
    /// IFC4 and later require a shape representation's
    /// `RepresentationIdentifier` (`HasRepresentationIdentifier`), and none
    /// was given nor does the context carry a `ContextIdentifier`.
    MissingRepresentationIdentifier {
        /// The representation context.
        context: EntityId,
    },
    /// An item a shape representation cannot hold: a topological item
    /// other than a vertex point, edge curve or face surface
    /// (`NoTopologicalItem`).
    InvalidRepresentationItem {
        /// The item.
        item: EntityId,
        /// What is wrong.
        detail: String,
    },
    /// The product's `IfcProductDefinitionShape` is shared with other
    /// products; changing it would change their shape too.
    SharedProductShape {
        /// The product definition shape.
        shape: EntityId,
        /// Every product it is the shape of.
        products: Vec<EntityId>,
    },
    /// The representation is not assigned to that product or type.
    NotAssigned {
        /// The representation.
        representation: EntityId,
        /// The product or type.
        to: EntityId,
    },
    /// A copy cannot be made as asked (#479): an offset onto a placement
    /// that is not an `IfcLocalPlacement` with a Cartesian `Location`, a
    /// moved copy whose openings, fillings or ports are not placed relative
    /// to it, or references that form a cycle.
    UnsupportedCopy {
        /// The entity that cannot be copied so.
        entity: EntityId,
        /// Why.
        detail: String,
    },
    /// An entity cannot be appended into the target model's release
    /// (#479): the target release does not declare its type, lacks an
    /// attribute it sets, or does not admit one of its values. Nothing is
    /// dropped silently; migrate the source model first.
    IncompatibleRelease {
        /// The entity, in the source model.
        entity: EntityId,
        /// Its type.
        type_name: String,
        /// The source model's release.
        source: String,
        /// The target model's release.
        target: String,
        /// What the target lacks.
        detail: String,
    },
    /// The source and target models measure in different units (#479):
    /// appended coordinates and values would change meaning. Convert one
    /// model's length unit first (`convert_length_unit`, feature `properties`, #469);
    /// the append never converts on its own.
    UnitMismatch {
        /// The `IfcUnitEnum` constant, e.g. `LENGTHUNIT`.
        unit_type: String,
        /// The source model's unit, e.g. `MILLI METRE`; `none` when it
        /// declares none.
        source: String,
        /// The target model's unit.
        target: String,
    },
}

impl AuthoringError {
    pub(super) fn at(op: Option<usize>, failure: AuthoringFailure) -> Self {
        Self {
            op,
            failure: Box::new(failure),
        }
    }
}

impl fmt::Display for AuthoringFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownEntity { schema, type_name } => {
                write!(f, "{schema} does not declare entity `{type_name}`")
            }
            Self::AbstractEntity { type_name } => {
                write!(f, "`{type_name}` is abstract and cannot be instantiated")
            }
            Self::NotA {
                type_name,
                expected,
            } => write!(f, "`{type_name}` is not an {expected}"),
            Self::Author(error) => error.fmt(f),
            Self::Cardinality {
                entity,
                attribute,
                detail,
            } => write!(f, "`{entity}.{attribute}`: {detail}"),
            Self::MissingEntity(id) => write!(f, "no entity {id}"),
            Self::MissingReference {
                entity,
                attribute,
                target,
            } => write!(f, "`{entity}.{attribute}` references missing {target}"),
            Self::WrongReferenceType {
                entity,
                attribute,
                target,
                actual,
                expected,
            } => write!(
                f,
                "`{entity}.{attribute}` references {target}, an {actual}; it accepts {expected}"
            ),
            Self::DuplicateGlobalId { global_id, holder } => {
                write!(f, "GlobalId `{global_id}` is already held by {holder}")
            }
            Self::SeedReused { global_id, holder } => write!(
                f,
                "GlobalId `{global_id}` derived from the batch seed is already held by {holder}; the seed names an earlier batch: draw one seed per batch"
            ),
            Self::AlreadyRelated {
                id,
                relationship,
                existing,
            } => write!(
                f,
                "{id} already has its one {relationship} ({existing}); the schema allows one"
            ),
            Self::InvalidRelationship {
                relationship,
                detail,
            } => write!(f, "{relationship}: {detail}"),
            Self::StillReferenced { id, by } => write!(
                f,
                "{id} is still referenced by {by}, which is not a relationship; \
                 remove or edit it first"
            ),
            Self::PlacementHasDependents {
                placement,
                dependents,
            } => write!(
                f,
                "placement {placement} has placements relative to it that stay ({}); \
                 re-base them or remove their products first",
                ids(dependents)
            ),
            Self::TypeHasOccurrences {
                type_object,
                occurrences,
            } => write!(
                f,
                "type {type_object} still types {}; untype them or remove them first",
                ids(occurrences)
            ),
            Self::UnsupportedRebase { placement, detail } => {
                write!(f, "cannot re-base onto placement {placement}: {detail}")
            }
            Self::RepresentationConflict {
                object,
                existing,
                detail,
            } => write!(
                f,
                "cannot map the type's representations onto {object}: {existing} {detail}"
            ),
            Self::UnsupportedRepresentationMap { map, detail } => {
                write!(f, "representation map {map} cannot be mapped: {detail}")
            }
            Self::MaterialUsageUndetermined { object, detail } => {
                write!(f, "cannot derive a material usage for {object}: {detail}")
            }
            Self::PredefinedTypeConflict {
                object,
                object_value,
                type_value,
            } => write!(
                f,
                "{object} has PredefinedType {object_value}, but its type provides {type_value}"
            ),
            Self::InvalidHandle { handle, detail } => {
                write!(f, "handle {handle}: {detail}")
            }
            Self::InvalidPlacement { detail } => write!(f, "placement: {detail}"),
            Self::IdRangeExhausted { next_id } => write!(
                f,
                "the model's next id {next_id} reaches the handle range; \
                 references could not be told from handles"
            ),
            Self::Conflict(conflicts) => write!(
                f,
                "the planned transaction failed its preflight: {conflicts:?}"
            ),
            Self::RepresentationTypeMismatch {
                representation_type,
                items,
                expected,
            } => write!(
                f,
                "RepresentationType '{representation_type}' does not admit {} \
                 (CorrectItemsForType); it admits {expected}",
                ids(items)
            ),
            Self::NoRepresentationType { items } => write!(
                f,
                "no RepresentationType of the release admits every item ({}); name one",
                ids(items)
            ),
            Self::UndecidableRepresentationType {
                representation_type,
                item,
            } => write!(
                f,
                "cannot check RepresentationType '{representation_type}': \
                 the dimension of {item} is not derivable here"
            ),
            Self::MissingRepresentationIdentifier { context } => write!(
                f,
                "a shape representation needs a RepresentationIdentifier \
                 (HasRepresentationIdentifier), and context {context} has no ContextIdentifier"
            ),
            Self::InvalidRepresentationItem { item, detail } => {
                write!(f, "representation item {item}: {detail}")
            }
            Self::SharedProductShape { shape, products } => write!(
                f,
                "product definition shape {shape} is shared by {}; \
                 changing it would change them all",
                ids(products)
            ),
            Self::NotAssigned { representation, to } => {
                write!(f, "representation {representation} is not assigned to {to}")
            }
            Self::UnsupportedCopy { entity, detail } => {
                write!(f, "cannot copy {entity}: {detail}")
            }
            Self::IncompatibleRelease {
                entity,
                type_name,
                source,
                target,
                detail,
            } => write!(
                f,
                "{entity} ({type_name}) cannot go from {source} into {target}: {detail}; \
                 migrate the source model first"
            ),
            Self::UnitMismatch {
                unit_type,
                source,
                target,
            } => write!(
                f,
                "the source model's {unit_type} is {source}, the target's {target}; \
                 convert one model's length unit first (`convert_length_unit`, #469)"
            ),
        }
    }
}

fn ids(ids: &[EntityId]) -> String {
    ids.iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ")
}

impl fmt::Display for AuthoringError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.op {
            Some(op) => write!(f, "op {op}: {}", self.failure),
            None => self.failure.fmt(f),
        }
    }
}

impl std::error::Error for AuthoringError {}
