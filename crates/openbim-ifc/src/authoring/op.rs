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
}

/// How [`AuthorOp::Contain`] and [`AuthorOp::Aggregate`] treat an object
/// that already has the one relationship of their kind the schema allows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct RelateOptions {
    /// Take each object out of the relationship it already has (removing
    /// that relationship when it is left relating nothing) instead of
    /// refusing with [`AlreadyRelated`](super::AuthoringFailure::AlreadyRelated).
    pub replace: bool,
}

impl RelateOptions {
    /// Refuse an object that is already related: the default.
    #[must_use]
    pub const fn new() -> Self {
        Self { replace: false }
    }

    /// With [`Self::replace`] set to `replace`.
    #[must_use]
    pub const fn replace(mut self, replace: bool) -> Self {
        self.replace = replace;
        self
    }
}

/// What [`AuthorOp::AssignType`] propagates from the type to each object
/// (#480). Every option is off by default: the operation then only writes
/// the `IfcRelDefinesByType`, and refuses an object that is already typed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct AssignTypeOptions {
    /// Map each of the type's `RepresentationMaps` onto the object: one
    /// `IfcShapeRepresentation` per map, in the map's representation
    /// context, with its `RepresentationIdentifier`, the
    /// `RepresentationType` `MappedRepresentation` and one
    /// `IfcMappedItem` whose `MappingTarget` is [`Self::mapping_target`]
    /// or a fresh identity operator. The representations go into the
    /// object's `IfcProductDefinitionShape`, created when it has none.
    /// Refused with [`RepresentationConflict`] when the object already
    /// has a representation with the same identifier (or, unnamed, in the
    /// same context), and with [`UnsupportedRepresentationMap`] for a map
    /// whose representation is not an `IfcShapeRepresentation`.
    ///
    /// [`RepresentationConflict`]: super::AuthoringFailure::RepresentationConflict
    /// [`UnsupportedRepresentationMap`]: super::AuthoringFailure::UnsupportedRepresentationMap
    pub map_representations: bool,
    /// The `IfcCartesianTransformationOperator` every mapped item uses;
    /// `None` creates one per object at the origin of its placement with
    /// no axes or scale (the identity), 3D or 2D as the map's
    /// `MappingOrigin` is.
    pub mapping_target: Option<EntityId>,
    /// Derive the object's material usage from the type's material set:
    /// an `IfcMaterialLayerSetUsage` for an `IfcMaterialLayerSet`
    /// (`LayerSetDirection` from [`Self::layer_set_direction`] or the
    /// object's class, `DirectionSense` `POSITIVE`,
    /// `OffsetFromReferenceLine` 0), an `IfcMaterialProfileSetUsage` for
    /// an `IfcMaterialProfileSet` (no `CardinalPoint`), each in its own
    /// `IfcRelAssociatesMaterial`. Any other material (a single material,
    /// a list, a constituent set) applies through the type and is not
    /// copied. Refused with [`MaterialUsageUndetermined`] where the
    /// defaults cannot be determined, and with [`AlreadyRelated`] for an
    /// object that already has a material.
    ///
    /// [`MaterialUsageUndetermined`]: super::AuthoringFailure::MaterialUsageUndetermined
    /// [`AlreadyRelated`]: super::AuthoringFailure::AlreadyRelated
    pub material_usage: bool,
    /// The `LayerSetDirection` of a derived layer set usage; `None` takes
    /// the one the IFC documentation fixes for the object's class
    /// (`AXIS2` for `IfcWall`, `AXIS3` for `IfcSlab` and `IfcPlate`,
    /// subtypes included) and refuses any other class.
    pub layer_set_direction: Option<LayerAxis>,
    /// How the object's `PredefinedType` is reconciled with the type's.
    pub predefined_type: PredefinedTypeMode,
    /// Unassign each object's previous type instead of refusing: the
    /// object leaves its `IfcRelDefinesByType` (removed when it is left
    /// relating nothing), and the representations mapped from the
    /// previous type's maps and the set usage derived from its material
    /// set are removed from the object, with what only they used.
    pub replace: bool,
}

impl AssignTypeOptions {
    /// Only the relationship, refusing an object already typed: the
    /// default.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            map_representations: false,
            mapping_target: None,
            material_usage: false,
            layer_set_direction: None,
            predefined_type: PredefinedTypeMode::Ignore,
            replace: false,
        }
    }

    /// Everything the type carries: representations mapped, material
    /// usage derived, `PredefinedType` synchronised.
    #[must_use]
    pub const fn propagate() -> Self {
        Self::new()
            .map_representations(true)
            .material_usage(true)
            .predefined_type(PredefinedTypeMode::Sync)
    }

    /// With [`Self::map_representations`] set.
    #[must_use]
    pub const fn map_representations(mut self, map: bool) -> Self {
        self.map_representations = map;
        self
    }

    /// With [`Self::mapping_target`] set.
    #[must_use]
    pub const fn mapping_target(mut self, target: Option<EntityId>) -> Self {
        self.mapping_target = target;
        self
    }

    /// With [`Self::material_usage`] set.
    #[must_use]
    pub const fn material_usage(mut self, derive: bool) -> Self {
        self.material_usage = derive;
        self
    }

    /// With [`Self::layer_set_direction`] set.
    #[must_use]
    pub const fn layer_set_direction(mut self, direction: Option<LayerAxis>) -> Self {
        self.layer_set_direction = direction;
        self
    }

    /// With [`Self::predefined_type`] set.
    #[must_use]
    pub const fn predefined_type(mut self, mode: PredefinedTypeMode) -> Self {
        self.predefined_type = mode;
        self
    }

    /// With [`Self::replace`] set.
    #[must_use]
    pub const fn replace(mut self, replace: bool) -> Self {
        self.replace = replace;
        self
    }
}

