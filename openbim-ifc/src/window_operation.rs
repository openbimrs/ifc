//! Window operation geometry: where each panel is, which side it hangs on,
//! and what it sweeps when it opens (#170). The window counterpart of
//! [`door_operation`](crate::door_operation).
//!
//! # Why this lives in the facade
//!
//! The answer joins the window's placement (`ifc-geometry`) with its
//! partitioning and `IfcWindowPanelProperties`/`IfcWindowLiningProperties`
//! (`ifc-properties`). ADR 0003 forbids those sibling crates from depending
//! on each other, so the join is an orchestration item, compiled only with
//! both `geometry-select` and `properties`.
//!
//! # The convention, from the IFC4 ADD2 TC1 documentation
//!
//! - The window's local placement fixes the frame. `OverallWidth` "reflects
//!   the X Dimension" and `OverallHeight` "the Z Dimension" of the window
//!   opening; IfcWindow Figure 296 draws them as "the extent of the window in
//!   the positive Z and X axis of the local placement". Panels lie in the
//!   placement's XZ plane.
//! - "The IfcWindow only defines the local placement which determines the
//!   opening direction of the window", and "all window panels are assumed to
//!   open into the same direction" (IfcWindow, "Window opening operation by
//!   window type"). "The window panel (for side hung windows) opens always
//!   into the direction of the positive Y axis of the local placement"
//!   (Figure 298), and "the positive y-axis determines the direction"
//!   (IfcWindowPanelOperationEnum, Figure 321).
//! - `SIDEHUNGLEFTHAND` is a "panel that opens to the left when viewed from
//!   the outside", `SIDEHUNGRIGHTHAND` "to the right"; the figures "are shown
//!   as viewed from the outside (in direction of the positive y-axis)"
//!   (IfcWindowPanelOperationEnum). Looking along +y with +z up, +x is on the
//!   right, so left is local low x. The plan drawings of Figure 298 agree:
//!   the `SideHungLeftHand` panel is hinged at the placement origin, the
//!   `SideHungRightHand` panel at the far jamb, both opening towards +y.
//! - `TILTANDTURNLEFTHAND` is a "panel that opens to the left and is bottom
//!   hung", `TILTANDTURNRIGHTHAND` "to the right"; `TOPHUNG` "panel is top
//!   hung" and `BOTTOMHUNG` "bottom hung".
//! - `PanelPosition` places each panel in the partitioning, drawn "as
//!   elevations in the XZ plane of the local placement of the window,
//!   looking into the direction of the positive Y axis"
//!   (IfcWindowPanelPositionEnum, Figure 321), which lists the positions per
//!   partitioning, e.g. `DOUBLE_PANEL_VERTICAL` "first ... LEFT, second ...
//!   RIGHT". Panels are returned in that listed order.
//! - Where the panels meet is `IfcWindowLiningProperties`: `FirstMullionOffset`
//!   and `SecondMullionOffset` are the mullion centrelines "measured along the
//!   x-axis of the window placement co-ordinate system", `FirstTransomOffset`
//!   the transom centreline "measured along the z-axis", each a ratio of the
//!   window ("0.5 indicates that the mullion is positioned in the middle").
//!   `SecondTransomOffset`'s text says "measured along the x-axis", but
//!   Figure 326 draws it from the window's bottom along z, above
//!   `FirstTransomOffset`, and a transom is the "horizontal separator"; the
//!   figure is followed. Which offsets apply to which partitioning is
//!   Figure 326's list.
//!
//! Panels therefore tile `[0, OverallWidth] × [0, OverallHeight]` of the
//! placement's XZ plane at y = 0, split at the mullion and transom
//! centrelines. Lining, mullion and transom thicknesses and the IFC4 lining
//! offsets are **not** applied, as `door_operation` does not apply the door
//! lining; a caller that needs the panel inside the lining reads those
//! attributes itself. `FrameDepth` and `FrameThickness` are reported, not
//! applied.
//!
//! A hinged panel's sector is the quarter turn from closed to perpendicular
//! to the window plane, towards +y, as the door sectors are. IFC records no
//! opening angle, and Figure 298 draws the panel only ajar: the sector is
//! the symbolic bound, not a hardware stop.
//!
//! IFC2X3 has no `IfcWindow.PartitioningType`; its `IfcWindowStyle` states
//! the partitioning as `OperationType : IfcWindowStyleOperationEnum`, whose
//! members IFC4 kept as `IfcWindowTypePartitioningEnum` (the EXPRESS of both
//! releases lists them alike). The panel and lining sets hang off the
//! style's `HasPropertySets`, as they hang off an `IfcWindowType`'s.
//!
//! # What is refused
//!
//! Never defaulted: the `NOTDEFINED` and `USERDEFINED` partitionings
//! ("windows which are subdivided into more than three panels have to be
//! defined by the geometry only"); the `PIVOTHORIZONTAL` and `PIVOTVERTICAL`
//! panel operations ("hinges are in the middle", but which half turns
//! towards +y is not stated), `OTHEROPERATION` and `NOTDEFINED`; a window
//! without panel properties, `OverallWidth` or `OverallHeight`; panels that
//! contradict the partitioning; and a split whose lining offset is missing
//! or does not fall inside the window. See [`WindowOperationError`].

