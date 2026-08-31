use ifc_model::{Entity, Model, Value};
use ifc_schema::{ifc2x3, ifc4};
use ifc_style::{PresentationStyleMember, StyleError, StyleSource, StyleView, SurfaceSide};

fn real(value: f64) -> Value {
    Value::Real(value)
}

fn text(value: &str) -> Value {
    Value::Text(value.into())
}

fn enumeration(value: &str) -> Value {
    Value::Enum(value.into())
}

#[test]
fn reads_the_ifc4_surface_style_graph_and_direct_style_wins_over_layers() {
    let schema = ifc4();
    let mut model = Model::new();
    let item = model.push(Entity::new("IFCREPRESENTATIONITEM", vec![text("body")]));
    let colour = model.push(Entity::new(
        "IFCCOLOURRGB",
        vec![Value::Null, real(0.2), real(0.4), real(0.6)],
    ));
    let shading = model.push(Entity::new(
        "IFCSURFACESTYLESHADING",
        vec![Value::Ref(colour), real(0.25)],
    ));
    let direct_style = model.push(Entity::new(
        "IFCSURFACESTYLE",
        vec![
            text("direct"),
            enumeration("BOTH"),
            Value::List(vec![Value::Ref(shading)]),
        ],
    ));
    let layer_style = model.push(Entity::new(
        "IFCSURFACESTYLE",
        vec![
            text("layer"),
            enumeration("POSITIVE"),
            Value::List(vec![Value::Ref(shading)]),
        ],
    ));
    let styled_item = model.push(Entity::new(
        "IFCSTYLEDITEM",
        vec![
            Value::Ref(item),
            Value::List(vec![Value::Ref(direct_style)]),
            Value::Null,
        ],
    ));
    let layer = model.push(Entity::new(
        "IFCPRESENTATIONLAYERWITHSTYLE",
        vec![
            text("A-WALL"),
            Value::Null,
            Value::List(vec![Value::Ref(item)]),
            Value::Null,
            Value::Bool(true),
            Value::Bool(false),
            Value::Bool(false),
            Value::List(vec![Value::Ref(layer_style)]),
        ],
    ));

    let view = StyleView::new(&model, schema);
    let rgb = view.colour_rgb(colour).unwrap();
    assert_eq!(rgb.channels().unwrap(), [0.2, 0.4, 0.6]);
    assert_eq!(rgb.name().unwrap(), None);
    let shading_view = view.surface_style_shading(shading).unwrap();
    assert_eq!(shading_view.surface_colour().unwrap(), colour);
    assert_eq!(shading_view.transparency().unwrap(), Some(0.25));
    let surface = view.surface_style(direct_style).unwrap();
    assert_eq!(surface.name().unwrap(), Some("direct"));
    assert_eq!(surface.side().unwrap(), SurfaceSide::Both);
    assert_eq!(surface.elements().unwrap(), vec![shading]);
    assert_eq!(
        view.styled_item(styled_item).unwrap().styles().unwrap(),
        vec![direct_style]
    );
    assert_eq!(
        view.presentation_layer(layer)
            .unwrap()
            .assigned_items()
            .unwrap(),
        vec![item]
    );

    let resolved = view.resolve_item_style(item).unwrap();
    assert_eq!(
        resolved.source(),
        StyleSource::DirectStyledItem(styled_item)
    );
    assert_eq!(resolved.effective_styles(), &[direct_style]);
    assert_eq!(resolved.layer_styles(), &[(layer, vec![layer_style])]);
}

#[test]
fn resolves_ifc2x3_presentation_style_assignment_wrappers() {
    let schema = ifc2x3();
    let mut model = Model::new();
    let item = model.push(Entity::new("IFCREPRESENTATIONITEM", vec![text("body")]));
    let colour = model.push(Entity::new(
        "IFCCOLOURRGB",
        vec![Value::Null, real(1.0), real(0.0), real(0.0)],
    ));
    let shading = model.push(Entity::new(
        "IFCSURFACESTYLESHADING",
        vec![Value::Ref(colour)],
    ));
    let surface = model.push(Entity::new(
        "IFCSURFACESTYLE",
        vec![
            Value::Null,
            enumeration("BOTH"),
            Value::List(vec![Value::Ref(shading)]),
        ],
    ));
    let assignment = model.push(Entity::new(
        "IFCPRESENTATIONSTYLEASSIGNMENT",
        vec![Value::List(vec![Value::Ref(surface)])],
    ));
    model.push(Entity::new(
        "IFCSTYLEDITEM",
        vec![
            Value::Ref(item),
            Value::List(vec![Value::Ref(assignment)]),
            Value::Null,
        ],
    ));

    let resolved = StyleView::new(&model, schema)
        .resolve_item_style(item)
        .unwrap();
    assert_eq!(resolved.effective_styles(), &[surface]);
}

