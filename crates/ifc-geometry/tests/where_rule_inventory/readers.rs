//! Pass/fail pairs for the rules that needed a new reader.
//!
//! Each pair differs in exactly the one fact its rule reads, so a mutation
//! to that rule is the only thing that can flip it.

use ifc_model::{Entity, EntityId, Model, Value};

/// `(entity, rule, conforming, violating)` for the reader-backed rules.
pub fn cases() -> Vec<(&'static str, &'static str, Model, Model)> {
    let mut out = Vec::new();
    let r = |id: u64| Value::Ref(EntityId(id));
    let list = |ids: &[u64]| Value::List(ids.iter().map(|i| Value::Ref(EntityId(*i))).collect());

    // --- advanced brep faces -------------------------------------------
    {
        // AdvancedBrep(Outer) -> ClosedShell(CfsFaces) -> face
        let brep = |face: &str| {
            let mut m = Model::new();
            m.insert(EntityId(1), Entity::new(face, vec![]));
            m.insert(EntityId(2), Entity::new("IFCCLOSEDSHELL", vec![list(&[1])]));
            m.insert(EntityId(10), Entity::new("IFCADVANCEDBREP", vec![r(2)]));
            m
        };
        out.push((
            "ifcadvancedbrep",
            "HasAdvancedFaces",
            brep("IFCADVANCEDFACE"),
            brep("IFCFACESURFACE"),
        ));

        // The voided form: the outer shell stays advanced either way, so
        // only the void shell distinguishes the two models.
        let voided = |void_face: &str| {
            let mut m = Model::new();
            m.insert(EntityId(1), Entity::new("IFCADVANCEDFACE", vec![]));
            m.insert(EntityId(2), Entity::new("IFCCLOSEDSHELL", vec![list(&[1])]));
            m.insert(EntityId(3), Entity::new(void_face, vec![]));
            m.insert(EntityId(4), Entity::new("IFCCLOSEDSHELL", vec![list(&[3])]));
            m.insert(
                EntityId(10),
                Entity::new("IFCADVANCEDBREPWITHVOIDS", vec![r(2), list(&[4])]),
            );
            m
        };
        out.push((
            "ifcadvancedbrepwithvoids",
            "VoidsHaveAdvancedFaces",
            voided("IFCADVANCEDFACE"),
            voided("IFCFACESURFACE"),
        ));
    }

    // --- boolean result operands ---------------------------------------
    {
        let pt = |m: &mut Model, id: u64, n: usize| {
            m.insert(
                EntityId(id),
                Entity::new(
                    "IFCCARTESIANPOINT",
                    vec![Value::List(vec![Value::Real(0.0); n])],
                ),
            );
        };
        // Two polylines whose dimensionality the rule compares.
        let dims = |second: usize| {
            let mut m = Model::new();
            pt(&mut m, 1, 3);
            pt(&mut m, 2, 3);
            pt(&mut m, 3, second);
            pt(&mut m, 4, second);
            m.insert(EntityId(5), Entity::new("IFCPOLYLINE", vec![list(&[1, 2])]));
            m.insert(EntityId(6), Entity::new("IFCPOLYLINE", vec![list(&[3, 4])]));
            m.insert(
                EntityId(10),
                Entity::new(
                    "IFCBOOLEANRESULT",
                    vec![Value::Enum("UNION".into()), r(5), r(6)],
                ),
            );
            m
        };
        out.push(("ifcbooleanresult", "SameDim", dims(3), dims(2)));

        // A tessellated operand must declare itself closed. Closed is slot
        // 2 on the triangulated form: Coordinates, Normals, Closed.
        let tess = |closed: Option<bool>, first: bool| {
            let mut m = Model::new();
            let flag = closed.map_or(Value::Null, Value::Bool);
            m.insert(
                EntityId(1),
                Entity::new(
                    "IFCTRIANGULATEDFACESET",
                    vec![Value::Null, Value::Null, flag],
                ),
            );
            m.insert(EntityId(2), Entity::new("IFCEXTRUDEDAREASOLID", vec![]));
            let (a, b) = if first { (1, 2) } else { (2, 1) };
            m.insert(
                EntityId(10),
                Entity::new(
                    "IFCBOOLEANRESULT",
                    vec![Value::Enum("UNION".into()), r(a), r(b)],
                ),
            );
            m
        };
        out.push((
            "ifcbooleanresult",
            "FirstOperandClosed",
            tess(Some(true), true),
            tess(Some(false), true),
        ));
        // The second-operand case also proves an OMITTED flag violates,
        // which is the EXISTS(Closed) half of the rule.
        out.push((
            "ifcbooleanresult",
            "SecondOperandClosed",
            tess(Some(true), false),
            tess(None, false),
        ));
    }

    out
}