mod error;
mod geometry;
mod layout;
mod read;

#[cfg(test)]
mod tests;

use ifc_geometry::Transform;
use ifc_model::{EntityId, Model};
use ifc_properties::ExactSource;

use crate::operation::{Sector, Side};

pub use error::WindowOperationError;

/// How one window operates, panel by panel, in world coordinates (metres).
///
/// Returned by [`window_operation`].
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct WindowOperation {
    /// The `IfcWindow` (or IFC4 `IfcWindowStandardCase`).
    pub window: EntityId,
    /// The partitioning the panels were laid out by.
    pub partitioning: WindowPartitioning,
    /// Where the partitioning was stated: the occurrence's
    /// `PartitioningType`, or its type object's (`IfcWindowType`
    /// `PartitioningType`, IFC2X3 `IfcWindowStyle` `OperationType`).
    pub partitioning_source: ExactSource,
    /// Where the `IfcWindowPanelProperties` came from: the occurrence's own
    /// sets when it has any, else its type's.
    pub panel_source: ExactSource,
    /// Where the `IfcWindowLiningProperties` that split the panels came
    /// from, chosen the same way; `None` for a single panel, which needs no
    /// split and reads none.
    pub lining_source: Option<ExactSource>,
    /// `IfcWindow.OverallWidth` in metres.
    pub overall_width: f64,
    /// `IfcWindow.OverallHeight` in metres.
    pub overall_height: f64,
    /// One entry per panel, in the order `IfcWindowPanelPositionEnum`
    /// (Figure 321) lists the partitioning's positions.
    pub panels: Vec<WindowPanel>,
}

/// `IfcWindowTypePartitioningEnum` (IFC4, IFC4X3) and
/// `IfcWindowStyleOperationEnum` (IFC2X3): the panel layouts derived here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum WindowPartitioning {
    /// `SINGLE_PANEL`.
    SinglePanel,
    /// `DOUBLE_PANEL_VERTICAL`: LEFT and RIGHT, split by a mullion.
    DoublePanelVertical,
    /// `DOUBLE_PANEL_HORIZONTAL`: TOP and BOTTOM, split by a transom.
    DoublePanelHorizontal,
    /// `TRIPLE_PANEL_VERTICAL`: LEFT, MIDDLE and RIGHT.
    TriplePanelVertical,
    /// `TRIPLE_PANEL_HORIZONTAL`: TOP, MIDDLE and BOTTOM.
    TriplePanelHorizontal,
    /// `TRIPLE_PANEL_BOTTOM`: LEFT and RIGHT above a full-width BOTTOM.
    TriplePanelBottom,
    /// `TRIPLE_PANEL_TOP`: a full-width TOP above LEFT and RIGHT.
    TriplePanelTop,
    /// `TRIPLE_PANEL_LEFT`: a full-height LEFT beside TOP and BOTTOM.
    TriplePanelLeft,
    /// `TRIPLE_PANEL_RIGHT`: TOP and BOTTOM beside a full-height RIGHT.
    TriplePanelRight,
}

