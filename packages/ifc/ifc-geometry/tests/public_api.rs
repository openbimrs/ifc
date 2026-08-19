//! Proof that the public API really exposes a view per entity.
//!
//! # Why this exists beside `schema_coverage.rs`
//!
//! That test greps source text, which cannot see through `macro_rules!`: the
//! CSG primitive views are macro-generated, so a textual search reports them
//! missing even though the types exist. Conversely a grep can be satisfied by
//! a mere string literal.
//!
//! This test names the types. It only compiles if they are genuinely public
//! and genuinely constructible, which is the property a consumer depends on.
//! The compiler is the assertion.

use ifc_geometry::curve;
use ifc_geometry::resource;
use ifc_geometry::solid;
use ifc_geometry::surface;
use ifc_model::{Entity, EntityId, Value};

/// A throwaway entity to construct views against.
fn entity(type_name: &str) -> Entity {
    Entity::new(type_name, vec![Value::Null; 8])
}

/// The CSG primitives, which are macro-generated and so invisible to a grep.
#[test]
fn csg_primitive_views_are_real_types_not_just_matched_strings() {
    let block = entity("IFCBLOCK");
    let sphere = entity("IFCSPHERE");
    let cone = entity("IFCRIGHTCIRCULARCONE");
    let cylinder = entity("IFCRIGHTCIRCULARCYLINDER");
    let pyramid = entity("IFCRECTANGULARPYRAMID");

    // Naming each type is the assertion: this does not compile otherwise.
    let _: solid::csg::Block<'_> = solid::csg::Block::new(EntityId(1), &block);
    let _: solid::csg::Sphere<'_> = solid::csg::Sphere::new(EntityId(2), &sphere);
    let _: solid::csg::RightCircularCone<'_> =
        solid::csg::RightCircularCone::new(EntityId(3), &cone);
    let _: solid::csg::RightCircularCylinder<'_> =
        solid::csg::RightCircularCylinder::new(EntityId(4), &cylinder);
    let _: solid::csg::RectangularPyramid<'_> =
        solid::csg::RectangularPyramid::new(EntityId(5), &pyramid);
}

/// The primitives every geometry graph bottoms out in.
#[test]
fn foundation_views_are_public() {
    let point = entity("IFCCARTESIANPOINT");
    let direction = entity("IFCDIRECTION");
    let placement = entity("IFCAXIS2PLACEMENT3D");

    let _ = resource::point::CartesianPoint::new(EntityId(1), &point);
    let _ = resource::direction::Direction::new(EntityId(2), &direction);
    let _ = resource::placement::Axis2Placement3D::new(EntityId(3), &placement);
}

/// The classifiers a consumer dispatches on.
///
/// These are the entry points for "what kind of thing is this entity", so
/// they must be reachable from outside the crate.
#[test]
fn kind_classifiers_are_reachable_and_agree_with_the_schema() {
    assert!(
        curve::CurveKind::classify("IFCTRIMMEDCURVE").is_some(),
        "a trimmed curve must classify"
    );
    assert!(
        surface::SurfaceKind::classify("IFCPLANE").is_some(),
        "a plane must classify"
    );
    assert!(
        solid::SolidKind::classify("IFCEXTRUDEDAREASOLID").is_some(),
        "an extruded area solid must classify"
    );

    // An entity from another schema must NOT be claimed as geometry.
    assert!(
        curve::CurveKind::classify("IFCWALL").is_none(),
        "a wall is not a curve"
    );
    assert!(
        solid::SolidKind::classify("IFCCOSTITEM").is_none(),
        "a cost item is not a solid"
    );
}

/// The placement resolver, which is the crate's most load-bearing service.
#[test]
fn placement_resolution_is_reachable_from_outside_the_crate() {
    use ifc_geometry::constraint::local::PlacementResolver;
    use ifc_model::Model;

    let mut model = Model::new();
    model.insert(
        EntityId(11),
        Entity::new(
            "IFCCARTESIANPOINT",
            vec![Value::List(vec![
                Value::Real(1.0),
                Value::Real(2.0),
                Value::Real(3.0),
            ])],
        ),
    );
    model.insert(
        EntityId(10),
        Entity::new(
            "IFCAXIS2PLACEMENT3D",
            vec![Value::Ref(EntityId(11)), Value::Null, Value::Null],
        ),
    );
    model.insert(
        EntityId(1),
        Entity::new(
            "IFCLOCALPLACEMENT",
            vec![Value::Null, Value::Ref(EntityId(10))],
        ),
    );

    let mut resolver = PlacementResolver::new();
    let world = resolver.world_transform(&model, EntityId(1)).unwrap();
    assert_eq!(
        world.origin,
        [1.0, 2.0, 3.0],
        "an unparented placement is absolute"
    );
}

/// The kernel vocabulary must be public: it is the contract a geometry
/// backend is written against.
#[test]
fn the_kernel_contract_is_part_of_the_public_api() {
    use ifc_geometry::kernel::{BooleanOp, Contour, CsgShape, Primitive, Profile};
    use ifc_geometry::Transform;

    let square = Profile {
        outer: Contour {
            points: vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
        },
        inner: vec![],
    };

    let cut = Primitive::Boolean {
        op: BooleanOp::Difference,
        first: Box::new(Primitive::Extrusion {
            profile: square,
            direction: [0.0, 0.0, 1.0],
            depth: 2.0,
            placement: Transform::identity(),
        }),
        second: Box::new(Primitive::Csg {
            shape: CsgShape::Sphere { radius: 0.5 },
            placement: Transform::identity(),
        }),
    };

    assert!(
        cut.requires_boolean(),
        "a backend must be able to ask whether it needs boolean support"
    );
    assert_eq!(cut.kind(), "boolean");
}
