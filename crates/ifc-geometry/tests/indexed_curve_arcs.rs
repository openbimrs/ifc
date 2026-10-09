//! `IfcArcIndex` in the general curve lowering (#396).
//!
//! The fixture (`tools/gen_lowering_fixtures.py`, `indexed_curve_arcs.ifc`,
//! IFC4, declared `Precision` 1e-5 m) has four proxies whose `Axis`
//! representation is one `IfcIndexedPolyCurve`, named as their case:
//! `COLLINEAR_ARC`, `OUT_AND_BACK_ARC`, `NEAR_COLLINEAR_ARC` and `ARC`.
//!
//! IFC4 ADD2 TC1 and IFC4X3 ADD2: "The three points shall not be co-linear.
//! In case that this informal proposition is not maintained, the arc segment
//! shall be treated as a polyline segment." A collinear arc therefore lowers
//! as a straight `Polyline3` segment, judged within the model's `Precision`
//! by the helper the profile and half-space boundary reader shares; two
//! coincident points stay `Degenerate`.

#![cfg(feature = "lowering")]

use std::path::PathBuf;

use axiolid_curve::Curve3;
use axiolid_model::{CurveRelation, GeometryNode};
use ifc_geometry::lower::{lower_representation_item, LoweredGeometry, LoweringSession};
use ifc_geometry::transform::Transform;
use ifc_geometry::{units, GeometryError, GeometryResult};
use ifc_model::{Codec, EntityId, Model, Value};
use ifc_step::StepCodec;

fn text() -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../test/fixtures/synthetic-lowering/indexed_curve_arcs.ifc");
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

fn first_ref(model: &Model, id: EntityId, slot: usize) -> EntityId {
    match model.get(id).expect("entity exists").attributes.get(slot) {
        Some(Value::Ref(target)) => *target,
        Some(Value::List(items)) => match items.first() {
            Some(Value::Ref(target)) => *target,
            other => panic!("{id:?} slot {slot}: expected a reference, got {other:?}"),
        },
        other => panic!("{id:?} slot {slot}: expected a reference, got {other:?}"),
    }
}

/// The `Axis` curve of the proxy named `name`.
fn axis_curve(model: &Model, name: &str) -> EntityId {
    let proxies: Vec<EntityId> = model
        .of_type("IFCBUILDINGELEMENTPROXY")
        .filter(|(_, entity)| {
            matches!(entity.attributes.get(2), Some(Value::Text(text)) if &**text == name)
        })
        .map(|(id, _)| id)
        .collect();
    assert_eq!(proxies.len(), 1, "exactly one proxy named {name}");
    let shape = first_ref(model, proxies[0], 6);
    let representation = first_ref(model, shape, 2);
    first_ref(model, representation, 3)
}

fn lower(model: &Model, name: &str) -> GeometryResult<LoweredGeometry> {
    let scale = units::resolve(model);
    let mut session = LoweringSession::new(model, &scale);
    let root =
        lower_representation_item(&mut session, axis_curve(model, name), Transform::identity())?;
    Ok(session.finish(root).expect("session finishes"))
}

/// What each segment of the lowered composite is: the points of a straight
/// `Polyline3`, or `None` for a trimmed circle.
fn segments(lowered: &LoweredGeometry) -> Vec<Option<Vec<[f64; 3]>>> {
    let Some(GeometryNode::CurveRelation(CurveRelation::Composite { segments })) =
        lowered.graph.get(lowered.root)
    else {
        panic!("expected a composite indexed curve");
    };
    segments
        .iter()
        .map(|segment| match lowered.graph.get(segment.curve) {
            Some(GeometryNode::Curve3(Curve3::Polyline(polyline))) => {
                assert!(!polyline.closed);
                Some(polyline.points.iter().map(|p| p.to_array()).collect())
            }
            Some(GeometryNode::CurveRelation(CurveRelation::Trimmed { basis, .. })) => {
                assert!(matches!(
                    lowered.graph.get(*basis),
                    Some(GeometryNode::Curve3(Curve3::Circle(_)))
                ));
                None
            }
            other => panic!("unexpected segment {other:?}"),
        })
        .collect()
}

