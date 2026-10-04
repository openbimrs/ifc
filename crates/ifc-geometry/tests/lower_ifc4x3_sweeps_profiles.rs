//! IFC4X3 solid, surface and profile families (#243).
//!
//! `IfcOpenCrossProfileDef` lowers exactly to an open polyline whose vertices
//! are hand-computed here. `IfcDirectrixDerivedReferenceSweptAreaSolid`
//! lowers as its supertype when the directrix defines only a tangent and is
//! refused by name when it defines a tangent plane. `IfcSectionedSolidHorizontal`
//! and `IfcSectionedSurface` lower onto stations; `tests/stations.rs` covers
//! them.

#![cfg(feature = "lowering")]
use axiolid_curve::Curve2;
use axiolid_model::{GeometryNode, SolidOperation};
use ifc_geometry::lower::{
    lower_open_profile_node, lower_profile, lower_representation_item, LoweringSession,
};
use ifc_geometry::transform::Transform;
use ifc_geometry::{GeometryError, UnitScale};
use ifc_model::{Entity, EntityId, Model, Value};

fn r(id: u64) -> Value {
    Value::Ref(EntityId(id))
}
fn n(value: f64) -> Value {
    Value::Real(value)
}
fn reals(values: &[f64]) -> Value {
    Value::List(values.iter().copied().map(n).collect())
}
fn refs(ids: &[u64]) -> Value {
    Value::List(ids.iter().copied().map(r).collect())
}
fn param(value: f64) -> Value {
    Value::Typed {
        type_name: "IFCPARAMETERVALUE".into(),
        value: Box::new(n(value)),
    }
}
fn put(model: &mut Model, id: u64, type_name: &str, attributes: Vec<Value>) {
    model.insert(EntityId(id), Entity::new(type_name, attributes));
}

/// Millimetres and degrees: a missed conversion of either quantity is visible.
const MM_DEG: UnitScale = UnitScale {
    length_to_metres: 0.001,
    angle_to_radians: std::f64::consts::PI / 180.0,
};

/// An `IfcOpenCrossProfileDef` at id 10, with `OffsetPoint` at id 9 when given.
fn open_cross(
    horizontal: bool,
    widths: &[f64],
    slopes: &[f64],
    tags: Option<&[&str]>,
    offset: Option<[f64; 2]>,
) -> Model {
    let mut model = Model::new();
    if let Some([x, y]) = offset {
        put(&mut model, 9, "IFCCARTESIANPOINT", vec![reals(&[x, y])]);
    }
    put(
        &mut model,
        10,
        "IFCOPENCROSSPROFILEDEF",
        vec![
            Value::Enum("CURVE".into()),
            Value::Text("carriageway".into()),
            Value::Bool(horizontal),
            reals(widths),
            reals(slopes),
            tags.map_or(Value::Null, |tags| {
                Value::List(tags.iter().map(|t| Value::Text((*t).into())).collect())
            }),
            offset.map_or(Value::Null, |_| r(9)),
        ],
    );
    model
}

/// Lower id 10 as an open profile and return its exact polyline vertices.
fn open_cross_vertices(model: &Model, units: &UnitScale) -> Vec<[f64; 2]> {
    let mut session = LoweringSession::new(model, units);
    let root = lower_open_profile_node(&mut session, EntityId(10)).expect("open cross lowers");
    let lowered = session.finish(root).expect("valid graph");
    let GeometryNode::OpenProfile(profile) = lowered.graph.get(root).expect("root") else {
        panic!("expected an OpenProfile")
    };
    let GeometryNode::Curve2(Curve2::Polyline(path)) =
        lowered.graph.get(profile.path).expect("path")
    else {
        panic!("expected an exact 2D polyline")
    };
    assert!(!path.closed, "an open cross profile is an open path");
    assert_eq!(lowered.provenance.source(root), Some(EntityId(10)));
    assert_eq!(
        lowered.provenance.source(profile.path),
        Some(EntityId(10)),
        "the chain has no curve entity; the profile is its source"
    );
    path.points.iter().map(|p| [p.x, p.y]).collect()
}

