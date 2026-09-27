//! Door operation geometry: where each leaf is, which side it hangs on, and
//! the floor sector it sweeps (#148).
//!
//! # Why this lives in the facade
//!
//! The answer joins the door's placement (`ifc-geometry`) with its operation
//! type and `IfcDoorPanelProperties` (`ifc-properties`). ADR 0003 forbids
//! those sibling crates from depending on each other, so the join is an
//! orchestration item, compiled only with both `geometry-select` and
//! `properties`.
//!
//! # The convention, from the IFC4 ADD2 TC1 documentation
//!
//! - The door's local placement fixes the frame: x runs across the opening
//!   width (`OverallWidth` "reflects the X Dimension"), and "the door panel
//!   (for swinging doors) opens always into the direction of the positive Y
//!   axis of the local placement" (IfcDoor, Figure 228).
//! - `SINGLE_SWING_LEFT`: "the hinges are on the left side as viewed in the
//!   direction of the positive y-axis"; `SINGLE_SWING_RIGHT` on the right
//!   (IfcDoorTypeOperationEnum). Left is the low-x side.
//! - `PanelPosition` "is given as shown in the XZ plane of the local
//!   placement, looking into the direction of the positive Y axis"
//!   (IfcDoorPanelPositionEnum, Figure 317, which draws a
//!   `DOUBLE_DOOR_SINGLE_SWING` with its LEFT panel hinged at x = 0 and its
//!   RIGHT panel at x = `OverallWidth`, both on the x axis).
//! - `PanelWidth` is "a ratio of the overall width of the door opening"
//!   (IfcDoorPanelProperties, Figure 324); "if omitted, it defaults to 1",
//!   and "a value has to be provided" for every operation with more than one
//!   panel.
//!
//! Leaves therefore lie on the placement's x axis, partitioning
//! `[0, OverallWidth]`. The panel's offset across the wall depth
//! (`IfcDoorLiningProperties.LiningOffset`, `LiningToPanelOffsetX/Y`,
//! `PanelDepth`) is **not** applied: IFC defines it only by figures
//! (Figure 323) that disagree on which panel face the offsets measure to. A
//! caller that needs the leaf inside the wall depth reads those attributes
//! itself; the sector here is the one Figures 228 and 317 draw.
//!
//! The sector is the quarter disc from the closed leaf to the leaf
//! perpendicular to the opening, as every figure draws it (half disc for a
//! double-acting leaf). It is the symbolic swing, not a hardware stop angle,
//! which IFC does not record.
//!
//! # What is refused
//!
//! Never defaulted: `NOTDEFINED` and `USERDEFINED` operations, `REVOLVING`,
//! folding and lifting doors, and the two
//! `DOUBLE_DOOR_SINGLE_SWING_OPPOSITE_*` operations, whose text ("one panel
//! swings in one direction and the other panel swings in the opposite
//! direction") does not say which leaf opens towards +y; a door with no
//! panel properties, a missing `OverallWidth`, and panels that contradict the
//! operation. See [`DoorOperationError`].

#![cfg(all(feature = "geometry-select", feature = "properties"))]

mod geometry;
mod layout;
mod read;

#[cfg(test)]
mod tests;

use std::{fmt, sync::Arc};

use ifc_geometry::{GeometryError, Transform};
use ifc_model::{EntityId, Model};
use ifc_properties::{ExactPropertyError, ExactSource, ExactUnitError};

use crate::operation::{ReadError, Sector, Side};

