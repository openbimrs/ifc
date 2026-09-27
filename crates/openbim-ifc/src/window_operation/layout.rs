//! From partitioning, panel sets and lining to panels in the window plane.
//!
//! The table is `IfcWindowPanelPositionEnum` Figure 321 (IFC4 ADD2 TC1),
//! which lists each partitioning's positions in order, read against
//! `IfcWindowLiningProperties` Figure 326 for where they meet (`m1`, `m2`:
//! `FirstMullionOffset`, `SecondMullionOffset` along x; `t1`, `t2`:
//! `FirstTransomOffset`, `SecondTransomOffset` along z; all ratios of the
//! window measured from the placement origin):
//!
//! | Partitioning | Positions, in order: x range × z range |
//! | --- | --- |
//! | `SINGLE_PANEL` | any: `[0,1] × [0,1]` |
//! | `DOUBLE_PANEL_VERTICAL` | LEFT `[0,m1]`, RIGHT `[m1,1]`, full height |
//! | `DOUBLE_PANEL_HORIZONTAL` | TOP `[t1,1]`, BOTTOM `[0,t1]`, full width |
//! | `TRIPLE_PANEL_VERTICAL` | LEFT `[0,m1]`, MIDDLE `[m1,m2]`, RIGHT `[m2,1]` |
//! | `TRIPLE_PANEL_HORIZONTAL` | TOP `[t2,1]`, MIDDLE `[t1,t2]`, BOTTOM `[0,t1]` |
//! | `TRIPLE_PANEL_BOTTOM` | LEFT `[0,m1]×[t1,1]`, RIGHT `[m1,1]×[t1,1]`, BOTTOM `[0,1]×[0,t1]` |
//! | `TRIPLE_PANEL_TOP` | TOP `[0,1]×[t1,1]`, LEFT `[0,m1]×[0,t1]`, RIGHT `[m1,1]×[0,t1]` |
//! | `TRIPLE_PANEL_LEFT` | LEFT `[0,m1]×[0,1]`, TOP `[m1,1]×[t1,1]`, BOTTOM `[m1,1]×[0,t1]` |
//! | `TRIPLE_PANEL_RIGHT` | TOP `[0,m1]×[t1,1]`, BOTTOM `[0,m1]×[0,t1]`, RIGHT `[m1,1]×[0,1]` |
//!
//! `SINGLE_PANEL` is not in Figure 321's table, so its one panel may state
//! any position. `IfcTypeObject.HasPropertySets` is a `SET`, so the sets
//! are matched to the table by `PanelPosition`, never by their order.

use std::sync::Arc;

use ifc_model::EntityId;

use super::read::{Lining, Panel};
use super::{
    RefusedWindowOperation, WindowOperationError, WindowPanelOperation, WindowPanelPosition,
    WindowPartitioning,
};

/// A panel placed in the window plane, in ratios of the overall size.
#[derive(Debug, Clone)]
pub(super) struct Placed {
    pub(super) panel: Panel,
    /// `[low, high]` along local x.
    pub(super) x: [f64; 2],
    /// `[low, high]` along local z.
    pub(super) z: [f64; 2],
}

/// A position of the partitioning and its `[low, high]` ratios along local
/// x and z.
type Cell = (WindowPanelPosition, [f64; 2], [f64; 2]);

/// The whole extent along one axis.
const ALL: [f64; 2] = [0.0, 1.0];

/// The supported partitioning named by an enumeration constant, or why it
/// is refused.
pub(super) fn classify_partitioning(
    written: &Arc<str>,
) -> Result<WindowPartitioning, WindowOperationError> {
    use WindowPartitioning as P;
    let refuse = |reason| {
        Err(WindowOperationError::RefusedPartitioning {
            partitioning: written.clone(),
            reason,
        })
    };
    Ok(match &**written {
        "SINGLE_PANEL" => P::SinglePanel,
        "DOUBLE_PANEL_VERTICAL" => P::DoublePanelVertical,
        "DOUBLE_PANEL_HORIZONTAL" => P::DoublePanelHorizontal,
        "TRIPLE_PANEL_VERTICAL" => P::TriplePanelVertical,
        "TRIPLE_PANEL_HORIZONTAL" => P::TriplePanelHorizontal,
        "TRIPLE_PANEL_BOTTOM" => P::TriplePanelBottom,
        "TRIPLE_PANEL_TOP" => P::TriplePanelTop,
        "TRIPLE_PANEL_LEFT" => P::TriplePanelLeft,
        "TRIPLE_PANEL_RIGHT" => P::TriplePanelRight,
        "USERDEFINED" => return refuse(RefusedWindowOperation::UserDefined),
        // `NOTDEFINED`; the reader has already checked the constant against
        // the release's enumeration, so nothing else reaches this arm.
        _ => return refuse(RefusedWindowOperation::NotDefined),
    })
}