#[test]
fn rejects_out_of_range_colour_and_ambiguous_direct_assignments() {
    let schema = ifc4();
    let mut model = Model::new();
    let item = model.push(Entity::new("IFCREPRESENTATIONITEM", vec![]));
    let bad = model.push(Entity::new(
        "IFCCOLOURRGB",
        vec![Value::Null, real(1.1), real(0.0), real(0.0)],
    ));
    let err = StyleView::new(&model, schema)
        .colour_rgb(bad)
        .unwrap()
        .channels()
        .unwrap_err();
    assert!(matches!(
        err,
        StyleError::OutOfRange {
            attribute: "Red",
            ..
        }
    ));

    let colour = model.push(Entity::new(
        "IFCCOLOURRGB",
        vec![Value::Null, real(1.0), real(0.0), real(0.0)],
    ));
    let shading = model.push(Entity::new(
        "IFCSURFACESTYLESHADING",
        vec![Value::Ref(colour)],
    ));
    let style = model.push(Entity::new(
        "IFCSURFACESTYLE",
        vec![
            Value::Null,
            enumeration("BOTH"),
            Value::List(vec![Value::Ref(shading)]),
        ],
    ));
    for _ in 0..2 {
        model.push(Entity::new(
            "IFCSTYLEDITEM",
            vec![
                Value::Ref(item),
                Value::List(vec![Value::Ref(style)]),
                Value::Null,
            ],
        ));
    }
    assert!(matches!(
        StyleView::new(&model, schema).resolve_item_style(item),
        Err(StyleError::AmbiguousStyleAssignment { item: found, count: 2 }) if found == item
    ));
}

#[test]
fn rejects_non_surface_style_select_members() {
    let schema = ifc4();
    let mut model = Model::new();
    let invalid_member = model.push(Entity::new("IFCREPRESENTATIONITEM", vec![]));
    let surface = model.push(Entity::new(
        "IFCSURFACESTYLE",
        vec![
            Value::Null,
            enumeration("BOTH"),
            Value::List(vec![Value::Ref(invalid_member)]),
        ],
    ));

    let error = StyleView::new(&model, schema)
        .surface_style(surface)
        .unwrap()
        .elements()
        .unwrap_err();

    assert!(matches!(
        error,
        StyleError::ReferenceType {
            target,
            expected: "IfcSurfaceStyleElementSelect",
            ..
        } if target == invalid_member
    ));
}

#[test]
fn malformed_styled_item_select_is_not_resolved_as_a_style() {
    let schema = ifc4();
    let mut model = Model::new();
    let item = model.push(Entity::new("IFCREPRESENTATIONITEM", vec![]));
    let invalid = model.push(Entity::new("IFCREPRESENTATIONITEM", vec![]));
    model.push(Entity::new(
        "IFCSTYLEDITEM",
        vec![
            Value::Ref(item),
            Value::List(vec![Value::Ref(invalid)]),
            Value::Null,
        ],
    ));

    assert!(matches!(
        StyleView::new(&model, schema).resolve_item_style(item),
        Err(StyleError::ReferenceType {
            target,
            expected: "IfcStyleAssignmentSelect",
            ..
        }) if target == invalid
    ));
}

#[test]
fn strict_aggregate_views_reject_wrong_member_types() {
    let schema = ifc4();
    let mut model = Model::new();
    let invalid = model.push(Entity::new("IFCREPRESENTATIONITEM", vec![]));
    let invalid_layer = model.push(Entity::new(
        "IFCCOLOURRGB",
        vec![Value::Null, real(1.0), real(0.0), real(0.0)],
    ));
    let curve_font = model.push(Entity::new(
        "IFCCURVESTYLEFONT",
        vec![Value::Null, Value::List(vec![Value::Ref(invalid)])],
    ));
    let fill = model.push(Entity::new(
        "IFCFILLAREASTYLE",
        vec![
            Value::Null,
            Value::List(vec![Value::Ref(invalid)]),
            Value::Bool(false),
        ],
    ));
    let tiles = model.push(Entity::new(
        "IFCFILLAREASTYLETILES",
        vec![Value::List(vec![Value::Ref(invalid)])],
    ));
    let bad_layer = model.push(Entity::new(
        "IFCPRESENTATIONLAYERWITHSTYLE",
        vec![
            text("bad"),
            Value::Null,
            Value::List(vec![Value::Ref(invalid_layer)]),
            Value::Null,
            Value::Bool(true),
            Value::Bool(false),
            Value::Bool(false),
            Value::List(vec![Value::Ref(invalid_layer)]),
        ],
    ));
    let view = StyleView::new(&model, schema);

    assert!(matches!(
        view.curve_style_font(curve_font).unwrap().patterns(),
        Err(StyleError::ReferenceType {
            expected: "IfcCurveStyleFontPattern",
            ..
        })
    ));
    assert!(matches!(
        view.fill_area_style(fill).unwrap().fill_styles(),
        Err(StyleError::ReferenceType {
            expected: "IfcFillStyleSelect",
            ..
        })
    ));
    assert!(matches!(
        view.fill_area_style_tiles(tiles).unwrap().tiling_pattern(),
        Err(StyleError::InvalidValue {
            attribute: "TilingPattern",
            ..
        })
    ));
    let layer = view.presentation_layer(bad_layer).unwrap();
    assert!(matches!(
        layer.assigned_items(),
        Err(StyleError::ReferenceType {
            expected: "IfcLayeredItem",
            ..
        })
    ));
    assert!(matches!(
        layer.layer_styles(),
        Err(StyleError::ReferenceType {
            expected: "IfcPresentationStyle",
            ..
        })
    ));
}

