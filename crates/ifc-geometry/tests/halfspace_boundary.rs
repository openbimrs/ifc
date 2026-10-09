//! `IfcCompositeCurve` and `IfcIndexedPolyCurve` half-space boundaries (#393,
//! #398).
//!
//! The fixture (`tools/gen_lowering_fixtures.py`,
//! `halfspace_boundaries_ifc4x3.ifc`, IFC4X3_ADD2) has eight walls, each a
//! 4 x 1 x 3 extrusion over `[0, 4] x [0, 1]` cut above `z = 2` by an
//! `IfcPolygonalBoundedHalfSpace`. The first six state the anticlockwise
//! pentagon `(1,-1) (3,-1) (3,0.5) (2,2) (1,0.5)`, named by how the boundary
//! is authored:
//!
//! - `POLYLINE`: an `IfcPolyline`, the twin the others must equal;
//! - `COMPOSITE`: three polyline segments, the middle one backwards with
//!   `SameSense` FALSE;
//! - `COMPOSITE_LINE`: a polyline closed by a trimmed `IfcLine`;
//! - `INDEXED`: an `IfcIndexedPolyCurve` without `Segments`;
//! - `INDEXED_SEGMENTS`: three `IfcLineIndex` segments;
//! - `INDEXED_COLLINEAR_ARC`: the same with the gable edge an `IfcArcIndex`
//!   through its midpoint, "treated as a polyline segment" (#396).
//!
//! Each lowers to the twin's `Polyline2` and compiles to its volume,
//! `12 - 11/6`. Two more have genuine circular arcs (#398) and lower to an
//! exact `Profile::Contour` boundary (axiolid/kernel#277, Axiolid ADR 0084):
//!
//! - `COMPOSITE_ARCS`: six segments, two of them trimmed `IfcCircle` arcs of
//!   radius 1.2 about `(1, 0.5)`, bounding the rectangle
//!   `[1, 3.5] x [-0.22, 2.5]` less that disk;
//! - `INDEXED_ARC`: the rectangle `[1, 3] x [-1, 0.5]` under the half circle
//!   of radius 1 about `(2, 0.5)`, an `IfcArcIndex`.
//!
//! Each compiles, exact and meshed, to its closed-form volume. The
//! refusals edit one record in memory: every item of a committed fixture
//! must lower (`tests/lower_dispatch_corpus.rs`).
//!
//! `BoundaryType` is read in the file's release (#397): IFC4X3 ADD2 admits
//! the indexed boundaries, IFC4 ADD2 TC1 does not.

#![cfg(feature = "lowering")]

use std::path::PathBuf;

use axiolid_core::Point2;
use axiolid_curve::{Curve2, Polyline2};
use axiolid_model::{GeometryNode, SolidOperation};
use axiolid_profile::{Contour, ContourProfile, Profile};
use ifc_geometry::lower::{lower_half_space_node, LoweringSession};
use ifc_geometry::transform::Transform;
use ifc_geometry::{units, GeometryError, GeometryResult};
use ifc_model::{Codec, EntityId, Model, Value};
use ifc_step::StepCodec;

const FIXTURE: &str = "halfspace_boundaries_ifc4x3.ifc";
const TWINS: [&str; 5] = [
    "COMPOSITE",
    "COMPOSITE_LINE",
    "INDEXED",
    "INDEXED_SEGMENTS",
    "INDEXED_COLLINEAR_ARC",
];
const PENTAGON: [[f64; 2]; 5] = [[1.0, -1.0], [3.0, -1.0], [3.0, 0.5], [2.0, 2.0], [1.0, 0.5]];

fn text() -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../test/fixtures/synthetic-lowering")
        .join(FIXTURE);
    std::fs::read_to_string(path).expect("fixture reads")
}

fn parse(text: &str) -> Model {
    StepCodec
        .read_bytes(text.as_bytes())
        .expect("fixture parses")
}

/// The fixture with each `(record, replacement)` applied; each record is
/// unique.
fn edited(edits: &[(&str, &str)]) -> Model {
    let mut text = text();
    for (record, replacement) in edits {
        assert_eq!(text.matches(record).count(), 1, "{record} is unique");
        text = text.replace(record, replacement);
    }
    parse(&text)
}

