//! Deriving a placement frame from the basis curve.
//!
//! The expected values here are computed independently: the plan point by
//! Fresnel quadrature over the clothoid, the height from the grade, the
//! frame axes from the reference-up construction. Nothing is read back
//! from the code under test.

#![cfg(feature = "compile")]

use axiolid_evaluate::ReferenceCurveEvaluator;
use ifc_alignment::{CurveMeasure, PointByDistance};
use ifc_geometry::constraint::placement::derive::derive_placement_transform;
use ifc_geometry::units::UnitScale;
use ifc_model::{Entity, EntityId, Model, Value};
use std::sync::Arc;

const TOL: f64 = 1e-6;

fn metres() -> UnitScale {
    UnitScale {
        length_to_metres: 1.0,
        angle_to_radians: 1.0,
    }
}

fn close(actual: f64, expected: f64, what: &str) {
    assert!(
        (actual - expected).abs() < TOL,
        "{what}: got {actual}, expected {expected}"
    );
}

/// Alignment fixture: clothoid plan, constant-grade profile.
///
/// Copied from the ifc-alignment gradient tests so this crate does not
/// depend on another crate's test module.
fn alignment_model_with(plan_kind: &str, end_radius: f64) -> (Model, EntityId) {
    let mut model = Model::new();
    model.header_mut().schema = vec!["IFC4X3_ADD2".to_owned()];
    let mut id = 0;
    let mut next = || {
        id += 1;
        EntityId(id)
    };

    let origin = next();
    model.insert(
        origin,
        Entity::new(
            "IFCCARTESIANPOINT",
            vec![Value::List(vec![Value::Real(0.0), Value::Real(0.0)])],
        ),
    );

    // A clothoid: straight into a 300 m radius over 100 m.
    let h_params = next();
    model.insert(
        h_params,
        Entity::new(
            "IFCALIGNMENTHORIZONTALSEGMENT",
            vec![
                Value::Null,
                Value::Null,
                Value::Ref(origin),
                Value::Real(0.0),
                Value::Real(0.0),
                Value::Real(end_radius),
                Value::Real(100.0),
                Value::Null,
                Value::Enum(Arc::from(plan_kind)),
            ],
        ),
    );

    // A 100 m constant 2%% grade from height 10.
    let v_params = next();
    model.insert(
        v_params,
        Entity::new(
            "IFCALIGNMENTVERTICALSEGMENT",
            vec![
                Value::Null,
                Value::Null,
                Value::Real(0.0),
                Value::Real(100.0),
                Value::Real(10.0),
                Value::Real(0.02),
                Value::Real(0.02),
                Value::Null,
                Value::Enum(Arc::from("CONSTANTGRADIENT")),
            ],
        ),
    );

    let product = |name: &str| {
        let mut attrs = vec![Value::Null; 7];
        attrs[0] = Value::Text(Arc::from(name));
        attrs
    };
    let wrapper = |design: EntityId, name: &str| {
        let mut attrs = product(name);
        attrs.push(Value::Ref(design));
        attrs
    };

    let h_seg = next();
    model.insert(
        h_seg,
        Entity::new("IFCALIGNMENTSEGMENT", wrapper(h_params, "hs")),
    );
    let v_seg = next();
    model.insert(
        v_seg,
        Entity::new("IFCALIGNMENTSEGMENT", wrapper(v_params, "vs")),
    );

    let horizontal = next();
    model.insert(
        horizontal,
        Entity::new("IFCALIGNMENTHORIZONTAL", product("H")),
    );
    let vertical_layout = next();
    model.insert(
        vertical_layout,
        Entity::new("IFCALIGNMENTVERTICAL", product("V")),
    );

    let alignment = next();
    let mut align_attrs = product("A1");
    align_attrs.push(Value::Null);
    model.insert(alignment, Entity::new("IFCALIGNMENT", align_attrs));

    let mut nest = |parent: EntityId, children: Vec<EntityId>, tag: &str| {
        let id = next();
        model.insert(
            id,
            Entity::new(
                "IFCRELNESTS",
                vec![
                    Value::Text(Arc::from(tag)),
                    Value::Null,
                    Value::Null,
                    Value::Null,
                    Value::Ref(parent),
                    Value::List(children.into_iter().map(Value::Ref).collect()),
                ],
            ),
        );
    };
    nest(horizontal, vec![h_seg], "nh");
    nest(vertical_layout, vec![v_seg], "nv");
    nest(alignment, vec![horizontal, vertical_layout], "na");

    (model, alignment)
}

