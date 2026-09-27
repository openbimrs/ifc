//! From operation type and panel sets to leaves along the door's width.
//!
//! The table below is `IfcDoorTypeOperationEnum` (IFC4 ADD2 TC1, IFC4X3
//! ADD2) and `IfcDoorStyleOperationEnum` (IFC2X3 TC1) read against the
//! IFC4 documentation; members the releases share are spelled alike.
//!
//! | Operation | Panels (position: `PanelOperation`, motion) |
//! | --- | --- |
//! | `SINGLE_SWING_LEFT`/`_RIGHT` | one: `SWINGING`, hinged left/right |
//! | `DOUBLE_SWING_LEFT`/`_RIGHT` | one: `DOUBLE_ACTING`, hinged left/right |
//! | `SLIDING_TO_LEFT`/`_RIGHT` | one: `SLIDING`, towards -x/+x |
//! | `ROLLINGUP` | one: `ROLLINGUP` |
//! | `DOUBLE_DOOR_SINGLE_SWING` | LEFT: `SWINGING` hinged left; RIGHT: `SWINGING` hinged right |
//! | `DOUBLE_DOOR_DOUBLE_SWING` | LEFT: `DOUBLE_ACTING` hinged left; RIGHT: hinged right |
//! | `DOUBLE_DOOR_SLIDING` | LEFT: `SLIDING` towards -x; RIGHT: towards +x |
//! | `SWING_FIXED_LEFT`/`_RIGHT` | LEFT and RIGHT: one `SWINGING` hinged left/right, one `FIXEDPANEL` |
//!
//! The two-leaf rows follow `IfcDoorPanelPositionEnum` Figure 317 (LEFT
//! hinged at x = 0, RIGHT at the far jamb) and Figure 189 of
//! `IfcDoorTypeOperationEnum`. `SWING_FIXED_*` says only on which side of the
//! swinging panel the hinges are; which position swings is read from the
//! panel sets, never assumed.

use std::sync::Arc;

use ifc_model::EntityId;

use super::read::Panel;
use super::{DoorOperationError, DoorOperationType, PanelPosition, RefusedOperation, Side};

/// How a placed leaf moves, in the door's local terms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Motion {
    /// Swings into +y, hinged on the given local side.
    Swing(Side),
    /// Swings both ways, hinged on the given local side.
    DoubleSwing(Side),
    /// Slides towards the given local side.
    Slide(Side),
    RollUp,
    Fixed,
}

/// A leaf placed along the width, in fractions of `OverallWidth`.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Placed {
    pub(super) set: EntityId,
    pub(super) position: PanelPosition,
    pub(super) motion: Motion,
    /// Low-x edge of the closed leaf.
    pub(super) start: f64,
    pub(super) width: f64,
}

/// Tolerance on the sum of two `PanelWidth` fractions: authoring tools
/// write ratios such as `0.6667`/`0.3333`.
const PARTITION_TOLERANCE: f64 = 1e-6;

/// The supported operation named by an enumeration constant, or why it is
/// refused.
pub(super) fn classify(written: &Arc<str>) -> Result<DoorOperationType, DoorOperationError> {
    use DoorOperationType as T;
    let refuse = |reason| {
        Err(DoorOperationError::RefusedOperation {
            operation: written.clone(),
            reason,
        })
    };
    Ok(match &**written {
        "SINGLE_SWING_LEFT" => T::SingleSwingLeft,
        "SINGLE_SWING_RIGHT" => T::SingleSwingRight,
        "DOUBLE_DOOR_SINGLE_SWING" => T::DoubleDoorSingleSwing,
        "DOUBLE_SWING_LEFT" => T::DoubleSwingLeft,
        "DOUBLE_SWING_RIGHT" => T::DoubleSwingRight,
        "DOUBLE_DOOR_DOUBLE_SWING" => T::DoubleDoorDoubleSwing,
        "SLIDING_TO_LEFT" => T::SlidingToLeft,
        "SLIDING_TO_RIGHT" => T::SlidingToRight,
        "DOUBLE_DOOR_SLIDING" => T::DoubleDoorSliding,
        "ROLLINGUP" => T::RollingUp,
        "SWING_FIXED_LEFT" => T::SwingFixedLeft,
        "SWING_FIXED_RIGHT" => T::SwingFixedRight,
        "NOTDEFINED" => return refuse(RefusedOperation::NotDefined),
        "USERDEFINED" => return refuse(RefusedOperation::UserDefined),
        "REVOLVING" | "REVOLVING_VERTICAL" => return refuse(RefusedOperation::Revolving),
        "FOLDING_TO_LEFT" | "FOLDING_TO_RIGHT" | "DOUBLE_DOOR_FOLDING" => {
            return refuse(RefusedOperation::Folding)
        }
        "DOUBLE_DOOR_SINGLE_SWING_OPPOSITE_LEFT" | "DOUBLE_DOOR_SINGLE_SWING_OPPOSITE_RIGHT" => {
            return refuse(RefusedOperation::AmbiguousSwingDirection)
        }
        // IFC4X3 `LIFTING_HORIZONTAL`, `LIFTING_VERTICAL_LEFT`/`_RIGHT` and
        // `DOUBLE_DOOR_LIFTING_VERTICAL`; the reader has already checked the
        // constant against the release's enumeration, so nothing else
        // reaches this arm.
        _ => return refuse(RefusedOperation::Lifting),
    })
}

