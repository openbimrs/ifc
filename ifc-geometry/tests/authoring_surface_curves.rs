//! Curves on surfaces, and the segment forms.
//!
//! Two claims are load-bearing here: a boundary curve must say it is
//! closed, and a seam or intersection curve must carry exactly two
//! pcurves.

use ifc_geometry::authoring::{
    axis2_placement_3d, cartesian_point, circle, composite_curve_on_surface, curve_segment,
    gradient_curve, pcurve, plane, polyline, reparametrised_composite_curve_segment,
    spherical_surface, surface_curve, CurveMeasure, OnSurfaceKind, SurfaceCurveKind,
    SurfaceCurveRepresentation,
};
use ifc_geometry::curve::TransitionCode;
use ifc_model::{EntityId, Model, Transaction, Value};

/// A placement at the origin.
fn origin(tx: &mut Transaction) -> EntityId {
    let point = cartesian_point(tx, &[0.0, 0.0, 0.0]).expect("point");
    axis2_placement_3d(tx, point, None, None)
}

/// A boundary curve states closure; a plain one leaves it unknown.
///
/// `IsClosed` makes `ClosedCurve` true on the two boundary forms. The
/// crate writes UNKNOWN everywhere else, because closure needs an
/// evaluator -- so this is the one place the value is not a guess but
/// a schema requirement.
#[test]
fn a_boundary_curve_must_claim_closure() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let at = origin(&mut tx);
    let arc = circle(&mut tx, at, 1.0).expect("arc");
    let segment = ifc_geometry::authoring::composite_curve_segment(
        &mut tx,
        TransitionCode::Continuous,
        true,
        arc,
    );

    let plain = composite_curve_on_surface(&mut tx, OnSurfaceKind::Composite, &[segment])
        .expect("composite");
    let boundary =
        composite_curve_on_surface(&mut tx, OnSurfaceKind::Boundary, &[segment]).expect("boundary");
    let outer = composite_curve_on_surface(&mut tx, OnSurfaceKind::OuterBoundary, &[segment])
        .expect("outer boundary");
    assert!(
        composite_curve_on_surface(&mut tx, OnSurfaceKind::Boundary, &[]).is_err(),
        "LIST [1:?] needs a segment"
    );

    let mut model = model;
    tx.commit(&mut model).expect("commit");

    assert_eq!(
        model.get(plain).expect("plain").attributes[1],
        Value::LogicalUnknown,
        "closure was not evaluated, so it is not claimed"
    );
    for (id, label) in [
        (boundary, "IFCBOUNDARYCURVE"),
        (outer, "IFCOUTERBOUNDARYCURVE"),
    ] {
        let entity = model.get(id).expect("boundary");
        assert_eq!(entity.type_name.as_ref(), label);
        assert_eq!(
            entity.attributes[1],
            Value::Bool(true),
            "{label} carries IsClosed, so UNKNOWN would not conform"
        );
    }
}

