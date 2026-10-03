//! The public edit, outcome and error types, and the two entry points.

use std::fmt;

use ifc_model::{Conflict, EntityId, Model, Transaction, Value};
use ifc_properties::{ExactPropertyError, PropertyError};

use super::emit::emit;
use super::plan::Planner;

/// One edit of a property or quantity value, addressed by object, set name
/// and property name. Names are compared exactly, as the resolver does.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum PropertyEdit {
    /// Write `value` to property `name` of set `set` on `object`, creating
    /// the property and the set when the object states neither.
    Set {
        /// The occurrence or type object whose own set is written.
        object: EntityId,
        /// `IfcPropertySet.Name` or `IfcElementQuantity.Name`.
        set: String,
        /// `IfcProperty.Name` or `IfcPhysicalQuantity.Name`.
        name: String,
        /// The value in the read side's form: a typed `IfcValue` (or `$`)
        /// for a single value, a list of them for an enumerated or list
        /// value, a typed measure for a quantity, whose wrapper is dropped
        /// on write because the quantity's slot declares it.
        value: Value,
        /// The set's entity when the edit creates a set that neither the
        /// type object nor the catalog describes; checked against both, and
        /// against an existing set, when given.
        set_type: Option<SetType>,
    },
    /// Remove property `name` from set `set` stated by `object` itself.
    Remove {
        /// The occurrence or type object whose own set is edited.
        object: EntityId,
        /// The set's name.
        set: String,
        /// The property's name.
        name: String,
    },
}

impl PropertyEdit {
    /// A [`PropertyEdit::Set`] with no set type.
    pub fn set(
        object: EntityId,
        set: impl Into<String>,
        name: impl Into<String>,
        value: Value,
    ) -> Self {
        Self::Set {
            object,
            set: set.into(),
            name: name.into(),
            value,
            set_type: None,
        }
    }

    /// A [`PropertyEdit::Remove`].
    pub fn remove(object: EntityId, set: impl Into<String>, name: impl Into<String>) -> Self {
        Self::Remove {
            object,
            set: set.into(),
            name: name.into(),
        }
    }

    /// This edit with `set_type` named; a [`PropertyEdit::Remove`] is
    /// returned unchanged.
    #[must_use]
    pub fn with_set_type(mut self, set_type: SetType) -> Self {
        if let Self::Set { set_type: slot, .. } = &mut self {
            *slot = Some(set_type);
        }
        self
    }

    /// The addressed object.
    pub fn object(&self) -> EntityId {
        match self {
            Self::Set { object, .. } | Self::Remove { object, .. } => *object,
        }
    }

    /// The addressed set name.
    pub fn set_name(&self) -> &str {
        match self {
            Self::Set { set, .. } | Self::Remove { set, .. } => set,
        }
    }

    /// The addressed property name.
    pub fn name(&self) -> &str {
        match self {
            Self::Set { name, .. } | Self::Remove { name, .. } => name,
        }
    }
}

/// The entity of a set an edit writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SetType {
    /// `IfcPropertySet`.
    PropertySet,
    /// `IfcElementQuantity`.
    ElementQuantity,
}

impl SetType {
    /// The entity name, upper-case as the model stores it.
    pub const fn entity(self) -> &'static str {
        match self {
            Self::PropertySet => "IFCPROPERTYSET",
            Self::ElementQuantity => "IFCELEMENTQUANTITY",
        }
    }

    /// The set type an entity name (any case) names, if it is one.
    pub fn from_entity(name: &str) -> Option<Self> {
        if name.eq_ignore_ascii_case("IFCPROPERTYSET") {
            Some(Self::PropertySet)
        } else if name.eq_ignore_ascii_case("IFCELEMENTQUANTITY") {
            Some(Self::ElementQuantity)
        } else {
            None
        }
    }
}

/// What a committed batch did.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct PropertyEditOutcome {
    /// Per edit, in order: the entity that holds the property after the
    /// whole batch, or `None` when the batch leaves no such property on the
    /// object (a removal, or a later removal of a set value).
    pub properties: Vec<Option<EntityId>>,
    /// Entities the batch created, in commit order.
    pub created: Vec<EntityId>,
    /// Entities the batch removed.
    pub removed: Vec<EntityId>,
    /// The model revision after the commit.
    pub revision: u64,
}

/// A planned batch: its transaction, not yet committed, and the property
/// each edit resolves to once it is.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct StagedPropertyEdits {
    /// Every edit of the batch, opened against the planned model.
    pub transaction: Transaction,
    /// As [`PropertyEditOutcome::properties`].
    pub properties: Vec<Option<EntityId>>,
}

/// Why a batch was refused. Nothing was written.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct PropertyEditError {
    /// The position of the refused edit; `None` when the refusal concerns
    /// the batch (its commit, or the model's release).
    pub edit: Option<usize>,
    /// Why.
    pub failure: PropertyEditFailure,
}

