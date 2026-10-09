//! Where-rules on geometry entities IFC4 does not declare (#402).
//!
//! IFC2X3 TC1's `Ifc2DCompositeCurve` and `IfcRationalBezierCurve`, and the
//! linear-referencing geometry of IFC4X1 on: `IfcSectionedSolid`,
//! `IfcSectionedSolidHorizontal`, `IfcTriangulatedIrregularNetwork`, and
//! IFC4X3 ADD2's `IfcAxis2PlacementLinear`, `IfcPolynomialCurve` and
//! `IfcSectionedSurface`. Every case runs against every bundled release
//! and a file that declares none (read as IFC4 ADD2 TC1): a release that
//! does not declare the entity checks nothing on it.

use ifc_geometry::rules;
use ifc_model::{Entity, EntityId, Model, Value};

fn n(x: f64) -> Value {
    Value::Real(x)
}
fn r(id: u64) -> Value {
    Value::Ref(EntityId(id))
}
fn e(v: &str) -> Value {
    Value::Enum(v.into())
}
fn reals(xs: &[f64]) -> Value {
    Value::List(xs.iter().copied().map(n).collect())
}
fn refs(ids: &[u64]) -> Value {
    Value::List(ids.iter().copied().map(r).collect())
}

/// `FILE_SCHEMA` tokens for every bundled release, and a headerless file.
const RELEASES: [Option<&'static str>; 6] = [
    Some("IFC2X3"),
    Some("IFC4"),
    Some("IFC4X1"),
    Some("IFC4X2"),
    Some("IFC4X3_ADD2"),
    None,
];

/// The rule names violated by entity `id`, sorted.
fn rule_names(m: &Model, id: u64) -> Vec<&'static str> {
    let mut names: Vec<_> = rules::validate(m, EntityId(id))
        .into_iter()
        .map(|v| v.rule)
        .collect();
    names.sort_unstable();
    names
}

/// The names per release, in [`RELEASES`] order, from `validate` and from
/// `validate_model` alike.
fn expect(build: impl Fn(Option<&'static str>) -> Model, id: u64, expected: [&[&str]; 6]) {
    for (schema, want) in RELEASES.into_iter().zip(expected) {
        let mut model = build(schema);
        if let Some(schema) = schema {
            model.header_mut().schema = vec![schema.to_owned()];
        }
        assert_eq!(rule_names(&model, id), want, "{schema:?}");
        let mut whole: Vec<_> = rules::validate_model(&model)
            .into_iter()
            .filter(|v| v.entity == EntityId(id))
            .map(|v| v.rule)
            .collect();
        whole.sort_unstable();
        assert_eq!(whole, want, "validate_model, {schema:?}");
    }
}

/// Violated only in IFC2X3 TC1.
fn ifc2x3(rule: &'static [&'static str]) -> [&'static [&'static str]; 6] {
    [rule, &[], &[], &[], &[], &[]]
}
/// Violated in IFC4X1, IFC4X2 and IFC4X3 ADD2.
fn ifc4x1_on(rule: &'static [&'static str]) -> [&'static [&'static str]; 6] {
    [&[], &[], rule, rule, rule, &[]]
}
/// Violated only in IFC4X3 ADD2.
fn ifc4x3(rule: &'static [&'static str]) -> [&'static [&'static str]; 6] {
    [&[], &[], &[], &[], rule, &[]]
}
const NONE: [&[&str]; 6] = [&[]; 6];

/// A polyline #10 through two points of `dim` dimensions (#1, #2).
fn polyline(m: &mut Model, dim: usize) {
    let coords = |x: f64| reals(&[x, 0.0, 0.0][..dim]);
    m.insert(
        EntityId(1),
        Entity::new("IFCCARTESIANPOINT", vec![coords(0.0)]),
    );
    m.insert(
        EntityId(2),
        Entity::new("IFCCARTESIANPOINT", vec![coords(1.0)]),
    );
    m.insert(
        EntityId(10),
        Entity::new("IFCPOLYLINE", vec![refs(&[1, 2])]),
    );
}

