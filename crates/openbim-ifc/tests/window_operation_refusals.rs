//! Window operation geometry (#170): every refused partitioning and panel
//! operation, and every input that is missing or contradicts the
//! partitioning, is a typed error, never a default.

#![cfg(all(feature = "step", feature = "properties", feature = "geometry-select"))]

mod door_support;
mod window_support;

use ifc::{window_operation, RefusedWindowOperation, WindowOperationError as E};
use ifc_properties::SchemaVersion;
use window_support::*;

const IFC4: SchemaVersion = SchemaVersion::Ifc4;

fn casement() -> Window {
    Window::new(
        IFC4,
        "SINGLE_PANEL",
        vec![(20, panel(IFC4, 20, "SIDEHUNGLEFTHAND", "LEFT"))],
    )
}

/// A `DOUBLE_PANEL_VERTICAL` window with LEFT and RIGHT casements and the
/// given lining offsets (no lining for `None`).
fn double(offsets: Option<&[(&str, &str)]>) -> Window {
    let mut sets = vec![
        (20, panel(IFC4, 20, "SIDEHUNGLEFTHAND", "LEFT")),
        (21, panel(IFC4, 21, "SIDEHUNGRIGHTHAND", "RIGHT")),
    ];
    if let Some(offsets) = offsets {
        sets.push((25, lining(IFC4, 25, offsets)));
    }
    Window::new(IFC4, "DOUBLE_PANEL_VERTICAL", sets)
}

fn refused(window: &Window) -> E {
    window_operation(&window.model(), WINDOW).expect_err("refused")
}

#[test]
fn each_refused_partitioning_names_its_reason() {
    use RefusedWindowOperation as R;
    for (version, partitioning, reason) in [
        (IFC4, "NOTDEFINED", R::NotDefined),
        (IFC4, "USERDEFINED", R::UserDefined),
        (SchemaVersion::Ifc2x3, "USERDEFINED", R::UserDefined),
        (SchemaVersion::Ifc4x3, "NOTDEFINED", R::NotDefined),
    ] {
        let window = Window::new(
            version,
            partitioning,
            vec![(20, panel(version, 20, "SIDEHUNGLEFTHAND", "LEFT"))],
        );
        assert_eq!(
            refused(&window),
            E::RefusedPartitioning {
                partitioning: partitioning.into(),
                reason,
            },
            "{partitioning}"
        );
    }
}

#[test]
fn each_refused_panel_operation_names_its_reason() {
    use RefusedWindowOperation as R;
    for (operation, reason) in [
        ("PIVOTHORIZONTAL", R::Pivot),
        ("PIVOTVERTICAL", R::Pivot),
        ("OTHEROPERATION", R::OtherOperation),
        ("NOTDEFINED", R::NotDefined),
    ] {
        let window = Window::new(
            IFC4,
            "SINGLE_PANEL",
            vec![(20, panel(IFC4, 20, operation, "LEFT"))],
        );
        assert_eq!(
            refused(&window),
            E::RefusedPanelOperation {
                set: EntityId(20),
                operation: operation.into(),
                reason,
            },
            "{operation}"
        );
    }
}

#[test]
fn a_window_without_panel_properties_is_refused() {
    let window = Window::new(IFC4, "SINGLE_PANEL", Vec::new());
    assert_eq!(refused(&window), E::NoPanelProperties { window: WINDOW });
    // A lining alone says nothing about the panels.
    let window = Window::new(
        IFC4,
        "DOUBLE_PANEL_VERTICAL",
        vec![(25, lining(IFC4, 25, &[("FirstMullionOffset", "0.5")]))],
    );
    assert_eq!(refused(&window), E::NoPanelProperties { window: WINDOW });
}

#[test]
fn a_missing_overall_size_is_not_taken_from_geometry() {
    let mut window = casement();
    window.width = None;
    assert_eq!(refused(&window), E::MissingOverallWidth { window: WINDOW });
    let mut window = casement();
    window.height = None;
    assert_eq!(refused(&window), E::MissingOverallHeight { window: WINDOW });
}

#[test]
fn conflicting_occurrence_and_type_partitionings_are_refused() {
    let mut window = casement();
    window.occurrence_partitioning = Some("DOUBLE_PANEL_VERTICAL");
    assert_eq!(
        refused(&window),
        E::ConflictingPartitioningType {
            occurrence: "DOUBLE_PANEL_VERTICAL".into(),
            type_object: WINDOW_TYPE,
            type_value: "SINGLE_PANEL".into(),
        }
    );
}

#[test]
fn an_untyped_window_without_a_partitioning_is_refused() {
    let mut window = casement();
    window.type_partitioning = None;
    window.occurrence_sets = std::mem::take(&mut window.type_sets);
    assert_eq!(
        refused(&window),
        E::MissingPartitioningType { window: WINDOW }
    );
}

#[test]
fn an_ifc4_window_typed_by_a_window_style_is_refused() {
    // IFC4 `CorrectStyleAssigned`: the type object has to be an
    // IfcWindowType, although the deprecated style is still declared.
    let mut window = casement();
    window.type_entity = Some("IfcWindowStyle");
    assert!(matches!(
        refused(&window),
        E::UnsupportedTypeObject {
            type_object: WINDOW_TYPE,
            ..
        }
    ));
}