/// `IfcWindowPanelOperationEnum`: the panel operations derived here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum WindowPanelOperation {
    /// `SIDEHUNGRIGHTHAND`: hinged on the right, seen looking along +y.
    SideHungRightHand,
    /// `SIDEHUNGLEFTHAND`: hinged on the left, seen looking along +y.
    SideHungLeftHand,
    /// `TILTANDTURNRIGHTHAND`: turns on a right hinge, tilts on the bottom.
    TiltAndTurnRightHand,
    /// `TILTANDTURNLEFTHAND`: turns on a left hinge, tilts on the bottom.
    TiltAndTurnLeftHand,
    /// `TOPHUNG`.
    TopHung,
    /// `BOTTOMHUNG`.
    BottomHung,
    /// `SLIDINGHORIZONTAL`.
    SlidingHorizontal,
    /// `SLIDINGVERTICAL`.
    SlidingVertical,
    /// `REMOVABLECASEMENT`.
    RemovableCasement,
    /// `FIXEDCASEMENT`.
    FixedCasement,
}

/// `IfcWindowPanelPositionEnum`: where a panel sits in the window, as
/// written.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum WindowPanelPosition {
    /// `LEFT`: at local low x.
    Left,
    /// `MIDDLE`.
    Middle,
    /// `RIGHT`: at local high x.
    Right,
    /// `BOTTOM`: at local low z.
    Bottom,
    /// `TOP`: at local high z.
    Top,
    /// `NOTDEFINED`.
    NotDefined,
}

/// How a window panel moves.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum WindowPanelMotion {
    /// Turns about a vertical hinge on one side into local +y
    /// (`SIDEHUNG*`).
    Swing,
    /// Turns as [`Swing`](Self::Swing), or tilts about its bottom edge into
    /// local +y (`TILTANDTURN*`).
    TiltAndTurn,
    /// Tilts about its top edge into local +y (`TOPHUNG`).
    TopHung,
    /// Tilts about its bottom edge into local +y (`BOTTOMHUNG`).
    BottomHung,
    /// Slides within the window plane (`SLIDINGHORIZONTAL`,
    /// `SLIDINGVERTICAL`), sweeping nothing outside it.
    Slide {
        /// Unit world direction of the line it slides along: the image of
        /// local +x, or of local +z for a vertical slide. IFC does not state
        /// which way along it the panel opens.
        along: [f64; 3],
    },
    /// Is taken out rather than opened (`REMOVABLECASEMENT`).
    Removable,
    /// Does not open (`FIXEDCASEMENT`).
    Fixed,
}