/// Curve, surface and mapping cases for the reader-backed rules.
pub fn more_cases() -> Vec<(&'static str, &'static str, Model, Model)> {
    let mut out = Vec::new();
    let r = |id: u64| Value::Ref(EntityId(id));
    let list = |ids: &[u64]| Value::List(ids.iter().map(|i| Value::Ref(EntityId(*i))).collect());

    // --- composite curve continuity ------------------------------------
    {
        // Transition, SameSense, ParentCurve. The LAST segment decides
        // ClosedCurve, so a trailing CONTINUOUS means closed (0 allowed)
        // and a trailing DISCONTINUOUS means open (exactly 1 allowed).
        let seg = |m: &mut Model, id: u64, transition: &str| {
            m.insert(EntityId(100), Entity::new("IFCPOLYLINE", vec![]));
            m.insert(
                EntityId(id),
                Entity::new(
                    "IFCCOMPOSITECURVESEGMENT",
                    vec![Value::Enum(transition.into()), Value::Bool(true), r(100)],
                ),
            );
        };
        let curve = |kind: &str, first: &str, last: &str| {
            let mut m = Model::new();
            seg(&mut m, 1, first);
            seg(&mut m, 2, last);
            m.insert(EntityId(10), Entity::new(kind, vec![list(&[1, 2])]));
            m
        };
        // Closed (last CONTINUOUS): zero discontinuities conform; an
        // interior discontinuity makes the count 1 while still closed.
        out.push((
            "ifccompositecurve",
            "CurveContinuous",
            curve("IFCCOMPOSITECURVE", "CONTINUOUS", "CONTINUOUS"),
            curve("IFCCOMPOSITECURVE", "DISCONTINUOUS", "CONTINUOUS"),
        ));

        // A boundary curve must close, so a trailing DISCONTINUOUS fails.
        // The conforming model is closed and continuous throughout.
        out.push((
            "ifcboundarycurve",
            "IsClosed",
            curve("IFCBOUNDARYCURVE", "CONTINUOUS", "CONTINUOUS"),
            curve("IFCBOUNDARYCURVE", "CONTINUOUS", "DISCONTINUOUS"),
        ));
    }

    // --- basis surfaces -------------------------------------------------
    {
        // A p-curve names its surface in slot 0; a surface curve holds two
        // of them in slot 1.
        let surface_curve = |kind: &str, second_surface: u64| {
            let mut m = Model::new();
            m.insert(EntityId(1), Entity::new("IFCPLANE", vec![]));
            m.insert(EntityId(2), Entity::new("IFCPLANE", vec![]));
            m.insert(EntityId(3), Entity::new("IFCPCURVE", vec![r(1)]));
            m.insert(
                EntityId(4),
                Entity::new("IFCPCURVE", vec![r(second_surface)]),
            );
            m.insert(
                EntityId(10),
                Entity::new(kind, vec![Value::Null, list(&[3, 4]), Value::Null]),
            );
            m
        };
        // A seam runs twice over ONE surface; an intersection needs two.
        out.push((
            "ifcseamcurve",
            "SameSurface",
            surface_curve("IFCSEAMCURVE", 1),
            surface_curve("IFCSEAMCURVE", 2),
        ));
        out.push((
            "ifcintersectioncurve",
            "DistinctSurfaces",
            surface_curve("IFCINTERSECTIONCURVE", 2),
            surface_curve("IFCINTERSECTIONCURVE", 1),
        ));

        // A composite curve on surface intersects its segments' surfaces:
        // two segments on one plane share it, two on different planes
        // leave the intersection empty.
        let composite = |second_surface: u64| {
            let mut m = Model::new();
            m.insert(EntityId(1), Entity::new("IFCPLANE", vec![]));
            m.insert(EntityId(2), Entity::new("IFCPLANE", vec![]));
            m.insert(EntityId(3), Entity::new("IFCPCURVE", vec![r(1)]));
            m.insert(
                EntityId(4),
                Entity::new("IFCPCURVE", vec![r(second_surface)]),
            );
            for (seg, parent) in [(5u64, 3u64), (6, 4)] {
                m.insert(
                    EntityId(seg),
                    Entity::new(
                        "IFCCOMPOSITECURVESEGMENT",
                        vec![
                            Value::Enum("CONTINUOUS".into()),
                            Value::Bool(true),
                            r(parent),
                        ],
                    ),
                );
            }
            m.insert(
                EntityId(10),
                Entity::new("IFCCOMPOSITECURVEONSURFACE", vec![list(&[5, 6])]),
            );
            m
        };
        out.push((
            "ifccompositecurveonsurface",
            "SameSurface",
            composite(1),
            composite(2),
        ));
    }

    out
}

