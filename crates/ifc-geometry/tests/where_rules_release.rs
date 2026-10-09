//! Where-rules in the declared release's own text (#400).
//!
//! Every case runs against every bundled release and a file that declares
//! none (read as IFC4 ADD2 TC1), and asserts the rule's name in that
//! release: IFC2X3 TC1 numbers its rules (`WR1`, `WR31`, ...). The rule
//! texts each case relies on are compared across releases, from
//! `references/ifc-spec`, by `rules/release.rs`'s own tests.

use ifc_geometry::rules;
use ifc_model::{Entity, EntityId, Model, Value};

fn n(x: f64) -> Value {
    Value::Real(x)
}
fn r(id: u64) -> Value {
    Value::Ref(EntityId(id))
}

/// Build a model with a point, two directions and a placement referencing them.
fn placement_model(axis: &[f64], ref_dir: &[f64], location: &[f64]) -> Model {
    let mut m = Model::new();
    m.insert(
        EntityId(1),
        Entity::new(
            "IFCCARTESIANPOINT",
            vec![Value::List(location.iter().copied().map(n).collect())],
        ),
    );
    m.insert(
        EntityId(2),
        Entity::new(
            "IFCDIRECTION",
            vec![Value::List(axis.iter().copied().map(n).collect())],
        ),
    );
    m.insert(
        EntityId(3),
        Entity::new(
            "IFCDIRECTION",
            vec![Value::List(ref_dir.iter().copied().map(n).collect())],
        ),
    );
    m.insert(
        EntityId(4),
        Entity::new("IFCAXIS2PLACEMENT3D", vec![r(1), r(2), r(3)]),
    );
    m
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

/// `model` declaring `schema` in `FILE_SCHEMA`, or nothing.
fn declaring(schema: Option<&str>, mut model: Model) -> Model {
    if let Some(schema) = schema {
        model.header_mut().schema = vec![schema.to_owned()];
    }
    model
}

/// The rule names violated by entity `id`, sorted.
fn rule_names(m: &Model, id: u64) -> Vec<&'static str> {
    let mut names: Vec<_> = rules::validate(m, EntityId(id))
        .into_iter()
        .map(|v| v.rule)
        .collect();
    names.sort_unstable();
    names
}

/// Does `validate_model` agree with `validate` for `id`? It resolves the
/// governing rules once per type, so both paths are checked.
fn assert_model_agrees(m: &Model, id: u64) {
    let mut whole: Vec<_> = rules::validate_model(m)
        .into_iter()
        .filter(|v| v.entity == EntityId(id))
        .map(|v| v.rule)
        .collect();
    whole.sort_unstable();
    assert_eq!(whole, rule_names(m, id));
}