fn assert_vertices(actual: &[[f64; 2]], expected: &[[f64; 2]]) {
    assert_eq!(actual.len(), expected.len(), "{actual:?}");
    for (a, e) in actual.iter().zip(expected) {
        assert!(
            (a[0] - e[0]).abs() < 1e-12 && (a[1] - e[1]).abs() < 1e-12,
            "expected {expected:?}, got {actual:?}"
        );
    }
}

/// Horizontal widths are X runs; the chain starts at `OffsetPoint`.
///
/// Millimetres and degrees. Start (1000, 500) mm = (1, 0.5) m. Segment 1:
/// 3500 mm run at 45 degrees, so +3.5 m in X and +3.5 m in Y. Segment 2:
/// 2000 mm run at 135 degrees (towards -X, rising), so -2 m in X and +2 m in
/// Y. Vertices: (1, 0.5), (4.5, 4), (2.5, 6).
#[test]
fn an_open_cross_profile_with_horizontal_widths_lowers_to_its_exact_vertices() {
    let model = open_cross(
        true,
        &[3500.0, 2000.0],
        &[45.0, 135.0],
        Some(&["left", "crown", "right"]),
        Some([1000.0, 500.0]),
    );
    let vertices = open_cross_vertices(&model, &MM_DEG);
    assert_vertices(&vertices, &[[1.0, 0.5], [4.5, 4.0], [2.5, 6.0]]);
}

/// Along-slope widths are segment lengths; no offset starts at the origin.
///
/// 2 m at 30 degrees: (2 cos 30, 2 sin 30) = (sqrt 3, 1). Then 1 m at -90
/// degrees, which is legal along the slope: straight down to (sqrt 3, 0).
#[test]
fn an_open_cross_profile_with_slope_widths_lowers_to_its_exact_vertices() {
    let model = open_cross(false, &[2000.0, 1000.0], &[30.0, -90.0], None, None);
    let vertices = open_cross_vertices(&model, &MM_DEG);
    let root3 = 3f64.sqrt();
    assert_vertices(&vertices, &[[0.0, 0.0], [root3, 1.0], [root3, 0.0]]);
}

/// The open cross profile has no area, so the area-profile path refuses it.
#[test]
fn an_open_cross_profile_is_not_an_area_profile() {
    let model = open_cross(true, &[1.0], &[0.0], None, None);
    let error =
        lower_profile(&model, EntityId(10), &UnitScale::default()).expect_err("open, no area");
    assert!(
        error.is_unsupported() && error.to_string().contains("use lower_open_profile_node"),
        "{error}"
    );
}

/// The WHERE rules that make the construction well defined are enforced.
#[test]
fn malformed_open_cross_profiles_are_refused_as_degenerate() {
    let cases: [(Model, &str); 5] = [
        (
            open_cross(true, &[1.0, 1.0], &[0.0], None, None),
            "CorrespondingSlopeWidths",
        ),
        (
            open_cross(true, &[1.0], &[0.0], Some(&["only"]), None),
            "CorrespondingTags",
        ),
        (open_cross(true, &[-1.0], &[0.0], None, None), "negative"),
        (
            open_cross(true, &[1.0], &[std::f64::consts::FRAC_PI_2], None, None),
            "vertical",
        ),
        (
            open_cross(false, &[0.0, 0.0], &[0.3, 0.1], None, None),
            "returns to its start point",
        ),
    ];
    for (model, needle) in cases {
        let units = UnitScale::default();
        let mut session = LoweringSession::new(&model, &units);
        let error =
            lower_open_profile_node(&mut session, EntityId(10)).expect_err("malformed profile");
        assert!(
            matches!(error, GeometryError::Degenerate { .. }) && error.to_string().contains(needle),
            "expected a degenerate refusal naming {needle:?}, got: {error}"
        );
    }
}