/// One window panel.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct WindowPanel {
    /// The `IfcWindowPanelProperties` this panel was read from.
    pub panel_set: EntityId,
    /// The set's `PanelPosition`.
    pub position: WindowPanelPosition,
    /// The set's `OperationType`.
    pub operation: WindowPanelOperation,
    /// How the panel moves.
    pub motion: WindowPanelMotion,
    /// The panel's frame in world metres: the window's own axes, translated
    /// to the closed panel's low-x, low-z corner in the placement's XZ
    /// plane. The closed panel spans local `x ∈ [0, width]`,
    /// `z ∈ [0, height]` of this frame; a mirrored window placement stays
    /// mirrored here.
    pub frame_world: Transform,
    /// Panel width in metres, along local x.
    pub width: f64,
    /// Panel height in metres, along local z.
    pub height: f64,
    /// The side of a vertical hinge (side-hung and tilt-and-turn panels), as
    /// seen looking along the panel's opening direction (local +y) with
    /// world +z up: the operation's hand when the placement keeps plan
    /// handedness, the other side when it reverses it (a mirrored or
    /// upside-down placement). `None` for a panel with no side hinge.
    pub hinge_side: Option<Side>,
    /// What a panel turning on its side hinge sweeps: the sector in the
    /// plane of the panel's local low-z edge, centred on the hinge; the
    /// panel sweeps it along local +z for `height`. `None` without a side
    /// hinge.
    pub swing: Option<Sector>,
    /// What a panel tilting on its top or bottom hinge sweeps: the sector in
    /// the plane of the panel's local low-x edge, centred on the hinge; the
    /// panel sweeps it along local +x for `width`. `None` without a top or
    /// bottom hinge.
    pub tilt: Option<Sector>,
    /// `FrameDepth` in metres, perpendicular to the window plane; `None`
    /// for `$`.
    pub frame_depth: Option<f64>,
    /// `FrameThickness` in metres, within the window plane; `None` for `$`.
    pub frame_thickness: Option<f64>,
}

/// Why a partitioning or panel operation is refused rather than derived.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum RefusedWindowOperation {
    /// `NOTDEFINED`: nothing is stated.
    NotDefined,
    /// `USERDEFINED`: its meaning is free text, and a window of more than
    /// three panels is "defined by the geometry only".
    UserDefined,
    /// `OTHEROPERATION`: a "user defined operation type".
    OtherOperation,
    /// `PIVOTHORIZONTAL`/`PIVOTVERTICAL`: hinged in the middle, but which
    /// half turns towards local +y is not stated.
    Pivot,
}

/// The operation geometry of one window: each panel's frame, size, hinge
/// side and swept sectors, in world metres.
///
/// Binds to the release the header declares (IFC2X3, IFC4 or IFC4X3), as
/// the exact property resolver does, and reads:
///
/// - the partitioning: `IfcWindow.PartitioningType` (IFC4, IFC4X3) and the
///   type object's (`IfcWindowType.PartitioningType`; IFC2X3
///   `IfcWindowStyle.OperationType`). Either alone governs; when both are
///   stated they must agree;
/// - `IfcWindow.OverallWidth` and `OverallHeight`, in the project length
///   unit resolved by [`ifc_properties::exact_unit`];
/// - the `IfcWindowPanelProperties` from
///   [`ifc_properties::exact_predefined_sets`]: the occurrence's own sets
///   when it has any, else its type's, never a mix; and, for a partitioning
///   with more than one panel, the one `IfcWindowLiningProperties` chosen
///   the same way;
/// - the placement from [`ifc_geometry::product_world_transform`], scaled by
///   the same exact length unit.
///
/// See the module documentation for the convention and its citations.
///
/// # Errors
///
/// A [`WindowOperationError`] whenever any input is missing, contradictory
/// or outside what the specification defines precisely; nothing is
/// defaulted.
pub fn window_operation(
    model: &Model,
    window: EntityId,
) -> Result<WindowOperation, WindowOperationError> {
    let inputs = read::read_window(model, window)?;
    let placed = layout::place_panels(
        window,
        inputs.partitioning,
        &inputs.panels,
        inputs.lining.as_ref(),
    )?;
    let frame = geometry::WindowFrame::new(window, inputs.world)?;
    let size = [inputs.overall_width, inputs.overall_height];
    let panels = placed
        .iter()
        .map(|panel| frame.panel(panel, size))
        .collect::<Result<_, _>>()?;
    Ok(WindowOperation {
        window,
        partitioning: inputs.partitioning,
        partitioning_source: inputs.partitioning_source,
        panel_source: inputs.panel_source,
        lining_source: inputs.lining.as_ref().map(|lining| lining.source),
        overall_width: inputs.overall_width,
        overall_height: inputs.overall_height,
        panels,
    })
}
