//! Where a layer set usage measures from: the reference line of a wall and
//! the profile plane of a slab (#455).
//!
//! Kernel-free: nothing here links a solid kernel. Models are small STEP
//! texts built by `layer_set_reference/step.rs`; expected values come from
//! the quoted records.

#[path = "layer_set_reference/step.rs"]
mod step;

use ifc_geometry::{
    layer_set_reference, units, DirectionSense, GeometryError, LayerSetDirection, LayerSetReference,
};
use ifc_model::{EntityId, Model};
use step::{axis, extrusion, model, plain};

const PRODUCT: EntityId = EntityId(30);

fn reference(
    model: &Model,
    direction: LayerSetDirection,
) -> Result<LayerSetReference, GeometryError> {
    layer_set_reference(
        model,
        &units::resolve(model).expect("project units"),
        PRODUCT,
        direction,
    )
}

fn wall(body: &str, axis_items: Option<&str>, entities: &str) -> Model {
    model("IFCWALL", body, axis_items, entities)
}

fn close(actual: [f64; 3], expected: [f64; 3]) {
    for (a, e) in actual.iter().zip(expected) {
        assert!((a - e).abs() < 1e-9, "{actual:?} != {expected:?}");
    }
}

/// A wall whose axis is `#40`, y = `y`, and whose body is `#104`.
fn plain_wall(y: f64) -> Model {
    wall("#104", Some("#40"), &format!("{}{}", plain(0.0), axis(y)))
}

#[test]
fn a_wall_axis_on_the_local_x_axis_is_the_zero_reference() {
    let m = plain_wall(0.0);
    let r = reference(&m, LayerSetDirection::Axis2).expect("reference");
    assert_eq!(r.normal, [0.0, 1.0, 0.0]);
    assert_eq!(r.distance, 0.0);
    assert_eq!(r.source, EntityId(40));
    assert_eq!(r.product, PRODUCT);
}

#[test]
fn a_wall_axis_off_the_origin_shifts_the_reference_in_metres() {
    let r = reference(&plain_wall(100.0), LayerSetDirection::Axis2).expect("reference");
    assert!((r.distance - 0.1).abs() < 1e-12);
    let b = r
        .boundaries(
            &units::resolve(&plain_wall(100.0)).unwrap(),
            DirectionSense::Positive,
            0.0,
            &[200.0],
        )
        .expect("bounds");
    assert!((b.boundaries[0] - 0.1).abs() < 1e-12 && (b.boundaries[1] - 0.3).abs() < 1e-12);
}

#[test]
fn clipping_chains_are_walked_to_the_extrusion() {
    let one = "#110=IFCPLANE(#102);
         #111=IFCHALFSPACESOLID(#110,.F.);
         #112=IFCBOOLEANCLIPPINGRESULT(.DIFFERENCE.,#104,#111);";
    let two = "#113=IFCBOOLEANCLIPPINGRESULT(.DIFFERENCE.,#112,#111);";
    for (body, extra) in [("#112", one.to_string()), ("#113", format!("{one}{two}"))] {
        let m = wall(
            body,
            Some("#40"),
            &format!("{}{}{extra}", plain(0.0), axis(0.0)),
        );
        let r = reference(&m, LayerSetDirection::Axis2).expect("reference");
        assert_eq!(r.source, EntityId(40));
    }
}

#[test]
fn a_slab_measures_from_the_profile_plane_along_the_position_z() {
    let m = model("IFCSLAB", "#104", None, &plain(-280.0));
    let r = reference(&m, LayerSetDirection::Axis3).expect("reference");
    close(r.normal, [0.0, 0.0, 1.0]);
    assert!((r.distance + 0.28).abs() < 1e-12);
    assert_eq!(r.source, EntityId(104));
}

