//! The options the operations take, and the fields of an owner history.

use ifc_model::EntityId;

/// How [`AuthorOp::Contain`](super::AuthorOp::Contain) and [`AuthorOp::Aggregate`](super::AuthorOp::Aggregate) treat an object
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

/// What [`AuthorOp::AssignType`](super::AuthorOp::AssignType) propagates from the type to each object
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

/// How [`AuthorOp::AssignType`](super::AuthorOp::AssignType) reconciles an object's `PredefinedType`
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

/// How [`AuthorOp::RemoveProduct`](super::AuthorOp::RemoveProduct) treats what would otherwise refuse it.
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

/// What [`AuthorOp::Purge`](super::AuthorOp::Purge) removes.
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

/// The fields of [`AuthorOp::OwnerHistory`](super::AuthorOp::OwnerHistory).
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

/// Where [`AuthorOp::CopyProduct`](super::AuthorOp::CopyProduct) puts the
/// copy (#479).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[non_exhaustive]
pub enum CopyPlacement {
    /// Where the original is: a copy of its `ObjectPlacement`, relative to
    /// the same parent placement with the same transform. The default.
    #[default]
    Same,
    /// The original's placement moved by this vector, given in the parent
    /// placement's coordinate system (the frame its `Location` is in). Needs
    /// an `IfcLocalPlacement` whose `RelativePlacement` has an
    /// `IfcCartesianPoint` `Location`.
    Offset([f64; 3]),
    /// This `IfcObjectPlacement`, used as it is; the copied openings,
    /// fillings and ports that were relative to the original's placement
    /// are relative to it.
    At(EntityId),
}

/// What [`AuthorOp::CopyProduct`](super::AuthorOp::CopyProduct) copies
/// besides what the product solely owns, and which memberships the copy
/// joins (#479). [`Self::new`] (the default) copies openings with their
/// fillings and joins the original's container and whole.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct CopyProductOptions {
    /// Where the copy goes.
    pub placement: CopyPlacement,
    /// Copy the openings voiding the product, each with what it owns, and
    /// void the copy with them. Off: the copy has no openings.
    pub openings: bool,
    /// Copy the doors, windows and other elements filling a copied
    /// opening, each by the same rules, and fill the copied opening with
    /// them. Without [`Self::openings`] there is nothing to fill.
    pub fillings: bool,
    /// Contain the copy in the original's spatial structure element
    /// (joining its `IfcRelContainedInSpatialStructure`). Off: the copy is
    /// not contained.
    pub container: bool,
    /// Make the copy a part of the whole the original is a part of
    /// (joining its `IfcRelAggregates` or `IfcRelNests`). Off: the copy is
    /// not decomposing anything.
    pub aggregate: bool,
}

impl Default for CopyProductOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl CopyProductOptions {
    /// In place, with openings and fillings, in the original's container
    /// and whole: the default.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            placement: CopyPlacement::Same,
            openings: true,
            fillings: true,
            container: true,
            aggregate: true,
        }
    }

    /// With [`Self::placement`] set.
    #[must_use]
    pub const fn placement(mut self, placement: CopyPlacement) -> Self {
        self.placement = placement;
        self
    }

    /// With [`Self::openings`] set.
    #[must_use]
    pub const fn openings(mut self, copy: bool) -> Self {
        self.openings = copy;
        self
    }

    /// With [`Self::fillings`] set.
    #[must_use]
    pub const fn fillings(mut self, copy: bool) -> Self {
        self.fillings = copy;
        self
    }

    /// With [`Self::container`] set.
    #[must_use]
    pub const fn container(mut self, join: bool) -> Self {
        self.container = join;
        self
    }

    /// With [`Self::aggregate`] set.
    #[must_use]
    pub const fn aggregate(mut self, join: bool) -> Self {
        self.aggregate = join;
        self
    }
}

/// Which entities [`AuthorOp::CopyEntity`](super::AuthorOp::CopyEntity)
/// copies and which it shares (#479). The entity named is always copied;
/// every entity it reaches through its attributes is decided, in order:
///
/// 1. of a class in [`Self::copy`] (subtypes included): copied;
/// 2. of a class in [`Self::share`], or bound by a `UNIQUE` clause other
///    than `IfcRoot`'s (an `IfcApplication`, an `IfcPropertyEnumeration`,
///    whose copy would break the clause): shared, the copy references it;
/// 3. an `IfcRoot` (an object, a relationship, a property set): copied
///    with [`Self::copy_roots`], else shared;
/// 4. a definition (a material, a unit, a representation context, a
///    classification or document reference, the owner history and its
///    actors): copied with [`Self::copy_definitions`], else shared;
/// 5. any other resource entity: copied.
///
/// What is shared is not followed further. A class the declared release
/// does not declare is refused.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct CopyPolicy {
    /// Classes always copied, any case.
    pub copy: Vec<String>,
    /// Classes always shared, any case.
    pub share: Vec<String>,
    /// Copy reached `IfcRoot`s instead of sharing them.
    pub copy_roots: bool,
    /// Copy reached definitions instead of sharing them.
    pub copy_definitions: bool,
}

