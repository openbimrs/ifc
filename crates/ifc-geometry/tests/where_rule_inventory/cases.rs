//! Pass/fail model pairs for every implemented WHERE rule.
//!
//! Split out of the parent suite to keep each file under the repo line
//! cap; the coverage assertion lives with the test that consumes these.

use ifc_model::{Entity, EntityId, Model, Value};

pub fn cases() -> Vec<(&'static str, &'static str, Model, Model)> {
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
        out.push((
            "ifcaxis2placement3d",
            label,
            build(false, which),
            build(true, which),
        ));
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
        out.push(("ifcaxis2placement2d", label, build(false), build(true)));
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
        out.push(("ifcaxis1placement", label, build(false), build(true)));
    }

    // IfcDirection: a zero-length direction has no orientation.
    {
        let mut good = Model::new();
        dir(&mut good, 1, &[0.0, 0.0, 1.0]);
        let mut bad = Model::new();
        dir(&mut bad, 1, &[0.0, 0.0, 0.0]);
        out.push(("ifcdirection", "MagnitudeGreaterZero", good, bad));
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
        out.push((
            "ifcaxis2placement3d",
            "AxisToRefDirPosition",
            build(false),
            build(true),
        ));
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
        out.push((
            "ifcaxis2placement3d",
            "AxisAndRefDirProvision",
            build(false),
            build(true),
        ));
    }

    // IfcBooleanClippingResult: DIFFERENCE, and second operand a half space.
    {
        let build = |op: &str, first: &str, second: &str| {
            let mut m = Model::new();
            m.insert(EntityId(1), Entity::new(first, vec![]));
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
            "ifcbooleanclippingresult",
            "OperatorType",
            build("DIFFERENCE", "IFCEXTRUDEDAREASOLID", "IFCHALFSPACESOLID"),
            build("UNION", "IFCEXTRUDEDAREASOLID", "IFCHALFSPACESOLID"),
        ));
        // A faceted brep is a solid, but not one the schema admits here.
        out.push((
            "ifcbooleanclippingresult",
            "FirstOperandType",
            build("DIFFERENCE", "IFCEXTRUDEDAREASOLID", "IFCHALFSPACESOLID"),
            build("DIFFERENCE", "IFCFACETEDBREP", "IFCHALFSPACESOLID"),
        ));
        out.push((
            "ifcbooleanclippingresult",
            "SecondOperandType",
            build("DIFFERENCE", "IFCEXTRUDEDAREASOLID", "IFCHALFSPACESOLID"),
            build("DIFFERENCE", "IFCEXTRUDEDAREASOLID", "IFCEXTRUDEDAREASOLID"),
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
        out.push((
            "ifcextrudedareasolid",
            "ValidExtrusionDirection",
            build(false),
            build(true),
        ));
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
        out.push((
            "ifcpolygonalboundedhalfspace",
            "BoundaryType",
            build("IFCPOLYLINE"),
            build("IFCCIRCLE"),
        ));
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
        out.push(("ifcgridaxis", "WR1", build(false, 1), build(true, 1)));
        out.push(("ifcgridaxis", "WR2", build(false, 1), build(false, 2)));
    }

    out
}
