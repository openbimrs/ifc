use ifc_model::{Entity, Model, Transaction};
use ifc_schema::{ifc2x3, ifc4, ifc4x3};
use ifc_style::{
    create_colour_rgb, create_presentation_layer_with_style, create_styled_item,
    create_surface_style, create_surface_style_shading, ColourRgbDraft, PresentationLayerDraft,
    StyleError, StyleSource, StyleView, StyledItemDraft, SurfaceSide, SurfaceStyleDraft,
    SurfaceStyleShadingDraft,
};

#[test]
fn authors_a_complete_surface_style_graph_in_one_transaction() {
    let schema = ifc4x3();
    let mut model = Model::new();
    let item = model.push(Entity::new("IFCANNOTATIONFILLAREA", vec![]));
    let mut tx = Transaction::new(&model);

    let colour = create_colour_rgb(
        &mut tx,
        schema,
        ColourRgbDraft {
            name: Some("blue"),
            red: 0.1,
            green: 0.2,
            blue: 0.9,
        },
    )
    .unwrap();
    let shading = create_surface_style_shading(
        &mut tx,
        &model,
        schema,
        SurfaceStyleShadingDraft {
            surface_colour: colour,
            transparency: Some(0.25),
        },
    )
    .unwrap();
    let surface = create_surface_style(
        &mut tx,
        &model,
        schema,
        SurfaceStyleDraft {
            name: Some("annotation fill"),
            side: SurfaceSide::Both,
            elements: vec![shading],
        },
    )
    .unwrap();
    let styled = create_styled_item(
        &mut tx,
        &model,
        schema,
        StyledItemDraft {
            item: Some(item),
            styles: vec![surface],
            name: None,
        },
    )
    .unwrap();
    let layer = create_presentation_layer_with_style(
        &mut tx,
        &model,
        schema,
        PresentationLayerDraft {
            name: "A-ANNO",
            description: None,
            assigned_items: vec![item],
            identifier: Some("A-ANNO"),
            layer_on: Some(true),
            layer_frozen: Some(false),
            layer_blocked: Some(false),
            layer_styles: vec![surface],
        },
    )
    .unwrap();

    assert_eq!(model.len(), 1);
    tx.commit(&mut model).unwrap();
    let view = StyleView::new(&model, schema);
    assert_eq!(view.colour_rgb(colour).unwrap().blue().unwrap(), 0.9);
    assert_eq!(
        view.surface_style(surface).unwrap().elements().unwrap(),
        vec![shading]
    );
    assert_eq!(
        view.styled_item(styled).unwrap().item().unwrap(),
        Some(item)
    );
    assert_eq!(
        view.presentation_layer(layer).unwrap().name().unwrap(),
        "A-ANNO"
    );
}

#[test]
fn ifc2x3_writer_emits_the_required_presentation_style_assignment_wrapper() {
    let schema = ifc2x3();
    let mut model = Model::new();
    let item = model.push(Entity::new("IFCANNOTATIONFILLAREA", vec![]));
    let style = model.push(Entity::new(
        "IFCSURFACESTYLE",
        vec![
            ifc_model::Value::Null,
            ifc_model::Value::Enum("BOTH".into()),
            ifc_model::Value::List(vec![ifc_model::Value::Ref(item)]),
        ],
    ));
    let mut tx = Transaction::new(&model);
    let styled = create_styled_item(
        &mut tx,
        &model,
        schema,
        StyledItemDraft {
            item: Some(item),
            styles: vec![style],
            name: None,
        },
    )
    .unwrap();

    assert_eq!(
        tx.len(),
        2,
        "wrapper and styled item must be staged together"
    );
    tx.commit(&mut model).unwrap();
    let resolved = StyleView::new(&model, schema)
        .resolve_item_style(item)
        .unwrap();
    assert_eq!(resolved.source(), StyleSource::DirectStyledItem(styled));
    assert_eq!(resolved.effective_styles(), &[style]);
}

