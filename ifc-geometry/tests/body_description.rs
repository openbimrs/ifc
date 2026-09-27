//! Body description: representation kind and swept-solid parameters (#147).
//!
//! Kernel-free: every test here runs in the `--no-default-features` column
//! too, which is the point of the API. The lowering-agreement checks sit in
//! a `lowering`-gated module at the end.
//!
//! Models are small STEP texts built by `body_description/step.rs`; expected
//! values are derived by hand at each test from the quoted records.

#[path = "body_description/profiles.rs"]
mod profiles;
#[path = "body_description/refusals.rs"]
mod refusals;
#[path = "body_description/step.rs"]
mod step;

use ifc_geometry::{body_description, units, BodyKind, ProfileParameters, SweepPath};
use ifc_model::{Codec, EntityId, Model};
use ifc_step::StepCodec;
use step::{close, metres, model, PRODUCT};

/// The fixture with one extrusion per steel-section family, in millimetres.
fn profile_families() -> Model {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../test/fixtures/synthetic-surfaces/synthetic_profile_families.ifc");
    StepCodec.read_path(&path).expect("fixture parses")
}

/// A straight extrusion along +Z: kind, profile, direction and depth in SI.
///
/// ```text
/// #100=IFCRECTANGLEPROFILEDEF(.AREA.,'R200x100',$,200.,100.);   mm
/// #104=IFCEXTRUDEDAREASOLID(#100,#102,#103,3000.);              +Z, 3 m
/// ```
#[test]
fn an_extrusion_reports_its_profile_direction_and_depth_in_si() {
    let model = model(
        "IFC4",
        ".MILLI.",
        "1.,0.,0.",
        "#100=IFCRECTANGLEPROFILEDEF(.AREA.,'R200x100',$,200.,100.);
         #101=IFCCARTESIANPOINT((0.,0.,0.));
         #102=IFCAXIS2PLACEMENT3D(#101,$,$);
         #103=IFCDIRECTION((0.,0.,1.));
         #104=IFCEXTRUDEDAREASOLID(#100,#102,#103,3000.);",
        "#104",
    );
    let body = body_description(&model, &units::resolve(&model), PRODUCT)
        .expect("describes")
        .expect("has a body");
    assert_eq!(body.representation, EntityId(20));
    let item = body.sole_item().expect("one item");
    assert_eq!(item.item, EntityId(104));
    assert_eq!(item.kind, BodyKind::Extrusion);
    assert!(item.mapped_by.is_empty());
    let swept = item.swept.as_ref().expect("an extrusion is swept");
    assert_eq!(swept.profile.name.as_deref(), Some("R200x100"));
    let ProfileParameters::Rectangle { x_dim, y_dim, .. } = swept.profile.parameters else {
        panic!("expected a rectangle, got {:?}", swept.profile.parameters);
    };
    assert!(
        close([x_dim, y_dim], [0.2, 0.1]),
        "mm to m: {x_dim} {y_dim}"
    );
    let SweepPath::Extrusion {
        direction_world,
        depth,
        ..
    } = swept.path
    else {
        panic!("expected an extrusion path");
    };
    assert!(close(direction_world, [0.0, 0.0, 1.0]));
    assert!((depth - 3.0).abs() < 1e-12, "3000 mm is 3 m, got {depth}");
    // The product sits at (10, 20, 30) mm.
    assert!(close(swept.placement_world.origin, [0.01, 0.02, 0.03]));
}

/// A sheared extrusion in a rotated Position, under a rotated product.
///
/// ```text
/// #104 Position: Axis (1,0,0), RefDirection (0,1,0)
///      local X = (0,1,0), local Y = Z x X = (0,0,1), local Z = (1,0,0)
/// #105 ExtrudedDirection (0,3,4) -> unit (0,0.6,0.8) in Position space
///      = 0.6*Y + 0.8*Z = (0.8, 0, 0.6) in product space
/// product RefDirection (0,1,0): a quarter turn about Z, (x,y,z) -> (-y,x,z)
///      => world (0, 0.8, 0.6)
/// ```
///
/// Reading the ratios raw, or assuming +Z, or skipping either frame, gives a
/// different vector; depth stays 0.5 m along it.
#[test]
fn a_sheared_extrusion_in_rotated_frames_reports_its_world_direction() {
    let model = model(
        "IFC4",
        ".MILLI.",
        "0.,1.,0.",
        "#100=IFCRECTANGLEPROFILEDEF(.AREA.,$,$,200.,100.);
         #101=IFCCARTESIANPOINT((0.,0.,0.));
         #102=IFCDIRECTION((1.,0.,0.));
         #103=IFCDIRECTION((0.,1.,0.));
         #104=IFCAXIS2PLACEMENT3D(#101,#102,#103);
         #105=IFCDIRECTION((0.,3.,4.));
         #106=IFCEXTRUDEDAREASOLID(#100,#104,#105,500.);",
        "#106",
    );
    let body = body_description(&model, &units::resolve(&model), PRODUCT)
        .unwrap()
        .unwrap();
    let swept = body.sole_item().unwrap().swept.as_ref().unwrap();
    let SweepPath::Extrusion {
        direction_world,
        depth,
        ..
    } = swept.path
    else {
        panic!("expected an extrusion path");
    };
    assert!(
        close(direction_world, [0.0, 0.8, 0.6]),
        "got {direction_world:?}"
    );
    assert!((depth - 0.5).abs() < 1e-12);
    // The profile plane's normal (placement Z) is world X rotated a quarter
    // turn: world +Y.
    assert!(close(swept.placement_world.basis[2], [0.0, 1.0, 0.0]));
}