/// Seam and intersection curves need a pcurve on each surface.
#[test]
fn seam_and_intersection_curves_need_two_pcurves() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let at = origin(&mut tx);
    let flat = plane(&mut tx, at);
    let ball = spherical_surface(&mut tx, at, 1.0).expect("sphere");
    let arc = circle(&mut tx, at, 1.0).expect("arc");

    let on_plane = pcurve(&mut tx, flat, arc);
    let on_ball = pcurve(&mut tx, ball, arc);

    // A plain surface curve may carry just one.
    let single = surface_curve(
        &mut tx,
        SurfaceCurveKind::Plain,
        arc,
        &[on_plane],
        SurfaceCurveRepresentation::Curve3D,
    )
    .expect("one pcurve is legal on a plain surface curve");

    for kind in [SurfaceCurveKind::Intersection, SurfaceCurveKind::Seam] {
        assert!(
            surface_curve(
                &mut tx,
                kind,
                arc,
                &[on_plane],
                SurfaceCurveRepresentation::Curve3D
            )
            .is_err(),
            "TwoPCurves: one is not enough"
        );
    }
    assert!(
        surface_curve(
            &mut tx,
            SurfaceCurveKind::Plain,
            arc,
            &[],
            SurfaceCurveRepresentation::Curve3D,
        )
        .is_err(),
        "LIST [1:2] has a lower bound of one"
    );
    assert!(
        surface_curve(
            &mut tx,
            SurfaceCurveKind::Plain,
            arc,
            &[on_plane, on_ball, on_plane],
            SurfaceCurveRepresentation::Curve3D,
        )
        .is_err(),
        "and an upper bound of two"
    );

    let crossing = surface_curve(
        &mut tx,
        SurfaceCurveKind::Intersection,
        arc,
        &[on_plane, on_ball],
        SurfaceCurveRepresentation::PCurveS1,
    )
    .expect("intersection");

    let mut model = model;
    tx.commit(&mut model).expect("commit");

    let entity = model.get(single).expect("single");
    assert_eq!(entity.type_name.as_ref(), "IFCSURFACECURVE");
    assert_eq!(entity.attributes[2], Value::Enum("CURVE3D".into()));

    let entity = model.get(crossing).expect("crossing");
    assert_eq!(entity.type_name.as_ref(), "IFCINTERSECTIONCURVE");
    assert_eq!(entity.attributes[0], Value::Ref(arc), "Curve3D");
    assert_eq!(
        entity.attributes[1],
        Value::List(vec![Value::Ref(on_plane), Value::Ref(on_ball)]),
        "the pcurves keep their order"
    );
    assert_eq!(entity.attributes[2], Value::Enum("PCURVE_S1".into()));

    // A pcurve pairs a surface with a curve in its parameter space.
    let entity = model.get(on_plane).expect("pcurve");
    assert_eq!(entity.attributes[0], Value::Ref(flat), "BasisSurface");
    assert_eq!(entity.attributes[1], Value::Ref(arc), "ReferenceCurve");
}

/// A curve measure says whether it is a length or a parameter.
///
/// `IfcCurveMeasureSelect` admits both and they are not
/// interchangeable, so the measure type is the only thing telling a
/// consumer which parameterisation to use.
#[test]
fn a_curve_measure_states_which_kind_it_is() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let at = origin(&mut tx);
    let a = cartesian_point(&mut tx, &[0.0, 0.0, 0.0]).expect("a");
    let b = cartesian_point(&mut tx, &[10.0, 0.0, 0.0]).expect("b");
    let parent = polyline(&mut tx, &[a, b]).expect("parent");

    let by_length = curve_segment(
        &mut tx,
        TransitionCode::Continuous,
        at,
        CurveMeasure::Length(0.0),
        CurveMeasure::Length(4.0),
        parent,
    )
    .expect("length segment");

    let by_parameter = curve_segment(
        &mut tx,
        TransitionCode::Continuous,
        at,
        CurveMeasure::Parameter(0.0),
        CurveMeasure::Parameter(1.0),
        parent,
    )
    .expect("parameter segment");

    assert!(
        curve_segment(
            &mut tx,
            TransitionCode::Continuous,
            at,
            CurveMeasure::Length(-1.0),
            CurveMeasure::Length(4.0),
            parent,
        )
        .is_err(),
        "IfcNonNegativeLengthMeasure: a negative distance along a curve"
    );
    assert!(
        curve_segment(
            &mut tx,
            TransitionCode::Continuous,
            at,
            CurveMeasure::Parameter(f64::NAN),
            CurveMeasure::Parameter(1.0),
            parent,
        )
        .is_err(),
        "a non-finite parameter"
    );

    let mut model = model;
    tx.commit(&mut model).expect("commit");

    let entity = model.get(by_length).expect("length");
    match &entity.attributes[3] {
        Value::Typed { type_name, .. } => {
            assert_eq!(type_name.as_ref(), "IFCNONNEGATIVELENGTHMEASURE");
        }
        other => panic!("SegmentLength lost its measure: {other:?}"),
    }

    let entity = model.get(by_parameter).expect("parameter");
    match &entity.attributes[3] {
        Value::Typed { type_name, .. } => {
            assert_eq!(
                type_name.as_ref(),
                "IFCPARAMETERVALUE",
                "a parameter is not a length, and says so"
            );
        }
        other => panic!("SegmentLength lost its measure: {other:?}"),
    }
}