/// The expected names per release, in [`RELEASES`] order.
fn expect(m: impl Fn(Option<&'static str>) -> Model, id: u64, expected: [&[&str]; 6]) {
    for (schema, want) in RELEASES.into_iter().zip(expected) {
        let model = m(schema);
        assert_eq!(rule_names(&model, id), want, "{schema:?}");
        assert_model_agrees(&model, id);
    }
}

/// A clipping result whose FirstOperand (#1) is a `first` record.
fn clipping(schema: Option<&str>, first: &str) -> Model {
    let mut m = Model::new();
    let attrs = match first {
        // Directrix, Radius, InnerRadius, StartParam, EndParam.
        "IFCSWEPTDISKSOLID" => vec![r(90), n(1.0), Value::Null, n(0.0), n(1.0)],
        _ => vec![r(90), r(91), r(92), n(1.0)],
    };
    m.insert(EntityId(1), Entity::new(first, attrs));
    m.insert(
        EntityId(2),
        Entity::new("IFCHALFSPACESOLID", vec![r(93), Value::Bool(true)]),
    );
    m.insert(
        EntityId(3),
        Entity::new(
            "IFCBOOLEANCLIPPINGRESULT",
            vec![Value::Enum("DIFFERENCE".into()), r(1), r(2)],
        ),
    );
    declaring(schema, m)
}

/// `FirstOperandType`: IFC2X3 TC1 (`WR1`) admits a swept area or a nested
/// clipping result; IFC4 ADD2 TC1 on add the swept disk their text spells
/// `IFCSWEPTDISCSOLID` (read as `IfcSweptDiskSolid`; see
/// `rules/solid.rs`).
#[test]
fn first_operand_type_admits_a_swept_disk_from_ifc4_on() {
    expect(
        |s| clipping(s, "IFCSWEPTDISKSOLID"),
        3,
        [&["WR1"], &[], &[], &[], &[], &[]],
    );
    // A subtype of the swept area solid is admitted everywhere (TYPEOF).
    expect(|s| clipping(s, "IFCEXTRUDEDAREASOLID"), 3, [&[]; 6]);
    // A B-rep is admitted nowhere, named per release.
    expect(
        |s| clipping(s, "IFCFACETEDBREP"),
        3,
        [
            &["WR1"],
            &["FirstOperandType"],
            &["FirstOperandType"],
            &["FirstOperandType"],
            &["FirstOperandType"],
            &["FirstOperandType"],
        ],
    );
}

/// A boolean result between an extrusion and a tessellated face set that
/// does not declare itself closed.
fn open_tessellated_operand(schema: Option<&str>, set: &str) -> Model {
    let mut m = Model::new();
    m.insert(
        EntityId(1),
        Entity::new("IFCEXTRUDEDAREASOLID", vec![r(90), r(91), r(92), n(1.0)]),
    );
    // Coordinates, Normals, Closed, CoordIndex, PnIndex (and Flags on a
    // TIN): Closed is slot 2, written FALSE.
    let mut attrs = vec![r(94), Value::Null, Value::Bool(false), Value::List(vec![])];
    if set == "IFCTRIANGULATEDIRREGULARNETWORK" {
        attrs.extend([Value::Null, Value::List(vec![])]);
    }
    m.insert(EntityId(2), Entity::new(set, attrs));
    m.insert(
        EntityId(3),
        Entity::new(
            "IFCBOOLEANRESULT",
            vec![Value::Enum("UNION".into()), r(1), r(2)],
        ),
    );
    declaring(schema, m)
}

/// `SecondOperandClosed` exists from IFC4 ADD2 TC1 on; IFC2X3 TC1 declares
/// no operand rule but `WR1` (`SameDim`).
#[test]
fn operand_closed_rules_do_not_apply_to_ifc2x3() {
    let closed: &[&str] = &["SecondOperandClosed"];
    expect(
        |s| open_tessellated_operand(s, "IFCTRIANGULATEDFACESET"),
        3,
        [&[], closed, closed, closed, closed, closed],
    );
    // IfcTriangulatedIrregularNetwork (IFC4X1 on) is a triangulated face
    // set, so TYPEOF finds its Closed slot too. IFC4 does not declare it.
    expect(
        |s| open_tessellated_operand(s, "IFCTRIANGULATEDIRREGULARNETWORK"),
        3,
        [&[], &[], closed, closed, closed, &[]],
    );
}

/// `IfcDirection.MagnitudeGreaterZero` exists from IFC4 ADD2 TC1 on.
#[test]
fn magnitude_greater_zero_does_not_apply_to_ifc2x3() {
    let zero = |s| {
        let mut m = Model::new();
        m.insert(
            EntityId(1),
            Entity::new(
                "IFCDIRECTION",
                vec![Value::List(vec![n(0.0), n(0.0), n(0.0)])],
            ),
        );
        declaring(s, m)
    };
    let rule: &[&str] = &["MagnitudeGreaterZero"];
    expect(zero, 1, [&[], rule, rule, rule, rule, rule]);
}

/// `IfcRepresentationMap.ApplicableMappedRepr` exists from IFC4 ADD2 TC1 on,
/// and its `TYPEOF` test is answered in the release's representation
/// hierarchy.
#[test]
fn applicable_mapped_repr_does_not_apply_to_ifc2x3() {
    let map = |s: Option<&'static str>, mapped: &str| {
        let mut m = Model::new();
        m.insert(
            EntityId(1),
            Entity::new(
                mapped,
                vec![r(90), Value::Null, Value::Null, Value::List(vec![])],
            ),
        );
        m.insert(
            EntityId(2),
            Entity::new("IFCREPRESENTATIONMAP", vec![r(91), r(1)]),
        );
        declaring(s, m)
    };
    let rule: &[&str] = &["ApplicableMappedRepr"];
    expect(
        |s| map(s, "IFCSTYLEDREPRESENTATION"),
        2,
        [&[], rule, rule, rule, rule, rule],
    );
    expect(|s| map(s, "IFCSHAPEREPRESENTATION"), 2, [&[]; 6]);
    expect(|s| map(s, "IFCTOPOLOGYREPRESENTATION"), 2, [&[]; 6]);
}

/// A swept solid of `kind` along an unbounded line, with no StartParam and
/// EndParam to bound it.
fn unbounded_sweep(schema: Option<&str>, kind: &str) -> Model {
    let mut m = Model::new();
    m.insert(EntityId(1), Entity::new("IFCLINE", vec![r(90), r(91)]));
    let attrs = match kind {
        // Directrix, Radius, InnerRadius, StartParam, EndParam.
        "IFCSWEPTDISKSOLID" => vec![r(1), n(1.0), Value::Null, Value::Null, Value::Null],
        // SweptArea, Position, Directrix, StartParam, EndParam, and the
        // fixed reference or reference surface.
        _ => vec![r(92), r(93), r(1), Value::Null, Value::Null, r(94)],
    };
    m.insert(EntityId(2), Entity::new(kind, attrs));
    declaring(schema, m)
}