fn reference(model: &Model, id: EntityId, slot: usize) -> EntityId {
    let entity = model.get(id).expect("entity exists");
    match entity.attributes.get(slot) {
        Some(Value::Ref(target)) => *target,
        Some(Value::List(items)) => match items.first() {
            Some(Value::Ref(target)) => *target,
            other => panic!("{id:?} slot {slot}: expected a reference, got {other:?}"),
        },
        other => panic!("{id:?} slot {slot}: expected a reference, got {other:?}"),
    }
}

/// The wall named `name`.
fn wall(model: &Model, name: &str) -> EntityId {
    let found: Vec<EntityId> = model
        .of_type("IFCWALL")
        .filter(|(_, entity)| {
            matches!(entity.attributes.get(2), Some(Value::Text(text)) if &**text == name)
        })
        .map(|(id, _)| id)
        .collect();
    assert_eq!(found.len(), 1, "exactly one wall named {name}");
    found[0]
}

/// The wall's `IfcPolygonalBoundedHalfSpace`: Representation, its one
/// representation, its one item (the clipping result), the second operand.
fn half_space(model: &Model, name: &str) -> EntityId {
    let shape = reference(model, wall(model, name), 6);
    let representation = reference(model, shape, 2);
    let clip = reference(model, representation, 3);
    let operand = reference(model, clip, 2);
    let type_name = &model.get(operand).expect("operand").type_name;
    assert!(
        type_name.eq_ignore_ascii_case("IFCPOLYGONALBOUNDEDHALFSPACE"),
        "{name}: {type_name}"
    );
    operand
}

/// The lowered boundary node of the wall named `name`.
fn boundary_node(model: &Model, name: &str) -> GeometryResult<GeometryNode> {
    let scale = units::resolve(model);
    let mut session = LoweringSession::new(model, &scale);
    let node = lower_half_space_node(&mut session, half_space(model, name), Transform::identity())?;
    let lowered = session.finish(node).expect("session finishes");
    let Some(GeometryNode::SolidOperation(SolidOperation::BoundedHalfSpace { boundary, .. })) =
        lowered.graph.get(lowered.root)
    else {
        panic!("{name}: expected a BoundedHalfSpace operation");
    };
    Ok(lowered
        .graph
        .get(*boundary)
        .expect("the boundary node exists")
        .clone())
}

/// The lowered `Polyline2` boundary of the wall named `name`.
fn boundary(model: &Model, name: &str) -> GeometryResult<Polyline2> {
    match boundary_node(model, name)? {
        GeometryNode::Curve2(Curve2::Polyline(polyline)) => Ok(polyline),
        other => panic!("{name}: expected a Curve2 polyline boundary, got {other:?}"),
    }
}

/// The lowered profile-contour boundary of the wall named `name` (#398).
fn contour(model: &Model, name: &str) -> GeometryResult<Contour> {
    match boundary_node(model, name)? {
        GeometryNode::Profile(Profile::Contour(ContourProfile { outer, holes })) => {
            assert!(holes.is_empty(), "{name}: a boundary has no holes");
            Ok(outer)
        }
        other => panic!("{name}: expected a Profile::Contour boundary, got {other:?}"),
    }
}

fn points(polyline: &Polyline2) -> Vec<[f64; 2]> {
    polyline.points.iter().map(|p| p.to_array()).collect()
}

/// The issue's acceptance test, at the graph: every composite and indexed
/// boundary lowers to exactly the `Polyline2` of its `IfcPolyline` twin.
#[test]
fn each_boundary_lowers_to_its_polyline_twin() {
    let model = parse(&text());
    let twin = boundary(&model, "POLYLINE").expect("the polyline twin lowers");
    assert!(twin.closed);
    assert_eq!(points(&twin), PENTAGON);
    for name in TWINS {
        let lowered = boundary(&model, name).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(lowered, twin, "{name} differs from its polyline twin");
    }
}

fn degenerate(model: &Model, name: &str) -> (EntityId, String, String) {
    match boundary(model, name) {
        Err(GeometryError::Degenerate {
            entity,
            type_name,
            detail,
        }) => (entity, type_name, detail),
        other => panic!("{name}: expected a Degenerate refusal, got {other:?}"),
    }
}

