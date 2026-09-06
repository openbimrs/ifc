//! Executable inventory of the geometry-resource `WHERE` rules.
//!
//! `RULE-REG` asked for an inventory of every relevant EXPRESS `WHERE` rule
//! and its support state. Prose cannot answer "does the schema constrain
//! this?" -- only the schema can. `openbim-step` now parses rule labels and
//! expressions, so this file checks the committed inventory against what the
//! schema actually declares, and fails when the two drift apart.

use std::collections::{BTreeMap, BTreeSet};

use ifc_geometry::rules::validate_model;
use ifc_model::{Entity, EntityId, Model, Value};

const INVENTORY: &str = include_str!("../data/ifc4-where-rules.tsv");
const DECLARATIONS: &str = include_str!("../data/ifc4-add2-tc1-geometry-declarations.tsv");

/// Rows as `(entity, rule, state, expression)`.
fn rows() -> impl Iterator<Item = (&'static str, &'static str, &'static str, &'static str)> {
    INVENTORY
        .lines()
        .skip(1)
        .filter(|line| !line.is_empty())
        .map(|line| {
            let mut f = line.splitn(4, '\t');
            (
                f.next().expect("entity"),
                f.next().expect("rule"),
                f.next().expect("state"),
                f.next().expect("expression"),
            )
        })
}

/// The geometry-resource entities the inventory is scoped to.
fn geometry_entities() -> BTreeSet<String> {
    DECLARATIONS
        .lines()
        .skip(1)
        .filter_map(|line| {
            let mut f = line.split('\t');
            let _resource = f.next()?;
            let kind = f.next()?;
            let name = f.next()?;
            (kind == "entity").then(|| name.to_ascii_lowercase())
        })
        .collect()
}

/// The inventory must match the schema exactly: no invented rules, none missed.
///
/// This is the check that makes the inventory evidence rather than a claim.
/// A rule added, renamed or removed by a schema repin fails here instead of
/// silently leaving the catalogue stale -- the failure mode that made five
/// separate refusal rationales outlive their cause.
#[test]
fn the_inventory_matches_the_schema_exactly() {
    let scope = geometry_entities();
    let schema = ifc_schema::ifc4();
    let mut declared: BTreeSet<(String, String)> = BTreeSet::new();
    for name in schema.entity_names() {
        let lower = name.to_ascii_lowercase();
        if !scope.contains(&lower) {
            continue;
        }
        let entity = schema.entity(name).expect("named entity resolves");
        for rule in &entity.where_rules {
            declared.insert((lower.clone(), rule.label.clone()));
        }
    }

    let listed: BTreeSet<(String, String)> = rows()
        .map(|(entity, rule, _, _)| (entity.to_owned(), rule.to_owned()))
        .collect();

    let missing: Vec<_> = declared.difference(&listed).collect();
    let invented: Vec<_> = listed.difference(&declared).collect();
    assert!(
        missing.is_empty(),
        "schema declares rules the inventory omits: {missing:?}"
    );
    assert!(
        invented.is_empty(),
        "inventory lists rules the schema does not declare: {invented:?}"
    );
}

/// Every stored expression is the schema's own text, not a paraphrase.
#[test]
fn stored_expressions_match_the_schema_text() {
    let schema = ifc_schema::ifc4();
    let mut by_key: BTreeMap<(String, String), String> = BTreeMap::new();
    for name in schema.entity_names() {
        let entity = schema.entity(name).expect("named entity resolves");
        for rule in &entity.where_rules {
            by_key.insert(
                (name.to_ascii_lowercase(), rule.label.clone()),
                rule.expression.clone(),
            );
        }
    }
    for (entity, rule, _, expression) in rows() {
        let key = (entity.to_owned(), rule.to_owned());
        let actual = by_key.get(&key).expect("inventoried rule exists");
        assert_eq!(actual, expression, "{entity}.{rule} text drifted");
    }
}