/// How one door operates, leaf by leaf, in world coordinates (metres).
///
/// Returned by [`door_operation`].
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct DoorOperation {
    /// The `IfcDoor` (or `IfcDoorStandardCase`).
    pub door: EntityId,
    /// The operation type the leaves were derived from.
    pub operation: DoorOperationType,
    /// Where the operation type was stated: the occurrence's
    /// `OperationType`, or its type object's (`IfcDoorType`, IFC2X3
    /// `IfcDoorStyle`).
    pub operation_source: ExactSource,
    /// Where the `IfcDoorPanelProperties` came from: the occurrence's own
    /// sets when it has any, else its type's.
    pub panel_source: ExactSource,
    /// `IfcDoor.OverallWidth` in metres.
    pub overall_width: f64,
    /// One leaf per panel, ordered from local low x to high x.
    pub leaves: Vec<Leaf>,
}

/// The operation types this derivation supports.
///
/// Named after `IfcDoorTypeOperationEnum` (IFC4, IFC4X3) and
/// `IfcDoorStyleOperationEnum` (IFC2X3), which spell these members alike.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum DoorOperationType {
    /// `SINGLE_SWING_LEFT`: one leaf hinged on the left.
    SingleSwingLeft,
    /// `SINGLE_SWING_RIGHT`: one leaf hinged on the right.
    SingleSwingRight,
    /// `DOUBLE_DOOR_SINGLE_SWING`: LEFT leaf hinged left, RIGHT leaf hinged right.
    DoubleDoorSingleSwing,
    /// `DOUBLE_SWING_LEFT`: one double-acting leaf hinged on the left.
    DoubleSwingLeft,
    /// `DOUBLE_SWING_RIGHT`: one double-acting leaf hinged on the right.
    DoubleSwingRight,
    /// `DOUBLE_DOOR_DOUBLE_SWING`: two double-acting leaves hinged at the jambs.
    DoubleDoorDoubleSwing,
    /// `SLIDING_TO_LEFT`: one leaf sliding towards local -x.
    SlidingToLeft,
    /// `SLIDING_TO_RIGHT`: one leaf sliding towards local +x.
    SlidingToRight,
    /// `DOUBLE_DOOR_SLIDING`: LEFT leaf slides left, RIGHT leaf slides right.
    DoubleDoorSliding,
    /// `ROLLINGUP`: one leaf rolling up, sweeping no floor area.
    RollingUp,
    /// `SWING_FIXED_LEFT` (IFC4, IFC4X3): a swinging leaf hinged on its
    /// left beside a fixed panel.
    SwingFixedLeft,
    /// `SWING_FIXED_RIGHT` (IFC4, IFC4X3): a swinging leaf hinged on its
    /// right beside a fixed panel.
    SwingFixedRight,
}

/// `IfcDoorPanelPositionEnum`: where a panel sits in the door, as written.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PanelPosition {
    /// `LEFT`: at local low x.
    Left,
    /// `MIDDLE`.
    Middle,
    /// `RIGHT`: at local high x.
    Right,
    /// `NOTDEFINED`.
    NotDefined,
}

/// How a leaf moves.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum LeafMotion {
    /// Swings about its hinge into the local +y side (`SWINGING`).
    Swing,
    /// Swings about its hinge to both sides (`DOUBLE_ACTING`).
    DoubleSwing,
    /// Slides along the door's width (`SLIDING`).
    Slide {
        /// Unit world direction the leaf slides in when opening.
        direction: [f64; 3],
    },
    /// Rolls up out of the opening (`ROLLINGUP`).
    RollUp,
    /// Does not open (`FIXEDPANEL`).
    Fixed,
}

/// One door leaf.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Leaf {
    /// The `IfcDoorPanelProperties` this leaf was read from.
    pub panel_set: EntityId,
    /// The set's `PanelPosition`.
    pub position: PanelPosition,
    /// How the leaf moves.
    pub motion: LeafMotion,
    /// The leaf's frame in world metres: the door's own axes, translated to
    /// the leaf's closed low-x edge on the placement's x axis. The closed
    /// leaf spans local `x ∈ [0, width]` of this frame; a mirrored door
    /// placement stays mirrored here.
    pub frame_world: Transform,
    /// Leaf width in metres: `PanelWidth × OverallWidth`.
    pub width: f64,
    /// The hinge side of a swinging or double-acting leaf, as seen looking
    /// along the leaf's opening direction (local +y) from above (world +z).
    /// It is the operation's side when the placement keeps plan handedness
    /// and the opposite side when the placement mirrors it. `None` for a
    /// leaf that has no hinge.
    pub hinge_side: Option<Side>,
    /// The floor sector a hinged leaf sweeps; `None` for sliding, rolling
    /// and fixed leaves.
    pub swing: Option<Sector>,
}

