//! Profile families described in SI, one test per family.
//!
//! The steel sections come from `synthetic_profile_families.ifc` (millimetres
//! and degrees), so each assertion also proves the unit conversion; the
//! remaining families are authored inline.

use ifc_geometry::{describe_profile, units, ProfileDescription, ProfileParameters};
use ifc_model::{EntityId, Model};

use super::profile_families;
use super::step::{close, metres, model};

/// Describe entity `id` of `model` with the model's own units.
fn describe(model: &Model, id: u64) -> ProfileDescription {
    describe_profile(model, &units::resolve(model), EntityId(id))
        .unwrap_or_else(|error| panic!("#{id} describes: {error}"))
}

const DEG: f64 = std::f64::consts::PI / 180.0;

/// `#11=IFCISHAPEPROFILEDEF(.AREA.,'HEA300-like',$,300.,290.,8.5,14.,27.,3.,2.)`
#[test]
fn i_shape() {
    let profile = describe(&profile_families(), 11);
    assert_eq!(profile.name.as_deref(), Some("HEA300-like"));
    assert_eq!(profile.position, None, "Position `$` is the identity");
    let ProfileParameters::IShape {
        overall_width,
        overall_depth,
        web_thickness,
        flange_thickness,
        fillet_radius,
        flange_edge_radius,
        flange_slope,
        ..
    } = profile.parameters
    else {
        panic!("{:?}", profile.parameters);
    };
    assert!(close(
        [
            overall_width,
            overall_depth,
            web_thickness,
            flange_thickness
        ],
        [0.3, 0.29, 0.0085, 0.014]
    ));
    assert!(close(
        [fillet_radius.unwrap(), flange_edge_radius.unwrap()],
        [0.027, 0.003]
    ));
    assert!(
        (flange_slope.unwrap() - 2.0 * DEG).abs() < 1e-12,
        "a slope is an angle: degrees to radians, never the length factor"
    );
}

/// `#12=IFCASYMMETRICISHAPEPROFILEDEF(.AREA.,'asym-I',$,300.,290.,8.5,14.,27.,200.,12.,21.,$,$,$,$)`
#[test]
fn asymmetric_i_shape() {
    let ProfileParameters::AsymmetricIShape {
        bottom_flange_width,
        overall_depth,
        top_flange_width,
        top_flange_thickness,
        top_flange_fillet_radius,
        bottom_flange_edge_radius,
        ..
    } = describe(&profile_families(), 12).parameters
    else {
        panic!("asymmetric I expected");
    };
    assert!(close(
        [bottom_flange_width, overall_depth, top_flange_width],
        [0.3, 0.29, 0.2]
    ));
    assert_eq!(top_flange_thickness.map(|v| (v * 1e4).round()), Some(120.0));
    assert_eq!(
        top_flange_fillet_radius.map(|v| (v * 1e4).round()),
        Some(210.0)
    );
    assert_eq!(bottom_flange_edge_radius, None);
}

/// IFC2X3 declares the asymmetric I as an I-shape subtype whose slot 11 is
/// `CentreOfGravityInY`. Reading the IFC4 layout would report it as the
/// bottom flange edge radius.
#[test]
fn asymmetric_i_shape_in_ifc2x3_does_not_read_the_centre_of_gravity_as_a_radius() {
    let entities =
        "#100=IFCASYMMETRICISHAPEPROFILEDEF(.AREA.,$,#101,300.,290.,8.5,14.,27.,200.,12.,21.,150.);
                    #101=IFCAXIS2PLACEMENT2D(#102,$);
                    #102=IFCCARTESIANPOINT((0.,0.));";
    let ifc2x3 = model("IFC2X3", "$", "1.,0.,0.", entities, "");
    let ProfileParameters::AsymmetricIShape {
        bottom_flange_edge_radius,
        top_flange_width,
        ..
    } = describe(&ifc2x3, 100).parameters
    else {
        panic!("asymmetric I expected");
    };
    assert_eq!(
        bottom_flange_edge_radius, None,
        "slot 11 is CentreOfGravityInY"
    );
    assert_eq!(top_flange_width, 200.0);

    // The same record declared IFC4 reads slot 11 as the edge radius.
    let ifc4 = model("IFC4", "$", "1.,0.,0.", entities, "");
    let ProfileParameters::AsymmetricIShape {
        bottom_flange_edge_radius,
        ..
    } = describe(&ifc4, 100).parameters
    else {
        panic!("asymmetric I expected");
    };
    assert_eq!(bottom_flange_edge_radius, Some(150.0));
}