#[test]
fn a_non_window_is_refused() {
    let mut window = casement();
    window.type_partitioning = None;
    window.type_sets.clear();
    // A door has an `OverallWidth` and `OverallHeight` too, and no window
    // operation.
    window.window_entity = "IfcDoor";
    assert!(matches!(
        refused(&window),
        E::NotAWindow { entity: WINDOW, .. }
    ));
}

#[test]
fn a_window_type_is_refused_as_no_window() {
    // The exact property reader accepts a type object since
    // ifc-properties#193; the type still has no placement or partitioning
    // of its own, so it is refused here.
    assert!(matches!(
        window_operation(&casement().model(), WINDOW_TYPE),
        Err(E::NotAWindow {
            entity: WINDOW_TYPE,
            ..
        })
    ));
}

#[test]
fn panels_must_match_the_partitioning() {
    // Too few for the partitioning.
    let window = Window::new(
        IFC4,
        "DOUBLE_PANEL_VERTICAL",
        vec![
            (20, panel(IFC4, 20, "SIDEHUNGLEFTHAND", "LEFT")),
            (25, lining(IFC4, 25, &[("FirstMullionOffset", "0.5")])),
        ],
    );
    assert_eq!(
        refused(&window),
        E::PanelCount {
            expected: 2,
            found: 1
        }
    );
    // Too many for a single panel.
    let window = Window::new(
        IFC4,
        "SINGLE_PANEL",
        vec![
            (20, panel(IFC4, 20, "SIDEHUNGLEFTHAND", "LEFT")),
            (21, panel(IFC4, 21, "FIXEDCASEMENT", "RIGHT")),
        ],
    );
    assert_eq!(
        refused(&window),
        E::PanelCount {
            expected: 1,
            found: 2
        }
    );
    // A position the partitioning does not list, or one listed twice.
    for (second, found) in [("TOP", "TOP"), ("LEFT", "LEFT")] {
        let window = Window::new(
            IFC4,
            "DOUBLE_PANEL_VERTICAL",
            vec![
                (20, panel(IFC4, 20, "SIDEHUNGLEFTHAND", "LEFT")),
                (21, panel(IFC4, 21, "SIDEHUNGRIGHTHAND", second)),
                (25, lining(IFC4, 25, &[("FirstMullionOffset", "0.5")])),
            ],
        );
        assert_eq!(
            refused(&window),
            E::PanelMismatch {
                set: EntityId(21),
                attribute: "PanelPosition",
                found: found.into(),
            },
            "{second}"
        );
    }
    // Three panels whose positions belong to another triple layout.
    let window = Window::new(
        IFC4,
        "TRIPLE_PANEL_BOTTOM",
        vec![
            (20, panel(IFC4, 20, "FIXEDCASEMENT", "LEFT")),
            (21, panel(IFC4, 21, "FIXEDCASEMENT", "RIGHT")),
            (22, panel(IFC4, 22, "FIXEDCASEMENT", "TOP")),
            (
                25,
                lining(
                    IFC4,
                    25,
                    &[("FirstMullionOffset", "0.5"), ("FirstTransomOffset", "0.3")],
                ),
            ),
        ],
    );
    assert_eq!(
        refused(&window),
        E::PanelMismatch {
            set: EntityId(22),
            attribute: "PanelPosition",
            found: "TOP".into(),
        }
    );
}

#[test]
fn a_split_needs_one_lining_with_its_offsets_inside_the_window() {
    assert_eq!(
        refused(&double(None)),
        E::NoLiningProperties { window: WINDOW }
    );
    assert_eq!(
        refused(&double(Some(&[
            ("LiningDepth", "0.1"),
            ("LiningThickness", "0.05")
        ]))),
        E::MissingSplit {
            set: EntityId(25),
            attribute: "FirstMullionOffset",
        }
    );
    for value in [0.0, 1.0] {
        let written = format!("{value:.1}");
        let window = double(Some(&[("FirstMullionOffset", written.as_str())]));
        assert_eq!(
            refused(&window),
            E::InvalidSplit {
                set: EntityId(25),
                attribute: "FirstMullionOffset",
                value,
            }
        );
    }
    // A transom offset does not split a vertical partitioning.
    assert!(matches!(
        refused(&double(Some(&[("FirstTransomOffset", "0.5")]))),
        E::MissingSplit {
            attribute: "FirstMullionOffset",
            ..
        }
    ));
    // Two linings on the governing source.
    let mut window = double(Some(&[("FirstMullionOffset", "0.5")]));
    window
        .type_sets
        .push((26, lining(IFC4, 26, &[("FirstMullionOffset", "0.3")])));
    assert_eq!(refused(&window), E::LiningCount { found: 2 });
}

#[test]
fn a_second_offset_must_lie_beyond_the_first() {
    let window = Window::new(
        IFC4,
        "TRIPLE_PANEL_VERTICAL",
        vec![
            (20, panel(IFC4, 20, "FIXEDCASEMENT", "LEFT")),
            (21, panel(IFC4, 21, "FIXEDCASEMENT", "MIDDLE")),
            (22, panel(IFC4, 22, "FIXEDCASEMENT", "RIGHT")),
            (
                25,
                lining(
                    IFC4,
                    25,
                    &[
                        ("FirstMullionOffset", "0.6"),
                        ("SecondMullionOffset", "0.4"),
                    ],
                ),
            ),
        ],
    );
    assert_eq!(
        refused(&window),
        E::InvalidSplit {
            set: EntityId(25),
            attribute: "SecondMullionOffset",
            value: 0.4,
        }
    );
}