/// Leaves of `operation` placed from `panels`, ordered by position.
pub(super) fn place_leaves(
    operation: DoorOperationType,
    panels: &[Panel],
) -> Result<Vec<Placed>, DoorOperationError> {
    use DoorOperationType as T;
    use Side::{Left, Right};
    match operation {
        T::SingleSwingLeft => single(panels, "SWINGING", Motion::Swing(Left)),
        T::SingleSwingRight => single(panels, "SWINGING", Motion::Swing(Right)),
        T::DoubleSwingLeft => single(panels, "DOUBLE_ACTING", Motion::DoubleSwing(Left)),
        T::DoubleSwingRight => single(panels, "DOUBLE_ACTING", Motion::DoubleSwing(Right)),
        T::SlidingToLeft => single(panels, "SLIDING", Motion::Slide(Left)),
        T::SlidingToRight => single(panels, "SLIDING", Motion::Slide(Right)),
        T::RollingUp => single(panels, "ROLLINGUP", Motion::RollUp),
        T::DoubleDoorSingleSwing => pair(
            panels,
            "SWINGING",
            [Motion::Swing(Left), Motion::Swing(Right)],
        ),
        T::DoubleDoorDoubleSwing => pair(
            panels,
            "DOUBLE_ACTING",
            [Motion::DoubleSwing(Left), Motion::DoubleSwing(Right)],
        ),
        T::DoubleDoorSliding => pair(
            panels,
            "SLIDING",
            [Motion::Slide(Left), Motion::Slide(Right)],
        ),
        T::SwingFixedLeft => swing_fixed(panels, Left),
        T::SwingFixedRight => swing_fixed(panels, Right),
    }
}

fn mismatch(panel: &Panel, attribute: &'static str) -> DoorOperationError {
    DoorOperationError::PanelMismatch {
        set: panel.set,
        attribute,
        found: match attribute {
            "PanelPosition" => format!("{:?}", panel.position).to_ascii_uppercase().into(),
            _ => panel.operation.clone(),
        },
    }
}

fn count(panels: &[Panel], expected: usize) -> Result<(), DoorOperationError> {
    if panels.len() == expected {
        Ok(())
    } else {
        Err(DoorOperationError::PanelCount {
            expected,
            found: panels.len(),
        })
    }
}

/// A stated `PanelWidth` in `(0, 1]` (`IfcNormalisedRatioMeasure`, and a
/// panel of zero width is no panel).
fn fraction(panel: &Panel, width: f64) -> Result<f64, DoorOperationError> {
    if width > 0.0 && width <= 1.0 {
        Ok(width)
    } else {
        Err(DoorOperationError::InvalidPanelWidth {
            set: panel.set,
            width,
        })
    }
}

