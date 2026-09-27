//! Sweep writers that existed but had never been called.
//!
//! Each of these stages an entity type nothing else in the suite
//! produced, so a wrong type name or slot layout went unnoticed.

use ifc_model::{Entity, Model, Transaction, Value};
use ifc_schema::{ifc4, ifc4x3};
use ifc_style::{
    create_curve_style_font_and_scaling, create_fill_area_style, create_planar_extent,
    create_text_style_for_defined_font, FillStyleKind,
};

/// `IfcCurveStyleFontAndScaling` stages under both schemas.
///
/// IFC4 names slot 1 `CurveFont`; IFC4X3 renamed it `CurveStyleFont`.
/// The slot and its type are unchanged, so only the name differs.
#[test]
fn the_curve_style_font_scaling_writer_stages() {
    for (schema, expected_attribute) in [(ifc4(), "CurveFont"), (ifc4x3(), "CurveStyleFont")] {
        let mut model = Model::default();
        let mut tx = Transaction::new(&model);
        let font = tx.create(Entity::new("IFCCURVESTYLEFONT", vec![Value::Null; 2]));

        let id = create_curve_style_font_and_scaling(&mut tx, schema, Some("Dashed x2"), font, 2.0)
            .expect("curve style font and scaling");
        tx.commit(&mut model).expect("commit");

        assert_eq!(
            schema.attribute_names("IfcCurveStyleFontAndScaling")[1],
            expected_attribute,
            "schema slot name",
        );
        let staged = model.get(id).expect("staged");
        assert_eq!(staged.type_name.as_ref(), "IFCCURVESTYLEFONTANDSCALING");
        assert_eq!(
            staged.attributes[1],
            Value::Ref(font),
            "the font landed in slot 1 under {expected_attribute}",
        );
        assert_eq!(
            staged.attributes[2],
            Value::Real(2.0),
            "CurveFontScaling is the third slot",
        );
    }
}

/// A non-positive scale factor is refused.
///
/// `CurveFontScaling` is an `IfcPositiveRatioMeasure`, so zero and
/// negative factors are out of type, not merely unusual.
#[test]
fn a_non_positive_font_scaling_is_refused() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let font = tx.create(Entity::new("IFCCURVESTYLEFONT", vec![Value::Null; 2]));

    for scaling in [0.0, -1.0, f64::NAN] {
        assert!(
            create_curve_style_font_and_scaling(&mut tx, ifc4(), None, font, scaling).is_err(),
            "accepted scaling {scaling}",
        );
    }
}

/// `IfcTextStyleForDefinedFont` stages, with an optional background.
#[test]
fn the_text_style_for_defined_font_writer_stages() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let colour = tx.create(Entity::new("IFCCOLOURRGB", vec![Value::Null; 4]));
    let background = tx.create(Entity::new("IFCCOLOURRGB", vec![Value::Null; 4]));

    let plain =
        create_text_style_for_defined_font(&mut tx, ifc4(), colour, None).expect("text style");
    let filled = create_text_style_for_defined_font(&mut tx, ifc4(), colour, Some(background))
        .expect("text style with background");
    tx.commit(&mut model).expect("commit");

    assert_eq!(
        model.get(plain).expect("staged").type_name.as_ref(),
        "IFCTEXTSTYLEFORDEFINEDFONT",
    );
    assert_eq!(
        model.get(plain).expect("staged").attributes[1],
        Value::Null,
        "an absent background stays absent",
    );
    assert_eq!(
        model.get(filled).expect("staged").attributes[1],
        Value::Ref(background),
    );
}

/// `IfcFillAreaStyle.ModelorDraughting` was recapitalised in IFC4X3.
///
/// IFC4 spells it with a lowercase `o`. The slot and its type are
/// unchanged, so a writer with one spelling hardcoded refuses its own
/// output under the other schema.
#[test]
fn the_fill_area_style_flag_stages_under_both_schemas() {
    for (schema, expected_attribute) in [
        (ifc4(), "ModelorDraughting"),
        (ifc4x3(), "ModelOrDraughting"),
    ] {
        let mut model = Model::default();
        let mut tx = Transaction::new(&model);
        let colour = tx.create(Entity::new("IFCCOLOURRGB", vec![Value::Null; 4]));

        let id = create_fill_area_style(
            &mut tx,
            schema,
            Some("Hatch"),
            &[(colour, FillStyleKind::Colour)],
            Some(true),
        )
        .expect("fill area style");
        tx.commit(&mut model).expect("commit");

        assert_eq!(
            schema.attribute_names("IfcFillAreaStyle")[2],
            expected_attribute,
            "schema slot name",
        );
        let staged = model.get(id).expect("staged");
        assert_eq!(staged.type_name.as_ref(), "IFCFILLAREASTYLE");
        assert_eq!(
            staged.attributes[2],
            Value::Bool(true),
            "the flag landed in slot 2 under {expected_attribute}",
        );
    }
}

/// `IfcPlanarExtent` and its `IfcPlanarBox` subtype.
///
/// The box adds `Placement` after the two inherited sizes, so
/// passing a placement selects the subtype.
#[test]
fn the_planar_extent_writer_stages_both_forms() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);

    let extent =
        create_planar_extent(&mut tx, &model, ifc4(), 4.0, 2.0, None).expect("planar extent");
    let placement = tx.create(Entity::new("IFCAXIS2PLACEMENT2D", vec![Value::Null; 2]));
    let boxed = create_planar_extent(&mut tx, &model, ifc4(), 4.0, 2.0, Some(placement))
        .expect("planar box");
    tx.commit(&mut model).expect("commit");

    let plain = model.get(extent).expect("staged");
    assert_eq!(plain.type_name.as_ref(), "IFCPLANAREXTENT");
    assert_eq!(plain.attributes.len(), 2);
    let with_box = model.get(boxed).expect("staged");
    assert_eq!(with_box.type_name.as_ref(), "IFCPLANARBOX");
    assert_eq!(with_box.attributes[2], Value::Ref(placement));
}

/// Sizes must be finite and positive.
#[test]
fn a_non_positive_extent_size_is_refused() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    for (x, y) in [(0.0, 2.0), (4.0, -1.0), (f64::NAN, 2.0)] {
        assert!(
            create_planar_extent(&mut tx, &model, ifc4(), x, y, None).is_err(),
            "accepted a non-positive extent {x} x {y}",
        );
    }
}

/// A placement that is not an axis2 placement is refused.
///
/// `IfcAxis2Placement` is a SELECT, so the check names each
/// concrete member rather than relying on a supertype.
#[test]
fn a_non_axis2_placement_is_refused() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let wrong = tx.create(Entity::new("IFCCOLOURRGB", vec![Value::Null; 4]));
    assert!(
        create_planar_extent(&mut tx, &model, ifc4(), 1.0, 1.0, Some(wrong)).is_err(),
        "accepted a colour as a placement",
    );

    // Both SELECT members are accepted.
    for form in ["IFCAXIS2PLACEMENT2D", "IFCAXIS2PLACEMENT3D"] {
        let placement = tx.create(Entity::new(form, vec![Value::Null; 2]));
        assert!(
            create_planar_extent(&mut tx, &model, ifc4(), 1.0, 1.0, Some(placement)).is_ok(),
            "refused {form}",
        );
    }
}
