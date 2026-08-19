//! Lowering properties that the corpus alone cannot prove.
//!
//! # Why these are separate from the fixture tests
//!
//! The committed fixtures that lower are all metre-based and rectangle-based,
//! so they cannot detect a missing unit conversion, an ignored solid
//! placement, or a hardcoded circle segment count. Those need models built to
//! isolate exactly one variable. Each test here corresponds to a mutation
//! that survived the fixture suite.

use ifc_geometry::kernel::Primitive;
use ifc_geometry::lower::{lower_extruded_area_solid, lower_profile, Tolerance};
use ifc_geometry::transform::Transform;
use ifc_geometry::units::UnitScale;
use ifc_model::{Entity, EntityId, Model, Value};

fn r(id: u64) -> Value {
    Value::Ref(EntityId(id))
}

fn n(v: f64) -> Value {
    Value::Real(v)
}

/// A millimetre-scale unit assignment.
fn millimetres() -> UnitScale {
    let mut model = Model::new();
    model.insert(
        EntityId(1),
        Entity::new(
            "IFCSIUNIT",
            vec![
                Value::Derived,
                Value::Enum("LENGTHUNIT".into()),
                Value::Enum("MILLI".into()),
                Value::Enum("METRE".into()),
            ],
        ),
    );
    model.insert(
        EntityId(2),
        Entity::new("IFCUNITASSIGNMENT", vec![Value::List(vec![r(1)])]),
    );
    let mut project = vec![Value::Null; 9];
    project[8] = r(2);
    model.insert(EntityId(3), Entity::new("IFCPROJECT", project));
    ifc_geometry::units::resolve(&model)
}

/// A minimal extruded solid: rectangle profile, +Z direction, given depth.
fn extrusion_model(depth: f64, with_position: bool) -> (Model, EntityId) {
    let mut m = Model::new();
    m.insert(
        EntityId(1),
        Entity::new(
            "IFCDIRECTION",
            vec![Value::List(vec![n(0.0), n(0.0), n(1.0)])],
        ),
    );
    m.insert(
        EntityId(2),
        Entity::new(
            "IFCRECTANGLEPROFILEDEF",
            vec![
                Value::Enum("AREA".into()),
                Value::Null,
                Value::Null,
                n(100.0),
                n(200.0),
            ],
        ),
    );
    let position = if with_position {
        m.insert(
            EntityId(3),
            Entity::new(
                "IFCCARTESIANPOINT",
                vec![Value::List(vec![n(1000.0), n(0.0), n(0.0)])],
            ),
        );
        m.insert(
            EntityId(4),
            Entity::new("IFCAXIS2PLACEMENT3D", vec![r(3), Value::Null, Value::Null]),
        );
        r(4)
    } else {
        Value::Null
    };
    m.insert(
        EntityId(5),
        Entity::new("IFCEXTRUDEDAREASOLID", vec![r(2), position, r(1), n(depth)]),
    );
    (m, EntityId(5))
}

/// M1: depth must be converted to metres.
///
/// A 2500 mm extrusion is 2.5 m. Skipping the conversion yields a solid a
/// thousand times too tall, and no metre-based fixture can reveal it.
#[test]
fn extrusion_depth_is_converted_to_metres() {
    let (m, id) = extrusion_model(2500.0, false);
    let tol = Tolerance::building_scale();
    let p = lower_extruded_area_solid(&m, id, Transform::identity(), &millimetres(), &tol)
        .expect("lowers");
    match p {
        Primitive::Extrusion { depth, .. } => assert!(
            (depth - 2.5).abs() < 1e-12,
            "2500 mm must lower to 2.5 m, got {depth}"
        ),
        other => panic!("expected an extrusion, got {other:?}"),
    }
}

/// Profile dimensions are lengths too, and get the same conversion.
#[test]
fn profile_dimensions_are_converted_to_metres() {
    let (m, _) = extrusion_model(1.0, false);
    let tol = Tolerance::building_scale();
    let profile = lower_profile(&m, EntityId(2), &millimetres(), &tol).expect("lowers");
    let width = span(&profile.outer.points, 0);
    assert!(
        (width - 0.1).abs() < 1e-12,
        "a 100 mm profile is 0.1 m wide, got {width}"
    );
}

