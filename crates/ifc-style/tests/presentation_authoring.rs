//! Authoring the curve, fill, text and texture presentation entities.

use ifc_model::{Entity, Model, Transaction, Value};
use ifc_schema::{ifc4, ifc4x3};
use ifc_style::{
    create_colour_rgb_list, create_curve_style, create_curve_style_font,
    create_curve_style_font_pattern, create_draughting_predefined_colour,
    create_draughting_predefined_curve_font, create_fill_area_style, create_indexed_colour_map,
    create_pixel_texture, create_presentation_layer_assignment, create_text_style_font_model,
    create_texture_vertex, create_texture_vertex_list, CurveStyleDraft, CurveWidth, FillStyleKind,
    PixelTextureDraft, StyleError, PREDEFINED_COLOUR_NAMES,
};

/// A curve style must carry at least one of font, width or colour.
#[test]
fn an_empty_curve_style_is_refused() {
    let model = Model::new();
    let mut tx = Transaction::new(&model);
    let err = create_curve_style(&mut tx, ifc4x3(), CurveStyleDraft::default())
        .expect_err("IdentifiableCurveStyle rejects a style with nothing set");
    assert!(matches!(err, StyleError::AuthoringInvalid { .. }), "{err}");
    assert!(tx.is_empty(), "nothing is staged when the rule fails");
}

/// MeasureOfWidth: a width is a positive length, or the literal `by layer`.
#[test]
fn curve_width_carries_its_measure() {
    let mut model = Model::new();
    let mut tx = Transaction::new(&model);

    let id = create_curve_style(
        &mut tx,
        ifc4x3(),
        CurveStyleDraft {
            curve_width: Some(CurveWidth::PositiveLength(0.35)),
            ..CurveStyleDraft::default()
        },
    )
    .expect("a positive length is a legal width");

    tx.commit(&mut model).expect("the style commits");
    let entity = model.get(id).expect("the style is in the model");
    let Some(Value::Typed { type_name, value }) = entity.attribute(2) else {
        panic!(
            "CurveWidth is written as a typed measure, got {:?}",
            entity.attribute(2)
        );
    };
    assert_eq!(&**type_name, "IFCPOSITIVELENGTHMEASURE");
    assert_eq!(**value, Value::Real(0.35));
}

/// `by layer` is a descriptive measure, not a number.
#[test]
fn by_layer_width_is_a_descriptive_measure() {
    let mut model = Model::new();
    let mut tx = Transaction::new(&model);
    let id = create_curve_style(
        &mut tx,
        ifc4x3(),
        CurveStyleDraft {
            curve_width: Some(CurveWidth::ByLayer),
            ..CurveStyleDraft::default()
        },
    )
    .expect("by layer is a legal width");
    tx.commit(&mut model).expect("the style commits");
    let entity = model.get(id).expect("the style is in the model");
    let Some(Value::Typed { type_name, value }) = entity.attribute(2) else {
        panic!("expected a typed measure");
    };
    assert_eq!(&**type_name, "IFCDESCRIPTIVEMEASURE");
    assert_eq!(**value, Value::Text("by layer".into()));
}

/// A non-positive width is refused rather than written.
#[test]
fn a_non_positive_curve_width_is_refused() {
    let model = Model::new();
    let mut tx = Transaction::new(&model);
    for width in [0.0, -1.0, f64::NAN] {
        let err = create_curve_style(
            &mut tx,
            ifc4x3(),
            CurveStyleDraft {
                curve_width: Some(CurveWidth::PositiveLength(width)),
                ..CurveStyleDraft::default()
            },
        )
        .expect_err("IfcPositiveLengthMeasure excludes {width}");
        assert!(matches!(err, StyleError::AuthoringInvalid { .. }), "{err}");
    }
}

/// SizeOfPixelList: the pixel count must equal width * height.
///
/// This is the rule worth having. A short pixel list produces a texture
/// that decodes into garbage for every row after the first, and nothing
/// in the file says which row was wrong.
#[test]
fn a_pixel_texture_must_match_its_declared_size() {
    let model = Model::new();
    let mut tx = Transaction::new(&model);
    let err = create_pixel_texture(
        &mut tx,
        ifc4x3(),
        PixelTextureDraft {
            repeat_s: true,
            repeat_t: true,
            mode: None,
            width: 2,
            height: 2,
            colour_components: 3,
            pixel: &["FF0000", "00FF00", "0000FF"],
        },
    )
    .expect_err("three pixels cannot fill a 2x2 texture");
    assert!(matches!(err, StyleError::AuthoringInvalid { .. }), "{err}");
    assert!(tx.is_empty(), "nothing is staged");
}

