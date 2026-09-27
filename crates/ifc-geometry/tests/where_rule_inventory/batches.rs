//! Pass/fail model pairs for the newly enforced rule batches.
//!
//! Batches A-D: dimensionality, positive scalars, cardinality and type
//! membership. Split from the original cases purely for the line cap.

use ifc_model::{Entity, EntityId, Model, Value};

pub fn cases() -> Vec<(&'static str, &'static str, Model, Model)> {
    let mut out = Vec::new();
    let r = |id: u64| Value::Ref(EntityId(id));
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

    // --- Batch A: dimensionality ---
    // Dim is DERIVED, so each case varies the entity the rule reads
    // through, never a literal Dim attribute.
    {
        // A polyline whose second point has one coordinate too many.
        let poly = |ragged: bool| {
            let mut m = Model::new();
            pt(&mut m, 1, &[0.0, 0.0]);
            pt(
                &mut m,
                2,
                if ragged {
                    &[1.0, 1.0, 1.0]
                } else {
                    &[1.0, 1.0]
                },
            );
            m.insert(
                EntityId(10),
                Entity::new("IFCPOLYLINE", vec![Value::List(vec![r(1), r(2)])]),
            );
            m
        };
        out.push(("ifcpolyline", "SameDim", poly(false), poly(true)));
    }

    // Transformation operators: LocalOrigin fixes Dim, each Axis must match.
    {
        // slots: Axis1, Axis2, LocalOrigin, Scale (+ Axis3 on the 3D form).
        let op = |three: bool, bad: usize| {
            let mut m = Model::new();
            let n = if three { 3 } else { 2 };
            let good: &[f64] = if three { &[1.0, 0.0, 0.0] } else { &[1.0, 0.0] };
            let wrong: &[f64] = if three { &[1.0, 0.0] } else { &[1.0, 0.0, 0.0] };
            dir(&mut m, 1, if bad == 1 { wrong } else { good });
            dir(&mut m, 2, if bad == 2 { wrong } else { good });
            dir(&mut m, 3, if bad == 3 { wrong } else { good });
            // bad == 0 keeps every axis right; bad == 4 breaks LocalOrigin,
            // which is what fixes the operator Dim itself.
            let origin: Vec<f64> = if bad == 4 {
                vec![0.0; if three { 2 } else { 3 }]
            } else {
                vec![0.0; n]
            };
            pt(&mut m, 4, &origin);
            let mut attrs = vec![r(1), r(2), r(4), Value::Real(1.0)];
            if three {
                attrs.push(r(3));
            }
            let name = if three {
                "IFCCARTESIANTRANSFORMATIONOPERATOR3D"
            } else {
                "IFCCARTESIANTRANSFORMATIONOPERATOR2D"
            };
            m.insert(EntityId(10), Entity::new(name, attrs));
            m
        };
        let o2 = "ifccartesiantransformationoperator2d";
        let o3 = "ifccartesiantransformationoperator3d";
        out.push((o2, "Axis1Is2D", op(false, 0), op(false, 1)));
        out.push((o2, "Axis2Is2D", op(false, 0), op(false, 2)));
        out.push((o2, "DimEqual2", op(false, 0), op(false, 4)));
        out.push((o3, "Axis1Is3D", op(true, 0), op(true, 1)));
        out.push((o3, "Axis2Is3D", op(true, 0), op(true, 2)));
        out.push((o3, "Axis3Is3D", op(true, 0), op(true, 3)));
        out.push((o3, "DimIs3D", op(true, 0), op(true, 4)));
    }

    // One 2D and one 3D curve, reused by every fixed-Dim rule below.
    {
        let with_curve = |name: &str, attrs: Vec<Value>, three: bool| {
            let mut m = Model::new();
            pt(
                &mut m,
                1,
                if three { &[0.0, 0.0, 0.0] } else { &[0.0, 0.0] },
            );
            pt(
                &mut m,
                2,
                if three { &[1.0, 1.0, 1.0] } else { &[1.0, 1.0] },
            );
            m.insert(
                EntityId(3),
                Entity::new("IFCPOLYLINE", vec![Value::List(vec![r(1), r(2)])]),
            );
            m.insert(EntityId(10), Entity::new(name, attrs));
            m
        };
        // IfcPcurve.DimIs2D: the reference curve lives in (u, v).
        out.push((
            "ifcpcurve",
            "DimIs2D",
            with_curve("IFCPCURVE", vec![Value::Null, r(3)], false),
            with_curve("IFCPCURVE", vec![Value::Null, r(3)], true),
        ));
        // Offset curves: the basis carries the dimensionality.
        out.push((
            "ifcoffsetcurve2d",
            "DimIs2D",
            with_curve("IFCOFFSETCURVE2D", vec![r(3)], false),
            with_curve("IFCOFFSETCURVE2D", vec![r(3)], true),
        ));
        // IfcSurfaceCurve.CurveIs3D and the two 3D-directrix rules.
        out.push((
            "ifcsurfacecurve",
            "CurveIs3D",
            with_curve("IFCSURFACECURVE", vec![r(3)], true),
            with_curve("IFCSURFACECURVE", vec![r(3)], false),
        ));
        out.push((
            "ifcsweptdisksolid",
            "DirectrixDim",
            with_curve("IFCSWEPTDISKSOLID", vec![r(3)], true),
            with_curve("IFCSWEPTDISKSOLID", vec![r(3)], false),
        ));
        out.push((
            "ifcsectionedspine",
            "SpineCurveDim",
            with_curve("IFCSECTIONEDSPINE", vec![r(3)], true),
            with_curve("IFCSECTIONEDSPINE", vec![r(3)], false),
        ));
        // IfcPolygonalBoundedHalfSpace: boundary is 2D; slots 0-1 inherited.
        out.push((
            "ifcpolygonalboundedhalfspace",
            "BoundaryDim",
            with_curve(
                "IFCPOLYGONALBOUNDEDHALFSPACE",
                vec![Value::Null, Value::Null, Value::Null, r(3)],
                false,
            ),
            with_curve(
                "IFCPOLYGONALBOUNDEDHALFSPACE",
                vec![Value::Null, Value::Null, Value::Null, r(3)],
                true,
            ),
        ));
    }

    // IfcLine.SameDim: Pnt and Dir must agree.
    {
        let line = |mixed: bool| {
            let mut m = Model::new();
            pt(&mut m, 1, &[0.0, 0.0, 0.0]);
            dir(
                &mut m,
                2,
                if mixed { &[1.0, 0.0] } else { &[1.0, 0.0, 0.0] },
            );
            m.insert(EntityId(10), Entity::new("IFCLINE", vec![r(1), r(2)]));
            m
        };
        out.push(("ifcline", "SameDim", line(false), line(true)));
    }

    // IfcGeometricSet.ConsistentDim over mixed elements.
    {
        let set = |mixed: bool| {
            let mut m = Model::new();
            pt(&mut m, 1, &[0.0, 0.0, 0.0]);
            pt(
                &mut m,
                2,
                if mixed { &[1.0, 1.0] } else { &[1.0, 1.0, 1.0] },
            );
            m.insert(
                EntityId(10),
                Entity::new("IFCGEOMETRICSET", vec![Value::List(vec![r(1), r(2)])]),
            );
            m
        };
        out.push(("ifcgeometricset", "ConsistentDim", set(false), set(true)));
    }

    // IfcCompositeCurve.SameDim: segments carry their parent curve Dim.
    {
        let comp = |mixed: bool| {
            let mut m = Model::new();
            pt(&mut m, 1, &[0.0, 0.0, 0.0]);
            pt(&mut m, 2, &[1.0, 1.0, 1.0]);
            pt(&mut m, 3, &[0.0, 0.0]);
            pt(&mut m, 4, &[1.0, 1.0]);
            m.insert(
                EntityId(5),
                Entity::new("IFCPOLYLINE", vec![Value::List(vec![r(1), r(2)])]),
            );
            m.insert(
                EntityId(6),
                Entity::new("IFCPOLYLINE", vec![Value::List(vec![r(3), r(4)])]),
            );
            let second = if mixed { r(6) } else { r(5) };
            for (id, parent) in [(7u64, r(5)), (8u64, second)] {
                m.insert(
                    EntityId(id),
                    Entity::new(
                        "IFCCOMPOSITECURVESEGMENT",
                        vec![Value::Enum("CONTINUOUS".into()), Value::Bool(true), parent],
                    ),
                );
            }
            m.insert(
                EntityId(10),
                Entity::new(
                    "IFCCOMPOSITECURVE",
                    vec![Value::List(vec![r(7), r(8)]), Value::Bool(false)],
                ),
            );
            m
        };
        out.push(("ifccompositecurve", "SameDim", comp(false), comp(true)));
    }

    // IfcBSplineCurve.SameDim over the control point list.
    {
        let spline = |mixed: bool| {
            let mut m = Model::new();
            pt(&mut m, 1, &[0.0, 0.0, 0.0]);
            pt(
                &mut m,
                2,
                if mixed { &[1.0, 1.0] } else { &[1.0, 1.0, 1.0] },
            );
            m.insert(
                EntityId(10),
                Entity::new(
                    "IFCBSPLINECURVEWITHKNOTS",
                    vec![Value::Integer(1), Value::List(vec![r(1), r(2)])],
                ),
            );
            m
        };
        out.push(("ifcbsplinecurve", "SameDim", spline(false), spline(true)));
    }

    // IfcOffsetCurve3D.DimIs2D: the basis curve must be 3D.
    {
        let basis = |three: bool| {
            let mut m = Model::new();
            let dim = if three { 3 } else { 2 };
            pt(&mut m, 1, &vec![0.0; dim]);
            pt(&mut m, 2, &vec![1.0; dim]);
            m.insert(
                EntityId(3),
                Entity::new("IFCPOLYLINE", vec![Value::List(vec![r(1), r(2)])]),
            );
            m.insert(EntityId(10), Entity::new("IFCOFFSETCURVE3D", vec![r(3)]));
            m
        };
        out.push(("ifcoffsetcurve3d", "DimIs2D", basis(true), basis(false)));
    }

    // --- Batch B: positive scalars ---
    {
        // Scale is optional and NVL-defaults to 1.0, so the conforming
        // model leaves it unset: a rule reading the raw slot would fire.
        let op = |name: &str, extra: Vec<Value>, scale: Option<f64>| {
            let mut m = Model::new();
            pt(&mut m, 1, &[0.0, 0.0, 0.0]);
            let mut attrs = vec![
                Value::Null,
                Value::Null,
                r(1),
                scale.map(Value::Real).unwrap_or(Value::Null),
            ];
            attrs.extend(extra);
            m.insert(EntityId(10), Entity::new(name, attrs));
            m
        };
        let base = "IFCCARTESIANTRANSFORMATIONOPERATOR3D";
        out.push((
            "ifccartesiantransformationoperator",
            "ScaleGreaterZero",
            op(base, vec![], None),
            op(base, vec![], Some(-1.0)),
        ));
        // 2D non-uniform: Scale2 sits after the 2D operator slots.
        let n2 = "IFCCARTESIANTRANSFORMATIONOPERATOR2DNONUNIFORM";
        out.push((
            "ifccartesiantransformationoperator2dnonuniform",
            "Scale2GreaterZero",
            op(n2, vec![Value::Real(2.0)], None),
            op(n2, vec![Value::Real(-2.0)], None),
        ));
        // 3D non-uniform: Axis3 occupies slot 4, so Scale2/Scale3 are 5/6.
        let n3 = "IFCCARTESIANTRANSFORMATIONOPERATOR3DNONUNIFORM";
        out.push((
            "ifccartesiantransformationoperator3dnonuniform",
            "Scale2GreaterZero",
            op(
                n3,
                vec![Value::Null, Value::Real(2.0), Value::Real(3.0)],
                None,
            ),
            op(
                n3,
                vec![Value::Null, Value::Real(-2.0), Value::Real(3.0)],
                None,
            ),
        ));
        out.push((
            "ifccartesiantransformationoperator3dnonuniform",
            "Scale3GreaterZero",
            op(
                n3,
                vec![Value::Null, Value::Real(2.0), Value::Real(3.0)],
                None,
            ),
            op(
                n3,
                vec![Value::Null, Value::Real(2.0), Value::Real(-3.0)],
                None,
            ),
        ));
    }

    // Three single-slot magnitudes; only IfcVector admits zero.
    {
        let one = |name: &str, slot: usize, v: f64| {
            let mut m = Model::new();
            let mut attrs = vec![Value::Null; slot];
            attrs.push(Value::Real(v));
            m.insert(EntityId(10), Entity::new(name, attrs));
            m
        };
        out.push((
            "ifcreparametrisedcompositecurvesegment",
            "PositiveLengthParameter",
            one("IFCREPARAMETRISEDCOMPOSITECURVESEGMENT", 3, 1.5),
            one("IFCREPARAMETRISEDCOMPOSITECURVESEGMENT", 3, 0.0),
        ));
        out.push((
            "ifcsurfaceoflinearextrusion",
            "DepthGreaterZero",
            one("IFCSURFACEOFLINEAREXTRUSION", 3, 2.0),
            one("IFCSURFACEOFLINEAREXTRUSION", 3, 0.0),
        ));
        // Zero magnitude conforms; only a negative one violates.
        out.push((
            "ifcvector",
            "MagGreaterOrEqualZero",
            one("IFCVECTOR", 1, 0.0),
            one("IFCVECTOR", 1, -1.0),
        ));
    }

    // --- Batch C: cardinality ---
    {
        // Degree, ControlPointsList, CurveForm, ClosedCurve, SelfIntersect,
        // KnotMultiplicities, Knots, KnotSpec, [WeightsData].
        let spline = |name: &str, mults: usize, knots: usize, pts: usize, weights: usize| {
            let mut m = Model::new();
            let list = |n: usize| Value::List((0..n).map(|_| Value::Real(0.0)).collect());
            let mut attrs = vec![
                Value::Integer(2),
                list(pts),
                Value::Enum("UNSPECIFIED".into()),
                Value::Bool(false),
                Value::Bool(false),
                list(mults),
                list(knots),
                Value::Enum("UNSPECIFIED".into()),
            ];
            if weights > 0 {
                attrs.push(list(weights));
            }
            m.insert(EntityId(10), Entity::new(name, attrs));
            m
        };
        let bs = "IFCBSPLINECURVEWITHKNOTS";
        out.push((
            "ifcbsplinecurvewithknots",
            "CorrespondingKnotLists",
            // Conforming model omits the lists entirely: an unwritten
            // optional list must be skipped, not read as length 0.
            {
                let mut m = Model::new();
                // Knots written, KnotMultiplicities absent: reading the
                // absent slot as length 0 would report a false mismatch.
                m.insert(
                    EntityId(10),
                    Entity::new(
                        bs,
                        vec![
                            Value::Integer(2),
                            Value::List(vec![Value::Real(0.0); 3]),
                            Value::Enum("UNSPECIFIED".into()),
                            Value::Bool(false),
                            Value::Bool(false),
                            Value::Null,
                            Value::List(vec![Value::Real(0.0); 3]),
                        ],
                    ),
                );
                m
            },
            spline(bs, 2, 3, 3, 0),
        ));
        let rb = "IFCRATIONALBSPLINECURVEWITHKNOTS";
        out.push((
            "ifcrationalbsplinecurvewithknots",
            "SameNumOfWeightsAndPoints",
            spline(rb, 3, 3, 3, 3),
            spline(rb, 3, 3, 3, 2),
        ));
    }

    // IfcSectionedSpine: one placement per cross-section.
    {
        let spine = |sections: usize, positions: usize| {
            let mut m = Model::new();
            let refs = |n: usize| Value::List((0..n).map(|_| r(1)).collect());
            m.insert(
                EntityId(10),
                Entity::new(
                    "IFCSECTIONEDSPINE",
                    vec![r(1), refs(sections), refs(positions)],
                ),
            );
            m
        };
        out.push((
            "ifcsectionedspine",
            "CorrespondingSectionPositions",
            spine(3, 3),
            spine(3, 2),
        ));
    }

    // A seam or intersection curve associates exactly two p-curves.
    {
        let sc = |name: &str, n: usize| {
            let mut m = Model::new();
            let pcs = Value::List((0..n).map(|_| r(1)).collect());
            m.insert(
                EntityId(10),
                Entity::new(name, vec![r(2), pcs, Value::Enum("CURVE3D".into())]),
            );
            m
        };
        out.push((
            "ifcseamcurve",
            "TwoPCurves",
            sc("IFCSEAMCURVE", 2),
            sc("IFCSEAMCURVE", 1),
        ));
        out.push((
            "ifcintersectioncurve",
            "TwoPCurves",
            sc("IFCINTERSECTIONCURVE", 2),
            sc("IFCINTERSECTIONCURVE", 1),
        ));
    }

    // --- Batch D: type membership ---
    {
        // A polyline is a bounded curve by subtype, so these cases also
        // prove the rules go through is_a rather than an exact name match.
        let one_ref = |name: &str, slot: usize, target: &str| {
            let mut m = Model::new();
            m.insert(EntityId(1), Entity::new(target, vec![]));
            let mut attrs = vec![Value::Null; slot];
            attrs.push(r(1));
            m.insert(EntityId(10), Entity::new(name, attrs));
            m
        };
        out.push((
            "ifccompositecurvesegment",
            "ParentIsBoundedCurve",
            one_ref("IFCCOMPOSITECURVESEGMENT", 2, "IFCPOLYLINE"),
            one_ref("IFCCOMPOSITECURVESEGMENT", 2, "IFCLINE"),
        ));
        out.push((
            "ifctrimmedcurve",
            "NoTrimOfBoundedCurves",
            one_ref("IFCTRIMMEDCURVE", 0, "IFCCIRCLE"),
            one_ref("IFCTRIMMEDCURVE", 0, "IFCPOLYLINE"),
        ));
        out.push((
            "ifcsurfacecurve",
            "CurveIsNotPcurve",
            one_ref("IFCSURFACECURVE", 0, "IFCPOLYLINE"),
            one_ref("IFCSURFACECURVE", 0, "IFCPCURVE"),
        ));
        out.push((
            "ifcboxedhalfspace",
            "UnboundedSurface",
            one_ref("IFCBOXEDHALFSPACE", 0, "IFCPLANE"),
            one_ref("IFCBOXEDHALFSPACE", 0, "IFCCURVEBOUNDEDPLANE"),
        ));
    }

    // A curve set holding a surface, and a polygonal disk whose
    // directrix is not polyline-shaped.
    {
        let set = |member: &str| {
            let mut m = Model::new();
            m.insert(EntityId(1), Entity::new(member, vec![]));
            m.insert(
                EntityId(10),
                Entity::new("IFCGEOMETRICCURVESET", vec![Value::List(vec![r(1)])]),
            );
            m
        };
        out.push((
            "ifcgeometriccurveset",
            "NoSurfaces",
            set("IFCPOLYLINE"),
            set("IFCPLANE"),
        ));

        // An indexed poly-curve WITH segments is not polyline-shaped.
        let disk = |directrix: &str, segments: bool| {
            let mut m = Model::new();
            let mut d = vec![Value::Null];
            if segments {
                d.push(Value::List(vec![Value::Integer(1)]));
            }
            m.insert(EntityId(1), Entity::new(directrix, d));
            m.insert(
                EntityId(10),
                Entity::new("IFCSWEPTDISKSOLIDPOLYGONAL", vec![r(1), Value::Real(1.0)]),
            );
            m
        };
        out.push((
            "ifcsweptdisksolidpolygonal",
            "DirectrixIsPolyline",
            disk("IFCINDEXEDPOLYCURVE", false),
            disk("IFCINDEXEDPOLYCURVE", true),
        ));
    }

    // --- Batch E: degeneracy and profile kind ---
    {
        // BasisSurface, U1, V1, U2, V2, Usense, Vsense.
        let trimmed = |u1: f64, v1: f64, u2: f64, v2: f64, vsense: bool| {
            let mut m = Model::new();
            m.insert(
                EntityId(10),
                Entity::new(
                    "IFCRECTANGULARTRIMMEDSURFACE",
                    vec![
                        r(1),
                        Value::Real(u1),
                        Value::Real(v1),
                        Value::Real(u2),
                        Value::Real(v2),
                        Value::Bool(true),
                        Value::Bool(vsense),
                    ],
                ),
            );
            m
        };
        out.push((
            "ifcrectangulartrimmedsurface",
            "U1AndU2Different",
            trimmed(0.0, 0.0, 1.0, 1.0, true),
            trimmed(0.5, 0.0, 0.5, 1.0, true),
        ));
        out.push((
            "ifcrectangulartrimmedsurface",
            "V1AndV2Different",
            trimmed(0.0, 0.0, 1.0, 1.0, true),
            trimmed(0.0, 0.5, 1.0, 0.5, true),
        ));
        out.push((
            "ifcrectangulartrimmedsurface",
            "VsenseCompatible",
            trimmed(0.0, 0.0, 1.0, 1.0, true),
            trimmed(0.0, 0.0, 1.0, 1.0, false),
        ));
    }

    // Radius degeneracies and the minimum coordinate count.
    {
        // Position is slot 0 on every elementary surface, so the two radii
        // sit at slots 1 and 2.
        let two = |name: &str, a: f64, b: f64| {
            let mut m = Model::new();
            m.insert(
                EntityId(10),
                Entity::new(name, vec![Value::Null, Value::Real(a), Value::Real(b)]),
            );
            m
        };
        out.push((
            "ifctoroidalsurface",
            "MajorLargerMinor",
            two("IFCTOROIDALSURFACE", 5.0, 1.0),
            // Equal radii already degenerate: the tube closes on the axis,
            // so the bound is >=, not >.
            two("IFCTOROIDALSURFACE", 5.0, 5.0),
        ));
        // Directrix, Radius, InnerRadius.
        let disk = |radius: f64, inner: f64| {
            let mut m = Model::new();
            m.insert(
                EntityId(10),
                Entity::new(
                    "IFCSWEPTDISKSOLID",
                    vec![r(1), Value::Real(radius), Value::Real(inner)],
                ),
            );
            m
        };
        out.push((
            "ifcsweptdisksolid",
            "InnerRadiusSize",
            disk(5.0, 2.0),
            disk(2.0, 5.0),
        ));
        // Directrix, Radius, InnerRadius, StartParam, EndParam, FilletRadius.
        let poly = |radius: f64, fillet: f64| {
            let mut m = Model::new();
            m.insert(EntityId(1), Entity::new("IFCPOLYLINE", vec![]));
            m.insert(
                EntityId(10),
                Entity::new(
                    "IFCSWEPTDISKSOLIDPOLYGONAL",
                    vec![
                        r(1),
                        Value::Real(radius),
                        Value::Null,
                        Value::Null,
                        Value::Null,
                        Value::Real(fillet),
                    ],
                ),
            );
            m
        };
        out.push((
            "ifcsweptdisksolidpolygonal",
            "CorrectRadii",
            poly(1.0, 2.0),
            poly(2.0, 1.0),
        ));
    }

    // A point needs two coordinates; a sweep needs the right profile kind.
    {
        let cp = |n: usize| {
            let mut m = Model::new();
            pt(&mut m, 10, &vec![0.0; n]);
            m
        };
        out.push(("ifccartesianpoint", "CP2Dor3D", cp(2), cp(1)));

        let sweep = |name: &str, kind: &str| {
            let mut m = Model::new();
            m.insert(
                EntityId(1),
                Entity::new(
                    "IFCARBITRARYCLOSEDPROFILEDEF",
                    vec![Value::Enum(kind.into())],
                ),
            );
            m.insert(EntityId(10), Entity::new(name, vec![r(1), Value::Null]));
            m
        };
        out.push((
            "ifcsweptareasolid",
            "SweptAreaType",
            sweep("IFCEXTRUDEDAREASOLID", "AREA"),
            sweep("IFCEXTRUDEDAREASOLID", "CURVE"),
        ));
        out.push((
            "ifcsweptsurface",
            "SweptCurveType",
            sweep("IFCSURFACEOFLINEAREXTRUSION", "CURVE"),
            sweep("IFCSURFACEOFLINEAREXTRUSION", "AREA"),
        ));
    }
    out
}