/// M5: the solid's own Position must reach the kernel.
///
/// Dropping it puts every solid that carries one at the product origin.
#[test]
fn the_solid_position_is_composed_into_the_placement() {
    let tol = Tolerance::building_scale();
    let (with_pos, id_with) = extrusion_model(1000.0, true);
    let (without_pos, id_without) = extrusion_model(1000.0, false);

    let a = lower_extruded_area_solid(
        &with_pos,
        id_with,
        Transform::identity(),
        &millimetres(),
        &tol,
    )
    .expect("lowers");
    let b = lower_extruded_area_solid(
        &without_pos,
        id_without,
        Transform::identity(),
        &millimetres(),
        &tol,
    )
    .expect("lowers");

    let (pa, pb) = match (a, b) {
        (
            Primitive::Extrusion { placement: pa, .. },
            Primitive::Extrusion { placement: pb, .. },
        ) => (pa, pb),
        _ => panic!("expected extrusions"),
    };
    assert_ne!(
        pa, pb,
        "a solid with a Position must not land in the same place as one without"
    );
    // The position offsets 1000 mm along X, which is 1 m.
    let moved = pa.apply([0.0, 0.0, 0.0]);
    assert!(
        (moved[0] - 1.0).abs() < 1e-12,
        "expected a 1 m offset, got {moved:?}"
    );
}

/// Width of a contour along one axis.
fn span(points: &[[f64; 2]], axis: usize) -> f64 {
    let lo = points.iter().map(|p| p[axis]).fold(f64::MAX, f64::min);
    let hi = points.iter().map(|p| p[axis]).fold(f64::MIN, f64::max);
    hi - lo
}

/// M6: circle refinement must follow the tolerance, not a fixed count.
///
/// Two circles of very different radii at the same tolerance must not receive
/// the same number of segments; a hardcoded count would give identical
/// contours and silently over- or under-refine.
#[test]
fn circle_refinement_follows_the_tolerance() {
    let tol = Tolerance::building_scale();
    let mut m = Model::new();
    m.insert(
        EntityId(1),
        Entity::new(
            "IFCCIRCLEPROFILEDEF",
            vec![
                Value::Enum("AREA".into()),
                Value::Null,
                Value::Null,
                n(0.05),
            ],
        ),
    );
    m.insert(
        EntityId(2),
        Entity::new(
            "IFCCIRCLEPROFILEDEF",
            vec![Value::Enum("AREA".into()), Value::Null, Value::Null, n(5.0)],
        ),
    );
    let metres = UnitScale::default();
    let small = lower_profile(&m, EntityId(1), &metres, &tol).expect("lowers");
    let large = lower_profile(&m, EntityId(2), &metres, &tol).expect("lowers");

    assert!(
        large.outer.points.len() > small.outer.points.len(),
        "a 5 m circle needs more segments than a 5 cm one at equal tolerance: \
         small={} large={}",
        small.outer.points.len(),
        large.outer.points.len()
    );

    // And the actual chord error must respect the requested sagitta.
    let n_large = large.outer.points.len() as f64;
    let per = std::f64::consts::TAU / n_large;
    let sagitta = 5.0 * (1.0 - (per / 2.0).cos());
    assert!(
        sagitta <= 1e-3 + 1e-12,
        "chord height {sagitta} exceeds the 1 mm tolerance"
    );

    // A coarser tolerance must produce a coarser contour.
    let coarse = Tolerance::from_sagitta(0.05).unwrap();
    let coarser = lower_profile(&m, EntityId(2), &metres, &coarse).expect("lowers");
    assert!(
        coarser.outer.points.len() < large.outer.points.len(),
        "a coarser tolerance must not refine further"
    );
}

/// A circle contour must not repeat its first point.
///
/// `Contour` closes implicitly; a duplicated vertex is a zero-length edge.
#[test]
fn a_circle_contour_does_not_repeat_its_first_point() {
    let tol = Tolerance::building_scale();
    let mut m = Model::new();
    m.insert(
        EntityId(1),
        Entity::new(
            "IFCCIRCLEPROFILEDEF",
            vec![Value::Enum("AREA".into()), Value::Null, Value::Null, n(1.0)],
        ),
    );
    let profile = lower_profile(&m, EntityId(1), &UnitScale::default(), &tol).expect("lowers");
    let pts = &profile.outer.points;
    let first = pts[0];
    let last = pts[pts.len() - 1];
    let d = ((first[0] - last[0]).powi(2) + (first[1] - last[1]).powi(2)).sqrt();
    assert!(
        d > 1e-9,
        "first and last point coincide: {first:?} {last:?}"
    );
}