/// The supported panel operation named by an enumeration constant, or why
/// it is refused.
pub(super) fn classify_panel(
    set: EntityId,
    written: &Arc<str>,
) -> Result<WindowPanelOperation, WindowOperationError> {
    use WindowPanelOperation as O;
    let refuse = |reason| {
        Err(WindowOperationError::RefusedPanelOperation {
            set,
            operation: written.clone(),
            reason,
        })
    };
    Ok(match &**written {
        "SIDEHUNGRIGHTHAND" => O::SideHungRightHand,
        "SIDEHUNGLEFTHAND" => O::SideHungLeftHand,
        "TILTANDTURNRIGHTHAND" => O::TiltAndTurnRightHand,
        "TILTANDTURNLEFTHAND" => O::TiltAndTurnLeftHand,
        "TOPHUNG" => O::TopHung,
        "BOTTOMHUNG" => O::BottomHung,
        "SLIDINGHORIZONTAL" => O::SlidingHorizontal,
        "SLIDINGVERTICAL" => O::SlidingVertical,
        "REMOVABLECASEMENT" => O::RemovableCasement,
        "FIXEDCASEMENT" => O::FixedCasement,
        "PIVOTHORIZONTAL" | "PIVOTVERTICAL" => return refuse(RefusedWindowOperation::Pivot),
        "OTHEROPERATION" => return refuse(RefusedWindowOperation::OtherOperation),
        // `NOTDEFINED`, as above.
        _ => return refuse(RefusedWindowOperation::NotDefined),
    })
}

/// Panels of `partitioning` placed from `panels` and the lining, in the
/// order Figure 321 lists them.
pub(super) fn place_panels(
    window: EntityId,
    partitioning: WindowPartitioning,
    panels: &[Panel],
    lining: Option<&Lining>,
) -> Result<Vec<Placed>, WindowOperationError> {
    if partitioning == WindowPartitioning::SinglePanel {
        // Figure 321 lists no position for a single panel.
        count(panels, 1)?;
        return Ok(vec![Placed {
            panel: panels[0].clone(),
            x: ALL,
            z: ALL,
        }]);
    }
    let lining = lining.ok_or(WindowOperationError::NoLiningProperties { window })?;
    let cells = cells(partitioning, lining)?;
    count(panels, cells.len())?;
    let listed: Vec<WindowPanelPosition> = cells.iter().map(|cell| cell.0).collect();
    cells
        .into_iter()
        .map(|(position, x, z)| {
            Ok(Placed {
                panel: at(panels, position, &listed)?.clone(),
                x,
                z,
            })
        })
        .collect()
}

