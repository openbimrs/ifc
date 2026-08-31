use ifc_model::{Entity, EntityId, Model, Transaction, Value};
use ifc_schema::ifc4x3;
use ifc_style::{
    create_annotation, create_annotation_fill_area, create_text_literal,
    create_text_literal_with_extent, AnnotationDraft, AnnotationFillAreaDraft, AnnotationType,
    BoxAlignment, StyleError, StyleView, TextLiteralDraft, TextLiteralWithExtentDraft, TextPath,
};
use std::sync::Arc;

fn text(value: &str) -> Value {
    Value::Text(Arc::from(value))
}

fn enumeration(value: &str) -> Value {
    Value::Enum(Arc::from(value))
}

fn ref_entity(model: &mut Model, type_name: &str) -> EntityId {
    model.push(Entity::new(type_name, vec![]))
}

#[test]
fn reads_annotation_text_and_fill_area_without_collapsing_optional_values() {
    let schema = ifc4x3();
    let mut model = Model::new();
    let placement = ref_entity(&mut model, "IFCAXIS2PLACEMENT2D");
    let extent = ref_entity(&mut model, "IFCPLANAREXTENT");
    let outer = ref_entity(&mut model, "IFCPOLYLINE");
    let inner = ref_entity(&mut model, "IFCCIRCLE");

    let annotation = model.push(Entity::new(
        "IFCANNOTATION",
        vec![
            text("3vB2YO$MX4xv5uCqZZG05x"),
            Value::Null,
            text("Fire note"),
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Null,
            enumeration("TEXT"),
        ],
    ));
    let literal = model.push(Entity::new(
        "IFCTEXTLITERAL",
        vec![text("EI 90"), Value::Ref(placement), enumeration("RIGHT")],
    ));
    let literal_with_extent = model.push(Entity::new(
        "IFCTEXTLITERALWITHEXTENT",
        vec![
            text("EI 90"),
            Value::Ref(placement),
            enumeration("RIGHT"),
            Value::Ref(extent),
            text("top-left"),
        ],
    ));
    let fill = model.push(Entity::new(
        "IFCANNOTATIONFILLAREA",
        vec![Value::Ref(outer), Value::List(vec![Value::Ref(inner)])],
    ));

    let view = StyleView::new(&model, schema);
    let annotation = view.annotation(annotation).unwrap();
    assert_eq!(annotation.global_id().unwrap(), "3vB2YO$MX4xv5uCqZZG05x");
    assert_eq!(annotation.name().unwrap(), Some("Fire note"));
    assert_eq!(
        annotation.predefined_type().unwrap(),
        Some(AnnotationType::Text)
    );
    assert_eq!(annotation.owner_history().unwrap(), None);
    assert_eq!(annotation.object_placement().unwrap(), None);
    assert_eq!(annotation.representation().unwrap(), None);

    let literal = view.text_literal(literal).unwrap();
    assert_eq!(literal.literal().unwrap(), "EI 90");
    assert_eq!(literal.placement().unwrap(), placement);
    assert_eq!(literal.path().unwrap(), TextPath::Right);

    let literal = view.text_literal_with_extent(literal_with_extent).unwrap();
    assert_eq!(literal.extent().unwrap(), extent);
    assert_eq!(literal.box_alignment().unwrap(), BoxAlignment::TopLeft);

    let fill = view.annotation_fill_area(fill).unwrap();
    assert_eq!(fill.outer_boundary().unwrap(), outer);
    assert_eq!(fill.inner_boundaries().unwrap(), vec![inner]);
}

#[test]
fn invalid_box_alignment_is_a_typed_error() {
    let schema = ifc4x3();
    let mut model = Model::new();
    let placement = ref_entity(&mut model, "IFCAXIS2PLACEMENT2D");
    let extent = ref_entity(&mut model, "IFCPLANAREXTENT");
    let id = model.push(Entity::new(
        "IFCTEXTLITERALWITHEXTENT",
        vec![
            text("EI 90"),
            Value::Ref(placement),
            enumeration("RIGHT"),
            Value::Ref(extent),
            text("left"),
        ],
    ));
    let error = StyleView::new(&model, schema)
        .text_literal_with_extent(id)
        .unwrap()
        .box_alignment()
        .unwrap_err();
    assert!(matches!(
        error,
        StyleError::InvalidValue {
            attribute: "BoxAlignment",
            ..
        }
    ));
}