fn unsupported(model: &Model, name: &str) -> (EntityId, String, &'static str) {
    match boundary(model, name) {
        Err(GeometryError::Unsupported {
            entity,
            type_name,
            detail,
        }) => (entity, type_name, detail),
        other => panic!("{name}: expected an Unsupported refusal, got {other:?}"),
    }
}

/// `COMPOSITE`'s last segment starts at `#28`, the end of the gable.
const GABLE_END: &str = "#28=IFCCARTESIANPOINT((1.,0.5));";
const CONTEXT: &str =
    "#3=IFCGEOMETRICREPRESENTATIONCONTEXT($,'Model',3,1.0000000000000001E-05,#2,$);";

/// A gap between segments is refused, naming the segment whose start
/// misses; it is judged within the model's `Precision`, so 1e-6 m joins
/// under the declared 1e-5 m, 1e-4 m does not, and does under 1e-3 m.
#[test]
fn a_gap_between_segments_is_refused_within_the_model_precision() {
    let near = edited(&[(GABLE_END, "#28=IFCCARTESIANPOINT((1.000001,0.5));")]);
    boundary(&near, "COMPOSITE").expect("a 1e-6 m gap is a joint under 1e-5 m");

    let gap = "#28=IFCCARTESIANPOINT((1.0001,0.5));";
    let model = edited(&[(GABLE_END, gap)]);
    let (entity, type_name, detail) = degenerate(&model, "COMPOSITE");
    assert_eq!(
        entity,
        EntityId(30),
        "the polyline that starts off the joint"
    );
    assert_eq!(type_name, "IFCCOMPOSITECURVE");
    assert!(detail.contains("Precision"), "{detail}");
    assert!(detail.contains("never closed"), "{detail}");

    let coarse = edited(&[
        (GABLE_END, gap),
        (
            CONTEXT,
            &CONTEXT.replace("1.0000000000000001E-05", "1.E-03"),
        ),
    ]);
    boundary(&coarse, "COMPOSITE").expect("a 1e-4 m gap is a joint under 1e-3 m");
}

/// `SameSense` is honoured: read forwards, the reversed gable segment
/// leaves a gap at both its ends.
#[test]
fn same_sense_false_is_what_joins_the_reversed_segment() {
    let model = edited(&[(
        "#32=IFCCOMPOSITECURVESEGMENT(.CONTINUOUS.,.F.,#27);",
        "#32=IFCCOMPOSITECURVESEGMENT(.CONTINUOUS.,.T.,#27);",
    )]);
    let (entity, _, detail) = degenerate(&model, "COMPOSITE");
    assert_eq!(entity, EntityId(27), "{detail}");
}

/// An open composite is refused, never closed with an edge the file did not
/// author.
#[test]
fn an_open_composite_is_refused() {
    let model = edited(&[(
        "#34=IFCCOMPOSITECURVE((#31,#32,#33),.F.);",
        "#34=IFCCOMPOSITECURVE((#31,#32),.F.);",
    )]);
    let (_, type_name, detail) = degenerate(&model, "COMPOSITE");
    assert_eq!(type_name, "IFCCOMPOSITECURVE");
    assert!(detail.contains("does not close"), "{detail}");
}

/// The `z == 0` rule of an `IfcPolyline` boundary holds inside a composite:
/// an explicit zero lowers, anything else is off `Position`'s XY plane.
#[test]
fn a_composite_point_off_the_plane_is_refused() {
    let apex = "#25=IFCCARTESIANPOINT((2.,2.));";
    let flat = edited(&[(apex, "#25=IFCCARTESIANPOINT((2.,2.,0.));")]);
    assert_eq!(
        points(&boundary(&flat, "COMPOSITE").expect("z = 0 lowers")),
        PENTAGON
    );

    let model = edited(&[(apex, "#25=IFCCARTESIANPOINT((2.,2.,0.5));")]);
    let (entity, type_name, detail) = degenerate(&model, "COMPOSITE");
    assert_eq!(
        (entity, type_name.as_str()),
        (EntityId(25), "IFCCARTESIANPOINT")
    );
    assert!(detail.contains("BoundaryDim"), "{detail}");
}