#[test]
fn surface_style_view_enforces_the_five_element_upper_bound() {
    let schema = ifc4();
    let mut model = Model::new();
    let colour = model.push(Entity::new(
        "IFCCOLOURRGB",
        vec![Value::Null, real(1.0), real(0.0), real(0.0)],
    ));
    let shading = model.push(Entity::new(
        "IFCSURFACESTYLESHADING",
        vec![Value::Ref(colour)],
    ));
    let surface = model.push(Entity::new(
        "IFCSURFACESTYLE",
        vec![
            Value::Null,
            enumeration("BOTH"),
            Value::List(vec![Value::Ref(shading); 6]),
        ],
    ));

    assert!(matches!(
        StyleView::new(&model, schema)
            .surface_style(surface)
            .unwrap()
            .elements(),
        Err(StyleError::InvalidValue {
            attribute: "Styles",
            ..
        })
    ));
}

#[test]
fn surface_style_view_rejects_duplicate_where_rule_category() {
    let schema = ifc4();
    let mut model = Model::new();
    let colour = model.push(Entity::new(
        "IFCCOLOURRGB",
        vec![Value::Null, real(1.0), real(0.0), real(0.0)],
    ));
    let first = model.push(Entity::new(
        "IFCSURFACESTYLESHADING",
        vec![Value::Ref(colour)],
    ));
    let second = model.push(Entity::new(
        "IFCSURFACESTYLESHADING",
        vec![Value::Ref(colour)],
    ));
    let surface = model.push(Entity::new(
        "IFCSURFACESTYLE",
        vec![
            Value::Null,
            enumeration("BOTH"),
            Value::List(vec![Value::Ref(first), Value::Ref(second)]),
        ],
    ));

    assert!(matches!(
        StyleView::new(&model, schema)
            .surface_style(surface)
            .unwrap()
            .elements(),
        Err(StyleError::InvalidValue {
            attribute: "Styles",
            ..
        })
    ));
}

#[test]
fn legacy_null_style_is_explicit_and_not_an_effective_style() {
    let schema = ifc2x3();
    let mut model = Model::new();
    let assignment = model.push(Entity::new(
        "IFCPRESENTATIONSTYLEASSIGNMENT",
        vec![Value::List(vec![Value::Typed {
            type_name: "IFCNULLSTYLE".into(),
            value: Box::new(enumeration("NULL")),
        }])],
    ));
    let wrapper = StyleView::new(&model, schema)
        .presentation_style_assignment(assignment)
        .unwrap();

    assert_eq!(
        wrapper.members().unwrap(),
        vec![PresentationStyleMember::Null]
    );
    assert!(wrapper.styles().unwrap().is_empty());
}

#[test]
fn malformed_legacy_wrapper_member_is_rejected_by_resolution() {
    let schema = ifc2x3();
    let mut model = Model::new();
    let item = model.push(Entity::new("IFCREPRESENTATIONITEM", vec![]));
    let invalid = model.push(Entity::new("IFCREPRESENTATIONITEM", vec![]));
    let assignment = model.push(Entity::new(
        "IFCPRESENTATIONSTYLEASSIGNMENT",
        vec![Value::List(vec![Value::Ref(invalid)])],
    ));
    model.push(Entity::new(
        "IFCSTYLEDITEM",
        vec![
            Value::Ref(item),
            Value::List(vec![Value::Ref(assignment)]),
            Value::Null,
        ],
    ));

    assert!(matches!(
        StyleView::new(&model, schema).resolve_item_style(item),
        Err(StyleError::ReferenceType {
            target,
            expected: "IfcPresentationStyle",
            ..
        }) if target == invalid
    ));
}

