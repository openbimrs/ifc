//! Standard sections: authored, then read back through the same crate.
//!
//! Two layers, deliberately. The slot assertions run in every build,
//! including the kernel-free one, because they are what catches a
//! transposed index. The lowering round-trips need `lowering` and prove
//! the stronger claim: the numbers survive into the neutral profile the
//! reader builds.

use ifc_geometry::authoring::{
    asymmetric_i_shape, c_shape, circle_hollow_profile, ellipse_profile, i_shape, l_shape,
    rectangle_hollow_profile, t_shape, trapezium_profile, u_shape, z_shape, AsymmetricIDims,
    AsymmetricIExtras, CShapeDims, FlangedDims, IShapeDims, IShapeExtras, LShapeExtras,
    ProfileHeader, RectangleHollowDims, RectangleHollowFillets, TShapeExtras, TrapeziumDims,
    UShapeExtras, ZShapeExtras,
};
use ifc_model::{Model, Transaction, Value};

/// Commit one staged profile and hand back the finished record.
fn author(
    build: impl FnOnce(&mut Transaction) -> ifc_model::EntityId,
) -> (Model, ifc_model::EntityId) {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let id = build(&mut tx);
    let mut model = model;
    tx.commit(&mut model).expect("commit");
    (model, id)
}

/// The real number in a slot, or `None` when the slot is `$`.
fn real_at(model: &Model, id: ifc_model::EntityId, index: usize) -> Option<f64> {
    match model.get(id).expect("entity").attribute(index)? {
        Value::Real(value) => Some(*value),
        _ => None,
    }
}

/// An I-section lands in the slots the schema declares, in order.
///
/// Asserted by index rather than by reading back through a view: a
/// writer and a reader that share a transposition agree with each other
/// and are both wrong.
/// One catalogue section: its type name, its schema arity, and a
/// closure that stages it with representative dimensions.
type SectionCase = (
    &'static str,
    usize,
    Box<dyn Fn(&mut Transaction) -> ifc_model::EntityId>,
);

#[test]
fn an_i_section_fills_its_declared_slots() {
    let (model, id) = author(|tx| {
        i_shape(
            tx,
            ProfileHeader {
                name: Some("IPE300"),
                position: None,
            },
            IShapeDims {
                width: 150.0,
                depth: 300.0,
                web_thickness: 7.1,
                flange_thickness: 10.7,
            },
            IShapeExtras {
                fillet_radius: Some(15.0),
                flange_edge_radius: Some(2.0),
                flange_slope: Some(0.0),
            },
        )
        .expect("I section")
    });
    let entity = model.get(id).expect("present");
    assert_eq!(entity.type_name.as_ref(), "IFCISHAPEPROFILEDEF");
    assert_eq!(entity.attributes.len(), 10, "schema arity");
    assert!(matches!(entity.attribute(0), Some(Value::Enum(t)) if t.as_ref() == "AREA"));
    assert!(matches!(entity.attribute(1), Some(Value::Text(t)) if t.as_ref() == "IPE300"));
    assert_eq!(real_at(&model, id, 3), Some(150.0), "OverallWidth");
    assert_eq!(real_at(&model, id, 4), Some(300.0), "OverallDepth");
    assert_eq!(real_at(&model, id, 5), Some(7.1), "WebThickness");
    assert_eq!(real_at(&model, id, 6), Some(10.7), "FlangeThickness");
    assert_eq!(real_at(&model, id, 7), Some(15.0), "FilletRadius");
    assert_eq!(real_at(&model, id, 8), Some(2.0), "FlangeEdgeRadius");
    assert_eq!(real_at(&model, id, 9), Some(0.0), "FlangeSlope");
}