/// The positions of a split partitioning and their extents.
fn cells(
    partitioning: WindowPartitioning,
    lining: &Lining,
) -> Result<Vec<Cell>, WindowOperationError> {
    use WindowPanelPosition::{Bottom, Left, Middle, Right, Top};
    use WindowPartitioning as P;
    let mullion = || split(lining, lining.first_mullion, "FirstMullionOffset", 0.0);
    let transom = || split(lining, lining.first_transom, "FirstTransomOffset", 0.0);
    Ok(match partitioning {
        P::DoublePanelVertical => {
            let m1 = mullion()?;
            vec![(Left, [0.0, m1], ALL), (Right, [m1, 1.0], ALL)]
        }
        P::DoublePanelHorizontal => {
            let t1 = transom()?;
            vec![(Top, ALL, [t1, 1.0]), (Bottom, ALL, [0.0, t1])]
        }
        P::TriplePanelVertical => {
            let m1 = mullion()?;
            let m2 = split(lining, lining.second_mullion, "SecondMullionOffset", m1)?;
            vec![
                (Left, [0.0, m1], ALL),
                (Middle, [m1, m2], ALL),
                (Right, [m2, 1.0], ALL),
            ]
        }
        P::TriplePanelHorizontal => {
            let t1 = transom()?;
            let t2 = split(lining, lining.second_transom, "SecondTransomOffset", t1)?;
            vec![
                (Top, ALL, [t2, 1.0]),
                (Middle, ALL, [t1, t2]),
                (Bottom, ALL, [0.0, t1]),
            ]
        }
        P::TriplePanelBottom => {
            let (m1, t1) = (mullion()?, transom()?);
            vec![
                (Left, [0.0, m1], [t1, 1.0]),
                (Right, [m1, 1.0], [t1, 1.0]),
                (Bottom, ALL, [0.0, t1]),
            ]
        }
        P::TriplePanelTop => {
            let (m1, t1) = (mullion()?, transom()?);
            vec![
                (Top, ALL, [t1, 1.0]),
                (Left, [0.0, m1], [0.0, t1]),
                (Right, [m1, 1.0], [0.0, t1]),
            ]
        }
        P::TriplePanelLeft => {
            let (m1, t1) = (mullion()?, transom()?);
            vec![
                (Left, [0.0, m1], ALL),
                (Top, [m1, 1.0], [t1, 1.0]),
                (Bottom, [m1, 1.0], [0.0, t1]),
            ]
        }
        P::TriplePanelRight => {
            let (m1, t1) = (mullion()?, transom()?);
            vec![
                (Top, [0.0, m1], [t1, 1.0]),
                (Bottom, [0.0, m1], [0.0, t1]),
                (Right, [m1, 1.0], ALL),
            ]
        }
        // Handled by the caller: one panel, no split.
        P::SinglePanel => vec![(WindowPanelPosition::NotDefined, ALL, ALL)],
    })
}

/// A lining offset the partitioning needs: stated, strictly inside the
/// window, and beyond `after` when it is the second of a pair.
fn split(
    lining: &Lining,
    value: Option<f64>,
    attribute: &'static str,
    after: f64,
) -> Result<f64, WindowOperationError> {
    let value = value.ok_or(WindowOperationError::MissingSplit {
        set: lining.set,
        attribute,
    })?;
    if value > after && value < 1.0 {
        Ok(value)
    } else {
        Err(WindowOperationError::InvalidSplit {
            set: lining.set,
            attribute,
            value,
        })
    }
}

fn count(panels: &[Panel], expected: usize) -> Result<(), WindowOperationError> {
    if panels.len() == expected {
        Ok(())
    } else {
        Err(WindowOperationError::PanelCount {
            expected,
            found: panels.len(),
        })
    }
}

/// The one panel at `position`. With the count already checked, a missing
/// position means another panel states one the partitioning does not list
/// (`listed`), or repeats one.
fn at<'p>(
    panels: &'p [Panel],
    position: WindowPanelPosition,
    listed: &[WindowPanelPosition],
) -> Result<&'p Panel, WindowOperationError> {
    let mut found = panels.iter().filter(|panel| panel.position == position);
    match (found.next(), found.next()) {
        (Some(panel), None) => Ok(panel),
        (Some(_), Some(second)) => Err(mismatch(second)),
        (None, _) => {
            let foreign = panels
                .iter()
                .find(|panel| !listed.contains(&panel.position));
            let repeated = || {
                panels.iter().enumerate().find_map(|(index, panel)| {
                    panels[..index]
                        .iter()
                        .any(|earlier| earlier.position == panel.position)
                        .then_some(panel)
                })
            };
            let stray = foreign.or_else(repeated).unwrap_or(&panels[0]);
            Err(mismatch(stray))
        }
    }
}

fn mismatch(panel: &Panel) -> WindowOperationError {
    WindowOperationError::PanelMismatch {
        set: panel.set,
        attribute: "PanelPosition",
        found: position_name(panel.position).into(),
    }
}

/// The enumeration constant of `position`.
fn position_name(position: WindowPanelPosition) -> &'static str {
    match position {
        WindowPanelPosition::Left => "LEFT",
        WindowPanelPosition::Middle => "MIDDLE",
        WindowPanelPosition::Right => "RIGHT",
        WindowPanelPosition::Bottom => "BOTTOM",
        WindowPanelPosition::Top => "TOP",
        WindowPanelPosition::NotDefined => "NOTDEFINED",
    }
}