/// The one panel of a single-leaf operation. `PanelWidth` `$` is 1, the
/// default the schema documents. A narrower panel must say which side it
/// stands on.
fn single(
    panels: &[Panel],
    operation: &str,
    motion: Motion,
) -> Result<Vec<Placed>, DoorOperationError> {
    count(panels, 1)?;
    let panel = &panels[0];
    if &*panel.operation != operation {
        return Err(mismatch(panel, "PanelOperation"));
    }
    let width = fraction(panel, panel.width.unwrap_or(1.0))?;
    let start = match panel.position {
        _ if width == 1.0 => 0.0,
        PanelPosition::Left => 0.0,
        PanelPosition::Right => 1.0 - width,
        PanelPosition::Middle | PanelPosition::NotDefined => {
            return Err(DoorOperationError::UnplacedPanel { set: panel.set })
        }
    };
    Ok(vec![Placed {
        set: panel.set,
        position: panel.position,
        motion,
        start,
        width,
    }])
}

/// The LEFT and RIGHT panels of a two-leaf operation whose panels both
/// state `operation` and move as `motions` (LEFT, RIGHT).
fn pair(
    panels: &[Panel],
    operation: &'static str,
    motions: [Motion; 2],
) -> Result<Vec<Placed>, DoorOperationError> {
    pair_with(panels, |_| Ok((operation, motions)))
}

/// Two panels at LEFT and RIGHT that partition the width. `expect` gives,
/// for the pair in (LEFT, RIGHT) order, the `PanelOperation` both must state
/// (`*` when it checked them itself) and their motions.
fn pair_with(
    panels: &[Panel],
    expect: impl Fn(&[&Panel; 2]) -> Result<(&'static str, [Motion; 2]), DoorOperationError>,
) -> Result<Vec<Placed>, DoorOperationError> {
    count(panels, 2)?;
    let at = |position| {
        let mut found = panels.iter().filter(|panel| panel.position == position);
        match (found.next(), found.next()) {
            (Some(panel), None) => Ok(panel),
            // Two at one position leaves the other unstated; report the one
            // whose position cannot stand.
            (Some(_), Some(second)) => Err(mismatch(second, "PanelPosition")),
            (None, _) => {
                let stray = panels
                    .iter()
                    .find(|panel| {
                        !matches!(panel.position, PanelPosition::Left | PanelPosition::Right)
                    })
                    .unwrap_or(&panels[0]);
                Err(mismatch(stray, "PanelPosition"))
            }
        }
    };
    let ordered = [at(PanelPosition::Left)?, at(PanelPosition::Right)?];
    let (operation, motions) = expect(&ordered)?;
    let mut widths = [0.0; 2];
    for (slot, panel) in ordered.iter().enumerate() {
        let stated = operation == "*" || &*panel.operation == operation;
        if !stated {
            return Err(mismatch(panel, "PanelOperation"));
        }
        let width = panel
            .width
            .ok_or(DoorOperationError::MissingPanelWidth { set: panel.set })?;
        widths[slot] = fraction(panel, width)?;
    }
    let sum = widths[0] + widths[1];
    if (sum - 1.0).abs() > PARTITION_TOLERANCE {
        return Err(DoorOperationError::PanelWidthsDoNotPartition { sum });
    }
    Ok(vec![
        Placed {
            set: ordered[0].set,
            position: PanelPosition::Left,
            motion: motions[0],
            start: 0.0,
            width: widths[0],
        },
        Placed {
            set: ordered[1].set,
            position: PanelPosition::Right,
            motion: motions[1],
            start: 1.0 - widths[1],
            width: widths[1],
        },
    ])
}

/// `SWING_FIXED_*`: one `SWINGING` and one `FIXEDPANEL` at LEFT and RIGHT,
/// whichever way round the sets say.
fn swing_fixed(panels: &[Panel], hinge: Side) -> Result<Vec<Placed>, DoorOperationError> {
    pair_with(panels, |ordered| {
        let motion = |panel: &Panel| match &*panel.operation {
            "SWINGING" => Ok(Motion::Swing(hinge)),
            "FIXEDPANEL" => Ok(Motion::Fixed),
            _ => Err(mismatch(panel, "PanelOperation")),
        };
        let motions = [motion(ordered[0])?, motion(ordered[1])?];
        if motions[0] == motions[1] {
            return Err(mismatch(ordered[1], "PanelOperation"));
        }
        Ok(("*", motions))
    })
}