/// `Ifc2DCompositeCurve` #30 of one segment over a polyline of `dim`
/// dimensions, ending with `transition`.
fn composite_2d(dim: usize, transition: &str) -> Model {
    let mut m = Model::new();
    polyline(&mut m, dim);
    m.insert(
        EntityId(20),
        Entity::new(
            "IFCCOMPOSITECURVESEGMENT",
            vec![e(transition), Value::Bool(true), r(10)],
        ),
    );
    m.insert(
        EntityId(30),
        Entity::new("IFC2DCOMPOSITECURVE", vec![refs(&[20]), Value::Bool(false)]),
    );
    m
}

/// IFC2X3 TC1 `Ifc2DCompositeCurve.WR1 : SELF\IfcCompositeCurve.ClosedCurve`
/// and `WR2 : SELF\IfcCurve.Dim = 2`.
#[test]
fn ifc2x3_two_d_composite_curve() {
    expect(|_| composite_2d(2, "CONTINUOUS"), 30, NONE);
    // Open: the last segment is DISCONTINUOUS (and CurveContinuous holds).
    expect(|_| composite_2d(2, "DISCONTINUOUS"), 30, ifc2x3(&["WR1"]));
    expect(|_| composite_2d(3, "CONTINUOUS"), 30, ifc2x3(&["WR2"]));
}

/// `IfcRationalBezierCurve` #10 over three points with `weights`.
fn bezier(weights: &[f64]) -> Model {
    let mut m = Model::new();
    for id in 1..=3 {
        m.insert(
            EntityId(id),
            Entity::new("IFCCARTESIANPOINT", vec![reals(&[id as f64, 0.0])]),
        );
    }
    m.insert(
        EntityId(10),
        Entity::new(
            "IFCRATIONALBEZIERCURVE",
            vec![
                Value::Integer(2),
                refs(&[1, 2, 3]),
                e("UNSPECIFIED"),
                Value::Bool(false),
                Value::Bool(false),
                reals(weights),
            ],
        ),
    );
    m
}

/// IFC2X3 TC1 `IfcRationalBezierCurve.WR1` (one weight per control point)
/// and `WR2 : IfcCurveWeightsPositive(SELF)`.
#[test]
fn ifc2x3_rational_bezier_curve() {
    expect(|_| bezier(&[1.0, 0.5, 2.0]), 10, NONE);
    expect(|_| bezier(&[1.0, 1.0]), 10, ifc2x3(&["WR1"]));
    expect(|_| bezier(&[1.0, 0.0, 1.0]), 10, ifc2x3(&["WR2"]));
    expect(|_| bezier(&[1.0, -1.0, 1.0]), 10, ifc2x3(&["WR2"]));
    // With the lengths apart, `Weights` is `IfcListToArray(...) = ?`, so
    // the function finds no weight <= 0 and WR2 holds.
    expect(|_| bezier(&[1.0, 0.0]), 10, ifc2x3(&["WR1"]));
}

/// `IfcAxis2PlacementLinear` #4 at `location` (#5), with Axis #2 and
/// RefDirection #3.
fn linear_placement(location: &str, axis: &[f64], ref_dir: &[f64]) -> Model {
    let mut m = Model::new();
    polyline(&mut m, 3);
    m.insert(EntityId(2), Entity::new("IFCDIRECTION", vec![reals(axis)]));
    m.insert(
        EntityId(3),
        Entity::new("IFCDIRECTION", vec![reals(ref_dir)]),
    );
    let point = match location {
        // DistanceAlong, OffsetLateral, OffsetVertical, OffsetLongitudinal,
        // BasisCurve.
        "IFCPOINTBYDISTANCEEXPRESSION" => vec![
            Value::Typed {
                type_name: "IFCLENGTHMEASURE".into(),
                value: Box::new(n(0.5)),
            },
            Value::Null,
            Value::Null,
            Value::Null,
            r(10),
        ],
        _ => vec![reals(&[0.0, 0.0, 0.0])],
    };
    m.insert(EntityId(5), Entity::new(location, point));
    m.insert(
        EntityId(4),
        Entity::new("IFCAXIS2PLACEMENTLINEAR", vec![r(5), r(2), r(3)]),
    );
    m
}