// ---------------------------------------------------------------------------
// Swept solids
// ---------------------------------------------------------------------------

/// A sweep at id 1 of type `sweep_type` along the directrix at id 20,
/// trimmed by parameter from 0 to 2, with `FixedReference` +Z.
///
/// The profile is a 1 x 0.5 rectangle; the default directrix is a straight
/// 3D polyline from the origin to (2, 0, 0).
fn sweep(sweep_type: &str) -> Model {
    let mut model = Model::new();
    put(
        &mut model,
        2,
        "IFCRECTANGLEPROFILEDEF",
        vec![
            Value::Enum("AREA".into()),
            Value::Null,
            Value::Null,
            n(1.0),
            n(0.5),
        ],
    );
    put(&mut model, 3, "IFCDIRECTION", vec![reals(&[0.0, 0.0, 1.0])]);
    put(
        &mut model,
        21,
        "IFCCARTESIANPOINT",
        vec![reals(&[0.0, 0.0, 0.0])],
    );
    put(
        &mut model,
        22,
        "IFCCARTESIANPOINT",
        vec![reals(&[2.0, 0.0, 0.0])],
    );
    put(&mut model, 20, "IFCPOLYLINE", vec![refs(&[21, 22])]);
    put(
        &mut model,
        1,
        sweep_type,
        vec![r(2), Value::Null, r(20), param(0.0), param(2.0), r(3)],
    );
    model
}

fn lower_item(model: &Model, id: u64) -> Result<SolidOperation, GeometryError> {
    lower_graph(model, id).map(|(operation, _)| operation)
}

/// The lowered solid operation and its curve and profile nodes, in order.
///
/// Node handles are branded per graph, so two lowerings are compared by the
/// leaf nodes' contents rather than by handle.
fn lower_graph(
    model: &Model,
    id: u64,
) -> Result<(SolidOperation, Vec<GeometryNode>), GeometryError> {
    let units = UnitScale::default();
    let mut session = LoweringSession::new(model, &units);
    let root = lower_representation_item(&mut session, EntityId(id), Transform::identity())?;
    let lowered = session.finish(root).expect("valid graph");
    let operation = lowered
        .graph
        .iter()
        .find_map(|(_, node)| match node {
            GeometryNode::SolidOperation(op) => Some(op.clone()),
            _ => None,
        })
        .expect("a solid operation");
    let nodes = lowered
        .graph
        .iter()
        .filter(|(_, node)| matches!(node, GeometryNode::Curve3(_) | GeometryNode::Profile(_)))
        .map(|(_, node)| node.clone())
        .collect();
    Ok((operation, nodes))
}

/// Over a tangent-only directrix the derived reference IS the fixed one.
///
/// IFC4.3: "In most cases ... exactly the same behaviour as
/// IfcFixedReferenceSweptAreaSolid". The lowered operation must be identical
/// to the supertype's for the same attributes, reference +Z kept, trim kept.
#[test]
fn a_directrix_derived_sweep_on_a_tangent_only_directrix_is_its_fixed_reference_sweep() {
    let (derived, derived_nodes) =
        lower_graph(&sweep("IFCDIRECTRIXDERIVEDREFERENCESWEPTAREASOLID"), 1)
            .expect("tangent-only directrix lowers");
    let (fixed, fixed_nodes) =
        lower_graph(&sweep("IFCFIXEDREFERENCESWEPTAREASOLID"), 1).expect("supertype lowers");
    let reference_and_range = |operation: &SolidOperation| match operation {
        SolidOperation::FixedReferenceSweep {
            reference_direction,
            parameter_range,
            ..
        } => (reference_direction.to_array(), *parameter_range),
        other => panic!("expected a FixedReferenceSweep, got {other:?}"),
    };
    let (reference, range) = reference_and_range(&derived);
    assert_eq!(reference, [0.0, 0.0, 1.0], "FixedReference +Z is kept");
    assert_eq!(range, Some((0.0, 2.0)), "the parameter trim is kept");
    assert_eq!(reference_and_range(&fixed), (reference, range));
    assert_eq!(
        derived_nodes, fixed_nodes,
        "same profile and directrix nodes as the supertype"
    );
}

