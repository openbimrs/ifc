//! `IfcCompositeCurve` and `IfcIndexedPolyCurve` half-space boundaries (#393).
//!
//! The fixture (`tools/gen_lowering_fixtures.py`,
//! `halfspace_boundaries_ifc4x3.ifc`, IFC4X3_ADD2) has six walls, each a
//! 4 x 1 x 3 extrusion over `[0, 4] x [0, 1]` cut above `z = 2` by an
//! `IfcPolygonalBoundedHalfSpace` whose boundary states the anticlockwise
//! pentagon `(1,-1) (3,-1) (3,0.5) (2,2) (1,0.5)`. The walls are named by
//! how the boundary is authored:
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
//! `12 - 11/6`. The genuine arc and the refusals edit one record in memory:
//! every item of a committed fixture must lower
//! (`tests/lower_dispatch_corpus.rs`), and a genuine arc does not lower as
//! a half-space boundary.
//!
//! `BoundaryType` is read in the file's release (#397): IFC4X3 ADD2 admits
//! the indexed boundaries, IFC4 ADD2 TC1 does not.

#![cfg(feature = "lowering")]

use std::path::PathBuf;

use axiolid_curve::{Curve2, Polyline2};
use axiolid_model::{GeometryNode, SolidOperation};
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

/// The lowered boundary of the wall named `name`.
fn boundary(model: &Model, name: &str) -> GeometryResult<Polyline2> {
    let scale = units::resolve(model);
    let mut session = LoweringSession::new(model, &scale);
    let node = lower_half_space_node(&mut session, half_space(model, name), Transform::identity())?;
    let lowered = session.finish(node).expect("session finishes");
    let Some(GeometryNode::SolidOperation(SolidOperation::BoundedHalfSpace { boundary, .. })) =
        lowered.graph.get(lowered.root)
    else {
        panic!("{name}: expected a BoundedHalfSpace operation");
    };
    match lowered.graph.get(*boundary) {
        Some(GeometryNode::Curve2(Curve2::Polyline(polyline))) => Ok(polyline.clone()),
        other => panic!("{name}: expected a Curve2 polyline boundary, got {other:?}"),
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

/// A trimmed `IfcCircle` is refused by name: the kernel's bounded half-space
/// takes no curved boundary, and the arc is never chorded.
#[test]
fn a_trimmed_circle_segment_is_refused_not_polygonised() {
    let model = edited(&[(
        "#44=IFCLINE(#41,#43);",
        "#44=IFCCIRCLE(#900,0.75);\n#900=IFCAXIS2PLACEMENT2D(#41,$);",
    )]);
    let (entity, type_name, detail) = unsupported(&model, "COMPOSITE_LINE");
    assert_eq!(
        (entity, type_name.as_str()),
        (EntityId(45), "IFCTRIMMEDCURVE")
    );
    assert!(detail.contains("polygonised"), "{detail}");
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
/// `INDEXED_COLLINEAR_ARC`) and lowers to the twin; a genuine arc is
/// refused by name, never polygonised.
#[test]
fn an_indexed_arc_is_refused_unless_collinear() {
    let model = parse(&text());
    assert_eq!(
        boundary(&model, "INDEXED_COLLINEAR_ARC").expect("a collinear arc is a polyline segment"),
        boundary(&model, "POLYLINE").expect("the twin lowers")
    );

    let model = with_arc("(2.6,1.4)");
    let (entity, type_name, detail) = unsupported(&model, "INDEXED_SEGMENTS");
    assert_eq!(
        (entity, type_name.as_str()),
        (EntityId(52), "IFCINDEXEDPOLYCURVE")
    );
    assert!(detail.contains("polygonised"), "{detail}");
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
    let indexed: Vec<EntityId> = ["INDEXED", "INDEXED_SEGMENTS", "INDEXED_COLLINEAR_ARC"]
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