/// IFC4X3 ADD2 `IfcAxis2PlacementLinear.WR1` (the location is a point by
/// distance expression) and `WR2` (Axis not parallel to RefDirection).
#[test]
fn ifc4x3_axis2_placement_linear() {
    const BY_DISTANCE: &str = "IFCPOINTBYDISTANCEEXPRESSION";
    let (z, x) = (&[0.0, 0.0, 1.0][..], &[1.0, 0.0, 0.0][..]);
    expect(|_| linear_placement(BY_DISTANCE, z, x), 4, NONE);
    expect(
        |_| linear_placement("IFCCARTESIANPOINT", z, x),
        4,
        ifc4x3(&["WR1"]),
    );
    expect(
        |_| linear_placement(BY_DISTANCE, z, &[0.0, 0.0, -2.0]),
        4,
        ifc4x3(&["WR2"]),
    );
    // IfcCrossProduct of 2D directions is `?`: WR2 is UNKNOWN, not FALSE.
    expect(
        |_| linear_placement(BY_DISTANCE, &[1.0, 0.0], &[1.0, 0.0]),
        4,
        NONE,
    );
}

/// `IfcPolynomialCurve` #10 at a `dim`-dimensional placement, with the
/// coefficient lists `given` (X, Y, Z).
fn polynomial(dim: usize, given: [bool; 3]) -> Model {
    let mut m = Model::new();
    m.insert(
        EntityId(1),
        Entity::new("IFCCARTESIANPOINT", vec![reals(&[0.0, 0.0, 0.0][..dim])]),
    );
    let placement = if dim == 2 {
        Entity::new("IFCAXIS2PLACEMENT2D", vec![r(1), Value::Null])
    } else {
        Entity::new("IFCAXIS2PLACEMENT3D", vec![r(1), Value::Null, Value::Null])
    };
    m.insert(EntityId(4), placement);
    let mut attrs = vec![r(4)];
    attrs.extend(given.map(|g| if g { reals(&[0.0, 1.0]) } else { Value::Null }));
    m.insert(EntityId(10), Entity::new("IFCPOLYNOMIALCURVE", attrs));
    m
}

/// IFC4X3 ADD2 `IfcPolynomialCurve.CorrectPositionDim` and
/// `ValidCoefficients`.
#[test]
fn ifc4x3_polynomial_curve() {
    expect(|_| polynomial(2, [true, true, false]), 10, NONE);
    expect(|_| polynomial(3, [true, true, true]), 10, NONE);
    expect(|_| polynomial(3, [false, true, true]), 10, NONE);
    expect(
        |_| polynomial(2, [true, true, true]),
        10,
        ifc4x3(&["CorrectPositionDim"]),
    );
    expect(
        |_| polynomial(3, [true, false, false]),
        10,
        ifc4x3(&["ValidCoefficients"]),
    );
    // Positioned linearly, Position.Dim is IfcPointDim of the point by
    // distance expression: its BasisCurve's Dim.
    let linear = |basis_dim: usize| {
        move |_| {
            let mut m = linear_placement(
                "IFCPOINTBYDISTANCEEXPRESSION",
                &[0.0, 0.0, 1.0],
                &[1.0, 0.0, 0.0],
            );
            polyline(&mut m, basis_dim);
            m.insert(
                EntityId(11),
                Entity::new(
                    "IFCPOLYNOMIALCURVE",
                    vec![r(4), reals(&[0.0, 1.0]), Value::Null, reals(&[0.0, 1.0])],
                ),
            );
            m
        }
    };
    expect(linear(3), 11, NONE);
    expect(linear(2), 11, ifc4x3(&["CorrectPositionDim"]));
}