/// Reparametrised segments and gradient curves.
#[test]
fn reparametrised_segments_and_gradient_curves_round_trip() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let at = origin(&mut tx);
    let arc = circle(&mut tx, at, 1.0).expect("arc");
    let a = cartesian_point(&mut tx, &[0.0, 0.0, 0.0]).expect("a");
    let b = cartesian_point(&mut tx, &[10.0, 0.0, 0.0]).expect("b");
    let base = polyline(&mut tx, &[a, b]).expect("base");

    let segment = reparametrised_composite_curve_segment(
        &mut tx,
        TransitionCode::ContSameGradient,
        true,
        arc,
        2.5,
    )
    .expect("reparametrised");
    assert!(
        reparametrised_composite_curve_segment(
            &mut tx,
            TransitionCode::Continuous,
            true,
            arc,
            0.0,
        )
        .is_err(),
        "a zero parameter length covers nothing"
    );

    let gradient = gradient_curve(&mut tx, &[segment], base, None).expect("gradient");
    assert!(
        gradient_curve(&mut tx, &[], base, None).is_err(),
        "LIST [1:?] needs a segment"
    );

    let mut model = model;
    tx.commit(&mut model).expect("commit");

    let entity = model.get(segment).expect("segment");
    assert_eq!(entity.attributes.len(), 4, "ParamLength is slot 3");
    assert_eq!(entity.attributes[2], Value::Ref(arc), "ParentCurve");

    let entity = model.get(gradient).expect("gradient");
    assert_eq!(entity.attributes[2], Value::Ref(base), "BaseCurve");
    assert_eq!(entity.attributes[3], Value::Null, "no end point stated");
}

/// `SegmentStart` and `SegmentLength` are distinct slots.
///
/// The earlier test starts every segment at zero, so a transposition
/// is invisible there. Here the two differ and their slots are
/// asserted directly.
#[test]
fn a_curve_segment_keeps_start_and_length_apart() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let at = origin(&mut tx);
    let a = cartesian_point(&mut tx, &[0.0, 0.0, 0.0]).expect("a");
    let b = cartesian_point(&mut tx, &[10.0, 0.0, 0.0]).expect("b");
    let parent = polyline(&mut tx, &[a, b]).expect("parent");

    // Start 2.0, run 7.0: a swap would move the segment and resize it.
    let id = curve_segment(
        &mut tx,
        TransitionCode::Continuous,
        at,
        CurveMeasure::Length(2.0),
        CurveMeasure::Length(7.0),
        parent,
    )
    .expect("segment");

    let mut model = model;
    tx.commit(&mut model).expect("commit");

    let entity = model.get(id).expect("segment");
    let unwrap = |index: usize| match &entity.attributes[index] {
        Value::Typed { value, .. } => match **value {
            Value::Real(v) => v,
            ref other => panic!("slot {index} is not a real: {other:?}"),
        },
        other => panic!("slot {index} lost its measure: {other:?}"),
    };
    assert_eq!(unwrap(2), 2.0, "SegmentStart is slot 2");
    assert_eq!(unwrap(3), 7.0, "SegmentLength is slot 3");
    assert_eq!(entity.attributes[4], Value::Ref(parent), "ParentCurve");
}

/// A reparametrised segment's `ParamLength` keeps its measure type.
///
/// It is an `IfcParameterValue`, and a bare real loses the only marker
/// saying so -- invisible to a round-trip, like every other measure
/// wrapper in this crate.
#[test]
fn a_reparametrised_segment_keeps_its_parameter_measure() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let at = origin(&mut tx);
    let arc = circle(&mut tx, at, 1.0).expect("arc");

    let id =
        reparametrised_composite_curve_segment(&mut tx, TransitionCode::Continuous, true, arc, 2.5)
            .expect("segment");

    let mut model = model;
    tx.commit(&mut model).expect("commit");

    match &model.get(id).expect("segment").attributes[3] {
        Value::Typed { type_name, value } => {
            assert_eq!(type_name.as_ref(), "IFCPARAMETERVALUE");
            assert_eq!(**value, Value::Real(2.5));
        }
        other => panic!("ParamLength lost its measure wrapper: {other:?}"),
    }
}
