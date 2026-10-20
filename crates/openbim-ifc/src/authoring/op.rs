//! The operations a batch carries.

use ifc_model::{EntityId, Value};

use super::{
    AssignTypeOptions, CopyPolicy, CopyProductOptions, OwnerHistoryOp, PurgeOptions, RelateOptions,
    RemoveProductOptions,
};

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
    /// Remove a product (or a type object) with everything it solely owns,
    /// as one step of the batch. Produces nothing; the batch outcome
    /// carries a [`RemovalReport`](super::RemovalReport) for it.
    ///
    /// Removed with it: its `ObjectPlacement`, its representation down to
    /// the items no one else uses, the property and quantity sets only it
    /// is defined by with their properties, its material usage (not the
    /// material definitions), its openings (each with the same cascade)
    /// and its ports, and every relationship left without an end. Kept,
    /// and reported with the reason: anything another entity that stays
    /// still uses (a shared property set, a type's representation maps,
    /// a style used elsewhere), related objects (its type, its container,
    /// the fillings of its openings), and definitions the cascade never
    /// deletes (material definitions, classification references, units,
    /// contexts, the owner history).
    ///
    /// Refused with [`PlacementHasDependents`] while another placement is
    /// relative to its placement (unless
    /// [`RemoveProductOptions::rebase_placements`]), with
    /// [`TypeHasOccurrences`] for a type object that still types objects
    /// (unless [`RemoveProductOptions::untype_occurrences`]), and with
    /// [`StillReferenced`] when an entity that stays, and is neither a
    /// relationship nor a layer assignment, still needs something removed.
    ///
    /// [`PlacementHasDependents`]: super::AuthoringFailure::PlacementHasDependents
    /// [`TypeHasOccurrences`]: super::AuthoringFailure::TypeHasOccurrences
    /// [`StillReferenced`]: super::AuthoringFailure::StillReferenced
    RemoveProduct {
        /// The `IfcProduct` or `IfcTypeObject` to remove.
        product: EntityId,
        /// What to do where the removal would otherwise be refused.
        options: RemoveProductOptions,
    },
    /// Remove unreferenced entities. Without classes: every entity that is
    /// not an `IfcRoot` and that no `IfcRoot` reaches through references,
    /// nor annotates something reached (a styled item, a layer
    /// assignment, a shape aspect, a resource relationship, a coordinate
    /// operation). With classes: entities of those classes, subtypes
    /// included, that nothing references once the rest of the purge is
    /// gone. `IfcRoot` entities, the owner history with its actors, and
    /// representation contexts are never purged unless
    /// [`PurgeOptions`] asks. Produces nothing; the batch outcome carries a
    /// [`RemovalReport`](super::RemovalReport) for it. Stage the batch
    /// ([`stage_authoring`](super::stage_authoring)) for a dry run.
    Purge(PurgeOptions),
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
    /// Type `objects` by `type_object` with one `IfcRelDefinesByType`;
    /// an object already typed is refused unless
    /// [`AssignTypeOptions::replace`]. With [`AssignTypeOptions`], the
    /// type is also propagated to each object (#480): its
    /// `RepresentationMaps` mapped onto the object as `IfcMappedItem`
    /// representations, a material set usage derived from its material
    /// set, and `PredefinedType` synchronised or validated. Produces the
    /// relationship.
    AssignType {
        /// The `IfcTypeObject`.
        type_object: EntityId,
        /// The objects it types; at least one.
        objects: Vec<EntityId>,
        /// `OwnerHistory` of the relationship and of every relationship
        /// the propagation creates.
        owner_history: Option<EntityId>,
        /// What to propagate, and whether to replace a previous type.
        options: AssignTypeOptions,
    },
    /// Contain `elements` in `structure` with one
    /// `IfcRelContainedInSpatialStructure`; an element already contained
    /// is refused unless [`RelateOptions::replace`]. Produces the
    /// relationship.
    Contain {
        /// The spatial structure element.
        structure: EntityId,
        /// The elements; at least one.
        elements: Vec<EntityId>,
        /// `OwnerHistory` of the relationship.
        owner_history: Option<EntityId>,
        /// Whether to move an element out of its previous container.
        options: RelateOptions,
    },
    /// Decompose `parent` into `parts` with one `IfcRelAggregates`; a part
    /// already aggregated is refused unless [`RelateOptions::replace`].
    /// Produces the relationship.
    Aggregate {
        /// The whole.
        parent: EntityId,
        /// The parts; at least one.
        parts: Vec<EntityId>,
        /// `OwnerHistory` of the relationship.
        owner_history: Option<EntityId>,
        /// Whether to move a part out of its previous whole.
        options: RelateOptions,
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
    /// Assemble an `IfcShapeRepresentation` of `items` in `context`.
    /// Produces the representation, not yet assigned to anything (see
    /// [`Self::AssignRepresentation`]).
    ///
    /// `representation_type` is checked against the items by the declared
    /// release's `IfcShapeRepresentationTypes` (`CorrectItemsForType`,
    /// IFC2X3 `WR24`); when `None`, the most specific type the release
    /// admits for every item is inferred (`SweptSolid` before
    /// `AdvancedSweptSolid` before `SolidModel`, ...). A label the release
    /// does not list is written as given: its rule does not constrain it.
    /// `identifier` defaults to the context's `ContextIdentifier`
    /// (`Body` for a `Body` sub-context); IFC4 and IFC4X3 require one
    /// (`HasRepresentationIdentifier`), IFC2X3 does not. The context must
    /// be an `IfcGeometricRepresentationContext` (`CorrectContext`), and no
    /// item a topological item other than a vertex point, edge curve or
    /// face surface (`NoTopologicalItem`).
    ///
    /// Refused with [`RepresentationTypeMismatch`],
    /// [`NoRepresentationType`], [`UndecidableRepresentationType`] (a
    /// dimension the release derives that the batch cannot read),
    /// [`MissingRepresentationIdentifier`] or [`InvalidRepresentationItem`].
    ///
    /// [`RepresentationTypeMismatch`]: super::AuthoringFailure::RepresentationTypeMismatch
    /// [`NoRepresentationType`]: super::AuthoringFailure::NoRepresentationType
    /// [`UndecidableRepresentationType`]: super::AuthoringFailure::UndecidableRepresentationType
    /// [`MissingRepresentationIdentifier`]: super::AuthoringFailure::MissingRepresentationIdentifier
    /// [`InvalidRepresentationItem`]: super::AuthoringFailure::InvalidRepresentationItem
    ShapeRepresentation {
        /// `ContextOfItems`: an `IfcGeometricRepresentationContext` or
        /// sub-context.
        context: EntityId,
        /// `Items`, at least one.
        items: Vec<EntityId>,
        /// `RepresentationIdentifier` (`Body`, `Axis`, `Box`, ...); the
        /// context's `ContextIdentifier` when `None`.
        identifier: Option<String>,
        /// `RepresentationType` (`SweptSolid`, `Brep`, ...); inferred from
        /// the items when `None`.
        representation_type: Option<String>,
    },
    /// Assign a shape representation to a product or a type.
    ///
    /// To an `IfcProduct`: appended to its `IfcProductDefinitionShape`,
    /// which is created when the product has none. Produces the product
    /// definition shape. To an `IfcTypeProduct`: wrapped in a new
    /// `IfcRepresentationMap` with its `MappingOrigin` at the origin, and
    /// appended to `RepresentationMaps`. Produces the representation map.
    /// The type's occurrences are not changed: mapping it onto them is the
    /// type assignment's concern.
    ///
    /// A shape representation belongs to exactly one product definition
    /// shape, representation map or shape aspect (`IfcShapeModel.WR11`), so
    /// one already used by any of them is refused with
    /// [`AlreadyRelated`](super::AuthoringFailure::AlreadyRelated). A product
    /// definition shape several products share is refused with
    /// [`SharedProductShape`](super::AuthoringFailure::SharedProductShape):
    /// extending it would change the others.
    AssignRepresentation {
        /// The `IfcShapeModel` (`IfcShapeRepresentation`, ...).
        representation: EntityId,
        /// The `IfcProduct` or `IfcTypeProduct`.
        to: EntityId,
    },
    /// Take a shape representation off a product or a type. Produces the
    /// representation, which stays in the model unassigned: assign it
    /// elsewhere or remove it ([`Self::RemoveRepresentation`]) in the same
    /// batch, as a shape representation used nowhere breaks
    /// `IfcShapeModel.WR11`.
    ///
    /// From a product: removed from its product definition shape; a
    /// product definition shape left empty goes, with its shape aspects
    /// and what only they use, and the product's `Representation` is
    /// unset. From a type: its representation map goes with its
    /// `MappingOrigin` when nothing else uses it, and leaves
    /// `RepresentationMaps` (unset when empty). The batch outcome carries
    /// a [`RemovalReport`](super::RemovalReport) of what went.
    ///
    /// Refused with [`NotAssigned`](super::AuthoringFailure::NotAssigned)
    /// when it is not assigned there, [`SharedProductShape`] for a product
    /// definition shape other products share, and [`StillReferenced`] for
    /// a representation map an `IfcMappedItem` still maps.
    ///
    /// [`SharedProductShape`]: super::AuthoringFailure::SharedProductShape
    /// [`StillReferenced`]: super::AuthoringFailure::StillReferenced
    UnassignRepresentation {
        /// The shape representation.
        representation: EntityId,
        /// The `IfcProduct` or `IfcTypeProduct` it is assigned to.
        from: EntityId,
    },
    /// Copy a shape representation: a new representation with copies of
    /// its items and of everything they are built from, unassigned.
    /// Produces the copy.
    ///
    /// Copied, each once (sharing inside the representation is kept): the
    /// representation, every representation item (geometry, topology,
    /// points, directions, placements, transformation operators, mapped
    /// items, styled items in `Items`) and profile definition it reaches,
    /// and what attaches to a copied item: its `IfcStyledItem`s, and the
    /// colour and texture maps of a tessellated item. Shared, not copied:
    /// the representation context, the `IfcRepresentationMap` a mapped
    /// item maps, presentation styles, colours and textures, and every
    /// other resource (units, materials). The copy joins the presentation
    /// layers the original or its items are on.
    CopyRepresentation {
        /// The `IfcShapeModel` to copy.
        representation: EntityId,
    },
    /// Remove a shape representation with what only it uses, by the
    /// cascade rules of [`Self::RemoveProduct`]. Produces nothing; the
    /// batch outcome carries a [`RemovalReport`](super::RemovalReport).
    ///
    /// It is first taken off what uses it: its product definition shape
    /// (which goes when left empty, unsetting its products'
    /// `Representation`), its representation map (which goes, leaving its
    /// type's `RepresentationMaps`) and its shape aspects (which go when
    /// left empty). Then it goes with its items down to what nothing else
    /// uses: an item, profile, point or style another representation uses
    /// is kept and reported as shared; the context is kept as a
    /// definition; a styled item goes with its item, and a presentation
    /// layer loses it. Refused with [`StillReferenced`] for a
    /// representation map an `IfcMappedItem` still maps.
    ///
    /// [`StillReferenced`]: super::AuthoringFailure::StillReferenced
    RemoveRepresentation {
        /// The `IfcShapeModel` to remove.
        representation: EntityId,
    },
    /// Copy a product, or a type object, with what it solely owns (#479):
    /// what [`Self::RemoveProduct`] would remove with it is copied, what it
    /// would keep is shared. Produces the copy; the batch outcome carries a
    /// [`CopyReport`](super::CopyReport) mapping every original to its copy.
    ///
    /// Copied, with new `GlobalId`s: the product; its `ObjectPlacement`
    /// ([`CopyProductOptions::placement`]); its representation by the rules
    /// of [`Self::CopyRepresentation`] (a type's `RepresentationMaps` with
    /// their origins); the property and quantity sets only it is defined
    /// by, with their properties and relationships; its material usage
    /// with its `IfcRelAssociatesMaterial`; its ports; and, by option, its
    /// openings and their fillings, placed relative to the copied
    /// placements. Shared, the copy joining the original's relationship:
    /// its type (`IfcRelDefinesByType`), property sets other objects use,
    /// material definitions, classification, document and library
    /// references, groups and, by option, its container and its whole.
    /// Not carried, and reported as skipped: relationships that would bind
    /// another object to the copy (connections, space boundaries, the
    /// parts of a whole, the occurrences of a type).
    ///
    /// Refused with [`UnsupportedCopy`] for an offset onto a placement
    /// that is not an `IfcLocalPlacement` with a Cartesian `Location`, or
    /// a moved copy whose openings, fillings or ports are not placed
    /// relative to it.
    ///
    /// [`UnsupportedCopy`]: super::AuthoringFailure::UnsupportedCopy
    CopyProduct {
        /// The `IfcProduct` or `IfcTypeObject` to copy.
        product: EntityId,
        /// Where the copy goes, and what else it copies and joins.
        options: CopyProductOptions,
    },
    /// Copy an entity and what it references, deciding each reached entity
    /// by an explicit [`CopyPolicy`] (#479): the low-level deep copy, which
    /// follows attributes only, never the relationships that reference an
    /// entity. Copied `IfcRoot`s get new `GlobalId`s; a reference shared
    /// inside the copied graph stays shared between the copies. Produces
    /// the copy; the batch outcome carries a
    /// [`CopyReport`](super::CopyReport).
    CopyEntity {
        /// The entity to copy.
        entity: EntityId,
        /// What it copies and what it shares.
        policy: CopyPolicy,
    },
}