/// The asymmetric I has the layout most likely to be mis-indexed:
/// `TopFlangeWidth` sits at 8, between the bottom fillet at 7 and the
/// top thickness at 9, and the two edge radii and slopes interleave
/// bottom-then-top from 11 to 14.
#[test]
fn an_asymmetric_i_interleaves_bottom_and_top_correctly() {
    let (model, id) = author(|tx| {
        asymmetric_i_shape(
            tx,
            ProfileHeader::default(),
            AsymmetricIDims {
                bottom_flange_width: 200.0,
                overall_depth: 400.0,
                web_thickness: 9.0,
                bottom_flange_thickness: 12.0,
                top_flange_width: 180.0,
            },
            AsymmetricIExtras {
                top_flange_thickness: Some(14.0),
                bottom_flange_fillet_radius: Some(18.0),
                top_flange_fillet_radius: Some(16.0),
                bottom_flange_edge_radius: Some(3.0),
                top_flange_edge_radius: Some(4.0),
                bottom_flange_slope: Some(0.05),
                top_flange_slope: Some(0.06),
            },
        )
        .expect("asymmetric I")
    });
    assert_eq!(model.get(id).expect("present").attributes.len(), 15);
    assert_eq!(real_at(&model, id, 3), Some(200.0), "BottomFlangeWidth");
    assert_eq!(real_at(&model, id, 4), Some(400.0), "OverallDepth");
    assert_eq!(real_at(&model, id, 5), Some(9.0), "WebThickness");
    assert_eq!(real_at(&model, id, 6), Some(12.0), "BottomFlangeThickness");
    assert_eq!(
        real_at(&model, id, 7),
        Some(18.0),
        "BottomFlangeFilletRadius"
    );
    assert_eq!(real_at(&model, id, 8), Some(180.0), "TopFlangeWidth");
    assert_eq!(real_at(&model, id, 9), Some(14.0), "TopFlangeThickness");
    assert_eq!(real_at(&model, id, 10), Some(16.0), "TopFlangeFilletRadius");
    assert_eq!(real_at(&model, id, 11), Some(3.0), "BottomFlangeEdgeRadius");
    assert_eq!(real_at(&model, id, 12), Some(0.05), "BottomFlangeSlope");
    assert_eq!(real_at(&model, id, 13), Some(4.0), "TopFlangeEdgeRadius");
    assert_eq!(real_at(&model, id, 14), Some(0.06), "TopFlangeSlope");
}