/// PixelAsByteAndSameLength: every pixel is whole bytes, all equal length.
#[test]
fn ragged_or_half_byte_pixels_are_refused() {
    let model = Model::new();
    let mut tx = Transaction::new(&model);
    for pixels in [
        ["FF0000", "00FF0"].as_slice(), // second is not whole bytes
        ["FF0000", "00FF"].as_slice(),  // whole bytes, but shorter
    ] {
        let err = create_pixel_texture(
            &mut tx,
            ifc4x3(),
            PixelTextureDraft {
                repeat_s: false,
                repeat_t: false,
                mode: None,
                width: 2,
                height: 1,
                colour_components: 3,
                pixel: pixels,
            },
        )
        .expect_err("PixelAsByteAndSameLength rejects it");
        assert!(matches!(err, StyleError::AuthoringInvalid { .. }), "{err}");
    }
}

/// NumberOfColours bounds the component count to 1..=4.
#[test]
fn colour_component_count_is_bounded() {
    let model = Model::new();
    let mut tx = Transaction::new(&model);
    for components in [0, 5] {
        let err = create_pixel_texture(
            &mut tx,
            ifc4x3(),
            PixelTextureDraft {
                repeat_s: false,
                repeat_t: false,
                mode: None,
                width: 1,
                height: 1,
                colour_components: components,
                pixel: &["FF"],
            },
        )
        .expect_err("NumberOfColours admits 1..=4 only");
        assert!(matches!(err, StyleError::AuthoringInvalid { .. }), "{err}");
    }
}

/// A well-formed pixel texture commits, so the refusals above are not vacuous.
#[test]
fn a_well_formed_pixel_texture_commits() {
    let mut model = Model::new();
    let mut tx = Transaction::new(&model);
    let id = create_pixel_texture(
        &mut tx,
        ifc4x3(),
        PixelTextureDraft {
            repeat_s: true,
            repeat_t: false,
            mode: Some("MODULATE"),
            width: 2,
            height: 2,
            colour_components: 3,
            pixel: &["FF0000", "00FF00", "0000FF", "FFFFFF"],
        },
    )
    .expect("a 2x2 texture with four equal-length pixels is legal");
    tx.commit(&mut model).expect("commits");
    let entity = model.get(id).expect("in the model");
    assert_eq!(entity.type_name.as_ref(), "IFCPIXELTEXTURE");
    assert_eq!(entity.attributes.len(), 9, "IfcPixelTexture has nine slots");
}

/// Predefined names are a closed, lower-case list.
///
/// Case matters: the schema states them lower-case, so `Black` is not a
/// legal predefined colour however reasonable it looks.
#[test]
fn predefined_names_are_closed_and_case_sensitive() {
    let model = Model::new();
    let mut tx = Transaction::new(&model);

    for name in PREDEFINED_COLOUR_NAMES {
        create_draughting_predefined_colour(&mut tx, ifc4x3(), name)
            .expect("every listed name is accepted");
    }
    for name in ["Black", "orange", ""] {
        let err = create_draughting_predefined_colour(&mut tx, ifc4x3(), name)
            .expect_err("PreDefinedColourNames is closed");
        assert!(matches!(err, StyleError::AuthoringInvalid { .. }), "{err}");
    }
    let err = create_draughting_predefined_curve_font(&mut tx, ifc4x3(), "solid")
        .expect_err("solid is not one of the listed curve fonts");
    assert!(matches!(err, StyleError::AuthoringInvalid { .. }), "{err}");
    create_draughting_predefined_curve_font(&mut tx, ifc4x3(), "chain double dash")
        .expect("a listed multi-word font name is accepted");
}