/// `IfcLayerSetDirectionEnum`: the axis of the element's placement the
/// layers of a set usage are stacked along.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum LayerAxis {
    /// `AXIS1`.
    Axis1,
    /// `AXIS2`: walls.
    Axis2,
    /// `AXIS3`: slabs and plates.
    Axis3,
}

impl LayerAxis {
    /// The enumeration constant, `AXIS1` to `AXIS3`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Axis1 => "AXIS1",
            Self::Axis2 => "AXIS2",
            Self::Axis3 => "AXIS3",
        }
    }

    /// The constant named `name`, any case.
    #[must_use]
    pub fn parse(name: &str) -> Option<Self> {
        [Self::Axis1, Self::Axis2, Self::Axis3]
            .into_iter()
            .find(|axis| axis.as_str().eq_ignore_ascii_case(name.trim_matches('.')))
    }
}

/// How [`AuthorOp::AssignType`] reconciles an object's `PredefinedType`
/// with its type's. The IFC documentation (IFC4 ADD2 TC1 `IfcWall`,
/// `IfcDoor`, ...) states that an occurrence's `PredefinedType` "shall
/// only be used, if no type is assigned, providing its own
/// `PredefinedType`". A type provides one when its `PredefinedType` is set
/// and not `NOTDEFINED`; a `USERDEFINED` type names it in `ElementType`
/// (`ProcessType`, `ResourceType`), an object in `ObjectType`. Nothing is
/// done where the object or the type has no `PredefinedType` attribute in
/// the declared release (IFC2X3 `IfcWall`, `IfcDoorStyle`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum PredefinedTypeMode {
    /// Leave the object's `PredefinedType` as it is: the default.
    #[default]
    Ignore,
    /// Refuse with [`PredefinedTypeConflict`] an object whose own
    /// `PredefinedType` is set, not `NOTDEFINED`, and differs from the
    /// one the type provides (both `USERDEFINED`: when `ObjectType` and
    /// the type's `ElementType` differ). An equal value is accepted.
    ///
    /// [`PredefinedTypeConflict`]: super::AuthoringFailure::PredefinedTypeConflict
    Validate,
    /// Where the type provides a `PredefinedType`, unset the object's
    /// (and its `ObjectType` when that held its `USERDEFINED` name), so
    /// the type's applies; otherwise leave the object's.
    Sync,
}

impl PredefinedTypeMode {
    /// The mode named `name` (`ignore`, `validate`, `sync`), any case.
    #[must_use]
    pub fn parse(name: &str) -> Option<Self> {
        match name.to_ascii_lowercase().as_str() {
            "ignore" => Some(Self::Ignore),
            "validate" => Some(Self::Validate),
            "sync" => Some(Self::Sync),
            _ => None,
        }
    }
}

/// How [`AuthorOp::RemoveProduct`] treats what would otherwise refuse it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct RemoveProductOptions {
    /// Re-base every placement relative to a removed placement onto the
    /// removed placement's own parent, composing the transforms so nothing
    /// moves, instead of refusing. Needs `IfcLocalPlacement`s over
    /// `IfcAxis2Placement3D`s along the way.
    pub rebase_placements: bool,
    /// Remove a type object's `IfcRelDefinesByType`, leaving its
    /// occurrences untyped, instead of refusing.
    pub untype_occurrences: bool,
}

impl RemoveProductOptions {
    /// No option set: refuse dependent placements and typed occurrences.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            rebase_placements: false,
            untype_occurrences: false,
        }
    }

    /// With [`Self::rebase_placements`] set to `rebase`.
    #[must_use]
    pub const fn rebase_placements(mut self, rebase: bool) -> Self {
        self.rebase_placements = rebase;
        self
    }

    /// With [`Self::untype_occurrences`] set to `untype`.
    #[must_use]
    pub const fn untype_occurrences(mut self, untype: bool) -> Self {
        self.untype_occurrences = untype;
        self
    }
}

/// What [`AuthorOp::Purge`] removes.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct PurgeOptions {
    /// The entity types to purge, subtypes included, any case; empty for
    /// every resource entity no `IfcRoot` reaches. A type the declared
    /// release does not declare is refused.
    pub classes: Vec<String>,
    /// Also purge unreferenced `IfcRoot` entities of `classes`. `IfcProject`
    /// only when `classes` names `IfcProject` itself.
    pub include_roots: bool,
    /// Also purge an unreferenced owner history with its person,
    /// organization and application, and, without `classes`, unused
    /// representation contexts.
    pub include_owner_history: bool,
}

impl PurgeOptions {
    /// The default purge: resource entities no `IfcRoot` reaches.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Purge unreferenced entities of `classes` instead.
    #[must_use]
    pub fn classes<S: Into<String>>(mut self, classes: impl IntoIterator<Item = S>) -> Self {
        self.classes = classes.into_iter().map(Into::into).collect();
        self
    }

    /// With [`Self::include_roots`] set.
    #[must_use]
    pub const fn include_roots(mut self, include: bool) -> Self {
        self.include_roots = include;
        self
    }

    /// With [`Self::include_owner_history`] set.
    #[must_use]
    pub const fn include_owner_history(mut self, include: bool) -> Self {
        self.include_owner_history = include;
        self
    }
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