#[test]
fn invalid_colour_is_rejected_before_any_edit_is_staged() {
    let model = Model::new();
    let mut tx = Transaction::new(&model);
    let err = create_colour_rgb(
        &mut tx,
        ifc4x3(),
        ColourRgbDraft {
            name: None,
            red: 1.25,
            green: 0.0,
            blue: 1.0,
        },
    )
    .unwrap_err();
    assert!(matches!(
        err,
        StyleError::AuthoringInvalid {
            entity: "IfcColourRgb",
            attribute: "Red",
            ..
        }
    ));
    assert!(tx.is_empty());
}

#[test]
fn empty_layer_styles_are_valid_in_all_supported_schemas() {
    for schema in [ifc2x3(), ifc4(), ifc4x3()] {
        let mut model = Model::new();
        let item = model.push(Entity::new("IFCANNOTATIONFILLAREA", vec![]));
        let mut tx = Transaction::new(&model);
        let layer = create_presentation_layer_with_style(
            &mut tx,
            &model,
            schema,
            PresentationLayerDraft {
                name: "A-ANNO",
                description: None,
                assigned_items: vec![item],
                identifier: None,
                layer_on: Some(true),
                layer_frozen: Some(false),
                layer_blocked: Some(false),
                layer_styles: vec![],
            },
        )
        .unwrap();

        tx.commit(&mut model).unwrap();
        assert!(StyleView::new(&model, schema)
            .presentation_layer(layer)
            .unwrap()
            .layer_styles()
            .unwrap()
            .is_empty());
    }
}

#[test]
fn ifc2x3_writer_emits_a_valid_nonempty_layer_style_aggregate() {
    let schema = ifc2x3();
    let mut model = Model::new();
    let item = model.push(Entity::new("IFCANNOTATIONFILLAREA", vec![]));
    let style = model.push(Entity::new(
        "IFCSURFACESTYLE",
        vec![
            ifc_model::Value::Null,
            ifc_model::Value::Enum("BOTH".into()),
            ifc_model::Value::List(vec![ifc_model::Value::Ref(item)]),
        ],
    ));
    let mut tx = Transaction::new(&model);

    let layer = create_presentation_layer_with_style(
        &mut tx,
        &model,
        schema,
        PresentationLayerDraft {
            name: "A-ANNO",
            description: None,
            assigned_items: vec![item],
            identifier: None,
            layer_on: Some(true),
            layer_frozen: Some(false),
            layer_blocked: Some(false),
            layer_styles: vec![style],
        },
    )
    .unwrap();

    tx.commit(&mut model).unwrap();
    assert_eq!(
        StyleView::new(&model, schema)
            .presentation_layer(layer)
            .unwrap()
            .layer_styles()
            .unwrap(),
        vec![style]
    );
}

#[test]
fn surface_style_authoring_rejects_duplicate_where_rule_category_without_partial_staging() {
    let schema = ifc4();
    let model = Model::new();
    let mut tx = Transaction::new(&model);
    let colour = create_colour_rgb(
        &mut tx,
        schema,
        ColourRgbDraft {
            name: None,
            red: 1.0,
            green: 0.0,
            blue: 0.0,
        },
    )
    .unwrap();
    let first = create_surface_style_shading(
        &mut tx,
        &model,
        schema,
        SurfaceStyleShadingDraft {
            surface_colour: colour,
            transparency: None,
        },
    )
    .unwrap();
    let second = create_surface_style_shading(
        &mut tx,
        &model,
        schema,
        SurfaceStyleShadingDraft {
            surface_colour: colour,
            transparency: Some(0.5),
        },
    )
    .unwrap();
    let staged_before = tx.len();

    let error = create_surface_style(
        &mut tx,
        &model,
        schema,
        SurfaceStyleDraft {
            name: None,
            side: SurfaceSide::Both,
            elements: vec![first, second],
        },
    )
    .unwrap_err();

    assert!(matches!(
        error,
        StyleError::AuthoringInvalid {
            entity: "IfcSurfaceStyle",
            attribute: "Styles",
            ..
        }
    ));
    assert_eq!(tx.len(), staged_before);
}