/// MaxOneColour and MaxOneExtHatchStyle bound their member counts.
#[test]
fn fill_area_style_bounds_its_singleton_members() {
    let mut model = Model::new();
    let a = model.push(Entity::new("IFCCOLOURRGB", vec![]));
    let b = model.push(Entity::new("IFCCOLOURRGB", vec![]));
    let mut tx = Transaction::new(&model);

    let err = create_fill_area_style(
        &mut tx,
        ifc4x3(),
        Some("Hatch"),
        &[(a, FillStyleKind::Colour), (b, FillStyleKind::Colour)],
        None,
    )
    .expect_err("MaxOneColour admits one colour");
    assert!(matches!(err, StyleError::AuthoringInvalid { .. }), "{err}");

    let err = create_fill_area_style(&mut tx, ifc4x3(), None, &[], None)
        .expect_err("FillStyles is SET [1:?]");
    assert!(matches!(err, StyleError::AuthoringInvalid { .. }), "{err}");

    create_fill_area_style(
        &mut tx,
        ifc4x3(),
        Some("Hatch"),
        &[(a, FillStyleKind::Colour), (b, FillStyleKind::Hatching)],
        Some(true),
    )
    .expect("one colour plus a hatching is legal");
}

/// Normalised ratio channels are bounded to [0, 1].
#[test]
fn colour_rgb_list_channels_are_normalised() {
    let model = Model::new();
    let mut tx = Transaction::new(&model);
    let err = create_colour_rgb_list(&mut tx, ifc4x3(), &[[0.5, 1.5, 0.0]])
        .expect_err("1.5 is not a normalised ratio");
    assert!(matches!(err, StyleError::AuthoringInvalid { .. }), "{err}");
    let err = create_colour_rgb_list(&mut tx, ifc4x3(), &[]).expect_err("ColourList is LIST [1:?]");
    assert!(matches!(err, StyleError::AuthoringInvalid { .. }), "{err}");
    create_colour_rgb_list(&mut tx, ifc4x3(), &[[0.0, 0.5, 1.0]])
        .expect("in-range channels are accepted");
}

/// STEP indices are one-based, so zero is not a legal colour index.
#[test]
fn colour_indices_are_one_based() {
    let mut model = Model::new();
    let mesh = model.push(Entity::new("IFCTRIANGULATEDFACESET", vec![]));
    let colours = model.push(Entity::new("IFCCOLOURRGBLIST", vec![]));
    let mut tx = Transaction::new(&model);
    let err = create_indexed_colour_map(&mut tx, ifc4x3(), mesh, colours, &[1, 0], None)
        .expect_err("IfcPositiveInteger excludes zero");
    assert!(matches!(err, StyleError::AuthoringInvalid { .. }), "{err}");
    create_indexed_colour_map(&mut tx, ifc4x3(), mesh, colours, &[1, 2], Some(0.5))
        .expect("one-based indices with a valid opacity are accepted");
}

/// The curve font chain stages across two schema versions.
#[test]
fn the_curve_font_chain_stages_on_ifc4_and_ifc4x3() {
    for schema in [ifc4(), ifc4x3()] {
        let mut model = Model::new();
        let mut tx = Transaction::new(&model);
        let pattern = create_curve_style_font_pattern(&mut tx, schema, 0.0, 2.0)
            .expect("a zero visible length is legal, an invisible one is not");
        let font = create_curve_style_font(&mut tx, schema, Some("dashed"), &[pattern])
            .expect("a font with one pattern is legal");
        tx.commit(&mut model).expect("commits");
        assert_eq!(
            model.get(font).expect("font").type_name.as_ref(),
            "IFCCURVESTYLEFONT"
        );
    }
}

/// VisibleLengthGreaterEqualZero admits zero; the invisible length does not.
#[test]
fn pattern_lengths_follow_their_measure_types() {
    let model = Model::new();
    let mut tx = Transaction::new(&model);
    create_curve_style_font_pattern(&mut tx, ifc4x3(), 0.0, 1.0)
        .expect("zero visible length is legal");
    for (visible, invisible) in [(-0.1, 1.0), (1.0, 0.0), (1.0, -1.0)] {
        let err = create_curve_style_font_pattern(&mut tx, ifc4x3(), visible, invisible)
            .expect_err("out-of-range length");
        assert!(matches!(err, StyleError::AuthoringInvalid { .. }), "{err}");
    }
    let err = create_curve_style_font(&mut tx, ifc4x3(), None, &[])
        .expect_err("PatternList is LIST [1:?]");
    assert!(matches!(err, StyleError::AuthoringInvalid { .. }), "{err}");
}