/// Swept-solid, trim and mapping cases.
pub fn final_cases() -> Vec<(&'static str, &'static str, Model, Model)> {
    let mut out = Vec::new();
    let r = |id: u64| Value::Ref(EntityId(id));
    let list = |ids: &[u64]| Value::List(ids.iter().map(|i| Value::Ref(EntityId(*i))).collect());

    // --- DirectrixBounded -----------------------------------------------
    {
        // A polyline is an IfcBoundedCurve (exactly one of the two names),
        // so it conforms with no params. An IfcLine is neither bounded nor
        // conic, so without StartParam/EndParam it violates.
        let disk = |directrix: &str| {
            let mut m = Model::new();
            m.insert(EntityId(1), Entity::new(directrix, vec![]));
            // Directrix, Radius, InnerRadius, StartParam, EndParam.
            m.insert(
                EntityId(10),
                Entity::new(
                    "IFCSWEPTDISKSOLID",
                    vec![
                        r(1),
                        Value::Real(1.0),
                        Value::Null,
                        Value::Null,
                        Value::Null,
                    ],
                ),
            );
            m
        };
        out.push((
            "ifcsweptdisksolid",
            "DirectrixBounded",
            disk("IFCPOLYLINE"),
            disk("IFCLINE"),
        ));

        // The other half of the rule: an unbounded directrix conforms when
        // the file supplies both trim parameters. Same IfcLine as the
        // violating model above, so only the params differ.
        let disk_with_params = |start: Value, end: Value| {
            let mut m = Model::new();
            m.insert(EntityId(1), Entity::new("IFCLINE", vec![]));
            m.insert(
                EntityId(10),
                Entity::new(
                    "IFCSWEPTDISKSOLID",
                    vec![r(1), Value::Real(1.0), Value::Null, start, end],
                ),
            );
            m
        };
        out.push((
            "ifcsweptdisksolid",
            "DirectrixBounded",
            disk_with_params(Value::Real(0.0), Value::Real(1.0)),
            // Only StartParam given: EXISTS(StartParam) AND EXISTS(EndParam)
            // fails, so the unbounded line is still unbounded.
            disk_with_params(Value::Real(0.0), Value::Null),
        ));

        // The two swept-area forms carry the directrix at slot 2.
        let swept = |kind: &str, directrix: &str| {
            let mut m = Model::new();
            m.insert(EntityId(1), Entity::new(directrix, vec![]));
            m.insert(
                EntityId(10),
                Entity::new(
                    kind,
                    vec![
                        Value::Null,
                        Value::Null,
                        r(1),
                        Value::Null,
                        Value::Null,
                        Value::Null,
                    ],
                ),
            );
            m
        };
        for kind in [
            "IFCSURFACECURVESWEPTAREASOLID",
            "IFCFIXEDREFERENCESWEPTAREASOLID",
        ] {
            out.push((
                if kind.starts_with("IFCSURFACE") {
                    "ifcsurfacecurvesweptareasolid"
                } else {
                    "ifcfixedreferencesweptareasolid"
                },
                "DirectrixBounded",
                swept(kind, "IFCPOLYLINE"),
                swept(kind, "IFCLINE"),
            ));
        }
    }

    // --- trim value kinds ------------------------------------------------
    {
        // BasisCurve, Trim1, Trim2. A point and a parameter differ in kind
        // and conform; two parameters do not.
        let trimmed = |slot: usize, mixed: bool| {
            let mut m = Model::new();
            m.insert(EntityId(1), Entity::new("IFCCIRCLE", vec![]));
            m.insert(EntityId(2), Entity::new("IFCCARTESIANPOINT", vec![]));
            let pair = if mixed {
                Value::List(vec![r(2), Value::Real(1.0)])
            } else {
                Value::List(vec![Value::Real(0.0), Value::Real(1.0)])
            };
            let ok = Value::List(vec![r(2), Value::Real(1.0)]);
            let (t1, t2) = if slot == 1 { (pair, ok) } else { (ok, pair) };
            m.insert(
                EntityId(10),
                Entity::new("IFCTRIMMEDCURVE", vec![r(1), t1, t2]),
            );
            m
        };
        out.push((
            "ifctrimmedcurve",
            "Trim1ValuesConsistent",
            trimmed(1, true),
            trimmed(1, false),
        ));
        out.push((
            "ifctrimmedcurve",
            "Trim2ValuesConsistent",
            trimmed(2, true),
            trimmed(2, false),
        ));
    }

    // --- Usense, mapped representation, revolution axis, spine ----------
    {
        // On a plane the u parameter is a length, so Usense must match the
        // direction from U1 to U2.
        let trimmed_surface = |usense: bool| {
            let mut m = Model::new();
            m.insert(EntityId(1), Entity::new("IFCPLANE", vec![]));
            m.insert(
                EntityId(10),
                Entity::new(
                    "IFCRECTANGULARTRIMMEDSURFACE",
                    vec![
                        r(1),
                        Value::Real(0.0),
                        Value::Real(0.0),
                        Value::Real(1.0),
                        Value::Real(1.0),
                        Value::Bool(usense),
                        Value::Bool(true),
                    ],
                ),
            );
            m
        };
        out.push((
            "ifcrectangulartrimmedsurface",
            "UsenseCompatible",
            trimmed_surface(true),
            trimmed_surface(false),
        ));

        let map = |mapped: &str| {
            let mut m = Model::new();
            m.insert(EntityId(1), Entity::new(mapped, vec![]));
            m.insert(
                EntityId(10),
                Entity::new("IFCREPRESENTATIONMAP", vec![Value::Null, r(1)]),
            );
            m
        };
        out.push((
            "ifcrepresentationmap",
            "ApplicableMappedRepr",
            map("IFCSHAPEREPRESENTATION"),
            map("IFCSTYLEDREPRESENTATION"),
        ));

        // The revolution axis must lie in the profile's xy plane: both its
        // location and its direction need a zero z.
        let revolved = |loc_z: f64, dir_z: f64| {
            let mut m = Model::new();
            m.insert(
                EntityId(1),
                Entity::new(
                    "IFCCARTESIANPOINT",
                    vec![Value::List(vec![
                        Value::Real(0.0),
                        Value::Real(0.0),
                        Value::Real(loc_z),
                    ])],
                ),
            );
            m.insert(
                EntityId(2),
                Entity::new(
                    "IFCDIRECTION",
                    vec![Value::List(vec![
                        Value::Real(0.0),
                        Value::Real(1.0),
                        Value::Real(dir_z),
                    ])],
                ),
            );
            m.insert(
                EntityId(3),
                Entity::new("IFCAXIS1PLACEMENT", vec![r(1), r(2)]),
            );
            m.insert(
                EntityId(10),
                Entity::new(
                    "IFCREVOLVEDAREASOLID",
                    vec![Value::Null, Value::Null, r(3), Value::Real(1.0)],
                ),
            );
            m
        };
        out.push((
            "ifcrevolvedareasolid",
            "AxisStartInXY",
            revolved(0.0, 0.0),
            revolved(5.0, 0.0),
        ));
        out.push((
            "ifcrevolvedareasolid",
            "AxisDirectionInXY",
            revolved(0.0, 0.0),
            revolved(0.0, 5.0),
        ));

        // Every cross-section of a spine must share one ProfileType.
        let spine = |second: &str| {
            let mut m = Model::new();
            m.insert(
                EntityId(1),
                Entity::new(
                    "IFCARBITRARYCLOSEDPROFILEDEF",
                    vec![Value::Enum("AREA".into())],
                ),
            );
            m.insert(
                EntityId(2),
                Entity::new(
                    "IFCARBITRARYCLOSEDPROFILEDEF",
                    vec![Value::Enum(second.into())],
                ),
            );
            m.insert(
                EntityId(10),
                Entity::new(
                    "IFCSECTIONEDSPINE",
                    vec![Value::Null, list(&[1, 2]), Value::Null],
                ),
            );
            m
        };
        out.push((
            "ifcsectionedspine",
            "ConsistentProfileTypes",
            spine("AREA"),
            spine("CURVE"),
        ));
    }

    out
}