/// An arc starting off the previous segment's end is a gap, refused like a
/// polyline's, never closed (#398): here the polyline before the upper arc
/// stops 0.05 m above it. A circle of another radius misses its joints the
/// same way.
#[test]
fn a_gap_at_an_arc_end_is_refused() {
    let model = edited(&[(
        "#192=IFCCARTESIANPOINT((1.,1.7));",
        "#192=IFCCARTESIANPOINT((1.,1.75));",
    )]);
    let (entity, type_name, detail) = degenerate(&model, "COMPOSITE_ARCS");
    assert_eq!(type_name, "IFCCOMPOSITECURVE");
    assert_eq!(entity, EntityId(178), "the arc that starts off the joint");
    assert!(detail.contains("never closed"), "{detail}");

    let model = edited(&[("#177=IFCCIRCLE(#176,1.2);", "#177=IFCCIRCLE(#176,1.25);")]);
    let (_, type_name, detail) = degenerate(&model, "COMPOSITE_ARCS");
    assert_eq!(type_name, "IFCCOMPOSITECURVE");
    assert!(detail.contains("never closed"), "{detail}");
}

/// A boundary circle placed in 3D is refused by name: the boundary lies in
/// `Position`'s XY plane, and the 3D axis is not read as one.
#[test]
fn a_boundary_circle_placed_in_3d_is_refused() {
    let model = edited(&[(
        "#176=IFCAXIS2PLACEMENT2D(#175,$);",
        "#176=IFCAXIS2PLACEMENT3D(#901,$,$);\n#901=IFCCARTESIANPOINT((1.,0.5,0.));",
    )]);
    let (entity, type_name, detail) = unsupported(&model, "COMPOSITE_ARCS");
    assert_eq!(
        (entity, type_name.as_str()),
        (EntityId(176), "IFCAXIS2PLACEMENT3D")
    );
    assert!(detail.contains("half-space"), "{detail}");
}

/// Arc points of an `IfcArcIndex` that coincide define no circle, and are
/// refused as for a profile.
#[test]
fn an_indexed_arc_with_coincident_points_is_refused() {
    let model = edited(&[(
        "(3.,0.5),(2.,1.5),(1.,0.5)),$);",
        "(3.,0.5),(3.,0.5),(1.,0.5)),$);",
    )]);
    let (entity, type_name, detail) = degenerate(&model, "INDEXED_ARC");
    assert_eq!(
        (entity, type_name.as_str()),
        (EntityId(222), "IFCINDEXEDPOLYCURVE")
    );
    assert!(detail.contains("coincident"), "{detail}");
}

/// A trimmed line walked against its parameter (`SameSense` FALSE) closes
/// the boundary the same way: from `(1, 0.5)` down to `(1, -1)`.
#[test]
fn a_reversed_trimmed_line_segment_lowers_to_the_twin() {
    let model = edited(&[
        (
            "#41=IFCCARTESIANPOINT((1.,0.5));",
            "#41=IFCCARTESIANPOINT((1.,-1.));",
        ),
        ("#42=IFCDIRECTION((0.,-1.));", "#42=IFCDIRECTION((0.,1.));"),
        (
            "#47=IFCCOMPOSITECURVESEGMENT(.CONTINUOUS.,.T.,#45);",
            "#47=IFCCOMPOSITECURVESEGMENT(.CONTINUOUS.,.F.,#45);",
        ),
    ]);
    let lowered = boundary(&model, "COMPOSITE_LINE").expect("lowers");
    assert_eq!(
        lowered,
        boundary(&model, "POLYLINE").expect("the twin lowers")
    );
}

/// An IFC4X3 `IfcCurveSegment` member is refused by name.
#[test]
fn a_curve_segment_member_is_refused_by_name() {
    let model = edited(&[(
        "#47=IFCCOMPOSITECURVESEGMENT(.CONTINUOUS.,.T.,#45);",
        "#47=IFCCURVESEGMENT(.CONTINUOUS.,#900,IFCLENGTHMEASURE(0.),IFCLENGTHMEASURE(1.5),#44);\n\
         #900=IFCAXIS2PLACEMENT2D(#41,$);",
    )]);
    let (entity, type_name, _) = unsupported(&model, "COMPOSITE_LINE");
    assert_eq!(
        (entity, type_name.as_str()),
        (EntityId(47), "IFCCURVESEGMENT")
    );
}