/// One test per steel-section family, read through the body of the fixture.
///
/// `synthetic_profile_families.ifc` declares millimetres and degrees; its
/// product #89 holds one extrusion per family, in this order.
#[test]
fn every_item_of_the_profile_family_fixture_is_described_in_si() {
    let model = profile_families();
    let body = body_description(&model, &units::resolve(&model), EntityId(89))
        .unwrap()
        .unwrap();
    let families: Vec<&str> = body
        .items
        .iter()
        .map(|item| item.swept.as_ref().unwrap().profile.type_name.as_str())
        .collect();
    assert_eq!(
        families,
        [
            "IFCISHAPEPROFILEDEF",
            "IFCASYMMETRICISHAPEPROFILEDEF",
            "IFCLSHAPEPROFILEDEF",
            "IFCTSHAPEPROFILEDEF",
            "IFCUSHAPEPROFILEDEF",
            "IFCCSHAPEPROFILEDEF",
            "IFCZSHAPEPROFILEDEF",
            "IFCELLIPSEPROFILEDEF",
            "IFCTRAPEZIUMPROFILEDEF",
            "IFCDERIVEDPROFILEDEF",
            "IFCMIRROREDPROFILEDEF",
            "IFCCOMPOSITEPROFILEDEF",
            "IFCCENTERLINEPROFILEDEF",
        ],
        "one item per family, in authored order"
    );
    assert!(body.sole_item().is_none(), "several items are not one");
    for item in &body.items {
        assert_eq!(item.kind, BodyKind::Extrusion);
        let SweepPath::Extrusion { depth, .. } = item.swept.as_ref().unwrap().path else {
            panic!("extrusion expected");
        };
        assert!((depth - 1.0).abs() < 1e-12, "1000 mm, got {depth}");
    }
}

/// A body with only an Axis representation has no body to describe.
#[test]
fn a_product_without_a_body_is_none_not_an_error() {
    let model = metres("", "");
    // Rename the only representation to Axis by rebuilding it.
    let mut model = model;
    model.insert(
        EntityId(20),
        ifc_model::Entity::new(
            "IFCSHAPEREPRESENTATION",
            vec![
                ifc_model::Value::Ref(EntityId(10)),
                ifc_model::Value::Text("Axis".into()),
                ifc_model::Value::Text("Curve2D".into()),
                ifc_model::Value::List(Vec::new()),
            ],
        ),
    );
    assert_eq!(
        body_description(&model, &units::resolve(&model), PRODUCT).unwrap(),
        None
    );
}

/// A revolution reports its axis in world coordinates and its angle.
///
/// ```text
/// #101=IFCAXIS1PLACEMENT(#102,#103): origin (0,-500,0) mm, axis +X
/// product at (10,20,30) mm, unrotated => world origin (0.01,-0.48,0.03) m
/// ```
#[test]
fn a_revolution_reports_its_world_axis_and_angle() {
    let model = model(
        "IFC4",
        ".MILLI.",
        "1.,0.,0.",
        "#100=IFCCIRCLEPROFILEDEF(.AREA.,$,$,50.);
         #101=IFCAXIS1PLACEMENT(#102,#103);
         #102=IFCCARTESIANPOINT((0.,-500.,0.));
         #103=IFCDIRECTION((2.,0.,0.));
         #104=IFCREVOLVEDAREASOLID(#100,$,#101,1.5);",
        "#104",
    );
    let body = body_description(&model, &units::resolve(&model), PRODUCT)
        .unwrap()
        .unwrap();
    let item = body.sole_item().unwrap();
    assert_eq!(item.kind, BodyKind::Revolution);
    let SweepPath::Revolution {
        axis_origin_world,
        axis_direction_world,
        angle,
        ..
    } = item.swept.as_ref().unwrap().path
    else {
        panic!("expected a revolution path");
    };
    assert!(close(axis_origin_world, [0.01, -0.48, 0.03]));
    assert!(close(axis_direction_world, [1.0, 0.0, 0.0]));
    assert!((angle - 1.5).abs() < 1e-12);
}