/// MeasureOfFontSize needs a positive length, written with its measure.
#[test]
fn font_size_carries_a_positive_length_measure() {
    let mut model = Model::new();
    let mut tx = Transaction::new(&model);
    let id = create_text_style_font_model(&mut tx, ifc4x3(), "Body", &["Arial"], 12.0)
        .expect("a positive size is legal");
    tx.commit(&mut model).expect("commits");
    let entity = model.get(id).expect("in the model");
    let Some(Value::Typed { type_name, .. }) = entity.attribute(5) else {
        panic!("FontSize carries a measure, got {:?}", entity.attribute(5));
    };
    assert_eq!(&**type_name, "IFCLENGTHMEASURE");

    let mut tx = Transaction::new(&model);
    for (name, family, size) in [
        ("", vec!["Arial"], 12.0),
        ("Body", vec![], 12.0),
        ("Body", vec!["Arial"], 0.0),
    ] {
        let err = create_text_style_font_model(&mut tx, ifc4x3(), name, &family, size)
            .expect_err("a blank name, empty family or non-positive size is refused");
        assert!(matches!(err, StyleError::AuthoringInvalid { .. }), "{err}");
    }
}

/// Texture vertices carry exactly two parameter values by construction.
#[test]
fn texture_vertices_are_pairs() {
    let model = Model::new();
    let mut tx = Transaction::new(&model);
    create_texture_vertex(&mut tx, ifc4x3(), [0.0, 1.0]).expect("a finite pair is legal");
    let err = create_texture_vertex(&mut tx, ifc4x3(), [0.0, f64::INFINITY])
        .expect_err("a non-finite coordinate is refused");
    assert!(matches!(err, StyleError::AuthoringInvalid { .. }), "{err}");
    let err = create_texture_vertex_list(&mut tx, ifc4x3(), &[])
        .expect_err("TexCoordsList is LIST [1:?]");
    assert!(matches!(err, StyleError::AuthoringInvalid { .. }), "{err}");
    create_texture_vertex_list(&mut tx, ifc4x3(), &[[0.0, 0.0], [1.0, 1.0]])
        .expect("a list of pairs is legal");
}

/// ApplicableItems: a layer assignment needs a name and at least one item.
#[test]
fn a_layer_assignment_needs_a_name_and_items() {
    let mut model = Model::new();
    let item = model.push(Entity::new("IFCSHAPEREPRESENTATION", vec![]));
    let mut tx = Transaction::new(&model);
    let err = create_presentation_layer_assignment(&mut tx, ifc4x3(), "  ", None, &[item], None)
        .expect_err("a blank name satisfies EXISTS while naming nothing");
    assert!(matches!(err, StyleError::AuthoringInvalid { .. }), "{err}");
    let err = create_presentation_layer_assignment(&mut tx, ifc4x3(), "A-WALL", None, &[], None)
        .expect_err("AssignedItems is SET [1:?]");
    assert!(matches!(err, StyleError::AuthoringInvalid { .. }), "{err}");
    create_presentation_layer_assignment(
        &mut tx,
        ifc4x3(),
        "A-WALL",
        Some("Walls"),
        &[item],
        Some("A-WALL"),
    )
    .expect("a named assignment with one item is legal");
}

/// Equal-length pixels that are still not whole bytes are refused.
///
/// Separate from the ragged case: two five-digit pixels agree with each
/// other, so only the `BLENGTH MOD 8` half of the rule can reject them.
#[test]
fn equal_length_half_byte_pixels_are_refused() {
    let model = Model::new();
    let mut tx = Transaction::new(&model);
    let err = create_pixel_texture(
        &mut tx,
        ifc4x3(),
        PixelTextureDraft {
            repeat_s: false,
            repeat_t: false,
            mode: None,
            width: 2,
            height: 1,
            colour_components: 3,
            pixel: &["FF000", "00FF0"],
        },
    )
    .expect_err("five hex digits is not a whole number of bytes");
    assert!(matches!(err, StyleError::AuthoringInvalid { .. }), "{err}");
}