/// `INDEXED_SEGMENTS` with its gable edge (3, 4) as an `IfcArcIndex`
/// through a sixth point `middle`.
fn with_arc(middle: &str) -> Model {
    edited(&[
        (
            "(2.,2.),(1.,0.5)),$);",
            &format!("(2.,2.),(1.,0.5),{middle}),$);"),
        ),
        ("IFCLINEINDEX((3,4))", "IFCARCINDEX((3,6,4))"),
    ])
}

/// An `IfcArcIndex` whose middle point lies on the chord is "treated as a
/// polyline segment" (#335's fallback, committed as
/// `INDEXED_COLLINEAR_ARC`) and lowers to the twin; a genuine arc lowers to
/// an exact profile contour (#398), never polygonised.
#[test]
fn an_indexed_arc_is_exact_unless_collinear() {
    let model = parse(&text());
    assert_eq!(
        boundary(&model, "INDEXED_COLLINEAR_ARC").expect("a collinear arc is a polyline segment"),
        boundary(&model, "POLYLINE").expect("the twin lowers")
    );

    // The gable's apex moved onto the circle through (3, 0.5) and (2, 2).
    let model = with_arc("(2.6,1.4)");
    let lowered = contour(&model, "INDEXED_SEGMENTS").expect("an arc boundary lowers");
    let arcs = circles(&lowered);
    assert_eq!(arcs.len(), 1, "{lowered:?}");
    assert_eq!(lowered.segments.len(), 5, "{lowered:?}");
}

/// An open indexed curve is refused, with and without `Segments`.
#[test]
fn an_open_indexed_curve_is_refused() {
    let model = edited(&[(
        "IFCLINEINDEX((3,4)),IFCLINEINDEX((4,5,1))",
        "IFCLINEINDEX((3,4)),IFCLINEINDEX((4,5))",
    )]);
    let (entity, _, detail) = degenerate(&model, "INDEXED_SEGMENTS");
    assert_eq!(entity, EntityId(52));
    assert!(detail.contains("open"), "{detail}");

    let model = edited(&[("(1.,0.5),(1.,-1.)),$)", "(1.,0.5),(1.,-0.9)),$)")]);
    let (entity, _, detail) = degenerate(&model, "INDEXED");
    assert_eq!(entity, EntityId(50));
    assert!(detail.contains("open"), "{detail}");
}

/// A 3D point list lowers when every `z` is 0, as a 3D `IfcPolyline` point
/// does, and is refused, naming the list, when one is off the plane.
#[test]
fn an_indexed_point_off_the_plane_is_refused() {
    let list =
        "#49=IFCCARTESIANPOINTLIST2D(((1.,-1.),(3.,-1.),(3.,0.5),(2.,2.),(1.,0.5),(1.,-1.)),$);";
    let flat = "#49=IFCCARTESIANPOINTLIST3D(((1.,-1.,0.),(3.,-1.,0.),(3.,0.5,0.),(2.,2.,0.),(1.,0.5,0.),(1.,-1.,0.)),$);";
    let model = edited(&[(list, flat)]);
    assert_eq!(
        points(&boundary(&model, "INDEXED").expect("z = 0 lowers")),
        PENTAGON
    );

    let model = edited(&[(list, &flat.replace("(2.,2.,0.)", "(2.,2.,0.25)"))]);
    let (entity, type_name, detail) = degenerate(&model, "INDEXED");
    assert_eq!(
        (entity, type_name.as_str()),
        (EntityId(49), "IFCCARTESIANPOINTLIST3D")
    );
    assert!(detail.contains("point 4"), "{detail}");
    assert!(detail.contains("BoundaryDim"), "{detail}");
}

/// Any other curve family stays a named gap.
#[test]
fn another_boundary_family_is_unsupported() {
    let original = text();
    let record = original
        .lines()
        .find(|line| line.contains("IFCPOLYGONALBOUNDEDHALFSPACE(") && line.ends_with(",#50);"))
        .expect("the INDEXED wall's half-space")
        .to_owned();
    let model = edited(&[(&record, &record.replace(",#50);", ",#45);"))]);
    let (entity, type_name, _) = unsupported(&model, "INDEXED");
    assert_eq!(
        (entity, type_name.as_str()),
        (EntityId(45), "IFCTRIMMEDCURVE")
    );
}