impl CopyPolicy {
    /// Share `IfcRoot`s and definitions, copy the rest: the default.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// With [`Self::copy`] set.
    #[must_use]
    pub fn copy<S: Into<String>>(mut self, classes: impl IntoIterator<Item = S>) -> Self {
        self.copy = classes.into_iter().map(Into::into).collect();
        self
    }

    /// With [`Self::share`] set.
    #[must_use]
    pub fn share<S: Into<String>>(mut self, classes: impl IntoIterator<Item = S>) -> Self {
        self.share = classes.into_iter().map(Into::into).collect();
        self
    }

    /// With [`Self::copy_roots`] set.
    #[must_use]
    pub const fn copy_roots(mut self, copy: bool) -> Self {
        self.copy_roots = copy;
        self
    }

    /// With [`Self::copy_definitions`] set.
    #[must_use]
    pub const fn copy_definitions(mut self, copy: bool) -> Self {
        self.copy_definitions = copy;
        self
    }
}

/// How [`apply_append`](super::apply_append) treats the `GlobalId`s of the
/// `IfcRoot`s it copies (#479).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum GlobalIdMode {
    /// Keep every `GlobalId`; one the target already holds is refused with
    /// [`DuplicateGlobalId`](super::AuthoringFailure::DuplicateGlobalId).
    /// The default: the same object keeps its identity across models.
    #[default]
    Keep,
    /// Give every copy a new `GlobalId`.
    Regenerate,
    /// Keep each `GlobalId` the target does not hold yet, and give a new
    /// one where it does; the report lists the copies regenerated.
    RegenerateColliding,
    /// Merge by identity: an `IfcRoot` whose `GlobalId` the target holds
    /// on an entity of the same type is that entity, reused rather than
    /// copied (a shared type, property set or storey), and a relationship
    /// the target holds so is joined by the copies (a type assignment, a
    /// containment). Any other `GlobalId` is kept; one the target holds on
    /// an entity of another type is refused.
    Merge,
}

impl GlobalIdMode {
    /// The mode's name, as the bindings spell it: `keep`, `regenerate`,
    /// `regenerate-colliding`, `merge`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Keep => "keep",
            Self::Regenerate => "regenerate",
            Self::RegenerateColliding => "regenerate-colliding",
            Self::Merge => "merge",
        }
    }

    /// The mode named `name`, any case, `_` or `-`.
    #[must_use]
    pub fn parse(name: &str) -> Option<Self> {
        let name = name.to_ascii_lowercase().replace('_', "-");
        [
            Self::Keep,
            Self::Regenerate,
            Self::RegenerateColliding,
            Self::Merge,
        ]
        .into_iter()
            .find(|mode| mode.as_str() == name)
    }
}

/// Which resources [`apply_append`](super::apply_append) maps to an equal
/// one the target already holds instead of copying (#479). Equal means the
/// same type and the same attribute values, references compared the same
/// way, all the way down; anything else is copied. Every kind is on by
/// default.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct ReuseOptions {
    /// Units (`IfcNamedUnit`, `IfcDerivedUnit`, `IfcMonetaryUnit`) and
    /// what they are built from.
    pub units: bool,
    /// Representation contexts and sub-contexts.
    pub contexts: bool,
    /// The owner history and its person, organization and application.
    pub owner_history: bool,
    /// Material definitions: materials, layers, layer and profile sets,
    /// constituents, material lists. Never a material usage, which belongs
    /// to its element.
    pub materials: bool,
    /// Presentation styles and colours.
    pub styles: bool,
}

impl Default for ReuseOptions {
    fn default() -> Self {
        Self::all()
    }
}

impl ReuseOptions {
    /// Reuse every kind: the default.
    #[must_use]
    pub const fn all() -> Self {
        Self {
            units: true,
            contexts: true,
            owner_history: true,
            materials: true,
            styles: true,
        }
    }

    /// Reuse nothing: copy every resource.
    #[must_use]
    pub const fn none() -> Self {
        Self {
            units: false,
            contexts: false,
            owner_history: false,
            materials: false,
            styles: false,
        }
    }
}

/// How [`apply_append`](super::apply_append) copies entities from another
/// model (#479).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct AppendOptions {
    /// Keep or regenerate the copies' `GlobalId`s.
    pub global_ids: GlobalIdMode,
    /// Which resources map to equal ones of the target.
    pub reuse: ReuseOptions,
}

impl AppendOptions {
    /// Keep `GlobalId`s, reuse every equal resource: the default.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            global_ids: GlobalIdMode::Keep,
            reuse: ReuseOptions::all(),
        }
    }

    /// With [`Self::global_ids`] set.
    #[must_use]
    pub const fn global_ids(mut self, mode: GlobalIdMode) -> Self {
        self.global_ids = mode;
        self
    }

    /// With [`Self::reuse`] set.
    #[must_use]
    pub const fn reuse(mut self, reuse: ReuseOptions) -> Self {
        self.reuse = reuse;
        self
    }
}