/// `#13=IFCLSHAPEPROFILEDEF(.AREA.,'L150x100x10',$,150.,100.,10.,12.,6.,$)`
#[test]
fn l_shape() {
    let ProfileParameters::LShape {
        depth,
        width,
        thickness,
        fillet_radius,
        edge_radius,
        leg_slope,
        ..
    } = describe(&profile_families(), 13).parameters
    else {
        panic!("L expected");
    };
    assert!(close(
        [
            depth,
            width.unwrap(),
            thickness,
            fillet_radius.unwrap(),
            edge_radius.unwrap()
        ],
        [0.15, 0.1, 0.01, 0.012, 0.006]
    ));
    assert_eq!(leg_slope, None);
}

/// `#14=IFCTSHAPEPROFILEDEF(.AREA.,'T200x200x12',$,200.,200.,12.,15.,18.,4.,3.,$,$)`
#[test]
fn t_shape() {
    let ProfileParameters::TShape {
        depth,
        flange_width,
        web_thickness,
        flange_thickness,
        fillet_radius,
        flange_edge_radius,
        web_edge_radius,
        ..
    } = describe(&profile_families(), 14).parameters
    else {
        panic!("T expected");
    };
    assert!(close(
        [
            depth,
            flange_width,
            web_thickness,
            flange_thickness,
            fillet_radius.unwrap(),
            flange_edge_radius.unwrap(),
            web_edge_radius.unwrap()
        ],
        [0.2, 0.2, 0.012, 0.015, 0.018, 0.004, 0.003]
    ));
}

/// `#15=IFCUSHAPEPROFILEDEF(.AREA.,'UPN200',$,200.,75.,8.5,11.5,11.5,6.,$)`
#[test]
fn u_shape() {
    let ProfileParameters::UShape {
        depth,
        flange_width,
        web_thickness,
        flange_thickness,
        fillet_radius,
        edge_radius,
        flange_slope,
        ..
    } = describe(&profile_families(), 15).parameters
    else {
        panic!("U expected");
    };
    assert!(close(
        [
            depth,
            flange_width,
            web_thickness,
            flange_thickness,
            fillet_radius.unwrap(),
            edge_radius.unwrap()
        ],
        [0.2, 0.075, 0.0085, 0.0115, 0.0115, 0.006]
    ));
    assert_eq!(flange_slope, None);
}

/// `#16=IFCCSHAPEPROFILEDEF(.AREA.,'C200x75x20',$,200.,75.,2.5,20.,5.)`
#[test]
fn c_shape() {
    let ProfileParameters::CShape {
        depth,
        width,
        wall_thickness,
        girth,
        internal_fillet_radius,
        ..
    } = describe(&profile_families(), 16).parameters
    else {
        panic!("C expected");
    };
    assert!(close(
        [
            depth,
            width,
            wall_thickness,
            girth,
            internal_fillet_radius.unwrap()
        ],
        [0.2, 0.075, 0.0025, 0.02, 0.005]
    ));
}

/// `#17=IFCZSHAPEPROFILEDEF(.AREA.,'Z200x75',$,200.,75.,2.5,2.5,5.,3.)`
#[test]
fn z_shape() {
    let ProfileParameters::ZShape {
        depth,
        flange_width,
        web_thickness,
        flange_thickness,
        fillet_radius,
        edge_radius,
        ..
    } = describe(&profile_families(), 17).parameters
    else {
        panic!("Z expected");
    };
    assert!(close(
        [
            depth,
            flange_width,
            web_thickness,
            flange_thickness,
            fillet_radius.unwrap(),
            edge_radius.unwrap()
        ],
        [0.2, 0.075, 0.0025, 0.0025, 0.005, 0.003]
    ));
}

/// `#18=IFCELLIPSEPROFILEDEF(.AREA.,'ellipse',$,200.,120.)`
#[test]
fn ellipse() {
    let ProfileParameters::Ellipse {
        semi_axis_1,
        semi_axis_2,
        ..
    } = describe(&profile_families(), 18).parameters
    else {
        panic!("ellipse expected");
    };
    assert!(close([semi_axis_1, semi_axis_2], [0.2, 0.12]));
}

/// `#19=IFCTRAPEZIUMPROFILEDEF(.AREA.,'trapezium',$,300.,180.,150.,-40.)`
#[test]
fn trapezium_keeps_its_negative_offset() {
    let ProfileParameters::Trapezium {
        bottom_x_dim,
        top_x_dim,
        y_dim,
        top_x_offset,
        ..
    } = describe(&profile_families(), 19).parameters
    else {
        panic!("trapezium expected");
    };
    assert!(close(
        [bottom_x_dim, top_x_dim, y_dim, top_x_offset],
        [0.3, 0.18, 0.15, -0.04]
    ));
}