/// Each clipped wall compiles with the reference backend to its polyline
/// twin's volume, `12 - 11/6`.
#[cfg(feature = "compile-reference-backend")]
#[test]
fn each_clipped_wall_compiles_to_its_polyline_twins_volume() {
    use axiolid_core::Tolerance;
    use ifc_geometry::compile::compile_product_mesh;

    let model = parse(&text());
    let volume = |name: &str| {
        let mesh = compile_product_mesh(&model, wall(&model, name), Tolerance::MILLIMETRE)
            .unwrap_or_else(|error| panic!("{name} must compile, got {error}"))
            .expect("the wall has a body");
        let base = mesh.positions[0];
        mesh.indices
            .chunks_exact(3)
            .map(|t| {
                let [a, b, c] = [t[0], t[1], t[2]].map(|i| mesh.positions[i as usize] - base);
                a.dot(b.cross(c)) / 6.0
            })
            .sum::<f64>()
    };
    let twin = volume("POLYLINE");
    assert!(
        (twin - (12.0 - 11.0 / 6.0)).abs() < 1e-6,
        "POLYLINE: {twin}"
    );
    for name in TWINS {
        let clipped = volume(name);
        assert!(
            (clipped - twin).abs() < 1e-9,
            "{name}: {clipped}, its polyline twin {twin}"
        );
    }
}

/// The circles of a contour's arc segments.
fn circles(contour: &Contour) -> Vec<axiolid_curve::Circle2> {
    contour
        .segments
        .iter()
        .filter_map(|segment| match &segment.curve {
            Curve2::Circle(circle) => Some(*circle),
            _ => None,
        })
        .collect()
}

/// A segment's start and end, in traversal order.
fn ends(segment: &axiolid_profile::ProfileSegment) -> (Point2, Point2) {
    let at = |t: f64| match &segment.curve {
        Curve2::Line(l) => l.origin + l.direction * t,
        Curve2::Circle(c) => {
            let (sin, cos) = t.sin_cos();
            c.frame.origin + c.frame.x * (c.radius * cos) + c.frame.y * (c.radius * sin)
        }
        other => panic!("a boundary segment is a line or an arc, got {other:?}"),
    };
    let (a, b) = (at(segment.domain.start), at(segment.domain.end));
    if segment.same_sense {
        (a, b)
    } else {
        (b, a)
    }
}

/// The contour's joints, each segment's start in traversal order, after
/// checking that every segment starts where the one before ends.
fn joints(contour: &Contour) -> Vec<[f64; 2]> {
    let n = contour.segments.len();
    (0..n)
        .map(|i| {
            let (start, _) = ends(&contour.segments[i]);
            let (_, previous) = ends(&contour.segments[(i + n - 1) % n]);
            assert!(
                start.distance(previous) < 1e-9,
                "joint {i}: {start:?} {previous:?}"
            );
            start.to_array()
        })
        .collect()
}

fn close(a: [f64; 2], b: [f64; 2]) -> bool {
    (a[0] - b[0]).abs() < 1e-9 && (a[1] - b[1]).abs() < 1e-9
}