/// Why an operation type is refused rather than derived.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum RefusedOperation {
    /// `NOTDEFINED`: "a door with a lining, but no panels".
    NotDefined,
    /// `USERDEFINED`: its meaning is free text.
    UserDefined,
    /// `REVOLVING` (and IFC4X3 `REVOLVING_VERTICAL`): four leaves about a
    /// central axis, described by one panel set.
    Revolving,
    /// A folding operation: the fold count and geometry are not recorded.
    Folding,
    /// `DOUBLE_DOOR_SINGLE_SWING_OPPOSITE_LEFT`/`_RIGHT`: the specification
    /// does not state which leaf opens towards local +y.
    AmbiguousSwingDirection,
    /// An IFC4X3 lifting operation, whose motion leaves the floor plane.
    Lifting,
}

/// Why a door's operation geometry cannot be derived exactly.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum DoorOperationError {
    /// A property, set or relationship could not be read exactly.
    Property(ExactPropertyError),
    /// The project length unit could not be resolved exactly.
    Unit(ExactUnitError),
    /// The door's placement could not be resolved.
    Placement(GeometryError),
    /// The entity is not an `IfcDoor` in the declared release.
    NotADoor {
        /// The entity queried.
        entity: EntityId,
        /// Its IFC type name.
        type_name: Arc<str>,
    },
    /// The door is typed by an object that is not an `IfcDoorType` (IFC4,
    /// IFC4X3) or `IfcDoorStyle` (IFC2X3).
    UnsupportedTypeObject {
        /// The type object.
        type_object: EntityId,
        /// Its IFC type name.
        type_name: Arc<str>,
    },
    /// A door or type attribute read here is not a value its declaration
    /// accepts.
    MalformedAttribute {
        /// The entity holding the attribute.
        entity: EntityId,
        /// The attribute's schema name.
        attribute: &'static str,
    },
    /// Neither the occurrence nor a type object states an operation type.
    MissingOperationType {
        /// The door.
        door: EntityId,
    },
    /// Occurrence and type object state different operation types. IFC4
    /// says the occurrence's "shall only be used, if no type object
    /// IfcDoorType is assigned", so neither can be chosen.
    ConflictingOperationType {
        /// The occurrence's value.
        occurrence: Arc<str>,
        /// The type object.
        type_object: EntityId,
        /// The type object's value.
        type_value: Arc<str>,
    },
    /// The operation type is one this derivation refuses.
    RefusedOperation {
        /// The enumeration constant as written.
        operation: Arc<str>,
        /// Why it is refused.
        reason: RefusedOperation,
    },
    /// `OverallWidth` is `$`. IFC suggests taking it from the opening's
    /// geometry, which this derivation does not guess at.
    MissingOverallWidth {
        /// The door.
        door: EntityId,
    },
    /// Neither the occurrence nor its type carries an
    /// `IfcDoorPanelProperties`: the leaves are unknown.
    NoPanelProperties {
        /// The door.
        door: EntityId,
    },
    /// The operation needs a different number of panel sets.
    PanelCount {
        /// Sets the operation needs.
        expected: usize,
        /// Sets found (from the governing source).
        found: usize,
    },
    /// A panel set's `PanelOperation` or `PanelPosition` contradicts the
    /// operation type, which it "has to correspond with".
    PanelMismatch {
        /// The panel set.
        set: EntityId,
        /// `PanelOperation` or `PanelPosition`.
        attribute: &'static str,
        /// The value found.
        found: Arc<str>,
    },
    /// A panel of a multi-panel operation states no `PanelWidth`.
    MissingPanelWidth {
        /// The panel set.
        set: EntityId,
    },
    /// A `PanelWidth` outside `(0, 1]`.
    InvalidPanelWidth {
        /// The panel set.
        set: EntityId,
        /// The stated fraction.
        width: f64,
    },
    /// The panels' widths do not partition the opening: they sum to
    /// something other than 1.
    PanelWidthsDoNotPartition {
        /// The sum of the stated fractions.
        sum: f64,
    },
    /// A single panel narrower than the opening at position `MIDDLE` or
    /// `NOTDEFINED`, whose place along the width is not defined.
    UnplacedPanel {
        /// The panel set.
        set: EntityId,
    },
    /// The door's world transform is not rigid, so a leaf width is not a
    /// length along its axes.
    NonRigidPlacement {
        /// The door.
        door: EntityId,
    },
    /// The door's opening plane is vertical in the world (its local z is
    /// horizontal), so a hinge has no left or right seen from above.
    NoPlanHandedness {
        /// The door.
        door: EntityId,
    },
}

