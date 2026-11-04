//! Swept solids described in their own item frame (#640).
//!
//! `local_sweep` states the authored profile, `Position` and path whatever the
//! enclosing frame does; `swept` is its world form, present only when the
//! item's frame is rigid.

use ifc_geometry::{
    body_description, units, BodyDescription, BodyKind, LocalSweepPath, ProfileParameters,
};
use ifc_model::Model;

use super::step::{close, degrees, metres, model, PRODUCT};

fn describe(model: &Model) -> BodyDescription {
    body_description(
        model,
        &units::resolve(model).expect("project units"),
        PRODUCT,
    )
    .expect("describes")
    .expect("has a body")
}

fn rectangle(profile: &ifc_geometry::ProfileDescription) -> (f64, f64) {
    match profile.parameters {
        ProfileParameters::Rectangle { x_dim, y_dim, .. } => (x_dim, y_dim),
        ref other => panic!("not a rectangle: {other:?}"),
    }
}

/// A rigid, rotated product: the world placement is `item_world` composed
/// with the solid's `Position`, bit for bit.
///
/// ```text
/// product placement: (10,20,30), local X = (0,1,0)
/// #104 Position at (1,2,3), extruded along +Z
/// ```
#[test]
fn a_rigid_item_composes_to_the_world_placement_bitwise() {
    let model = model(
        "IFC4",
        "$",
        "0.,1.,0.",
        "#100=IFCRECTANGLEPROFILEDEF(.AREA.,$,$,2.,1.);
         #101=IFCCARTESIANPOINT((1.,2.,3.));
         #102=IFCAXIS2PLACEMENT3D(#101,$,$);
         #103=IFCDIRECTION((0.,0.,1.));
         #104=IFCEXTRUDEDAREASOLID(#100,#102,#103,4.);",
        "#104",
    );
    let body = describe(&model);
    let item = body.sole_item().unwrap();
    let local = item.local_sweep.as_ref().expect("local");
    let swept = item.swept.as_ref().expect("rigid frame has a world form");
    let position = local.position.expect("Position authored");
    assert_eq!(position.origin, [1.0, 2.0, 3.0]);
    assert_eq!(item.item_world.compose(&position), swept.placement_world);
    assert_eq!(rectangle(&local.profile), (2.0, 1.0));
}

/// An absent `Position` (IFC4 OPTIONAL) is `None`, and the world placement is
/// the item frame itself.
#[test]
fn an_absent_position_is_none_and_the_world_placement_is_the_item_frame() {
    let model = model(
        "IFC4",
        "$",
        "0.,1.,0.",
        "#100=IFCRECTANGLEPROFILEDEF(.AREA.,$,$,2.,1.);
         #103=IFCDIRECTION((0.,0.,1.));
         #104=IFCEXTRUDEDAREASOLID(#100,$,#103,4.);",
        "#104",
    );
    let body = describe(&model);
    let item = body.sole_item().unwrap();
    assert_eq!(item.local_sweep.as_ref().unwrap().position, None);
    assert_eq!(
        item.swept.as_ref().unwrap().placement_world,
        item.item_world
    );
}

/// The structure of Annex E `mapped-shape-with-transformation`: a
/// non-uniform operator scales the item, so there is no world form, but the
/// authored 1 m x 1 m profile and 2 m depth are stated.
///
/// ```text
/// #104 1000 x 1000 mm extruded 2000 mm; operator scales (0.5, 0.5, 1)
/// ```
#[test]
fn a_non_uniformly_mapped_extrusion_is_described_locally() {
    let model = model(
        "IFC4",
        ".MILLI.",
        "1.,0.,0.",
        "#100=IFCRECTANGLEPROFILEDEF(.AREA.,$,$,1000.,1000.);
         #101=IFCCARTESIANPOINT((0.,0.,0.));
         #102=IFCAXIS2PLACEMENT3D(#101,$,$);
         #103=IFCDIRECTION((0.,0.,1.));
         #104=IFCEXTRUDEDAREASOLID(#100,$,#103,2000.);
         #200=IFCSHAPEREPRESENTATION(#10,'Body','SweptSolid',(#104));
         #201=IFCREPRESENTATIONMAP(#102,#200);
         #202=IFCCARTESIANTRANSFORMATIONOPERATOR3DNONUNIFORM($,$,#101,0.5,$,0.5,1.);
         #203=IFCMAPPEDITEM(#201,#202);",
        "#203",
    );
    let body = describe(&model);
    let item = body.sole_item().unwrap();
    assert!(item.swept.is_none());
    let local = item.local_sweep.as_ref().expect("local");
    assert_eq!(rectangle(&local.profile), (1.0, 1.0));
    assert!(matches!(
        local.path,
        LocalSweepPath::Extrusion { direction, depth, .. }
            if close(direction, [0.0, 0.0, 1.0]) && (depth - 2.0).abs() < 1e-12
    ));
}