/// `implemented` means the rules module names that rule; nothing else counts.
///
/// Without this, a row could claim support that no code provides -- the exact
/// gap between a documented capability and a real one.
#[test]
fn implemented_rows_are_named_by_the_rules_module() {
    const PLACEMENT: &str = include_str!("../src/rules/placement.rs");
    const SOLID: &str = include_str!("../src/rules/solid.rs");
    const GRID: &str = include_str!("../src/rules/grid.rs");
    for (entity, rule, state, _) in rows() {
        if state != "implemented" {
            continue;
        }
        // Both the rule label AND the entity type must appear in the same
        // module. Matching the bare label let one IfcBooleanResult.SameDim
        // implementation mark SameDim implemented for four other entities.
        let quoted = format!("\"{rule}\"");
        let typed = format!("\"{}\"", entity.to_ascii_uppercase());
        assert!(
            [PLACEMENT, SOLID, GRID]
                .iter()
                .any(|src| src.contains(&quoted) && src.contains(&typed)),
            "{entity}.{rule} claims implementation but no module names both it and its entity"
        );
    }
}

/// Support state is a closed vocabulary.
#[test]
fn every_row_declares_a_known_state() {
    for (entity, rule, state, expression) in rows() {
        assert!(
            matches!(state, "implemented" | "inventoried"),
            "{entity}.{rule} has unknown state {state}"
        );
        assert!(!expression.is_empty(), "{entity}.{rule} has no expression");
    }
}