#[test]
fn a_tilted_position_sets_the_normal_whatever_the_extrusion_does() {
    let s = 0.866_025_403_784_438_6;
    let m = model(
        "IFCSLAB",
        "#104",
        None,
        &format!(
            "#100=IFCRECTANGLEPROFILEDEF(.AREA.,'R',$,200.,100.);
             #101=IFCCARTESIANPOINT((0.,0.,1000.));
             #105=IFCDIRECTION((0.,-0.5,{s}));
             #106=IFCDIRECTION((1.,0.,0.));
             #102=IFCAXIS2PLACEMENT3D(#101,#105,#106);
             #103=IFCDIRECTION((0.,0.5,{s}));
             #104=IFCEXTRUDEDAREASOLID(#100,#102,#103,2800.);"
        ),
    );
    let r = reference(&m, LayerSetDirection::Axis3).expect("reference");
    close(r.normal, [0.0, -0.5, s]);
    assert!((r.distance - s).abs() < 1e-12);
}

fn refusal(model: &Model, direction: LayerSetDirection) -> GeometryError {
    reference(model, direction).expect_err("refused")
}

fn assert_unsupported(err: &GeometryError, id: u64, needle: &str) {
    match err {
        GeometryError::Unsupported { entity, detail, .. } => {
            assert_eq!(*entity, EntityId(id), "{err}");
            assert!(detail.contains(needle), "{err}");
        }
        other => panic!("expected Unsupported, got {other:?}"),
    }
}

fn assert_degenerate(err: &GeometryError, id: u64, needle: &str) {
    match err {
        GeometryError::Degenerate { entity, detail, .. } => {
            assert_eq!(*entity, EntityId(id), "{err}");
            assert!(detail.contains(needle), "{err}");
        }
        other => panic!("expected Degenerate, got {other:?}"),
    }
}

#[test]
fn axis1_and_mismatched_classes_are_unsupported() {
    let w = plain_wall(0.0);
    assert_unsupported(&refusal(&w, LayerSetDirection::Axis1), 30, "AXIS1");
    assert_unsupported(
        &refusal(&w, LayerSetDirection::Axis3),
        30,
        "AXIS2 for IfcWall",
    );
    let slab = model("IFCSLAB", "#104", None, &plain(0.0));
    assert_unsupported(
        &refusal(&slab, LayerSetDirection::Axis2),
        30,
        "AXIS3 for IfcSlab",
    );
    for class in ["IFCWALLELEMENTEDCASE", "IFCBEAM"] {
        let m = model(
            class,
            "#104",
            Some("#40"),
            &format!("{}{}", plain(0.0), axis(0.0)),
        );
        assert_unsupported(
            &refusal(&m, LayerSetDirection::Axis2),
            30,
            "interpreted along",
        );
    }
}

#[test]
fn a_missing_product_is_a_missing_entity() {
    let m = plain_wall(0.0);
    let err = layer_set_reference(
        &m,
        &units::resolve(&m).unwrap(),
        EntityId(999),
        LayerSetDirection::Axis2,
    )
    .expect_err("refused");
    assert!(matches!(
        err,
        GeometryError::MissingEntity { referrer, missing }
            if referrer == EntityId(999) && missing == EntityId(999)
    ));
}