/// What a sectioned solid or surface case changes from a conforming one.
#[derive(Clone, Copy, Default)]
struct Sectioned {
    /// The directrix is 2D.
    flat: bool,
    /// The second cross-section's ProfileType.
    second_kind: Option<&'static str>,
    /// The second cross-section is a circle, not a rectangle.
    second_circle: bool,
    /// One position fewer than cross-sections.
    short: bool,
    /// The offset slot (1 lateral, 2 vertical, 3 longitudinal) the first
    /// position sets.
    offset: Option<usize>,
}

/// Directrix #10, cross-sections #30/#31 of `kind`, positions #40/#41, and
/// the sectioned item #50 of type `item`, in the attribute layout of
/// `schema` (IFC4X3 ADD2 positions are linear placements at #42/#43).
fn sectioned(schema: Option<&str>, item: &str, kind: &str, case: Sectioned) -> Model {
    let mut m = Model::new();
    polyline(&mut m, if case.flat { 2 } else { 3 });
    let profile = |kind: &str, circle: bool| {
        if circle {
            Entity::new(
                "IFCCIRCLEPROFILEDEF",
                vec![e(kind), Value::Null, Value::Null, n(1.0)],
            )
        } else {
            Entity::new(
                "IFCRECTANGLEPROFILEDEF",
                vec![e(kind), Value::Null, Value::Null, n(1.0), n(1.0)],
            )
        }
    };
    m.insert(EntityId(30), profile(kind, false));
    m.insert(
        EntityId(31),
        profile(case.second_kind.unwrap_or(kind), case.second_circle),
    );
    let ifc4x3 = schema == Some("IFC4X3_ADD2");
    let positions: &[u64] = if case.short { &[40] } else { &[40, 41] };
    for (i, id) in [40u64, 41].into_iter().enumerate() {
        let mut distance = vec![n(i as f64), Value::Null, Value::Null, Value::Null];
        if let Some(slot) = case.offset.filter(|_| i == 0) {
            distance[slot] = n(0.25);
        }
        if ifc4x3 {
            // DistanceAlong, the three offsets, BasisCurve.
            distance.push(r(10));
            m.insert(
                EntityId(id + 2),
                Entity::new("IFCPOINTBYDISTANCEEXPRESSION", distance),
            );
            m.insert(
                EntityId(id),
                Entity::new(
                    "IFCAXIS2PLACEMENTLINEAR",
                    vec![r(id + 2), Value::Null, Value::Null],
                ),
            );
        } else {
            // DistanceAlong, the three offsets, AlongHorizontal.
            distance.push(Value::Null);
            m.insert(EntityId(id), Entity::new("IFCDISTANCEEXPRESSION", distance));
        }
    }
    let attrs = match item {
        "IFCSECTIONEDSURFACE" => vec![r(10), refs(positions), refs(&[30, 31])],
        // IFC4X1 and IFC4X2 add FixedAxisVertical.
        _ if ifc4x3 => vec![r(10), refs(&[30, 31]), refs(positions)],
        _ => vec![r(10), refs(&[30, 31]), refs(positions), Value::Bool(true)],
    };
    m.insert(EntityId(50), Entity::new(item, attrs));
    m
}

/// `IfcSectionedSolid` (`ConsistentProfileTypes`, `DirectrixIs3D`,
/// `SectionsSameType`) and `IfcSectionedSolidHorizontal`
/// (`CorrespondingSectionPositions`, `NoLongitudinalOffsets`), IFC4X1 on.
#[test]
fn ifc4x1_sectioned_solid_horizontal() {
    const SOLID: &str = "IFCSECTIONEDSOLIDHORIZONTAL";
    let solid = |case: Sectioned| move |s| sectioned(s, SOLID, "AREA", case);
    expect(solid(Sectioned::default()), 50, NONE);
    let cases: [(Sectioned, &'static [&'static str]); 5] = [
        (
            Sectioned {
                second_kind: Some("CURVE"),
                ..Sectioned::default()
            },
            &["ConsistentProfileTypes"],
        ),
        (
            Sectioned {
                flat: true,
                ..Sectioned::default()
            },
            &["DirectrixIs3D"],
        ),
        (
            Sectioned {
                second_circle: true,
                ..Sectioned::default()
            },
            &["SectionsSameType"],
        ),
        (
            Sectioned {
                short: true,
                ..Sectioned::default()
            },
            &["CorrespondingSectionPositions"],
        ),
        (
            Sectioned {
                offset: Some(3),
                ..Sectioned::default()
            },
            &["NoLongitudinalOffsets"],
        ),
    ];
    for (case, rule) in cases {
        expect(solid(case), 50, ifc4x1_on(rule));
    }
    // Lateral and vertical offsets are allowed on the solid.
    for slot in [1, 2] {
        let case = Sectioned {
            offset: Some(slot),
            ..Sectioned::default()
        };
        expect(solid(case), 50, NONE);
    }
}