/// `(rule label, conforming model, violating model)` for each implemented rule.
fn cases() -> Vec<(&'static str, Model, Model)> {
    let mut out = Vec::new();
    let pt = |m: &mut Model, id: u64, c: &[f64]| {
        m.insert(
            EntityId(id),
            Entity::new(
                "IFCCARTESIANPOINT",
                vec![Value::List(c.iter().map(|v| Value::Real(*v)).collect())],
            ),
        );
    };
    let dir = |m: &mut Model, id: u64, c: &[f64]| {
        m.insert(
            EntityId(id),
            Entity::new(
                "IFCDIRECTION",
                vec![Value::List(c.iter().map(|v| Value::Real(*v)).collect())],
            ),
        );
    };

    // IfcAxis2Placement3D: Location/Axis/RefDirection must all be 3D.
    for (label, bad_loc, bad_axis, bad_ref) in [
        ("LocationIs3D", true, false, false),
        ("AxisIs3D", false, true, false),
        ("RefDirIs3D", false, false, true),
    ] {
        let build = |broken: bool, which: u8| {
            let mut m = Model::new();
            pt(
                &mut m,
                1,
                if broken && which == 0 {
                    &[0.0, 0.0]
                } else {
                    &[0.0, 0.0, 0.0]
                },
            );
            dir(
                &mut m,
                2,
                if broken && which == 1 {
                    &[0.0, 1.0]
                } else {
                    &[0.0, 0.0, 1.0]
                },
            );
            dir(
                &mut m,
                3,
                if broken && which == 2 {
                    &[1.0, 0.0]
                } else {
                    &[1.0, 0.0, 0.0]
                },
            );
            m.insert(
                EntityId(10),
                Entity::new(
                    "IFCAXIS2PLACEMENT3D",
                    vec![
                        Value::Ref(EntityId(1)),
                        Value::Ref(EntityId(2)),
                        Value::Ref(EntityId(3)),
                    ],
                ),
            );
            m
        };
        let which = if bad_loc {
            0
        } else if bad_axis {
            1
        } else {
            2
        };
        let _ = bad_ref;
        out.push((label, build(false, which), build(true, which)));
    }

    // IfcAxis2Placement2D: Location and RefDirection must be 2D.
    for (label, which) in [("LocationIs2D", 0u8), ("RefDirIs2D", 1u8)] {
        let build = |broken: bool| {
            let mut m = Model::new();
            pt(
                &mut m,
                1,
                if broken && which == 0 {
                    &[0.0, 0.0, 0.0]
                } else {
                    &[0.0, 0.0]
                },
            );
            dir(
                &mut m,
                2,
                if broken && which == 1 {
                    &[1.0, 0.0, 0.0]
                } else {
                    &[1.0, 0.0]
                },
            );
            m.insert(
                EntityId(10),
                Entity::new(
                    "IFCAXIS2PLACEMENT2D",
                    vec![Value::Ref(EntityId(1)), Value::Ref(EntityId(2))],
                ),
            );
            m
        };
        out.push((label, build(false), build(true)));
    }

    // IfcAxis1Placement: Location and Axis must be 3D.
    for (label, which) in [("LocationIs3D", 0u8), ("AxisIs3D", 1u8)] {
        let build = |broken: bool| {
            let mut m = Model::new();
            pt(
                &mut m,
                1,
                if broken && which == 0 {
                    &[0.0, 0.0]
                } else {
                    &[0.0, 0.0, 0.0]
                },
            );
            dir(
                &mut m,
                2,
                if broken && which == 1 {
                    &[0.0, 1.0]
                } else {
                    &[0.0, 0.0, 1.0]
                },
            );
            m.insert(
                EntityId(11),
                Entity::new(
                    "IFCAXIS1PLACEMENT",
                    vec![Value::Ref(EntityId(1)), Value::Ref(EntityId(2))],
                ),
            );
            m
        };
        out.push((label, build(false), build(true)));
    }

    // IfcDirection: a zero-length direction has no orientation.
    {
        let mut good = Model::new();
        dir(&mut good, 1, &[0.0, 0.0, 1.0]);
        let mut bad = Model::new();
        dir(&mut bad, 1, &[0.0, 0.0, 0.0]);
        out.push(("MagnitudeGreaterZero", good, bad));
    }

    // IfcAxis2Placement3D: Axis and RefDirection must not be parallel.
    {
        let build = |parallel: bool| {
            let mut m = Model::new();
            pt(&mut m, 1, &[0.0, 0.0, 0.0]);
            dir(&mut m, 2, &[0.0, 0.0, 1.0]);
            dir(
                &mut m,
                3,
                if parallel {
                    &[0.0, 0.0, 2.0]
                } else {
                    &[1.0, 0.0, 0.0]
                },
            );
            m.insert(
                EntityId(10),
                Entity::new(
                    "IFCAXIS2PLACEMENT3D",
                    vec![
                        Value::Ref(EntityId(1)),
                        Value::Ref(EntityId(2)),
                        Value::Ref(EntityId(3)),
                    ],
                ),
            );
            m
        };
        out.push(("AxisToRefDirPosition", build(false), build(true)));
    }

    // IfcAxis2Placement3D: Axis and RefDirection are both-or-neither.
    {
        let build = |half: bool| {
            let mut m = Model::new();
            pt(&mut m, 1, &[0.0, 0.0, 0.0]);
            dir(&mut m, 2, &[0.0, 0.0, 1.0]);
            let refdir = if half {
                Value::Null
            } else {
                Value::Ref(EntityId(3))
            };
            dir(&mut m, 3, &[1.0, 0.0, 0.0]);
            m.insert(
                EntityId(10),
                Entity::new(
                    "IFCAXIS2PLACEMENT3D",
                    vec![Value::Ref(EntityId(1)), Value::Ref(EntityId(2)), refdir],
                ),
            );
            m
        };
        out.push(("AxisAndRefDirProvision", build(false), build(true)));
    }

    // IfcBooleanClippingResult: DIFFERENCE, and second operand a half space.
    {
        let build = |op: &str, second: &str| {
            let mut m = Model::new();
            m.insert(EntityId(1), Entity::new("IFCEXTRUDEDAREASOLID", vec![]));
            m.insert(EntityId(2), Entity::new(second, vec![]));
            m.insert(
                EntityId(10),
                Entity::new(
                    "IFCBOOLEANCLIPPINGRESULT",
                    vec![
                        Value::Enum(op.into()),
                        Value::Ref(EntityId(1)),
                        Value::Ref(EntityId(2)),
                    ],
                ),
            );
            m
        };
        out.push((
            "FirstOperandType",
            build("DIFFERENCE", "IFCHALFSPACESOLID"),
            build("UNION", "IFCHALFSPACESOLID"),
        ));
        out.push((
            "SecondOperandType",
            build("DIFFERENCE", "IFCHALFSPACESOLID"),
            build("DIFFERENCE", "IFCEXTRUDEDAREASOLID"),
        ));
    }

    // IfcExtrudedAreaSolid: direction must leave the profile plane.
    {
        let build = |in_plane: bool| {
            let mut m = Model::new();
            dir(
                &mut m,
                1,
                if in_plane {
                    &[1.0, 0.0, 0.0]
                } else {
                    &[0.0, 0.0, 1.0]
                },
            );
            m.insert(
                EntityId(10),
                Entity::new(
                    "IFCEXTRUDEDAREASOLID",
                    vec![
                        Value::Null,
                        Value::Null,
                        Value::Ref(EntityId(1)),
                        Value::Real(2.0),
                    ],
                ),
            );
            m
        };
        out.push(("ValidExtrusionDirection", build(false), build(true)));
    }

    // IfcPolygonalBoundedHalfSpace: boundary must be a polyline or composite.
    {
        let build = |boundary: &str| {
            let mut m = Model::new();
            m.insert(EntityId(1), Entity::new(boundary, vec![]));
            m.insert(
                EntityId(10),
                Entity::new(
                    "IFCPOLYGONALBOUNDEDHALFSPACE",
                    vec![
                        Value::Null,
                        Value::Bool(false),
                        Value::Null,
                        Value::Ref(EntityId(1)),
                    ],
                ),
            );
            m
        };
        out.push(("BoundaryType", build("IFCPOLYLINE"), build("IFCCIRCLE")));
    }

    // IfcGridAxis WR1: the axis curve must be 2D.
    // WR2: the axis must belong to exactly one of the U/V/W lists.
    {
        let build = |dim3: bool, lists: usize| {
            let mut m = Model::new();
            pt(&mut m, 1, if dim3 { &[0.0, 0.0, 0.0] } else { &[0.0, 0.0] });
            pt(&mut m, 2, if dim3 { &[1.0, 0.0, 0.0] } else { &[1.0, 0.0] });
            m.insert(
                EntityId(3),
                Entity::new(
                    "IFCPOLYLINE",
                    vec![Value::List(vec![
                        Value::Ref(EntityId(1)),
                        Value::Ref(EntityId(2)),
                    ])],
                ),
            );
            m.insert(
                EntityId(4),
                Entity::new(
                    "IFCGRIDAXIS",
                    vec![
                        Value::Text("A".into()),
                        Value::Ref(EntityId(3)),
                        Value::Bool(true),
                    ],
                ),
            );
            let axis = Value::List(vec![Value::Ref(EntityId(4))]);
            let mut grid = vec![Value::Null; 7];
            grid.push(if lists >= 1 {
                axis.clone()
            } else {
                Value::Null
            });
            grid.push(if lists >= 2 {
                axis.clone()
            } else {
                Value::Null
            });
            grid.push(Value::Null);
            m.insert(EntityId(5), Entity::new("IFCGRID", grid));
            m
        };
        out.push(("WR1", build(false, 1), build(true, 1)));
        out.push(("WR2", build(false, 1), build(false, 2)));
    }

    out
}

/// Every `implemented` row must actually fire on violating data and stay
/// silent on conforming data.
///
/// The other inventory tests prove the catalog matches the schema and that
/// a rules module names each implemented rule. Neither proves the check
/// works: a rule that never fires would satisfy both. This runs each one
/// against a conforming and a violating model and asserts both directions.
#[test]
fn implemented_rules_fire_on_violations_and_stay_silent_otherwise() {
    // Every implemented rule must have a case: otherwise this test only
    // proves whatever the author remembered to write.
    let covered: BTreeSet<&str> = cases().into_iter().map(|(l, _, _)| l).collect();
    for (entity, rule, state, _) in rows() {
        if state == "implemented" {
            assert!(
                covered.contains(rule),
                "{entity}.{rule} is implemented but has no pass/fail case"
            );
        }
    }
    for (label, good, bad) in cases() {
        let clean = validate_model(&good);
        assert!(
            !clean.iter().any(|v| v.rule == label),
            "{label} fired on conforming data: {clean:?}"
        );
        let violations = validate_model(&bad);
        assert!(
            violations.iter().any(|v| v.rule == label),
            "{label} did not fire on violating data: {violations:?}"
        );
    }
}
