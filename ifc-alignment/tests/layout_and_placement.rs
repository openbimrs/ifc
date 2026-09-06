use axiolid_model::{CurveRelation, GeometryNode, Transition};
use ifc_alignment::{
    lower_horizontal_layout, resolve_linear_placement, station_equations, AlignmentUnits,
    CantLayout, CurveMeasure,
};
use ifc_model::{Codec, EntityId};
use ifc_step::StepCodec;

fn fixture() -> ifc_model::Model {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../test/fixtures/synthetic-surfaces/synthetic_alignment_layout.ifc");
    StepCodec.read_path(&path).expect("fixture parses")
}

fn units() -> AlignmentUnits {
    AlignmentUnits {
        length_to_metres: 1.0,
        angle_to_radians: 1.0,
    }
}

#[test]
fn horizontal_layout_assembles_a_continuous_composite_curve() {
    let model = fixture();
    let lowered =
        lower_horizontal_layout(&model, EntityId(121), units()).expect("horizontal layout lowers");
    assert_eq!(lowered.sources, vec![EntityId(101), EntityId(103)]);
    let Some(GeometryNode::CurveRelation(CurveRelation::Composite { segments })) =
        lowered.graph.get(lowered.root)
    else {
        panic!("expected a composite curve at the root");
    };
    assert_eq!(segments.len(), 2);
    assert_eq!(segments[0].transition, Transition::Discontinuous);
    assert_eq!(segments[1].transition, Transition::Continuous);
    for segment in segments {
        assert!(matches!(
            lowered.graph.get(segment.curve),
            Some(GeometryNode::CurveRelation(CurveRelation::Trimmed { .. }))
        ));
    }
}

#[test]
fn cant_layout_resolves_the_full_profile_and_queries_by_distance() {
    let model = fixture();
    let layout = CantLayout::resolve(&model, EntityId(321), units()).expect("cant layout resolves");
    assert_eq!(layout.segments().len(), 2);
    assert_eq!(layout.length(), 10.0);
    assert_eq!(layout.rail_head_distance, 1.5);

    let at_start = layout.cant_at_distance(0.0).expect("cant at start");
    assert_eq!(at_start.left, 0.0);
    assert_eq!(at_start.right, 0.0);

    let at_seam = layout
        .cant_at_distance(5.0)
        .expect("cant at the segment seam");
    assert_eq!(at_seam.left, 0.05);
    assert_eq!(at_seam.right, -0.05);

    let at_end = layout.cant_at_distance(10.0).expect("cant at end");
    assert_eq!(at_end.left, 0.10);
    assert_eq!(at_end.right, -0.10);

    assert!(layout.cant_at_distance(10.5).is_err());
}

#[test]
fn linear_placement_resolves_the_point_by_distance_expression() {
    let model = fixture();
    let placement = resolve_linear_placement(&model, EntityId(514), units())
        .expect("linear placement resolves");
    let point = placement.relative_placement;
    assert_eq!(point.distance_along, CurveMeasure::Length(5.0));
    assert_eq!(point.offset_lateral, Some(0.5));
    assert_eq!(point.offset_vertical, None);
    assert_eq!(point.offset_longitudinal, None);
}

#[test]
fn station_equations_resolve_from_pset_stationing_and_the_linear_placement() {
    let model = fixture();
    let equations = station_equations(&model, units()).expect("station equations resolve");
    assert_eq!(equations.len(), 1);
    let equation = &equations[0];
    assert_eq!(equation.referent, EntityId(520));
    assert_eq!(equation.distance_along, 5.0);
    assert_eq!(equation.station, 5.0);
    assert_eq!(equation.incoming_station, Some(5.0));
    assert!(equation.has_increasing_station);
}

#[test]
fn a_clothoid_segment_inside_a_layout_is_a_typed_refusal_not_an_approximation() {
    use ifc_alignment::AlignmentError;
    use ifc_model::{Entity, Model, Value};
    use std::sync::Arc;

    // A minimal, otherwise-valid IFC4X3 horizontal layout whose sole segment
    // is CLOTHOID: no closed-form Cartesian position exists for it (Fresnel
    // integral), so this must refuse rather than approximate, both alone
    // and inside a multi-segment composite.
    let mut model = Model::new();
    *model.header_mut() = ifc_model::Header {
        schema: vec!["IFC4X3_ADD2".to_string()],
        ..ifc_model::Header::default()
    };
    model.insert(
        EntityId(1),
        Entity::new(
            "IFCCARTESIANPOINT",
            vec![Value::List(vec![Value::Real(0.0), Value::Real(0.0)])],
        ),
    );
    model.insert(
        EntityId(2),
        Entity::new(
            "IFCALIGNMENTHORIZONTALSEGMENT",
            vec![
                Value::Null,
                Value::Null,
                Value::Ref(EntityId(1)),
                Value::Real(0.0),
                Value::Real(100.0),
                Value::Real(50.0),
                Value::Real(20.0),
                Value::Null,
                Value::Enum(Arc::from("CLOTHOID")),
            ],
        ),
    );
    model.insert(
        EntityId(3),
        Entity::new(
            "IFCALIGNMENTSEGMENT",
            vec![
                Value::Text(Arc::from("seg")),
                Value::Null,
                Value::Null,
                Value::Null,
                Value::Null,
                Value::Null,
                Value::Null,
                Value::Ref(EntityId(2)),
            ],
        ),
    );
    model.insert(
        EntityId(4),
        Entity::new(
            "IFCALIGNMENTHORIZONTAL",
            vec![
                Value::Text(Arc::from("horiz")),
                Value::Null,
                Value::Null,
                Value::Null,
                Value::Null,
                Value::Null,
                Value::Null,
            ],
        ),
    );
    model.insert(
        EntityId(5),
        Entity::new(
            "IFCRELNESTS",
            vec![
                Value::Text(Arc::from("nest")),
                Value::Null,
                Value::Null,
                Value::Null,
                Value::Ref(EntityId(4)),
                Value::List(vec![Value::Ref(EntityId(3))]),
            ],
        ),
    );

    assert!(matches!(
        lower_horizontal_layout(&model, EntityId(4), units()),
        Err(AlignmentError::Unsupported { .. })
    ));
    assert!(matches!(
        ifc_alignment::lower_horizontal_segment(&model, EntityId(2), units()),
        Err(AlignmentError::Unsupported { .. })
    ));
}