/// #398: a boundary with circular arcs lowers to one exact profile contour,
/// its arcs `Circle2` segments on the authored circle, walked the authored
/// way, and its joints the authored points.
#[test]
fn an_arc_boundary_lowers_to_an_exact_profile_contour() {
    let model = parse(&text());

    let composite = contour(&model, "COMPOSITE_ARCS").expect("COMPOSITE_ARCS lowers");
    assert_eq!(composite.segments.len(), 6, "{composite:?}");
    let arcs = circles(&composite);
    assert_eq!(arcs.len(), 2, "{composite:?}");
    for arc in &arcs {
        assert_eq!(arc.radius, 1.2);
        assert_eq!(arc.frame.origin.to_array(), [1.0, 0.5]);
    }
    let expected = [
        [1.96, -0.22],
        [3.5, -0.22],
        [3.5, 2.5],
        [1.0, 2.5],
        [1.0, 1.7],
        [2.2, 0.5],
    ];
    let found = joints(&composite);
    assert!(
        found.iter().zip(expected).all(|(a, b)| close(*a, b)),
        "{found:?}"
    );
    // The bite runs clockwise: both arcs against their increasing angle.
    assert!(composite.segments[4..].iter().all(|s| !s.same_sense));

    let indexed = contour(&model, "INDEXED_ARC").expect("INDEXED_ARC lowers");
    assert_eq!(indexed.segments.len(), 4, "{indexed:?}");
    let arcs = circles(&indexed);
    assert_eq!(arcs.len(), 1, "{indexed:?}");
    assert!((arcs[0].radius - 1.0).abs() < 1e-12, "{:?}", arcs[0]);
    assert!(close(arcs[0].frame.origin.to_array(), [2.0, 0.5]));
    let expected = [[1.0, -1.0], [3.0, -1.0], [3.0, 0.5], [1.0, 0.5]];
    let found = joints(&indexed);
    assert!(
        found.iter().zip(expected).all(|(a, b)| close(*a, b)),
        "{found:?}"
    );
    // The half circle over the top runs anticlockwise.
    assert!(indexed.segments[2].same_sense);
}

/// The closed-form volume each arc-bounded wall keeps: `12` less the part
/// of its plan inside the boundary, times the clipped height 1.
#[cfg(feature = "compile-reference-backend")]
fn arc_wall_volume(name: &str) -> f64 {
    // `integral of sqrt(r^2 - u^2) du` from 0 to `u`.
    let half_disk =
        |r: f64, u: f64| 0.5 * u * (r * r - u * u).sqrt() + 0.5 * r * r * (u / r).asin();
    let removed = match name {
        // [1, 3.5] x [0, 1] less the disk of radius 1.2 about (1, 0.5).
        "COMPOSITE_ARCS" => 2.5 - 2.0 * half_disk(1.2, 0.5),
        // [1, 3] x [0, 0.5] and the half disk of radius 1 about (2, 0.5)
        // below y = 1: 1 + sqrt(3)/4 + pi/6.
        "INDEXED_ARC" => 1.0 + 2.0 * half_disk(1.0, 0.5),
        _ => unreachable!(),
    };
    12.0 - removed
}

#[cfg(feature = "compile-reference-backend")]
const ARC_WALLS: [&str; 2] = ["COMPOSITE_ARCS", "INDEXED_ARC"];

/// The mesh compiler clips each arc-bounded wall within its chord budget of
/// the closed form: a chord moves the cut by at most the budget, over the
/// wall's 1 m thickness and the 1 m clipped height.
#[cfg(feature = "compile-reference-backend")]
#[test]
fn each_arc_bounded_wall_meshes_to_its_closed_form_volume() {
    use axiolid_core::Tolerance;
    use ifc_geometry::compile::compile_product_mesh;

    let model = parse(&text());
    for name in ARC_WALLS {
        let mesh = compile_product_mesh(&model, wall(&model, name), Tolerance::MILLIMETRE)
            .unwrap_or_else(|error| panic!("{name} must compile, got {error}"))
            .expect("the wall has a body");
        let base = mesh.positions[0];
        let measured = mesh
            .indices
            .chunks_exact(3)
            .map(|t| {
                let [a, b, c] = [t[0], t[1], t[2]].map(|i| mesh.positions[i as usize] - base);
                a.dot(b.cross(c)) / 6.0
            })
            .sum::<f64>();
        let expected = arc_wall_volume(name);
        assert!(
            (measured - expected).abs() <= 1e-3,
            "{name}: mesh volume {measured}, closed form {expected}"
        );
    }
}