/// A mirrored operator (`Axis2 = -Y` against `Axis1 = X`, `Axis3 = Z`: the
/// derived frame keeps the sign) has no world form either.
#[test]
fn a_mirrored_mapped_extrusion_is_described_locally() {
    let model = metres(
        "#100=IFCRECTANGLEPROFILEDEF(.AREA.,$,$,1.,1.);
         #101=IFCCARTESIANPOINT((0.,0.,0.));
         #102=IFCAXIS2PLACEMENT3D(#101,$,$);
         #103=IFCDIRECTION((0.,0.,1.));
         #104=IFCEXTRUDEDAREASOLID(#100,$,#103,1.);
         #105=IFCDIRECTION((0.,-1.,0.));
         #106=IFCDIRECTION((1.,0.,0.));
         #200=IFCSHAPEREPRESENTATION(#10,'Body','SweptSolid',(#104));
         #201=IFCREPRESENTATIONMAP(#102,#200);
         #202=IFCCARTESIANTRANSFORMATIONOPERATOR3D(#106,#105,#101,$,$);
         #203=IFCMAPPEDITEM(#201,#202);",
        "#203",
    );
    let body = describe(&model);
    let item = body.sole_item().unwrap();
    assert_eq!(item.is_mirrored(), Some(true));
    assert!(item.swept.is_none());
    let local = item.local_sweep.as_ref().expect("local");
    assert_eq!(rectangle(&local.profile), (1.0, 1.0));
    assert!(matches!(
        local.path,
        LocalSweepPath::Extrusion { direction: [0.0, 0.0, 1.0], depth, .. } if depth == 1.0
    ));
}

/// A revolution in a degree model: axis and angle are local and in radians.
#[test]
fn a_revolution_is_local_with_its_angle_in_radians() {
    let model = degrees(
        "#100=IFCCIRCLEPROFILEDEF(.AREA.,$,$,0.05);
         #101=IFCAXIS1PLACEMENT(#102,#103);
         #102=IFCCARTESIANPOINT((0.,-0.5,0.));
         #103=IFCDIRECTION((2.,0.,0.));
         #104=IFCREVOLVEDAREASOLID(#100,$,#101,90.);",
        "#104",
    );
    let body = describe(&model);
    let item = body.sole_item().unwrap();
    assert_eq!(item.kind, BodyKind::Revolution);
    let local = item.local_sweep.as_ref().unwrap();
    assert!(local.position.is_none());
    assert!(matches!(
        local.path,
        LocalSweepPath::Revolution { axis_origin, axis_direction, angle, .. }
            if close(axis_origin, [0.0, -0.5, 0.0])
                && close(axis_direction, [1.0, 0.0, 0.0])
                && (angle - std::f64::consts::FRAC_PI_2).abs() < 1e-12
    ));
}

/// A fixed-reference sweep names its directrix.
#[test]
fn a_fixed_reference_sweep_is_local_by_directrix() {
    let model = metres(
        "#104=IFCCARTESIANPOINT((0.,0.,0.));
         #105=IFCCARTESIANPOINT((5.,0.,0.));
         #106=IFCPOLYLINE((#104,#105));
         #107=IFCDIRECTION((0.,0.,1.));
         #108=IFCCIRCLEPROFILEDEF(.AREA.,$,$,0.1);
         #109=IFCFIXEDREFERENCESWEPTAREASOLID(#108,$,#106,$,$,#107);",
        "#109",
    );
    let body = describe(&model);
    let local = body.sole_item().unwrap().local_sweep.as_ref().unwrap();
    assert!(matches!(
        local.path,
        LocalSweepPath::Directrix { directrix, .. } if directrix == ifc_model::EntityId(106)
    ));
}

/// The IFC4X3 directrix-derived sweep has the supertype's slots.
#[test]
fn a_directrix_derived_sweep_is_local_by_directrix() {
    let model = super::step::model(
        "IFC4X3_ADD2",
        "$",
        "1.,0.,0.",
        "#104=IFCCARTESIANPOINT((0.,0.,0.));
         #105=IFCCARTESIANPOINT((5.,0.,0.));
         #106=IFCPOLYLINE((#104,#105));
         #107=IFCDIRECTION((0.,0.,1.));
         #108=IFCCIRCLEPROFILEDEF(.AREA.,$,$,0.1);
         #109=IFCDIRECTRIXDERIVEDREFERENCESWEPTAREASOLID(#108,$,#106,$,$,#107);",
        "#109",
    );
    let body = describe(&model);
    let item = body.sole_item().unwrap();
    assert_eq!(item.kind, BodyKind::DirectrixSweep);
    assert!(matches!(
        item.local_sweep.as_ref().unwrap().path,
        LocalSweepPath::Directrix { directrix, .. } if directrix == ifc_model::EntityId(106)
    ));
}

/// The revolution axis is normalised once, as before #640: this value is what
/// the world description produced before the local description existed.
#[test]
fn a_revolution_axis_in_a_rotated_frame_keeps_its_bits() {
    let model = model(
        "IFC4",
        "$",
        "1.,1.,0.",
        "#100=IFCCIRCLEPROFILEDEF(.AREA.,$,$,0.05);
         #101=IFCAXIS1PLACEMENT(#102,#103);
         #102=IFCCARTESIANPOINT((0.,0.,0.));
         #103=IFCDIRECTION((0.,1.,3.));
         #104=IFCREVOLVEDAREASOLID(#100,$,#101,1.5);",
        "#104",
    );
    let body = describe(&model);
    let swept = body.sole_item().unwrap().swept.as_ref().unwrap();
    let ifc_geometry::SweepPath::Revolution {
        axis_direction_world,
        ..
    } = swept.path
    else {
        panic!("revolution");
    };
    assert_eq!(axis_direction_world[0], -0.22360679774997894);
}