/// `#22=IFCDERIVEDPROFILEDEF(.AREA.,'derived-2x',#11,#21,'scaled')` with
/// `#21` scaling by 2 and translating by (50, 25) mm.
///
/// The translation is a length: 0.05 m, not 50.
#[test]
fn derived_profile_converts_its_operator_translation() {
    let profile = describe(&profile_families(), 22);
    let ProfileParameters::Derived {
        ref parent,
        operator,
        ref label,
        ..
    } = profile.parameters
    else {
        panic!("derived expected");
    };
    assert_eq!(parent.entity, EntityId(11));
    assert_eq!(label.as_deref(), Some("scaled"));
    assert!(
        close(operator.origin, [0.05, 0.025]),
        "{:?}",
        operator.origin
    );
    assert!(close(operator.x_axis, [2.0, 0.0]));
    assert!(close(operator.y_axis, [0.0, 2.0]));
}

/// `#23=IFCMIRROREDPROFILEDEF(.AREA.,'mirrored-L',#11,*,'mirrored')`
#[test]
fn mirrored_profile() {
    let ProfileParameters::Mirrored { parent, label, .. } =
        describe(&profile_families(), 23).parameters
    else {
        panic!("mirrored expected");
    };
    assert_eq!(parent.entity, EntityId(11));
    assert_eq!(label.as_deref(), Some("mirrored"));
}

/// `#26=IFCCOMPOSITEPROFILEDEF(.AREA.,'composite',(#24,#25),'two-part')`
#[test]
fn composite_profile_keeps_member_order() {
    let ProfileParameters::Composite {
        profiles, label, ..
    } = describe(&profile_families(), 26).parameters
    else {
        panic!("composite expected");
    };
    assert_eq!(label.as_deref(), Some("two-part"));
    let members: Vec<EntityId> = profiles.iter().map(|p| p.entity).collect();
    assert_eq!(members, [EntityId(24), EntityId(25)]);
}

/// `#31=IFCCENTERLINEPROFILEDEF(.AREA.,'centerline',#30,8.)`: FULL width.
#[test]
fn center_line_profile() {
    let ProfileParameters::CenterLine {
        curve, thickness, ..
    } = describe(&profile_families(), 31).parameters
    else {
        panic!("centre line expected");
    };
    assert_eq!(curve, EntityId(30));
    assert!((thickness - 0.008).abs() < 1e-12);
}

/// The rectangle and circle families, including hollow and rounded forms.
#[test]
fn rectangle_and_circle_families() {
    let model = model(
        "IFC4",
        ".MILLI.",
        "1.,0.,0.",
        "#100=IFCRECTANGLEPROFILEDEF(.AREA.,$,$,200.,100.);
         #101=IFCROUNDEDRECTANGLEPROFILEDEF(.AREA.,$,$,200.,100.,10.);
         #102=IFCRECTANGLEHOLLOWPROFILEDEF(.AREA.,$,$,200.,100.,8.,4.,12.);
         #103=IFCCIRCLEPROFILEDEF(.AREA.,$,$,50.);
         #104=IFCCIRCLEHOLLOWPROFILEDEF(.AREA.,$,$,50.,5.);",
        "",
    );
    assert!(matches!(
        describe(&model, 100).parameters,
        ProfileParameters::Rectangle { x_dim, y_dim, .. } if close([x_dim, y_dim], [0.2, 0.1])
    ));
    assert!(matches!(
        describe(&model, 101).parameters,
        ProfileParameters::RoundedRectangle { rounding_radius, .. }
            if (rounding_radius - 0.01).abs() < 1e-12
    ));
    let ProfileParameters::RectangleHollow {
        wall_thickness,
        inner_fillet_radius,
        outer_fillet_radius,
        ..
    } = describe(&model, 102).parameters
    else {
        panic!("hollow rectangle expected");
    };
    assert!(close(
        [
            wall_thickness,
            inner_fillet_radius.unwrap(),
            outer_fillet_radius.unwrap()
        ],
        [0.008, 0.004, 0.012]
    ));
    assert!(matches!(
        describe(&model, 103).parameters,
        ProfileParameters::Circle { radius, .. } if (radius - 0.05).abs() < 1e-12
    ));
    assert!(matches!(
        describe(&model, 104).parameters,
        ProfileParameters::CircleHollow { radius, wall_thickness, .. }
            if close([radius, wall_thickness], [0.05, 0.005])
    ));
}