impl fmt::Display for DoorOperationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "door operation cannot be derived exactly: {self:?}")
    }
}

impl std::error::Error for DoorOperationError {}

impl From<ExactPropertyError> for DoorOperationError {
    fn from(error: ExactPropertyError) -> Self {
        Self::Property(error)
    }
}

impl From<ExactUnitError> for DoorOperationError {
    fn from(error: ExactUnitError) -> Self {
        Self::Unit(error)
    }
}

impl DoorOperationError {
    /// The door-level error for a shared read failure.
    fn read(error: ReadError) -> Self {
        match error {
            ReadError::Malformed { entity, attribute } => {
                Self::MalformedAttribute { entity, attribute }
            }
            ReadError::Unit(error) => Self::Unit(error),
            ReadError::Placement(error) => Self::Placement(error),
        }
    }
}

/// The operation geometry of one door: each leaf's frame, width, hinge side
/// and swing sector, in world metres.
///
/// Binds to the release the header declares (IFC2X3, IFC4 or IFC4X3), as
/// the exact property resolver does, and reads:
///
/// - the operation type: `IfcDoor.OperationType` (IFC4, IFC4X3) and the
///   type object's `OperationType` (`IfcDoorType`; IFC2X3 `IfcDoorStyle`).
///   Either alone governs; when both are stated they must agree;
/// - `IfcDoor.OverallWidth`, in the project length unit resolved by
///   [`ifc_properties::exact_unit`];
/// - the `IfcDoorPanelProperties` from
///   [`ifc_properties::exact_predefined_sets`]: the occurrence's own sets
///   when it has any, else its type's, never a mix;
/// - the placement from [`ifc_geometry::product_world_transform`], scaled by
///   the same exact length unit.
///
/// See the module documentation for the convention and its citations.
///
/// # Errors
///
/// A [`DoorOperationError`] whenever any input is missing, contradictory or
/// outside what the specification defines precisely; nothing is defaulted
/// except `PanelWidth` of a single panel, which the schema documents as 1.
pub fn door_operation(model: &Model, door: EntityId) -> Result<DoorOperation, DoorOperationError> {
    let inputs = read::read_door(model, door)?;
    let leaves = layout::place_leaves(inputs.operation, &inputs.panels)?;
    let placement = geometry::DoorFrame::new(door, inputs.world)?;
    let leaves = leaves
        .into_iter()
        .map(|placed| placement.leaf(&placed, inputs.overall_width))
        .collect::<Result<_, _>>()?;
    Ok(DoorOperation {
        door,
        operation: inputs.operation,
        operation_source: inputs.operation_source,
        panel_source: inputs.panel_source,
        overall_width: inputs.overall_width,
        leaves,
    })
}