#[test]
fn malformed_optional_annotation_name_is_not_reported_as_absent() {
    let schema = ifc4x3();
    let mut model = Model::new();
    let id = model.push(Entity::new(
        "IFCANNOTATION",
        vec![
            text("3vB2YO$MX4xv5uCqZZG05x"),
            Value::Null,
            Value::Integer(7),
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Null,
            Value::Null,
        ],
    ));

    let error = StyleView::new(&model, schema)
        .annotation(id)
        .unwrap()
        .name()
        .unwrap_err();
    assert!(matches!(
        error,
        StyleError::InvalidValue {
            attribute: "Name",
            ..
        }
    ));
}

#[test]
fn authors_the_requested_annotation_graph_atomically() {
    let schema = ifc4x3();
    let mut model = Model::new();
    let placement = ref_entity(&mut model, "IFCAXIS2PLACEMENT2D");
    let extent = ref_entity(&mut model, "IFCPLANAREXTENT");
    let outer = ref_entity(&mut model, "IFCPOLYLINE");
    let inner = ref_entity(&mut model, "IFCCIRCLE");

    let mut tx = Transaction::new(&model);
    let annotation = create_annotation(
        &mut tx,
        &model,
        schema,
        AnnotationDraft {
            global_id: "3vB2YO$MX4xv5uCqZZG05x",
            name: Some("Fire note"),
            predefined_type: Some(AnnotationType::Text),
            ..AnnotationDraft::default()
        },
    )
    .unwrap();
    let literal = create_text_literal(
        &mut tx,
        &model,
        schema,
        TextLiteralDraft {
            literal: "EI 90",
            placement,
            path: TextPath::Right,
        },
    )
    .unwrap();
    let literal_with_extent = create_text_literal_with_extent(
        &mut tx,
        &model,
        schema,
        TextLiteralWithExtentDraft {
            literal: "EI 90",
            placement,
            path: TextPath::Right,
            extent,
            box_alignment: BoxAlignment::TopLeft,
        },
    )
    .unwrap();
    let fill = create_annotation_fill_area(
        &mut tx,
        &model,
        schema,
        AnnotationFillAreaDraft {
            outer_boundary: outer,
            inner_boundaries: vec![inner],
        },
    )
    .unwrap();

    assert_eq!(model.len(), 4, "staging must not mutate the model");
    tx.commit(&mut model).unwrap();
    let view = StyleView::new(&model, schema);
    assert_eq!(
        view.annotation(annotation).unwrap().name().unwrap(),
        Some("Fire note")
    );
    assert_eq!(
        view.text_literal(literal).unwrap().literal().unwrap(),
        "EI 90"
    );
    assert_eq!(
        view.text_literal_with_extent(literal_with_extent)
            .unwrap()
            .box_alignment()
            .unwrap(),
        BoxAlignment::TopLeft
    );
    assert_eq!(
        view.annotation_fill_area(fill)
            .unwrap()
            .inner_boundaries()
            .unwrap(),
        vec![inner]
    );
}

#[test]
fn authoring_rejects_wrong_reference_types_without_staging_partial_edits() {
    let schema = ifc4x3();
    let mut model = Model::new();
    let not_a_curve = ref_entity(&mut model, "IFCCOLOURRGB");
    let mut tx = Transaction::new(&model);

    let error = create_annotation_fill_area(
        &mut tx,
        &model,
        schema,
        AnnotationFillAreaDraft {
            outer_boundary: not_a_curve,
            inner_boundaries: vec![],
        },
    )
    .unwrap_err();

    assert!(matches!(
        error,
        StyleError::ReferenceType {
            target,
            expected: "IfcCurve",
            ..
        } if target == not_a_curve
    ));
    assert!(
        tx.is_empty(),
        "failed authoring must not stage partial edits"
    );
}

#[test]
fn user_defined_annotation_requires_object_type_before_staging() {
    let schema = ifc4x3();
    let model = Model::new();
    let mut tx = Transaction::new(&model);

    let error = create_annotation(
        &mut tx,
        &model,
        schema,
        AnnotationDraft {
            global_id: "3vB2YO$MX4xv5uCqZZG05x",
            predefined_type: Some(AnnotationType::UserDefined),
            object_type: Some("  "),
            ..AnnotationDraft::default()
        },
    )
    .unwrap_err();

    assert!(matches!(
        error,
        StyleError::AuthoringInvalid {
            entity: "IfcAnnotation",
            attribute: "ObjectType",
            ..
        }
    ));
    assert!(tx.is_empty());
}