/// Arbitrary profiles are reported by kind, with their curves by reference.
#[test]
fn arbitrary_profiles_name_their_curves() {
    let model = metres(
        "#100=IFCCARTESIANPOINT((0.,0.));
         #101=IFCCARTESIANPOINT((1.,0.));
         #102=IFCCARTESIANPOINT((0.,1.));
         #103=IFCPOLYLINE((#100,#101,#102,#100));
         #104=IFCPOLYLINE((#100,#101,#102,#100));
         #105=IFCARBITRARYCLOSEDPROFILEDEF(.AREA.,'slab',#103);
         #106=IFCARBITRARYPROFILEDEFWITHVOIDS(.AREA.,$,#103,(#104));
         #107=IFCPOLYLINE((#100,#101));
         #108=IFCARBITRARYOPENPROFILEDEF(.CURVE.,$,#107);",
        "",
    );
    let closed = describe(&model, 105);
    assert!(matches!(
        closed.parameters,
        ProfileParameters::ArbitraryClosed { outer_curve, .. } if outer_curve == EntityId(103)
    ));
    assert!(closed.bounds_area());
    assert!(matches!(
        describe(&model, 106).parameters,
        ProfileParameters::ArbitraryWithVoids { ref inner_curves, .. }
            if inner_curves == &[EntityId(104)]
    ));
    let open = describe(&model, 108);
    assert!(matches!(
        open.parameters,
        ProfileParameters::ArbitraryOpen { .. }
    ));
    assert!(!open.bounds_area(), "an open curve bounds no area");
}

/// `Position` is reported for every parameterised family, sections included.
///
/// ```text
/// #101=IFCAXIS2PLACEMENT2D(#102,#103): origin (100,50) mm, X along (0,2)
/// ```
#[test]
fn a_section_reports_its_position() {
    let model = model(
        "IFC4",
        ".MILLI.",
        "1.,0.,0.",
        "#100=IFCISHAPEPROFILEDEF(.AREA.,$,#101,300.,290.,8.5,14.,$,$,$);
         #101=IFCAXIS2PLACEMENT2D(#102,#103);
         #102=IFCCARTESIANPOINT((100.,50.));
         #103=IFCDIRECTION((0.,2.));",
        "",
    );
    let position = describe(&model, 100)
        .position
        .expect("Position is authored");
    assert!(close(position.origin, [0.1, 0.05]));
    assert!(close(position.x_axis, [0.0, 1.0]), "normalised once");
    assert!(close(position.y_axis(), [-1.0, 0.0]));
}

/// Lowering builds from the same description, so the two cannot disagree.
#[cfg(feature = "lowering")]
mod lowering_agreement {
    use super::*;
    use axiolid_profile::{Profile, SectionProfile};
    use ifc_geometry::lower::profile::lower_profile;

    /// A section's Position reaches the kernel: it used to be dropped for
    /// every family but rectangles and circles.
    #[test]
    fn a_positioned_section_lowers_with_its_position() {
        let model = model(
            "IFC4",
            ".MILLI.",
            "1.,0.,0.",
            "#100=IFCISHAPEPROFILEDEF(.AREA.,$,#101,300.,290.,8.5,14.,$,$,$);
             #101=IFCAXIS2PLACEMENT2D(#102,#103);
             #102=IFCCARTESIANPOINT((100.,50.));
             #103=IFCDIRECTION((0.,1.));",
            "",
        );
        let Profile::Derived { basis, transform } =
            lower_profile(&model, EntityId(100), &units::resolve(&model)).unwrap()
        else {
            panic!("a positioned section lowers as Derived");
        };
        assert!(matches!(*basis, Profile::Section(SectionProfile::I { .. })));
        assert!(close(transform.translation.to_array(), [0.1, 0.05]));
        assert!(close(transform.matrix2.x_axis.to_array(), [0.0, 1.0]));
    }

    /// The derived operator's translation reaches the kernel in metres.
    #[test]
    fn a_derived_profile_lowers_its_translation_in_metres() {
        let model = profile_families();
        let Profile::Derived { transform, .. } =
            lower_profile(&model, EntityId(22), &units::resolve(&model)).unwrap()
        else {
            panic!("derived");
        };
        assert!(
            close(transform.translation.to_array(), [0.05, 0.025]),
            "50 mm is 0.05 m, got {:?}",
            transform.translation
        );
    }
}