/// A product placed at station 40 with lateral and vertical offsets.
///
/// This is the case the cached-CartesianPosition path could not serve: the
/// file states only a distance along the curve, so the frame has to be
/// derived by evaluating it.
#[test]
fn a_placement_without_a_cached_position_is_derived_from_the_curve() {
    let (model, alignment) = alignment_model_with("CLOTHOID", 300.0);
    let evaluator = ReferenceCurveEvaluator::default();

    let expression = PointByDistance {
        entity: EntityId(9_000),
        distance_along: CurveMeasure::Length(40.0),
        offset_lateral: Some(3.5),
        offset_vertical: Some(1.2),
        offset_longitudinal: None,
        basis_curve: alignment,
    };

    let transform =
        derive_placement_transform(&model, &metres(), EntityId(9_001), &expression, &evaluator)
            .expect("a stated distance on a composed centreline must resolve");

    // Independent values: Fresnel quadrature for the plan point, grade for
    // the height, reference-up construction for the axes.
    let tangent = [
        0.999_444_596_579_483_1,
        0.026_658_175_183_230_975,
        0.019_996_001_199_600_14,
    ];
    let right = [0.026_663_506_285_210_72, -0.999_644_465_513_903_8, 0.0];

    close(transform.basis[0][0], tangent[0], "tangent x");
    close(transform.basis[0][1], tangent[1], "tangent y");
    close(transform.basis[0][2], tangent[2], "tangent z");
    close(transform.basis[2][0], right[0], "right x");
    close(transform.basis[2][1], right[1], "right y");
    close(transform.basis[2][2], right[2], "right z");

    // Origin: centreline point, offset 3.5 along right and 1.2 along up.
    let centre = [39.997_155_649_197_39, 0.355_537_495_999_310_67, 10.8];
    let up = [
        right[1] * tangent[2] - right[2] * tangent[1],
        right[2] * tangent[0] - right[0] * tangent[2],
        right[0] * tangent[1] - right[1] * tangent[0],
    ];
    for axis in 0..3 {
        let expected = centre[axis] + right[axis] * 3.5 + up[axis] * 1.2;
        close(transform.origin[axis], expected, "origin");
    }
}

/// The frame must be orthonormal, whatever the grade.
///
/// A placement frame that is not orthonormal skews the product it places,
/// and the skew grows with the offset rather than showing up at the origin.
#[test]
fn the_derived_frame_is_orthonormal() {
    let (model, alignment) = alignment_model_with("CLOTHOID", 300.0);
    let evaluator = ReferenceCurveEvaluator::default();

    let expression = PointByDistance {
        entity: EntityId(9_000),
        distance_along: CurveMeasure::Length(75.0),
        offset_lateral: None,
        offset_vertical: None,
        offset_longitudinal: None,
        basis_curve: alignment,
    };

    let transform =
        derive_placement_transform(&model, &metres(), EntityId(9_001), &expression, &evaluator)
            .expect("station 75 is inside the curve");

    for axis in transform.basis {
        let length = (axis[0] * axis[0] + axis[1] * axis[1] + axis[2] * axis[2]).sqrt();
        close(length, 1.0, "axis is unit length");
    }
    for (a, b) in [(0, 1), (1, 2), (0, 2)] {
        let dot = transform.basis[a][0] * transform.basis[b][0]
            + transform.basis[a][1] * transform.basis[b][1]
            + transform.basis[a][2] * transform.basis[b][2];
        close(dot, 0.0, "axes are perpendicular");
    }
}

/// A parameter is not a distance.
///
/// `IfcCurveMeasureSelect` lets the file say which it means, and the two
/// must not be conflated: on a curve where they differ, passing one as the
/// other places the product somewhere wrong but entirely plausible.
#[test]
fn a_parameter_is_not_silently_read_as_a_distance() {
    let (model, alignment) = alignment_model_with("CLOTHOID", 300.0);
    let evaluator = ReferenceCurveEvaluator::default();

    let at = |measure| PointByDistance {
        entity: EntityId(9_000),
        distance_along: measure,
        offset_lateral: None,
        offset_vertical: None,
        offset_longitudinal: None,
        basis_curve: alignment,
    };

    let by_distance = derive_placement_transform(
        &model,
        &metres(),
        EntityId(9_001),
        &at(CurveMeasure::Length(40.0)),
        &evaluator,
    );
    let by_parameter = derive_placement_transform(
        &model,
        &metres(),
        EntityId(9_001),
        &at(CurveMeasure::Parameter(40.0)),
        &evaluator,
    );

    // Whatever each resolves to, the two must not be treated as the same
    // request. Equal results would mean the select type was collapsed.
    match (by_distance, by_parameter) {
        (Ok(distance), Ok(parameter)) => assert_ne!(
            distance.origin, parameter.origin,
            "a parameter and a length must not resolve identically"
        ),
        (Ok(_), Err(_)) | (Err(_), Ok(_)) => {}
        (Err(left), Err(right)) => assert_eq!(
            format!("{left}"),
            format!("{right}"),
            "both routes refused, which is acceptable"
        ),
    }
}