/// A collinear arc whose middle point lies between the others is the one
/// straight edge from its start to its end.
#[test]
fn a_collinear_arc_lowers_as_its_straight_edge() {
    let lowered = lower(&parse(&text()), "COLLINEAR_ARC").expect("lowers");
    assert_eq!(
        segments(&lowered),
        vec![
            Some(vec![[0.0, 0.0, 0.0], [2.0, 0.0, 0.0]]),
            Some(vec![[2.0, 0.0, 0.0], [4.0, 0.0, 0.0]]),
            Some(vec![[4.0, 0.0, 0.0], [4.0, 2.0, 0.0]]),
        ]
    );
}

/// A collinear arc whose middle point lies beyond its end runs out to the
/// middle point and back: the polyline start -> mid -> end.
#[test]
fn an_out_and_back_collinear_arc_keeps_its_middle_point() {
    let lowered = lower(&parse(&text()), "OUT_AND_BACK_ARC").expect("lowers");
    assert_eq!(
        segments(&lowered),
        vec![
            Some(vec![[0.0, 0.0, 1.0], [4.0, 0.0, 1.0], [2.0, 0.0, 1.0]]),
            Some(vec![[2.0, 0.0, 1.0], [2.0, 2.0, 1.0]]),
        ]
    );
}

const CONTEXT: &str =
    "#3=IFCGEOMETRICREPRESENTATIONCONTEXT($,'Model',3,1.0000000000000001E-05,#2,$);";

/// Collinearity is judged within the model's `Precision`: a middle point
/// 4e-6 m off the chord is on it under the declared 1e-5 m, and makes an
/// arc under 1e-6 m.
#[test]
fn collinearity_uses_the_model_precision() {
    let lowered = lower(&parse(&text()), "NEAR_COLLINEAR_ARC").expect("lowers");
    assert_eq!(
        segments(&lowered),
        vec![Some(vec![[0.0, 0.0, 2.0], [2.0, 0.0, 2.0]])]
    );

    let fine = edited(&[(
        CONTEXT,
        &CONTEXT.replace("1.0000000000000001E-05", "1.E-06"),
    )]);
    let lowered = lower(&fine, "NEAR_COLLINEAR_ARC").expect("lowers");
    assert_eq!(segments(&lowered), vec![None]);
}

/// A genuine arc stays the exact trimmed circle.
#[test]
fn a_genuine_arc_stays_a_circle() {
    let lowered = lower(&parse(&text()), "ARC").expect("lowers");
    assert_eq!(segments(&lowered), vec![None]);
}

/// Two points that coincide within `Precision` define neither a circle nor
/// a polyline segment: `Degenerate`, naming the curve. A middle point 4e-6 m
/// from the start coincides with it under 1e-5 m, and does not under 1e-6 m.
#[test]
fn coincident_points_stay_degenerate() {
    let list = "#21=IFCCARTESIANPOINTLIST3D(((0.,0.,3.),(1.,1.,3.),(2.,0.,3.)));";
    let near = "#21=IFCCARTESIANPOINTLIST3D(((0.,0.,3.),(0.,0.000004,3.),(2.,0.,3.)));";
    let model = edited(&[(list, near)]);
    match lower(&model, "ARC") {
        Err(GeometryError::Degenerate {
            entity,
            type_name,
            detail,
        }) => {
            assert_eq!(
                (entity, type_name.as_str()),
                (EntityId(22), "IFCINDEXEDPOLYCURVE")
            );
            assert!(detail.contains("coincide"), "{detail}");
        }
        other => panic!("expected a Degenerate refusal, got {other:?}"),
    }

    let fine = edited(&[
        (list, near),
        (
            CONTEXT,
            &CONTEXT.replace("1.0000000000000001E-05", "1.E-06"),
        ),
    ]);
    let lowered = lower(&fine, "ARC").expect("distinct under 1e-6 m");
    assert_eq!(segments(&lowered), vec![None]);
}