/// The exact compiler clips each arc-bounded wall by a right cylinder per
/// arc, to the closed-form volume.
#[cfg(feature = "compile-reference-backend")]
#[test]
fn each_arc_bounded_wall_compiles_exactly_to_its_closed_form_volume() {
    use axiolid_contracts::ExecutionOptions;
    use axiolid_core::Tolerance;
    use axiolid_mesh_compile::ReferenceExactCompiler;
    use ifc_geometry::lower::{lower_product_representation, RepresentationPurpose};

    let model = parse(&text());
    let scale = units::resolve(&model);
    for name in ARC_WALLS {
        let mut session = LoweringSession::new(&model, &scale);
        let root = lower_product_representation(
            &mut session,
            wall(&model, name),
            RepresentationPurpose::Body,
        )
        .unwrap_or_else(|error| panic!("{name} must lower, got {error}"))
        .expect("the wall has a body");
        let lowered = session.finish(root).expect("session finishes");
        let (body, _) = ReferenceExactCompiler::new()
            .compile_exact_with_report(
                &lowered.graph,
                lowered.root,
                &ExecutionOptions::new(Tolerance::MILLIMETRE),
            )
            .unwrap_or_else(|error| panic!("{name} must compile exactly, got {error:?}"));
        let measured = axiolid_measure::exact_properties(&body, Tolerance::MILLIMETRE)
            .expect("measurable")
            .signed_volume;
        let expected = arc_wall_volume(name);
        assert!(
            (measured - expected).abs() <= 1e-9 * expected,
            "{name}: exact volume {measured}, closed form {expected}"
        );
    }
}

/// A boundary whose arc crosses one of its own edges does not bound a
/// region. The readers check closure, not crossing; the kernel refuses it
/// by name in both compilers (axiolid/kernel#277), and the refusal reaches
/// the caller as the product's.
#[cfg(feature = "compile-reference-backend")]
#[test]
fn a_self_crossing_arc_boundary_is_refused_by_the_compiler() {
    use axiolid_core::Tolerance;
    use ifc_geometry::compile::compile_product_mesh;

    // The arc's middle point moved below the bottom edge: the arc from
    // (3, 0.5) through (2, -1.5) to (1, 0.5) crosses y = -1 twice.
    let model = edited(&[(
        "(3.,0.5),(2.,1.5),(1.,0.5)),$);",
        "(3.,0.5),(2.,-1.5),(1.,0.5)),$);",
    )]);
    contour(&model, "INDEXED_ARC").expect("the reader admits it");
    let product = wall(&model, "INDEXED_ARC");
    match compile_product_mesh(&model, product, Tolerance::MILLIMETRE) {
        Err(GeometryError::CompilationRefused { entity, reason }) => {
            assert_eq!(entity, product);
            assert!(reason.contains("crosses or touches itself"), "{reason}");
        }
        other => panic!("expected a CompilationRefused, got {other:?}"),
    }
}

fn boundary_type_violations(model: &Model) -> Vec<(EntityId, String)> {
    ifc_geometry::rules::validate_model(model)
        .into_iter()
        .filter(|violation| violation.rule == "BoundaryType")
        .map(|violation| (violation.entity, violation.detail))
        .collect()
}

/// `BoundaryType` is the declared release's (#397). IFC4X3 ADD2 admits
/// `IfcIndexedPolyCurve`, so the fixture has no violation; declared IFC4
/// (ADD2 TC1 admits only `IfcPolyline` and `IfcCompositeCurve`), the same
/// file is flagged at exactly the half-spaces with an indexed boundary.
#[test]
fn boundary_type_is_read_in_the_declared_release() {
    let model = parse(&text());
    assert_eq!(model.header().schema_token(), Some("IFC4X3_ADD2"));
    assert_eq!(boundary_type_violations(&model), vec![]);

    let ifc4 = edited(&[("FILE_SCHEMA(('IFC4X3_ADD2'));", "FILE_SCHEMA(('IFC4'));")]);
    let flagged = boundary_type_violations(&ifc4);
    let indexed: Vec<EntityId> = [
        "INDEXED",
        "INDEXED_SEGMENTS",
        "INDEXED_COLLINEAR_ARC",
        "INDEXED_ARC",
    ]
    .into_iter()
    .map(|name| half_space(&ifc4, name))
    .collect();
    assert_eq!(
        flagged
            .iter()
            .map(|(entity, _)| *entity)
            .collect::<Vec<_>>(),
        indexed,
        "{flagged:?}"
    );
    for (_, message) in &flagged {
        assert!(message.contains("IFC4_ADD2_TC1"), "{message}");
        assert!(message.contains("IFCINDEXEDPOLYCURVE"), "{message}");
    }
}