/// `DirectrixBounded`: absent from IFC2X3 TC1; declared on the three swept
/// solids in IFC4 ADD2 TC1 to IFC4X2; on `IfcSweptDiskSolid` and
/// `IfcDirectrixCurveSweptAreaSolid` in IFC4X3 ADD2, where it binds every
/// subtype of the latter, including the new
/// `IfcDirectrixDerivedReferenceSweptAreaSolid`.
#[test]
fn directrix_bounded_follows_each_release() {
    let rule: &[&str] = &["DirectrixBounded"];
    for kind in [
        "IFCSWEPTDISKSOLID",
        "IFCSURFACECURVESWEPTAREASOLID",
        "IFCFIXEDREFERENCESWEPTAREASOLID",
    ] {
        expect(
            |s| unbounded_sweep(s, kind),
            2,
            [&[], rule, rule, rule, rule, rule],
        );
    }
    // Only IFC4X3 ADD2 declares the derived-reference sweep.
    expect(
        |s| unbounded_sweep(s, "IFCDIRECTRIXDERIVEDREFERENCESWEPTAREASOLID"),
        2,
        [&[], &[], &[], &[], rule, &[]],
    );
    // A bounded directrix satisfies it.
    let bounded = |s| {
        let mut m = unbounded_sweep(s, "IFCDIRECTRIXDERIVEDREFERENCESWEPTAREASOLID");
        m.insert(
            EntityId(1),
            Entity::new("IFCPOLYLINE", vec![Value::List(vec![r(90), r(91)])]),
        );
        m
    };
    expect(bounded, 2, [&[]; 6]);
}

/// A revolved solid about an axis at `location` (#1).
fn revolved(schema: Option<&str>, location: Entity) -> Model {
    let mut m = Model::new();
    m.insert(EntityId(1), location);
    m.insert(
        EntityId(2),
        Entity::new(
            "IFCDIRECTION",
            vec![Value::List(vec![n(0.0), n(1.0), n(0.0)])],
        ),
    );
    m.insert(
        EntityId(3),
        Entity::new("IFCAXIS1PLACEMENT", vec![r(1), r(2)]),
    );
    m.insert(
        EntityId(4),
        Entity::new("IFCREVOLVEDAREASOLID", vec![r(90), r(91), r(3), n(1.0)]),
    );
    declaring(schema, m)
}

/// `AxisStartInXY`: IFC4X3 ADD2 also demands a Cartesian location; every
/// release demands z = 0, IFC2X3 TC1 as `WR31`.
#[test]
fn axis_start_in_xy_follows_each_release() {
    let point = |coords: &[f64]| {
        Entity::new(
            "IFCCARTESIANPOINT",
            vec![Value::List(coords.iter().copied().map(n).collect())],
        )
    };
    let start: &[&str] = &["AxisStartInXY"];
    expect(
        |s| revolved(s, point(&[0.0, 0.0, 1.0])),
        4,
        [&["WR31"], start, start, start, start, start],
    );
    expect(|s| revolved(s, point(&[1.0, 0.0, 0.0])), 4, [&[]; 6]);
    // A point on a curve is an IfcPoint but not an IfcCartesianPoint. Only
    // IFC4X3 ADD2 types Location as IfcPoint and states the clause; earlier
    // releases read Coordinates[3] of it, which is indeterminate.
    let on_curve = || Entity::new("IFCPOINTONCURVE", vec![r(90), n(0.5)]);
    expect(
        |s| revolved(s, on_curve()),
        4,
        [&[], &[], &[], &[], start, &[]],
    );
    // IFC4X3 ADD2's IfcAxis1Placement.LocationIsCP reports the placement
    // itself, under its own name.
    expect(
        |s| revolved(s, on_curve()),
        3,
        [&[], &[], &[], &[], &["LocationIsCP"], &[]],
    );
}

/// Every violation carries the declared release's rule name: IFC2X3 TC1
/// numbers the placement rules `WR1`-`WR5`.
#[test]
fn violations_carry_the_release_rule_name() {
    let parallel = |s| {
        declaring(
            s,
            placement_model(&[0.0, 0.0, 1.0], &[0.0, 0.0, 5.0], &[0.0, 0.0, 0.0]),
        )
    };
    let named: &[&str] = &["AxisToRefDirPosition"];
    expect(parallel, 4, [&["WR4"], named, named, named, named, named]);
    let flat = |s| {
        declaring(
            s,
            placement_model(&[0.0, 0.0, 1.0], &[1.0, 0.0, 0.0], &[1.0, 2.0]),
        )
    };
    let named: &[&str] = &["LocationIs3D"];
    expect(flat, 4, [&["WR1"], named, named, named, named, named]);
}