/// IFC4X3 ADD2 `IfcSectionedSurface`: `AreaProfileTypes`,
/// `CorrespondingSectionPositions`, `DirectrixIs3D`, `NoOffsets`,
/// `SectionsSameType`.
#[test]
fn ifc4x3_sectioned_surface() {
    const SURFACE: &str = "IFCSECTIONEDSURFACE";
    let surface = |kind: &'static str, case: Sectioned| move |s| sectioned(s, SURFACE, kind, case);
    expect(surface("CURVE", Sectioned::default()), 50, NONE);
    // One CURVE section suffices, and the surface declares no
    // ConsistentProfileTypes.
    let mixed = Sectioned {
        second_kind: Some("AREA"),
        ..Sectioned::default()
    };
    expect(surface("CURVE", mixed), 50, NONE);
    expect(
        surface("AREA", Sectioned::default()),
        50,
        ifc4x3(&["AreaProfileTypes"]),
    );
    let cases: [(Sectioned, &'static [&'static str]); 3] = [
        (
            Sectioned {
                short: true,
                ..Sectioned::default()
            },
            &["CorrespondingSectionPositions"],
        ),
        (
            Sectioned {
                flat: true,
                ..Sectioned::default()
            },
            &["DirectrixIs3D"],
        ),
        (
            Sectioned {
                second_circle: true,
                ..Sectioned::default()
            },
            &["SectionsSameType"],
        ),
    ];
    for (case, rule) in cases {
        expect(surface("CURVE", case), 50, ifc4x3(rule));
    }
    for slot in [1, 2, 3] {
        let case = Sectioned {
            offset: Some(slot),
            ..Sectioned::default()
        };
        expect(surface("CURVE", case), 50, ifc4x3(&["NoOffsets"]));
    }
}

/// An `IfcTriangulatedIrregularNetwork` #2 with `closed` in its Closed slot.
fn tin(closed: Value) -> Model {
    let mut m = Model::new();
    m.insert(
        EntityId(1),
        Entity::new(
            "IFCCARTESIANPOINTLIST3D",
            vec![Value::List(vec![
                reals(&[0.0, 0.0, 0.0]),
                reals(&[1.0, 0.0, 0.0]),
                reals(&[0.0, 1.0, 0.0]),
            ])],
        ),
    );
    // Coordinates, Normals, Closed, CoordIndex, PnIndex, Flags.
    m.insert(
        EntityId(2),
        Entity::new(
            "IFCTRIANGULATEDIRREGULARNETWORK",
            vec![
                r(1),
                Value::Null,
                closed,
                Value::List(vec![Value::List(vec![
                    Value::Integer(1),
                    Value::Integer(2),
                    Value::Integer(3),
                ])]),
                Value::Null,
                Value::List(vec![Value::Integer(0)]),
            ],
        ),
    );
    m
}

/// `IfcTriangulatedIrregularNetwork.NotClosed : SELF\IfcTriangulatedFaceSet.
/// Closed = FALSE`, IFC4X1 on: an omitted Closed compares UNKNOWN.
#[test]
fn ifc4x1_tin_not_closed() {
    expect(|_| tin(Value::Bool(false)), 2, NONE);
    expect(|_| tin(Value::Null), 2, NONE);
    expect(|_| tin(Value::Bool(true)), 2, ifc4x1_on(&["NotClosed"]));
}