/// Units are converted before the distance crosses into the kernel.
///
/// The kernel is unitless, so both the geometry and the stated distance
/// must be expressed in the same unit before they meet. A file authored in
/// millimetres describes the same road as one authored in metres, and a
/// station at the same fraction along it must land in the same place.
///
/// Note that a unit scale rescales the CURVE as well as the distance:
/// stating 40000 against a millimetre file is 40 m of a 0.1 m curve, which
/// is off the end. The invariant is proportional, not numeric.
#[test]
fn a_distance_and_its_curve_are_measured_in_the_same_unit() {
    let (model, alignment) = alignment_model_with("CLOTHOID", 300.0);
    let evaluator = ReferenceCurveEvaluator::default();

    let at = |value| PointByDistance {
        entity: EntityId(9_000),
        distance_along: CurveMeasure::Length(value),
        offset_lateral: None,
        offset_vertical: None,
        offset_longitudinal: None,
        basis_curve: alignment,
    };

    // 40 of 100 authored units, read as metres.
    let in_metres =
        derive_placement_transform(&model, &metres(), EntityId(9_001), &at(40.0), &evaluator)
            .expect("40 m along a 100 m curve");

    // The same 40 of 100 authored units, read as millimetres: the curve is
    // now 0.1 m long and the station is 0.04 m along it.
    let millimetres = UnitScale {
        length_to_metres: 0.001,
        angle_to_radians: 1.0,
    };
    let in_millimetres =
        derive_placement_transform(&model, &millimetres, EntityId(9_001), &at(40.0), &evaluator)
            .expect("40 mm along a 100 mm curve");

    // Same fraction along the same shape: the position scales by exactly
    // the unit factor. If the distance were converted but the geometry not,
    // this would not hold.
    for axis in 0..3 {
        close(
            in_millimetres.origin[axis],
            in_metres.origin[axis] * 0.001,
            "origin scales with the unit",
        );
    }
}

/// A basis curve we cannot compose is refused by name.
#[test]
fn an_unsupported_basis_curve_is_refused_not_guessed() {
    let (mut model, _) = alignment_model_with("CLOTHOID", 300.0);
    let circle = EntityId(8_000);
    model.insert(
        circle,
        Entity::new("IFCCIRCLE", vec![Value::Null, Value::Real(50.0)]),
    );
    let evaluator = ReferenceCurveEvaluator::default();

    let expression = PointByDistance {
        entity: EntityId(9_000),
        distance_along: CurveMeasure::Length(10.0),
        offset_lateral: None,
        offset_vertical: None,
        offset_longitudinal: None,
        basis_curve: circle,
    };

    let error =
        derive_placement_transform(&model, &metres(), EntityId(9_001), &expression, &evaluator)
            .expect_err("a bare circle is not an alignment centreline");
    let text = format!("{error}");
    assert!(
        text.contains("IFCCIRCLE"),
        "refusal must name the offending type: {text}"
    );
}

/// A dangling basis curve is reported as missing, not as unsupported.
#[test]
fn a_dangling_basis_curve_is_reported_as_missing() {
    let (model, _) = alignment_model_with("CLOTHOID", 300.0);
    let evaluator = ReferenceCurveEvaluator::default();

    let expression = PointByDistance {
        entity: EntityId(9_000),
        distance_along: CurveMeasure::Length(10.0),
        offset_lateral: None,
        offset_vertical: None,
        offset_longitudinal: None,
        basis_curve: EntityId(7_777),
    };

    let error =
        derive_placement_transform(&model, &metres(), EntityId(9_001), &expression, &evaluator)
            .expect_err("a basis curve that is not in the file cannot be evaluated");
    let text = format!("{error}");
    assert!(
        text.contains("7777"),
        "refusal must name the missing entity: {text}"
    );
}