/// `IfcSweptSurface.WR1` is IFC2X3 TC1's alone: no derived profile may be
/// swept into a surface. IFC4 dropped it.
#[test]
fn ifc2x3_alone_forbids_sweeping_a_derived_profile() {
    let swept = |s| {
        let mut m = Model::new();
        // ProfileType, ProfileName, ParentProfile, Operator, Label.
        m.insert(
            EntityId(1),
            Entity::new(
                "IFCDERIVEDPROFILEDEF",
                vec![
                    Value::Enum("CURVE".into()),
                    Value::Null,
                    r(90),
                    r(91),
                    Value::Null,
                ],
            ),
        );
        // SweptCurve, Position, ExtrudedDirection, Depth.
        m.insert(
            EntityId(2),
            Entity::new(
                "IFCSURFACEOFLINEAREXTRUSION",
                vec![r(1), r(92), r(93), n(1.0)],
            ),
        );
        declaring(s, m)
    };
    expect(swept, 2, [&["WR1"], &[], &[], &[], &[], &[]]);
}

/// IFC4X3 ADD2 composes curves from `IfcCurveSegment`s, whose ParentCurve
/// is slot 4. `IfcSegmentDim` reads it, so `SameDim` compares the parent
/// curves; reading slot 2 would find no curve.
#[test]
fn ifc4x3_curve_segments_carry_their_parent_curve_dimension() {
    let m = |s| {
        let mut m = Model::new();
        let point = |coords: &[f64]| {
            Entity::new(
                "IFCCARTESIANPOINT",
                vec![Value::List(coords.iter().copied().map(n).collect())],
            )
        };
        m.insert(EntityId(1), point(&[0.0, 0.0]));
        m.insert(EntityId(2), point(&[1.0, 0.0]));
        m.insert(EntityId(3), point(&[0.0, 0.0, 0.0]));
        m.insert(EntityId(4), point(&[1.0, 0.0, 0.0]));
        m.insert(
            EntityId(5),
            Entity::new("IFCPOLYLINE", vec![Value::List(vec![r(1), r(2)])]),
        );
        m.insert(
            EntityId(6),
            Entity::new("IFCPOLYLINE", vec![Value::List(vec![r(3), r(4)])]),
        );
        // Transition, Placement, SegmentStart, SegmentLength, ParentCurve.
        for (id, parent) in [(7, 5), (8, 6)] {
            m.insert(
                EntityId(id),
                Entity::new(
                    "IFCCURVESEGMENT",
                    vec![
                        Value::Enum("CONTINUOUS".into()),
                        r(90),
                        n(0.0),
                        n(1.0),
                        r(parent),
                    ],
                ),
            );
        }
        m.insert(
            EntityId(9),
            Entity::new(
                "IFCCOMPOSITECURVE",
                vec![Value::List(vec![r(7), r(8)]), Value::Bool(false)],
            ),
        );
        declaring(s, m)
    };
    let model = m(Some("IFC4X3_ADD2"));
    let names = rule_names(&model, 9);
    assert!(names.contains(&"SameDim"), "{names:?}");
}

/// `IfcCompositeCurveOnSurface.SameSurface` in IFC4X3 ADD2, whose
/// `IfcGetBasisSurface` reads `Segments[i]\IfcCurveSegment.ParentCurve`:
/// curve segments on one surface share it; on two surfaces they share none.
#[test]
fn ifc4x3_curve_segments_on_one_surface_share_it() {
    let on_surfaces = |second_surface: u64| {
        let mut m = Model::new();
        // BasisSurface, ReferenceCurve.
        m.insert(EntityId(1), Entity::new("IFCPCURVE", vec![r(50), r(90)]));
        m.insert(
            EntityId(2),
            Entity::new("IFCPCURVE", vec![r(second_surface), r(91)]),
        );
        // Transition, Placement, SegmentStart, SegmentLength, ParentCurve.
        for (id, parent, transition) in [(3, 1, "CONTINUOUS"), (4, 2, "DISCONTINUOUS")] {
            m.insert(
                EntityId(id),
                Entity::new(
                    "IFCCURVESEGMENT",
                    vec![
                        Value::Enum(transition.into()),
                        r(92),
                        n(0.0),
                        n(1.0),
                        r(parent),
                    ],
                ),
            );
        }
        m.insert(
            EntityId(5),
            Entity::new(
                "IFCCOMPOSITECURVEONSURFACE",
                vec![Value::List(vec![r(3), r(4)]), Value::Bool(false)],
            ),
        );
        declaring(Some("IFC4X3_ADD2"), m)
    };
    assert_eq!(rule_names(&on_surfaces(50), 5), Vec::<&str>::new());
    assert_eq!(rule_names(&on_surfaces(51), 5), ["SameSurface"]);
}