/// Every section writes the arity its schema declares. A record one
/// slot short parses and then means something else from that slot on.
#[test]
fn every_section_writes_its_schema_arity() {
    let cases: Vec<SectionCase> = vec![
        (
            "IFCLSHAPEPROFILEDEF",
            9,
            Box::new(|tx| {
                l_shape(
                    tx,
                    ProfileHeader::default(),
                    100.0,
                    8.0,
                    LShapeExtras::default(),
                )
                .expect("L")
            }),
        ),
        (
            "IFCTSHAPEPROFILEDEF",
            12,
            Box::new(|tx| {
                t_shape(
                    tx,
                    ProfileHeader::default(),
                    FlangedDims {
                        depth: 120.0,
                        flange_width: 100.0,
                        web_thickness: 6.0,
                        flange_thickness: 9.0,
                    },
                    TShapeExtras::default(),
                )
                .expect("T")
            }),
        ),
        (
            "IFCUSHAPEPROFILEDEF",
            10,
            Box::new(|tx| {
                u_shape(
                    tx,
                    ProfileHeader::default(),
                    FlangedDims {
                        depth: 200.0,
                        flange_width: 75.0,
                        web_thickness: 6.0,
                        flange_thickness: 10.0,
                    },
                    UShapeExtras::default(),
                )
                .expect("U")
            }),
        ),
        (
            "IFCZSHAPEPROFILEDEF",
            9,
            Box::new(|tx| {
                z_shape(
                    tx,
                    ProfileHeader::default(),
                    FlangedDims {
                        depth: 200.0,
                        flange_width: 75.0,
                        web_thickness: 6.0,
                        flange_thickness: 10.0,
                    },
                    ZShapeExtras::default(),
                )
                .expect("Z")
            }),
        ),
        (
            "IFCCSHAPEPROFILEDEF",
            8,
            Box::new(|tx| {
                c_shape(
                    tx,
                    ProfileHeader::default(),
                    CShapeDims {
                        depth: 200.0,
                        width: 80.0,
                        wall_thickness: 3.0,
                        girth: 20.0,
                    },
                    None,
                )
                .expect("C")
            }),
        ),
        (
            "IFCELLIPSEPROFILEDEF",
            5,
            Box::new(|tx| {
                ellipse_profile(tx, ProfileHeader::default(), 60.0, 40.0).expect("ellipse")
            }),
        ),
        (
            "IFCTRAPEZIUMPROFILEDEF",
            7,
            Box::new(|tx| {
                trapezium_profile(
                    tx,
                    ProfileHeader::default(),
                    TrapeziumDims {
                        bottom_x_dim: 100.0,
                        top_x_dim: 60.0,
                        y_dim: 80.0,
                        top_x_offset: -10.0,
                    },
                )
                .expect("trapezium")
            }),
        ),
        (
            "IFCRECTANGLEHOLLOWPROFILEDEF",
            8,
            Box::new(|tx| {
                rectangle_hollow_profile(
                    tx,
                    ProfileHeader::default(),
                    RectangleHollowDims {
                        x_dim: 200.0,
                        y_dim: 100.0,
                        wall_thickness: 8.0,
                    },
                    RectangleHollowFillets {
                        inner: Some(4.0),
                        outer: Some(12.0),
                    },
                )
                .expect("RHS")
            }),
        ),
        (
            "IFCCIRCLEHOLLOWPROFILEDEF",
            5,
            Box::new(|tx| {
                circle_hollow_profile(tx, ProfileHeader::default(), 50.0, 5.0).expect("CHS")
            }),
        ),
    ];
    for (type_name, arity, build) in cases {
        let (model, id) = author(build);
        let entity = model.get(id).expect("present");
        assert_eq!(entity.type_name.as_ref(), type_name);
        assert_eq!(entity.attributes.len(), arity, "{type_name} arity");
        assert!(
            matches!(entity.attribute(0), Some(Value::Enum(t)) if t.as_ref() == "AREA"),
            "{type_name} ProfileType",
        );
    }
}
/// Sections that parse but cannot exist are refused at authoring time.
///
/// Each case violates exactly one WHERE rule from the EXPRESS schema.
/// The validator in this workspace does not implement profile WHERE
/// rules, so a writer that skipped these would emit a file nothing in
/// the pipeline rejects and no fabricator could roll.
#[test]
fn impossible_sections_are_refused() {
    let model = Model::default();

    // IfcIShapeProfileDef.ValidFlangeThickness: 2 * 60 >= 100.
    let mut tx = Transaction::new(&model);
    assert!(
        i_shape(
            &mut tx,
            ProfileHeader::default(),
            IShapeDims {
                width: 50.0,
                depth: 100.0,
                web_thickness: 5.0,
                flange_thickness: 60.0,
            },
            IShapeExtras::default()
        )
        .is_err(),
        "flanges thicker than the section is deep",
    );

    // IfcIShapeProfileDef.ValidWebThickness: web 60 >= width 50.
    let mut tx = Transaction::new(&model);
    assert!(
        i_shape(
            &mut tx,
            ProfileHeader::default(),
            IShapeDims {
                width: 50.0,
                depth: 300.0,
                web_thickness: 60.0,
                flange_thickness: 10.0,
            },
            IShapeExtras::default()
        )
        .is_err(),
        "web wider than the flange",
    );

    // IfcIShapeProfileDef.ValidFilletRadius: 100 exceeds both clearances.
    let mut tx = Transaction::new(&model);
    assert!(
        i_shape(
            &mut tx,
            ProfileHeader::default(),
            IShapeDims {
                width: 150.0,
                depth: 300.0,
                web_thickness: 7.1,
                flange_thickness: 10.7,
            },
            IShapeExtras {
                fillet_radius: Some(100.0),
                ..IShapeExtras::default()
            },
        )
        .is_err(),
        "fillet larger than the room between web and flanges",
    );

    // IfcLShapeProfileDef.ValidThickness: thickness 100 >= depth 100.
    let mut tx = Transaction::new(&model);
    assert!(
        l_shape(
            &mut tx,
            ProfileHeader::default(),
            100.0,
            100.0,
            LShapeExtras::default()
        )
        .is_err(),
        "leg thickness equal to the depth",
    );

    // ... and the width half of the same rule, only when Width is given.
    let mut tx = Transaction::new(&model);
    assert!(
        l_shape(
            &mut tx,
            ProfileHeader::default(),
            200.0,
            50.0,
            LShapeExtras {
                width: Some(40.0),
                ..LShapeExtras::default()
            },
        )
        .is_err(),
        "thickness exceeding the stated width",
    );

    // IfcUShapeProfileDef.ValidFlangeThickness bounds by Depth / 2, so a
    // thickness legal for a T is illegal here.
    let mut tx = Transaction::new(&model);
    assert!(
        u_shape(
            &mut tx,
            ProfileHeader::default(),
            FlangedDims {
                depth: 100.0,
                flange_width: 75.0,
                web_thickness: 6.0,
                flange_thickness: 50.0,
            },
            UShapeExtras::default(),
        )
        .is_err(),
        "channel flanges meeting in the middle",
    );

    // IfcCShapeProfileDef.ValidGirth: girth 100 >= depth/2 = 100.
    let mut tx = Transaction::new(&model);
    assert!(
        c_shape(
            &mut tx,
            ProfileHeader::default(),
            CShapeDims {
                depth: 200.0,
                width: 80.0,
                wall_thickness: 3.0,
                girth: 100.0
            },
            None
        )
        .is_err(),
        "girth reaching the mid-depth",
    );

    // IfcCircleHollowProfileDef.WR1: wall 50 >= radius 50, no bore left.
    let mut tx = Transaction::new(&model);
    assert!(
        circle_hollow_profile(&mut tx, ProfileHeader::default(), 50.0, 50.0).is_err(),
        "tube with no bore",
    );

    // IfcRectangleHollowProfileDef.ValidWallThickness: 60 >= 100/2.
    let mut tx = Transaction::new(&model);
    assert!(
        rectangle_hollow_profile(
            &mut tx,
            ProfileHeader::default(),
            RectangleHollowDims {
                x_dim: 200.0,
                y_dim: 100.0,
                wall_thickness: 60.0
            },
            RectangleHollowFillets::default()
        )
        .is_err(),
        "walls thicker than half the section",
    );

    // IfcAsymmetricIShapeProfileDef.ValidFlangeThickness: 12 + 390 >= 400.
    let mut tx = Transaction::new(&model);
    assert!(
        asymmetric_i_shape(
            &mut tx,
            ProfileHeader::default(),
            AsymmetricIDims {
                bottom_flange_width: 200.0,
                overall_depth: 400.0,
                web_thickness: 9.0,
                bottom_flange_thickness: 12.0,
                top_flange_width: 180.0,
            },
            AsymmetricIExtras {
                top_flange_thickness: Some(390.0),
                ..AsymmetricIExtras::default()
            },
        )
        .is_err(),
        "flanges that together exceed the depth",
    );
}
/// Zero is a legal `IfcNonNegativeLengthMeasure`, and a legal section.
///
/// The positive dimensions and the non-negative radii are different
/// measure types in the schema. A writer that used one rule for both
/// would reject a sharp-cornered section that every steel catalogue
/// contains.
#[test]
fn a_zero_fillet_is_a_sharp_corner_not_an_error() {
    let (model, id) = author(|tx| {
        i_shape(
            tx,
            ProfileHeader::default(),
            IShapeDims {
                width: 150.0,
                depth: 300.0,
                web_thickness: 7.1,
                flange_thickness: 10.7,
            },
            IShapeExtras {
                fillet_radius: Some(0.0),
                ..IShapeExtras::default()
            },
        )
        .expect("a sharp-cornered I is legal")
    });
    assert_eq!(real_at(&model, id, 7), Some(0.0), "zero fillet is written");

    // ... while a negative one is not a corner at all.
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    assert!(
        i_shape(
            &mut tx,
            ProfileHeader::default(),
            IShapeDims {
                width: 150.0,
                depth: 300.0,
                web_thickness: 7.1,
                flange_thickness: 10.7,
            },
            IShapeExtras {
                fillet_radius: Some(-1.0),
                ..IShapeExtras::default()
            },
        )
        .is_err(),
        "a negative radius is refused",
    );

    // A zero *dimension* stays an error: IfcPositiveLengthMeasure.
    let mut tx = Transaction::new(&model);
    assert!(
        i_shape(
            &mut tx,
            ProfileHeader::default(),
            IShapeDims {
                width: 150.0,
                depth: 0.0,
                web_thickness: 7.1,
                flange_thickness: 10.7,
            },
            IShapeExtras::default()
        )
        .is_err(),
        "a zero depth is refused",
    );
}

/// The strongest form: authored sections lower into the neutral profile
/// the reader builds, with the numbers intact.
///
/// Requires `lowering`. The authoring above does not, which is the
/// point of ADR 0011 -- but when the kernel is present, both directions
/// must agree about every slot.
#[test]
#[cfg(feature = "lowering")]
fn authored_sections_lower_with_their_numbers_intact() {
    use ifc_geometry::lower::lower_profile;
    use ifc_geometry::units::UnitScale;

    let (model, id) = author(|tx| {
        i_shape(
            tx,
            ProfileHeader {
                name: Some("IPE300"),
                position: None,
            },
            IShapeDims {
                width: 150.0,
                depth: 300.0,
                web_thickness: 7.1,
                flange_thickness: 10.7,
            },
            IShapeExtras {
                fillet_radius: Some(15.0),
                ..IShapeExtras::default()
            },
        )
        .expect("I section")
    });
    let profile = lower_profile(&model, id, &UnitScale::default()).expect("lowers");
    let text = format!("{profile:?}");
    for expected in ["300.0", "150.0", "7.1", "10.7", "15.0"] {
        assert!(
            text.contains(expected),
            "{expected} survived lowering: {text}"
        );
    }
}