#[test]
fn bogus_typed_null_style_member_is_rejected() {
    let schema = ifc2x3();
    let mut model = Model::new();
    let assignment = model.push(Entity::new(
        "IFCPRESENTATIONSTYLEASSIGNMENT",
        vec![Value::List(vec![Value::Typed {
            type_name: "IFCBOGUSSTYLE".into(),
            value: Box::new(enumeration("NULL")),
        }])],
    ));

    assert!(matches!(
        StyleView::new(&model, schema)
            .presentation_style_assignment(assignment)
            .unwrap()
            .members(),
        Err(StyleError::InvalidValue {
            attribute: "Styles",
            ..
        })
    ));
}

#[test]
fn curve_colour_rejects_non_colour_reference() {
    let schema = ifc4();
    let mut model = Model::new();
    let wrong = model.push(Entity::new("IFCREPRESENTATIONITEM", vec![]));
    let style = model.push(Entity::new(
        "IFCCURVESTYLE",
        vec![
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Ref(wrong),
            Value::Null,
        ],
    ));

    assert!(matches!(
        StyleView::new(&model, schema)
            .curve_style(style)
            .unwrap()
            .curve_colour(),
        Err(StyleError::ReferenceType {
            target,
            expected: "IfcColour",
            ..
        }) if target == wrong
    ));
}

#[test]
fn colour_or_factor_rejects_non_colour_reference() {
    let schema = ifc4();
    let mut model = Model::new();
    let colour = model.push(Entity::new(
        "IFCCOLOURRGB",
        vec![Value::Null, real(0.1), real(0.2), real(0.3)],
    ));
    let wrong = model.push(Entity::new("IFCREPRESENTATIONITEM", vec![]));
    let rendering = model.push(Entity::new(
        "IFCSURFACESTYLERENDERING",
        vec![
            Value::Ref(colour),
            Value::Null,
            Value::Ref(wrong),
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Null,
            enumeration("NOTDEFINED"),
        ],
    ));

    assert!(matches!(
        StyleView::new(&model, schema)
            .surface_style_rendering(rendering)
            .unwrap()
            .diffuse_colour(),
        Err(StyleError::ReferenceType {
            target,
            expected: "IfcColour",
            ..
        }) if target == wrong
    ));
}

#[test]
fn text_and_texture_single_references_are_strict() {
    let schema = ifc4();
    let mut model = Model::new();
    let wrong = model.push(Entity::new("IFCREPRESENTATIONITEM", vec![]));
    let text_style = model.push(Entity::new(
        "IFCTEXTSTYLE",
        vec![
            Value::Null,
            Value::Ref(wrong),
            Value::Ref(wrong),
            Value::Ref(wrong),
            Value::Null,
        ],
    ));
    let image = model.push(Entity::new(
        "IFCIMAGETEXTURE",
        vec![
            Value::Bool(true),
            Value::Bool(true),
            Value::Null,
            Value::Ref(wrong),
            Value::Null,
            text("texture.png"),
        ],
    ));
    let view = StyleView::new(&model, schema);
    let text = view.text_style(text_style).unwrap();

    for result in [
        text.text_character_appearance(),
        text.text_style(),
        text.text_font_style().map(Some),
    ] {
        assert!(matches!(
            result,
            Err(StyleError::ReferenceType { target, .. }) if target == wrong
        ));
    }
    assert!(matches!(
        view.image_texture(image)
            .unwrap()
            .surface_texture()
            .texture_transform(),
        Err(StyleError::ReferenceType {
            target,
            expected: "IfcCartesianTransformationOperator2D",
            ..
        }) if target == wrong
    ));
}

#[test]
fn text_font_and_texture_coordinates_are_required() {
    let schema = ifc4();
    let mut model = Model::new();
    let wrong = model.push(Entity::new("IFCREPRESENTATIONITEM", vec![]));
    let text_style = model.push(Entity::new(
        "IFCTEXTSTYLE",
        vec![
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Null,
        ],
    ));
    let map = model.push(Entity::new(
        "IFCINDEXEDTRIANGLETEXTUREMAP",
        vec![
            Value::Ref(wrong),
            Value::Ref(wrong),
            Value::Ref(wrong),
            Value::List(vec![]),
        ],
    ));
    let view = StyleView::new(&model, schema);

    assert!(matches!(
        view.text_style(text_style).unwrap().text_font_style(),
        Err(StyleError::MissingAttribute {
            attribute: "TextFontStyle",
            ..
        })
    ));
    assert!(matches!(
        view.indexed_texture_map(map).unwrap().tex_coords(),
        Err(StyleError::ReferenceType {
            target,
            expected: "IfcTextureVertexList",
            ..
        }) if target == wrong
    ));
}