/// A tapered extrusion reports both ends; a directrix sweep names its curve.
#[test]
fn tapered_and_directrix_sweeps_report_what_they_state() {
    let model = metres(
        "#100=IFCRECTANGLEPROFILEDEF(.AREA.,$,$,0.4,0.2);
         #101=IFCRECTANGLEPROFILEDEF(.AREA.,$,$,0.2,0.1);
         #102=IFCDIRECTION((0.,0.,1.));
         #103=IFCEXTRUDEDAREASOLIDTAPERED(#100,$,#102,2.,#101);
         #104=IFCCARTESIANPOINT((0.,0.,0.));
         #105=IFCCARTESIANPOINT((5.,0.,0.));
         #106=IFCPOLYLINE((#104,#105));
         #107=IFCDIRECTION((0.,0.,1.));
         #108=IFCCIRCLEPROFILEDEF(.AREA.,$,$,0.1);
         #109=IFCFIXEDREFERENCESWEPTAREASOLID(#108,$,#106,$,$,#107);",
        "#103,#109",
    );
    let body = body_description(&model, &units::resolve(&model), PRODUCT)
        .unwrap()
        .unwrap();
    let [tapered, directrix] = body.items.as_slice() else {
        panic!("two items");
    };
    assert_eq!(tapered.kind, BodyKind::TaperedExtrusion);
    let swept = tapered.swept.as_ref().unwrap();
    assert_eq!(swept.profile.entity, EntityId(100));
    assert_eq!(
        swept.end_profile.as_ref().map(|end| end.entity),
        Some(EntityId(101))
    );

    assert_eq!(directrix.kind, BodyKind::DirectrixSweep);
    let swept = directrix.swept.as_ref().unwrap();
    assert!(matches!(
        swept.path,
        SweepPath::Directrix { directrix, .. } if directrix == EntityId(106)
    ));
}

/// A B-rep is reported by kind only: it has no profile to state.
#[test]
fn a_brep_body_reports_its_kind_only() {
    let model = metres(
        "#100=IFCFACE(());
         #101=IFCCLOSEDSHELL((#100));
         #102=IFCFACETEDBREP(#101);",
        "#102",
    );
    let body = body_description(&model, &units::resolve(&model), PRODUCT)
        .unwrap()
        .unwrap();
    let item = body.sole_item().unwrap();
    assert_eq!(item.kind, BodyKind::Brep);
    assert_eq!(item.type_name, "IFCFACETEDBREP");
    assert!(item.swept.is_none());
}

/// The shared map used by the mapped-item tests.
///
/// ```text
/// #200 rectangle 0.2 x 0.1, extruded 1 m along its own +Z (#204)
/// #211 MappingTarget: Axis1 (0,1,0), Axis2 (0,0,1), Axis3 (1,0,0),
///      origin (100,0,0): the map's +Z lands on world +X
/// #220 the same solid authored in place: Position at (100,0,0) with
///      Axis (1,0,0), RefDirection (0,1,0)
/// ```
const MAPPED: &str = "#200=IFCRECTANGLEPROFILEDEF(.AREA.,$,$,0.2,0.1);
     #201=IFCCARTESIANPOINT((0.,0.,0.));
     #202=IFCAXIS2PLACEMENT3D(#201,$,$);
     #203=IFCDIRECTION((0.,0.,1.));
     #204=IFCEXTRUDEDAREASOLID(#200,#202,#203,1.);
     #205=IFCSHAPEREPRESENTATION(#10,'Body','SweptSolid',(#204));
     #206=IFCCARTESIANPOINT((0.,0.,0.));
     #207=IFCAXIS2PLACEMENT3D(#206,$,$);
     #208=IFCREPRESENTATIONMAP(#207,#205);
     #209=IFCCARTESIANPOINT((100.,0.,0.));
     #212=IFCDIRECTION((0.,1.,0.));
     #213=IFCDIRECTION((0.,0.,1.));
     #214=IFCDIRECTION((1.,0.,0.));
     #211=IFCCARTESIANTRANSFORMATIONOPERATOR3D(#212,#213,#209,$,#214);
     #215=IFCMAPPEDITEM(#208,#211);
     #216=IFCCARTESIANPOINT((100.,0.,0.));
     #217=IFCDIRECTION((1.,0.,0.));
     #218=IFCDIRECTION((0.,1.,0.));
     #219=IFCAXIS2PLACEMENT3D(#216,#217,#218);
     #220=IFCEXTRUDEDAREASOLID(#200,#219,#203,1.);";

