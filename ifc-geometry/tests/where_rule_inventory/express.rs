//! Pass/fail pairs for rules delegating to normative EXPRESS functions.
//!
//! The conforming B-splines are clamped cubics: degree 3, four control
//! points, knots [0,1] with multiplicities [4,4]. That satisfies
//! IfcConstraintsParamBSpline, so each violating twin can break exactly
//! one condition and nothing else.

use ifc_model::{Entity, EntityId, Model, Value};

pub fn cases() -> Vec<(&'static str, &'static str, Model, Model)> {
    let mut out = Vec::new();
    let r = |id: u64| Value::Ref(EntityId(id));
    let ints = |v: &[i64]| Value::List(v.iter().map(|n| Value::Integer(*n)).collect());
    let reals = |v: &[f64]| Value::List(v.iter().map(|n| Value::Real(*n)).collect());

    // Degree, ControlPointsList, CurveForm, ClosedCurve, SelfIntersect,
    // KnotMultiplicities, Knots, KnotSpec, [WeightsData].
    let curve = |degree: i64, cps: usize, mult: &[i64], knots: &[f64], weights: Option<&[f64]>| {
        let mut m = Model::new();
        let mut attrs = vec![
            Value::Integer(degree),
            Value::List((0..cps).map(|_| r(1)).collect()),
            Value::Enum("UNSPECIFIED".into()),
            Value::Bool(false),
            Value::Bool(false),
            ints(mult),
            reals(knots),
            Value::Enum("UNSPECIFIED".into()),
        ];
        let name = if let Some(w) = weights {
            attrs.push(reals(w));
            "IFCRATIONALBSPLINECURVEWITHKNOTS"
        } else {
            "IFCBSPLINECURVEWITHKNOTS"
        };
        m.insert(EntityId(10), Entity::new(name, attrs));
        m
    };

    out.push((
        "ifcbsplinecurvewithknots",
        "ConsistentBSpline",
        curve(3, 4, &[4, 4], &[0.0, 1.0], None),
        // Multiplicity sum 7 != Degree + UpCp + 2 = 8.
        curve(3, 4, &[4, 3], &[0.0, 1.0], None),
    ));
    out.push((
        "ifcrationalbsplinecurvewithknots",
        "WeightsGreaterZero",
        curve(3, 4, &[4, 4], &[0.0, 1.0], Some(&[1.0, 1.0, 1.0, 1.0])),
        curve(3, 4, &[4, 4], &[0.0, 1.0], Some(&[1.0, 0.0, 1.0, 1.0])),
    ));

    // UDegree, VDegree, ControlPointsList, SurfaceForm, UClosed, VClosed,
    // SelfIntersect, UMultiplicities, VMultiplicities, UKnots, VKnots,
    // KnotSpec, [WeightsData].
    let surface = |rows: usize,
                   cols: usize,
                   umult: &[i64],
                   vmult: &[i64],
                   uknots: &[f64],
                   vknots: &[f64],
                   weights: Option<(usize, usize, f64)>| {
        let mut m = Model::new();
        let grid = Value::List(
            (0..rows)
                .map(|_| Value::List((0..cols).map(|_| r(1)).collect()))
                .collect(),
        );
        let mut attrs = vec![
            Value::Integer(3),
            Value::Integer(3),
            grid,
            Value::Enum("UNSPECIFIED".into()),
            Value::Bool(false),
            Value::Bool(false),
            Value::Bool(false),
            ints(umult),
            ints(vmult),
            reals(uknots),
            reals(vknots),
            Value::Enum("UNSPECIFIED".into()),
        ];
        let name = if let Some((wr, wc, bad)) = weights {
            attrs.push(Value::List(
                (0..wr)
                    .map(|i| {
                        Value::List(
                            (0..wc)
                                .map(|j| {
                                    let v = if i == 0 && j == 0 { bad } else { 1.0 };
                                    Value::Real(v)
                                })
                                .collect(),
                        )
                    })
                    .collect(),
            ));
            "IFCRATIONALBSPLINESURFACEWITHKNOTS"
        } else {
            "IFCBSPLINESURFACEWITHKNOTS"
        };
        m.insert(EntityId(10), Entity::new(name, attrs));
        m
    };

    let good_u = &[4i64, 4][..];
    let good_v = &[4i64, 4][..];
    let k = &[0.0f64, 1.0][..];
    out.push((
        "ifcbsplinesurfacewithknots",
        "UDirectionConstraints",
        surface(4, 4, good_u, good_v, k, k, None),
        // U multiplicity sum 7 != 8; V left valid.
        surface(4, 4, &[4, 3], good_v, k, k, None),
    ));
    out.push((
        "ifcbsplinesurfacewithknots",
        "VDirectionConstraints",
        surface(4, 4, good_u, good_v, k, k, None),
        surface(4, 4, good_u, &[4, 3], k, k, None),
    ));
    out.push((
        "ifcbsplinesurfacewithknots",
        "CorrespondingULists",
        surface(4, 4, good_u, good_v, k, k, None),
        // Three multiplicities against two knots.
        surface(4, 4, &[4, 2, 2], good_v, k, k, None),
    ));
    out.push((
        "ifcbsplinesurfacewithknots",
        "CorrespondingVLists",
        surface(4, 4, good_u, good_v, k, k, None),
        surface(4, 4, good_u, &[4, 2, 2], k, k, None),
    ));
    out.push((
        "ifcrationalbsplinesurfacewithknots",
        "WeightValuesGreaterZero",
        surface(4, 4, good_u, good_v, k, k, Some((4, 4, 1.0))),
        // Zero is the boundary the schema excludes, not just negatives.
        surface(4, 4, good_u, good_v, k, k, Some((4, 4, 0.0))),
    ));
    out.push((
        "ifcrationalbsplinesurfacewithknots",
        "CorrespondingWeightsDataLists",
        surface(4, 4, good_u, good_v, k, k, Some((4, 4, 1.0))),
        // A 3x4 weight grid over a 4x4 control grid.
        surface(4, 4, good_u, good_v, k, k, Some((3, 4, 1.0))),
    ));

    // IfcIndexedPolyCurve.Consecutive: segments join end-to-start.
    let poly = |a: &[i64], b: &[i64]| {
        let mut m = Model::new();
        let seg = |v: &[i64]| Value::List(v.iter().map(|n| Value::Integer(*n)).collect());
        m.insert(
            EntityId(10),
            Entity::new(
                "IFCINDEXEDPOLYCURVE",
                vec![r(1), Value::List(vec![seg(a), seg(b)])],
            ),
        );
        m
    };
    out.push((
        "ifcindexedpolycurve",
        "Consecutive",
        poly(&[1, 2, 3], &[3, 4]),
        poly(&[1, 2, 3], &[9, 4]),
    ));

    // IfcLocalPlacement.WR21: a 3D relative placement needs a 3D parent.
    //
    // The schema function returns UNKNOWN for most shapes and EXPRESS treats
    // that as satisfied, so only this one FALSE branch may fire.
    let placement = |parent_dim: usize| {
        let mut m = Model::new();
        let coords = |n: usize| Value::List(vec![Value::Real(0.0); n]);
        m.insert(
            EntityId(1),
            Entity::new("IFCCARTESIANPOINT", vec![coords(parent_dim)]),
        );
        m.insert(
            EntityId(2),
            Entity::new(
                if parent_dim == 3 {
                    "IFCAXIS2PLACEMENT3D"
                } else {
                    "IFCAXIS2PLACEMENT2D"
                },
                vec![r(1), Value::Null, Value::Null],
            ),
        );
        m.insert(
            EntityId(3),
            Entity::new("IFCLOCALPLACEMENT", vec![Value::Null, r(2)]),
        );
        m.insert(
            EntityId(4),
            Entity::new("IFCCARTESIANPOINT", vec![coords(3)]),
        );
        m.insert(
            EntityId(5),
            Entity::new("IFCAXIS2PLACEMENT3D", vec![r(4), Value::Null, Value::Null]),
        );
        m.insert(
            EntityId(10),
            Entity::new("IFCLOCALPLACEMENT", vec![r(3), r(5)]),
        );
        m
    };
    out.push(("ifclocalplacement", "WR21", placement(3), placement(2)));

    // IfcTaperedSweptAreaProfiles: EndSweptArea is slot 4 on both forms.
    //
    // Conforming pair uses two profiles of the SAME parameterised type;
    // the violating twin swaps the end for an unrelated arbitrary profile,
    // which the schema admits only via IfcDerivedProfileDef.ParentProfile.
    let tapered = |name: &str, end: &str| {
        let mut m = Model::new();
        m.insert(
            EntityId(1),
            Entity::new("IFCCIRCLEPROFILEDEF", vec![Value::Enum("AREA".into())]),
        );
        m.insert(
            EntityId(2),
            Entity::new(end, vec![Value::Enum("AREA".into())]),
        );
        m.insert(
            EntityId(10),
            Entity::new(
                name,
                vec![r(1), Value::Null, Value::Null, Value::Null, r(2)],
            ),
        );
        m
    };
    // The OTHER admissible shape: the end profile derives from the start.
    // ParentProfile is slot 2 on IfcDerivedProfileDef; pointing it at an
    // unrelated profile breaks the correspondence the taper needs.
    let derived = |parent: u64| {
        let mut m = Model::new();
        m.insert(
            EntityId(1),
            Entity::new("IFCCIRCLEPROFILEDEF", vec![Value::Enum("AREA".into())]),
        );
        m.insert(
            EntityId(3),
            Entity::new("IFCCIRCLEPROFILEDEF", vec![Value::Enum("AREA".into())]),
        );
        m.insert(
            EntityId(2),
            Entity::new(
                "IFCDERIVEDPROFILEDEF",
                vec![Value::Enum("AREA".into()), Value::Null, r(parent)],
            ),
        );
        m.insert(
            EntityId(10),
            Entity::new(
                "IFCEXTRUDEDAREASOLIDTAPERED",
                vec![r(1), Value::Null, Value::Null, Value::Null, r(2)],
            ),
        );
        m
    };

    for (entity, name) in [
        ("ifcextrudedareasolidtapered", "IFCEXTRUDEDAREASOLIDTAPERED"),
        ("ifcrevolvedareasolidtapered", "IFCREVOLVEDAREASOLIDTAPERED"),
    ] {
        out.push((
            entity,
            "CorrectProfileAssignment",
            tapered(name, "IFCCIRCLEPROFILEDEF"),
            tapered(name, "IFCARBITRARYCLOSEDPROFILEDEF"),
        ));
    }
    // Same rule, exercised through the derived-profile branch: parent is
    // the start profile (conforming) versus an unrelated one (violating).
    out.push((
        "ifcextrudedareasolidtapered",
        "CorrectProfileAssignment",
        derived(1),
        derived(3),
    ));

    out
}
