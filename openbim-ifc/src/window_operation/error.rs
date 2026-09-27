//! Why a window's operation geometry cannot be derived exactly.

use std::{fmt, sync::Arc};

use ifc_geometry::GeometryError;
use ifc_model::EntityId;
use ifc_properties::{ExactPropertyError, ExactUnitError};

use super::RefusedWindowOperation;
use crate::operation::ReadError;

/// Why a window's operation geometry cannot be derived exactly.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum WindowOperationError {
    /// A property, set or relationship could not be read exactly.
    Property(ExactPropertyError),
    /// The project length unit could not be resolved exactly.
    Unit(ExactUnitError),
    /// The window's placement could not be resolved.
    Placement(GeometryError),
    /// The entity is not an `IfcWindow` in the declared release.
    NotAWindow {
        /// The entity queried.
        entity: EntityId,
        /// Its IFC type name.
        type_name: Arc<str>,
    },
    /// The window is typed by an object that is not an `IfcWindowType`
    /// (IFC4 `CorrectStyleAssigned`, IFC4X3 `CorrectTypeAssigned`) or, in
    /// IFC2X3, an `IfcWindowStyle`.
    UnsupportedTypeObject {
        /// The type object.
        type_object: EntityId,
        /// Its IFC type name.
        type_name: Arc<str>,
    },
    /// An attribute read here is not a value its declaration accepts.
    MalformedAttribute {
        /// The entity holding the attribute.
        entity: EntityId,
        /// The attribute's schema name.
        attribute: &'static str,
    },
    /// Neither the occurrence nor a type object states a partitioning.
    MissingPartitioningType {
        /// The window.
        window: EntityId,
    },
    /// Occurrence and type object state different partitionings. IFC4 says
    /// the occurrence's "shall only be used, if no type object IfcWindowType
    /// is assigned", so neither can be chosen.
    ConflictingPartitioningType {
        /// The occurrence's value.
        occurrence: Arc<str>,
        /// The type object.
        type_object: EntityId,
        /// The type object's value.
        type_value: Arc<str>,
    },
    /// The partitioning is one this derivation refuses.
    RefusedPartitioning {
        /// The enumeration constant as written.
        partitioning: Arc<str>,
        /// Why it is refused.
        reason: RefusedWindowOperation,
    },
    /// A panel's `OperationType` is one this derivation refuses.
    RefusedPanelOperation {
        /// The panel set.
        set: EntityId,
        /// The enumeration constant as written.
        operation: Arc<str>,
        /// Why it is refused.
        reason: RefusedWindowOperation,
    },
    /// `OverallWidth` is `$`. IFC suggests taking it from the opening's
    /// geometry, which this derivation does not guess at.
    MissingOverallWidth {
        /// The window.
        window: EntityId,
    },
    /// `OverallHeight` is `$`, likewise.
    MissingOverallHeight {
        /// The window.
        window: EntityId,
    },
    /// Neither the occurrence nor its type carries an
    /// `IfcWindowPanelProperties`: the panels are unknown.
    NoPanelProperties {
        /// The window.
        window: EntityId,
    },
    /// The partitioning needs a different number of panel sets.
    PanelCount {
        /// Sets the partitioning needs.
        expected: usize,
        /// Sets found (from the governing source).
        found: usize,
    },
    /// A panel set's `PanelPosition` is not one the partitioning lists, or
    /// repeats one.
    PanelMismatch {
        /// The panel set.
        set: EntityId,
        /// `PanelPosition`.
        attribute: &'static str,
        /// The value found.
        found: Arc<str>,
    },
    /// The partitioning splits the window, but neither the occurrence nor
    /// its type carries an `IfcWindowLiningProperties` to say where.
    NoLiningProperties {
        /// The window.
        window: EntityId,
    },
    /// The governing source carries several `IfcWindowLiningProperties`,
    /// which may place the splits differently.
    LiningCount {
        /// Sets found (from the governing source).
        found: usize,
    },
    /// A mullion or transom offset the partitioning needs is `$`.
    MissingSplit {
        /// The lining set.
        set: EntityId,
        /// The offset attribute.
        attribute: &'static str,
    },
    /// A mullion or transom offset outside `(0, 1)`, or a second offset not
    /// beyond the first: a panel would have no extent.
    InvalidSplit {
        /// The lining set.
        set: EntityId,
        /// The offset attribute.
        attribute: &'static str,
        /// The stated ratio.
        value: f64,
    },
    /// The window's world transform is not rigid, so a panel width is not a
    /// length along its axes.
    NonRigidPlacement {
        /// The window.
        window: EntityId,
    },
    /// A panel turns on a side hinge, but the window's local x-y plane is
    /// vertical in the world (a skylight lying flat), so the hinge has no
    /// left or right seen from above.
    NoPlanHandedness {
        /// The window.
        window: EntityId,
    },
}

impl fmt::Display for WindowOperationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "window operation cannot be derived exactly: {self:?}")
    }
}

impl std::error::Error for WindowOperationError {}

impl From<ExactPropertyError> for WindowOperationError {
    fn from(error: ExactPropertyError) -> Self {
        Self::Property(error)
    }
}

impl From<ExactUnitError> for WindowOperationError {
    fn from(error: ExactUnitError) -> Self {
        Self::Unit(error)
    }
}

impl WindowOperationError {
    /// The window-level error for a shared read failure.
    pub(super) fn read(error: ReadError) -> Self {
        match error {
            ReadError::Malformed { entity, attribute } => {
                Self::MalformedAttribute { entity, attribute }
            }
            ReadError::Unit(error) => Self::Unit(error),
            ReadError::Placement(error) => Self::Placement(error),
        }
    }
}