impl AuthorOp {
    /// Every entity id the operation names outside its attribute values,
    /// mutably, so a handle can be resolved in place.
    pub(super) fn ids_mut(&mut self) -> Vec<&mut EntityId> {
        let mut ids: Vec<&mut EntityId> = Vec::new();
        match self {
            Self::Create { .. } | Self::OwnerHistory(_) | Self::Purge(_) => {}
            Self::Edit { entity, .. }
            | Self::Remove { entity }
            | Self::RemoveProduct {
                product: entity, ..
            } => ids.push(entity),
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
                options,
            } => {
                ids.push(one);
                ids.extend(many.iter_mut());
                ids.extend(owner_history.as_mut());
                ids.extend(options.mapping_target.as_mut());
            }
            Self::Contain {
                structure: one,
                elements: many,
                owner_history,
                ..
            }
            | Self::Aggregate {
                parent: one,
                parts: many,
                owner_history,
                ..
            } => {
                ids.push(one);
                ids.extend(many.iter_mut());
                ids.extend(owner_history.as_mut());
            }
            Self::Placement { relative_to, .. } => ids.extend(relative_to.as_mut()),
            Self::ShapeRepresentation { context, items, .. } => {
                ids.push(context);
                ids.extend(items.iter_mut());
            }
            Self::AssignRepresentation {
                representation,
                to: target,
            }
            | Self::UnassignRepresentation {
                representation,
                from: target,
            } => {
                ids.push(representation);
                ids.push(target);
            }
            Self::CopyRepresentation { representation }
            | Self::RemoveRepresentation { representation } => ids.push(representation),
            Self::CopyProduct { product, options } => {
                ids.push(product);
                if let super::CopyPlacement::At(placement) = &mut options.placement {
                    ids.push(placement);
                }
            }
            Self::CopyEntity { entity, .. } => ids.push(entity),
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