/// Assert `result` is the named tangent-plane refusal on entity #1.
fn assert_tangent_plane_refusal(result: Result<SolidOperation, GeometryError>) {
    let error = result.expect_err("a tangent-plane directrix must be refused");
    assert!(error.is_unsupported(), "{error}");
    assert_eq!(error.entity(), Some(EntityId(1)), "the sweep is named");
    assert!(
        error.to_string().contains("defines a tangent plane")
            && error
                .to_string()
                .contains("IFCDIRECTRIXDERIVEDREFERENCESWEPTAREASOLID"),
        "{error}"
    );
}

/// A directrix built from `IfcCurveSegment`s carries a frame per point.
///
/// The IFC4X3 composite curve here is one segment whose parent is a line:
/// geometrically the same straight path as the polyline above, yet the
/// segment placement defines a tangent plane, which the derived reference
/// follows. That rotation has no neutral carrier, so it is refused.
#[test]
fn a_directrix_derived_sweep_on_a_segment_built_directrix_is_refused_by_name() {
    let mut model = sweep("IFCDIRECTRIXDERIVEDREFERENCESWEPTAREASOLID");
    put(
        &mut model,
        30,
        "IFCDIRECTION",
        vec![reals(&[1.0, 0.0, 0.0])],
    );
    put(&mut model, 31, "IFCVECTOR", vec![r(30), n(1.0)]);
    put(&mut model, 32, "IFCLINE", vec![r(21), r(31)]);
    put(
        &mut model,
        33,
        "IFCAXIS2PLACEMENT3D",
        vec![r(21), Value::Null, Value::Null],
    );
    let length = |v: f64| Value::Typed {
        type_name: "IFCLENGTHMEASURE".into(),
        value: Box::new(n(v)),
    };
    put(
        &mut model,
        34,
        "IFCCURVESEGMENT",
        vec![
            Value::Enum("DISCONTINUOUS".into()),
            r(33),
            length(0.0),
            length(2.0),
            r(32),
        ],
    );
    put(
        &mut model,
        20,
        "IFCCOMPOSITECURVE",
        vec![refs(&[34]), Value::Bool(false)],
    );
    assert_tangent_plane_refusal(lower_item(&model, 1));

    // The same segment-built curve under a trim is still caught.
    put(
        &mut model,
        35,
        "IFCCOMPOSITECURVE",
        vec![refs(&[34]), Value::Bool(false)],
    );
    put(
        &mut model,
        20,
        "IFCTRIMMEDCURVE",
        vec![
            r(35),
            Value::List(vec![param(0.0)]),
            Value::List(vec![param(1.0)]),
            Value::Bool(true),
            Value::Enum("PARAMETER".into()),
        ],
    );
    assert_tangent_plane_refusal(lower_item(&model, 1));
}

/// A curve on a surface carries the surface's tangent plane.
#[test]
fn a_directrix_derived_sweep_on_a_surface_curve_is_refused_by_name() {
    let mut model = sweep("IFCDIRECTRIXDERIVEDREFERENCESWEPTAREASOLID");
    put(
        &mut model,
        40,
        "IFCAXIS2PLACEMENT3D",
        vec![r(21), Value::Null, Value::Null],
    );
    put(&mut model, 41, "IFCPLANE", vec![r(40)]);
    put(
        &mut model,
        42,
        "IFCCARTESIANPOINT",
        vec![reals(&[0.0, 0.0])],
    );
    put(
        &mut model,
        43,
        "IFCCARTESIANPOINT",
        vec![reals(&[2.0, 0.0])],
    );
    put(&mut model, 44, "IFCPOLYLINE", vec![refs(&[42, 43])]);
    put(&mut model, 20, "IFCPCURVE", vec![r(41), r(44)]);
    assert_tangent_plane_refusal(lower_item(&model, 1));
}