/// Mapped geometry resolves to the same answer as the same solid in place.
#[test]
fn a_mapped_item_describes_like_the_same_solid_authored_in_place() {
    let model = metres(MAPPED, "#215,#220");
    let body = body_description(&model, &units::resolve(&model), PRODUCT)
        .unwrap()
        .unwrap();
    let [mapped, direct] = body.items.as_slice() else {
        panic!("the map resolves to one item beside the direct one");
    };
    assert_eq!(
        mapped.item,
        EntityId(204),
        "the geometric item, not the map"
    );
    assert_eq!(mapped.mapped_by, [EntityId(215)]);
    assert!(direct.mapped_by.is_empty());

    let (mapped, direct) = (
        mapped.swept.as_ref().unwrap(),
        direct.swept.as_ref().unwrap(),
    );
    assert_eq!(mapped.profile, direct.profile);
    let (
        SweepPath::Extrusion {
            direction_world: a,
            depth: da,
            ..
        },
        SweepPath::Extrusion {
            direction_world: b,
            depth: db,
            ..
        },
    ) = (&mapped.path, &direct.path)
    else {
        panic!("both are extrusions");
    };
    assert!(
        close(*a, [1.0, 0.0, 0.0]),
        "mapping rotates +Z to +X: {a:?}"
    );
    assert!(close(*a, *b));
    assert_eq!(da, db);
    for axis in 0..3 {
        assert!(close(
            mapped.placement_world.basis[axis],
            direct.placement_world.basis[axis]
        ));
    }
    assert!(close(
        mapped.placement_world.origin,
        direct.placement_world.origin
    ));
    assert!(close(mapped.placement_world.origin, [110.0, 20.0, 30.0]));
}

/// The product's placement composes above the mapping: rotating the product
/// rotates the described direction.
#[test]
fn the_product_placement_moves_the_description() {
    let near = metres(MAPPED, "#215");
    let far = model("IFC4", "$", "0.,1.,0.", MAPPED, "#215");
    let direction = |model: &Model| {
        let body = body_description(model, &units::resolve(model), PRODUCT)
            .unwrap()
            .unwrap();
        match body.sole_item().unwrap().swept.as_ref().unwrap().path {
            SweepPath::Extrusion {
                direction_world, ..
            } => direction_world,
            _ => panic!("extrusion"),
        }
    };
    assert!(close(direction(&near), [1.0, 0.0, 0.0]));
    // A quarter turn of the product turns world +X into +Y.
    assert!(close(direction(&far), [0.0, 1.0, 0.0]));
}

/// The context's `WorldCoordinateSystem` is model space: it composes above
/// the product placement, exactly as lowering composes it.
///
/// ```text
/// #6 context WCS: origin (100,0,0), RefDirection (0,1,0)  (a quarter turn)
/// mapped solid at (110,20,30) in the context frame, direction +X
///   => world origin (100,0,0) + R*(110,20,30) = (80,110,30), direction +Y
/// ```
#[test]
fn the_context_world_coordinate_system_is_applied() {
    use ifc_model::{Entity, Value};
    let mut model = metres(
        &format!(
            "{MAPPED}
             #7=IFCCARTESIANPOINT((100.,0.,0.));
             #8=IFCDIRECTION((0.,1.,0.));"
        ),
        "#215",
    );
    model.insert(
        EntityId(6),
        Entity::new(
            "IFCAXIS2PLACEMENT3D",
            vec![
                Value::Ref(EntityId(7)),
                Value::Null,
                Value::Ref(EntityId(8)),
            ],
        ),
    );
    let body = body_description(&model, &units::resolve(&model), PRODUCT)
        .unwrap()
        .unwrap();
    let swept = body.sole_item().unwrap().swept.as_ref().unwrap();
    assert!(
        close(swept.placement_world.origin, [80.0, 110.0, 30.0]),
        "{:?}",
        swept.placement_world.origin
    );
    let SweepPath::Extrusion {
        direction_world, ..
    } = swept.path
    else {
        panic!("extrusion");
    };
    assert!(close(direction_world, [0.0, 1.0, 0.0]));
}

/// The coverage link to lowering: every family the dispatcher lowers is a
/// kind, except the mapped item, which is resolved rather than classified.
#[cfg(feature = "lowering")]
#[test]
fn every_lowered_family_has_a_body_kind() {
    use ifc_geometry::lower::dispatch::IMPLEMENTED;
    let unclassified: Vec<&str> = IMPLEMENTED
        .iter()
        .copied()
        .filter(|name| *name != "IFCMAPPEDITEM")
        .filter(|name| BodyKind::classify(name).is_none())
        .collect();
    assert!(
        unclassified.is_empty(),
        "lowerable families with no body kind: {unclassified:?}"
    );
    assert_eq!(BodyKind::classify("IFCMAPPEDITEM"), None);
}