/// The reason a [`PropertyEditError`] gives.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum PropertyEditFailure {
    /// The addressed object is not in the model.
    MissingEntity(EntityId),
    /// The exact resolver refused the object or the file: an unsupported or
    /// missing release, an object that carries no property sets, or sets it
    /// cannot read unambiguously.
    Resolve(ExactPropertyError),
    /// A release-bound `ifc-properties` writer refused a record.
    Authoring(PropertyError),
    /// The declared release does not admit the value: no `IfcValue` member,
    /// a payload of the wrong type, another measure than a quantity or a
    /// stated unit declares, a blank name.
    InvalidValue(String),
    /// A template refused the value: the release's PSD/QTO catalog entry,
    /// or the property's own `IfcPropertyEnumeration`.
    Template(String),
    /// A removal names a property the object does not state.
    MissingProperty {
        /// The object.
        object: EntityId,
        /// The set name.
        set: String,
        /// The property name.
        name: String,
        /// The type object it is inherited from, when it is.
        inherited_from: Option<EntityId>,
    },
    /// The set's entity disagrees with the edit's set type, the type
    /// object's set of that name, or the catalog.
    WrongSetType(String),
    /// A form this writer does not edit: a bounded, table, reference or
    /// complex value, a predefined set, a set related through an
    /// `IfcPropertySetDefinitionSet`.
    Unsupported(String),
    /// The file's own records prevent the edit, such as an object without
    /// the `GlobalId` or IFC2X3 `OwnerHistory` a new set derives from.
    InvalidModel(String),
    /// A `Pset_` or `Qto_` set cannot be checked: the build leaves out the
    /// `property-catalog` feature.
    CatalogUnavailable(String),
    /// A `Pset_` or `Qto_` set cannot be checked yet: the build reads the
    /// catalog at runtime (`property-catalog-runtime`) and the release's
    /// edition has not been installed.
    CatalogNotLoaded {
        /// The set name.
        set: String,
        /// The catalog edition the release reads, such as `IFC4X3 ADD2`.
        edition: String,
    },
    /// The planned transaction failed its preflight.
    Conflict(Vec<Conflict>),
}

impl PropertyEditError {
    pub(super) fn at(edit: usize, failure: PropertyEditFailure) -> Self {
        Self {
            edit: Some(edit),
            failure,
        }
    }
}

impl fmt::Display for PropertyEditError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(edit) = self.edit {
            write!(f, "edit {edit}: ")?;
        }
        write!(f, "{}", self.failure)
    }
}

impl fmt::Display for PropertyEditFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingEntity(id) => write!(f, "{id} is not in the model"),
            Self::Resolve(error) => write!(f, "{error}"),
            Self::Authoring(error) => write!(f, "{error}"),
            Self::InvalidValue(detail)
            | Self::Template(detail)
            | Self::WrongSetType(detail)
            | Self::Unsupported(detail)
            | Self::InvalidModel(detail) => f.write_str(detail),
            Self::MissingProperty {
                object,
                set,
                name,
                inherited_from,
            } => {
                write!(f, "{object} states no property {set}.{name}")?;
                if let Some(type_object) = inherited_from {
                    write!(
                        f,
                        "; it is inherited from type {type_object}, which is where it is removed"
                    )?;
                }
                Ok(())
            }
            Self::CatalogUnavailable(set) => write!(
                f,
                "{set} is a catalog set and this build has no catalog to check it against"
            ),
            Self::CatalogNotLoaded { set, edition } => write!(
                f,
                "{set} is a catalog set and the {edition} catalog is not installed"
            ),
            Self::Conflict(conflicts) => write!(f, "the planned edit conflicts: {conflicts:?}"),
        }
    }
}

impl std::error::Error for PropertyEditError {}

/// Plan `edits` against `model` and stage them on one transaction, without
/// touching the model.
///
/// Edits apply in order: a later edit sees the sets and properties an
/// earlier one created, changed or removed.
///
/// # Errors
///
/// The first edit that cannot be planned, as a [`PropertyEditError`] naming
/// its position.
pub fn stage_property_edits(
    model: &Model,
    edits: &[PropertyEdit],
) -> Result<StagedPropertyEdits, PropertyEditError> {
    let mut planner = Planner::new(model)?;
    for (index, edit) in edits.iter().enumerate() {
        planner
            .apply(index, edit)
            .map_err(|failure| PropertyEditError::at(index, failure))?;
    }
    emit(planner)
}

/// Plan, stage and commit `edits` as one transaction: all of them or none.
///
/// # Errors
///
/// As [`stage_property_edits`], and [`PropertyEditFailure::Conflict`] when
/// the planned transaction fails its preflight. The model is unchanged on
/// every error.
pub fn apply_property_edits(
    model: &mut Model,
    edits: &[PropertyEdit],
) -> Result<PropertyEditOutcome, PropertyEditError> {
    let staged = stage_property_edits(model, edits)?;
    let applied = staged
        .transaction
        .commit(model)
        .map_err(|conflicts| PropertyEditError {
            edit: None,
            failure: PropertyEditFailure::Conflict(conflicts),
        })?;
    Ok(PropertyEditOutcome {
        properties: staged.properties,
        created: applied.created,
        removed: applied.removed.into_iter().map(|(id, _)| id).collect(),
        revision: applied.revision,
    })
}