#[test]
fn a_product_without_a_body_is_a_missing_attribute() {
    let m = step::without_body("IFCWALL", &plain(0.0));
    match refusal(&m, LayerSetDirection::Axis2) {
        GeometryError::MissingAttribute {
            entity, attribute, ..
        } => {
            assert_eq!(entity, PRODUCT);
            assert_eq!(attribute, "Body representation");
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn an_empty_body_is_unsupported() {
    let m = wall("", Some("#40"), &format!("{}{}", plain(0.0), axis(0.0)));
    assert_unsupported(
        &refusal(&m, LayerSetDirection::Axis2),
        20,
        "at least one item",
    );
}

/// A second extrusion `#110` over the same profile, at `location` along
/// `direction`.
fn second(location: &str, direction: &str) -> String {
    format!(
        "#120=IFCCARTESIANPOINT(({location}));
         #121=IFCAXIS2PLACEMENT3D(#120,$,$);
         #122=IFCDIRECTION(({direction}));
         #110=IFCEXTRUDEDAREASOLID(#100,#121,#122,1400.);"
    )
}

#[test]
fn coaxial_body_items_share_one_reference() {
    // An interrupted wall: two pieces, same orientation and profile plane,
    // the second one shorter.
    let m = wall(
        "#104,#110",
        Some("#40"),
        &format!(
            "{}{}{}",
            plain(0.0),
            axis(7.0),
            second("0.,0.,0.", "0.,0.,1.")
        ),
    );
    let r = reference(&m, LayerSetDirection::Axis2).expect("coaxial items");
    close(r.normal, [0.0, 1.0, 0.0]);
    assert!((r.distance - 0.007).abs() < 1e-9, "{}", r.distance);
}

#[test]
fn a_body_item_with_another_position_is_unsupported_and_named() {
    let m = wall(
        "#104,#110",
        Some("#40"),
        &format!(
            "{}{}{}",
            plain(0.0),
            axis(0.0),
            second("0.,0.,500.", "0.,0.,1.")
        ),
    );
    assert_unsupported(&refusal(&m, LayerSetDirection::Axis2), 110, "not coaxial");
}

#[test]
fn a_body_item_with_a_turned_position_is_unsupported_and_named() {
    // Same origin, z and ExtrudedDirection, but the x axis turned 90 degrees.
    let turned = "#120=IFCCARTESIANPOINT((0.,0.,0.));
         #121=IFCDIRECTION((0.,1.,0.));
         #122=IFCAXIS2PLACEMENT3D(#120,$,#121);
         #123=IFCDIRECTION((0.,0.,1.));
         #110=IFCEXTRUDEDAREASOLID(#100,#122,#123,1400.);";
    let m = wall(
        "#104,#110",
        Some("#40"),
        &format!("{}{}{turned}", plain(0.0), axis(0.0)),
    );
    assert_unsupported(&refusal(&m, LayerSetDirection::Axis2), 110, "not coaxial");
}

#[test]
fn a_body_item_shifted_across_the_wall_layers_is_unsupported_and_named() {
    let m = wall(
        "#104,#110",
        Some("#40"),
        &format!(
            "{}{}{}",
            plain(0.0),
            axis(0.0),
            second("0.,300.,0.", "0.,0.,1.")
        ),
    );
    assert_unsupported(&refusal(&m, LayerSetDirection::Axis2), 110, "not coaxial");
    // Along the wall and up it is fine.
    let m = wall(
        "#104,#110",
        Some("#40"),
        &format!(
            "{}{}{}",
            plain(0.0),
            axis(0.0),
            second("3000.,0.,0.", "0.,0.,1.")
        ),
    );
    reference(&m, LayerSetDirection::Axis2).expect("slid along the wall");
}

#[test]
fn a_slab_of_several_coaxial_items_shares_its_plane_and_refuses_another() {
    let m = model(
        "IFCSLAB",
        "#104,#110",
        None,
        &format!("{}{}", plain(0.0), second("3000.,300.,0.", "0.,0.,1.")),
    );
    let r = reference(&m, LayerSetDirection::Axis3).expect("coplanar items");
    close(r.normal, [0.0, 0.0, 1.0]);
    let m = model(
        "IFCSLAB",
        "#104,#110",
        None,
        &format!("{}{}", plain(0.0), second("0.,0.,50.", "0.,0.,1.")),
    );
    assert_unsupported(&refusal(&m, LayerSetDirection::Axis3), 110, "not coaxial");
}

#[test]
fn a_clipped_item_is_named_by_its_body_item_not_its_extrusion() {
    // #130 clips #110, which is off the plane of #104.
    let clipped = "#131=IFCCARTESIANPOINT((0.,0.,0.));
         #132=IFCAXIS2PLACEMENT3D(#131,$,$);
         #133=IFCPLANE(#132);
         #134=IFCHALFSPACESOLID(#133,.F.);
         #130=IFCBOOLEANCLIPPINGRESULT(.DIFFERENCE.,#110,#134);";
    let m = wall(
        "#104,#130",
        Some("#40"),
        &format!(
            "{}{}{}{clipped}",
            plain(0.0),
            axis(0.0),
            second("0.,300.,0.", "0.,0.,1.")
        ),
    );
    assert_unsupported(&refusal(&m, LayerSetDirection::Axis2), 130, "not coaxial");
    // A clipped coaxial item is accepted.
    let m = wall(
        "#104,#130",
        Some("#40"),
        &format!(
            "{}{}{}{clipped}",
            plain(0.0),
            axis(0.0),
            second("3000.,0.,0.", "0.,0.,1.")
        ),
    );
    reference(&m, LayerSetDirection::Axis2).expect("clipped coaxial item");
}

#[test]
fn a_body_item_with_another_direction_is_unsupported_and_named() {
    let m = wall(
        "#104,#110",
        Some("#40"),
        &format!(
            "{}{}{}",
            plain(0.0),
            axis(0.0),
            second("0.,0.,0.", "1.,0.,0.")
        ),
    );
    assert_unsupported(&refusal(&m, LayerSetDirection::Axis2), 110, "not coaxial");
}

#[test]
fn other_body_items_are_unsupported_and_name_the_item() {
    let tapered = "#110=IFCEXTRUDEDAREASOLIDTAPERED(#100,#102,#103,2800.,#100);";
    let m = wall(
        "#110",
        Some("#40"),
        &format!("{}{}{tapered}", plain(0.0), axis(0.0)),
    );
    assert_unsupported(
        &refusal(&m, LayerSetDirection::Axis2),
        110,
        "IfcExtrudedAreaSolid",
    );
    let mapped = "#110=IFCCARTESIANPOINT((0.,0.,0.));
         #111=IFCAXIS2PLACEMENT3D(#110,$,$);
         #112=IFCREPRESENTATIONMAP(#111,#20);
         #113=IFCCARTESIANTRANSFORMATIONOPERATOR3D($,$,#110,$,$);
         #114=IFCMAPPEDITEM(#112,#113);";
    let m = wall(
        "#114",
        Some("#40"),
        &format!("{}{}{mapped}", plain(0.0), axis(0.0)),
    );
    assert_unsupported(
        &refusal(&m, LayerSetDirection::Axis2),
        114,
        "IfcExtrudedAreaSolid",
    );
}

#[test]
fn a_union_clipping_is_degenerate() {
    let m = wall(
        "#112",
        Some("#40"),
        &format!(
            "{}{}#110=IFCPLANE(#102);
             #111=IFCHALFSPACESOLID(#110,.F.);
             #112=IFCBOOLEANCLIPPINGRESULT(.UNION.,#104,#111);",
            plain(0.0),
            axis(0.0)
        ),
    );
    assert_degenerate(&refusal(&m, LayerSetDirection::Axis2), 112, "DIFFERENCE");
}

#[test]
fn a_clipping_chain_past_the_limit_is_too_deep() {
    let mut chain = format!(
        "{}{}#110=IFCPLANE(#102);
         #111=IFCHALFSPACESOLID(#110,.F.);
         #200=IFCBOOLEANCLIPPINGRESULT(.DIFFERENCE.,#104,#111);",
        plain(0.0),
        axis(0.0)
    );
    for i in 201..=264 {
        chain.push_str(&format!(
            "#{i}=IFCBOOLEANCLIPPINGRESULT(.DIFFERENCE.,#{},#111);",
            i - 1
        ));
    }
    // 65 nested clippings: #200..=#264.
    let m = wall("#264", Some("#40"), &chain);
    assert!(matches!(
        refusal(&m, LayerSetDirection::Axis2),
        GeometryError::ChainTooDeep {
            entity: EntityId(264),
            kind: "boolean clipping",
            limit: 64,
            ..
        }
    ));
    // 64 is still walked.
    let m = wall("#263", Some("#40"), &chain);
    reference(&m, LayerSetDirection::Axis2).expect("64 clippings are fine");
}

#[test]
fn a_leaning_wall_extrusion_is_unsupported() {
    let m = wall(
        "#104",
        Some("#40"),
        &format!(
            "{}{}",
            extrusion("0.,0.,0.", "$", "$", "0.,0.1,0.995"),
            axis(0.0)
        ),
    );
    assert_unsupported(
        &refusal(&m, LayerSetDirection::Axis2),
        104,
        "neither perpendicular",
    );
}

#[test]
fn wall_axis_problems_are_refused() {
    let body = plain(0.0);
    let no_axis = wall("#104", None, &body);
    match refusal(&no_axis, LayerSetDirection::Axis2) {
        GeometryError::MissingAttribute {
            entity, attribute, ..
        } => {
            assert_eq!(entity, PRODUCT);
            assert_eq!(attribute, "Axis representation");
        }
        other => panic!("{other:?}"),
    }
    let two = wall("#104", Some("#40,#40"), &format!("{body}{}", axis(0.0)));
    assert_unsupported(
        &refusal(&two, LayerSetDirection::Axis2),
        22,
        "exactly one curve",
    );
    let trimmed = format!(
        "{body}#50=IFCCIRCLE(#102,500.);
         #51=IFCTRIMMEDCURVE(#50,(IFCPARAMETERVALUE(0.)),(IFCPARAMETERVALUE(1.)),.T.,.PARAMETER.);"
    );
    let m = wall("#104", Some("#51"), &trimmed);
    assert_unsupported(&refusal(&m, LayerSetDirection::Axis2), 51, "IfcPolyline");
    let slanted = format!(
        "{body}#41=IFCCARTESIANPOINT((0.,0.));
         #42=IFCCARTESIANPOINT((5000.,500.));
         #40=IFCPOLYLINE((#41,#42));"
    );
    let m = wall("#104", Some("#40"), &slanted);
    assert_unsupported(&refusal(&m, LayerSetDirection::Axis2), 40, "parallel");
    let point = format!(
        "{body}#41=IFCCARTESIANPOINT((100.,0.));
         #42=IFCCARTESIANPOINT((100.,0.));
         #40=IFCPOLYLINE((#41,#42));"
    );
    let m = wall("#104", Some("#40"), &point);
    assert_degenerate(&refusal(&m, LayerSetDirection::Axis2), 40, "no extent");
}

#[test]
fn a_slab_extruded_within_its_profile_plane_is_degenerate() {
    let m = model(
        "IFCSLAB",
        "#104",
        None,
        &extrusion("0.,0.,0.", "$", "$", "1.,0.,0."),
    );
    assert_degenerate(&refusal(&m, LayerSetDirection::Axis3), 104, "profile plane");
}

#[test]
fn a_wall_extruded_through_its_thickness_is_accepted() {
    // An edited profile: the Position's z is -y, so the profile lies in the
    // wall's elevation plane and the extrusion runs along the local y axis.
    let m = wall(
        "#104",
        Some("#40"),
        &format!(
            "{}{}",
            extrusion("0.,0.,0.", "#105", "#106", "0.,0.,1.").replace(
                "#103=",
                "#105=IFCDIRECTION((0.,-1.,0.));\n#106=IFCDIRECTION((1.,0.,0.));\n#103="
            ),
            axis(0.0)
        ),
    );
    let r = reference(&m, LayerSetDirection::Axis2).expect("reference");
    assert_eq!(r.normal, [0.0, 1.0, 0.0]);
    assert_eq!(r.source, EntityId(40));
}

#[test]
fn an_absent_position_is_the_identity() {
    let m = model(
        "IFCSLAB",
        "#104",
        None,
        "#100=IFCRECTANGLEPROFILEDEF(.AREA.,'R',$,200.,100.);
         #103=IFCDIRECTION((0.,0.,1.));
         #104=IFCEXTRUDEDAREASOLID(#100,$,#103,2800.);",
    );
    let r = reference(&m, LayerSetDirection::Axis3).expect("reference");
    close(r.normal, [0.0, 0.0, 1.0]);
    assert_eq!(r.distance, 0.0);
}
