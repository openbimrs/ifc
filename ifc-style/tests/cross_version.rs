use ifc_model::{Entity, Model, Value};
use ifc_schema::{ifc2x3, ifc4, ifc4x3, Schema};
use ifc_style::{AnnotationType, ColourOrFactor, StyleView};
use std::sync::Arc;

fn named_entity(schema: &Schema, type_name: &str, fields: Vec<(&str, Value)>) -> Entity {
    let attributes = schema.attributes(type_name);
    assert!(
        !attributes.is_empty(),
        "{type_name} must exist in {}",
        schema.name()
    );
    let mut values = vec![Value::Null; attributes.len()];
    for (name, value) in fields {
        let index = attributes
            .iter()
            .position(|attribute| attribute.name.eq_ignore_ascii_case(name))
            .unwrap_or_else(|| panic!("{type_name}.{name} must exist in {}", schema.name()));
        values[index] = value;
    }
    Entity::new(type_name, values)
}

fn text(value: &str) -> Value {
    Value::Text(Arc::from(value))
}

#[test]
fn image_texture_exposes_version_specific_mode_without_slot_guesses() {
    for schema in [ifc2x3(), ifc4(), ifc4x3()] {
        let mut fields = vec![
            ("RepeatS", Value::Bool(true)),
            ("RepeatT", Value::Bool(false)),
            ("URLReference", text("textures/fill.png")),
        ];
        if schema.version() == Some(ifc_schema::SchemaVersion::Ifc2x3) {
            fields.push(("TextureType", Value::Enum(Arc::from("SPECULAR"))));
        } else {
            fields.push(("Mode", text("DIFFUSE")));
        }
        let mut model = Model::new();
        let id = model.push(named_entity(schema, "IfcImageTexture", fields));
        let texture = StyleView::new(&model, schema).image_texture(id).unwrap();
        assert!(texture.surface_texture().repeat_s().unwrap());
        assert!(!texture.surface_texture().repeat_t().unwrap());
        assert_eq!(texture.url_reference().unwrap(), "textures/fill.png");
        if schema.version() == Some(ifc_schema::SchemaVersion::Ifc2x3) {
            assert_eq!(
                texture.surface_texture().texture_type().unwrap(),
                Some("SPECULAR")
            );
            assert_eq!(texture.surface_texture().mode().unwrap(), None);
        } else {
            assert_eq!(texture.surface_texture().mode().unwrap(), Some("DIFFUSE"));
            assert_eq!(texture.surface_texture().texture_type().unwrap(), None);
        }
    }
}

#[test]
fn inherited_curve_style_slots_are_resolved_by_name_in_all_schemas() {
    for schema in [ifc2x3(), ifc4(), ifc4x3()] {
        let mut fields = vec![("Name", text("dash-dot"))];
        if schema.version() != Some(ifc_schema::SchemaVersion::Ifc2x3) {
            fields.push(("ModelOrDraughting", Value::Bool(true)));
        }
        let mut model = Model::new();
        let id = model.push(named_entity(schema, "IfcCurveStyle", fields));
        let style = StyleView::new(&model, schema).curve_style(id).unwrap();
        assert_eq!(style.name().unwrap(), Some("dash-dot"));
        assert_eq!(
            style.model_or_draughting().unwrap(),
            (schema.version() != Some(ifc_schema::SchemaVersion::Ifc2x3)).then_some(true)
        );
    }
}

#[test]
fn rendering_selects_and_annotation_predefined_type_remain_typed() {
    for schema in [ifc2x3(), ifc4(), ifc4x3()] {
        let mut model = Model::new();
        let colour = model.push(named_entity(
            schema,
            "IfcColourRgb",
            vec![
                ("Red", Value::Real(0.2)),
                ("Green", Value::Real(0.3)),
                ("Blue", Value::Real(0.4)),
            ],
        ));
        let rendering = model.push(named_entity(
            schema,
            "IfcSurfaceStyleRendering",
            vec![
                ("SurfaceColour", Value::Ref(colour)),
                ("Transparency", Value::Real(0.1)),
                ("DiffuseColour", Value::Real(0.75)),
                ("ReflectanceMethod", Value::Enum(Arc::from("PHONG"))),
            ],
        ));
        let view = StyleView::new(&model, schema);
        let rendering = view.surface_style_rendering(rendering).unwrap();
        assert_eq!(rendering.surface_colour().unwrap(), colour);
        assert_eq!(rendering.transparency().unwrap(), Some(0.1));
        assert_eq!(
            rendering.diffuse_colour().unwrap(),
            Some(ColourOrFactor::Factor(0.75))
        );
        assert_eq!(rendering.reflectance_method().unwrap(), "PHONG");

        let mut annotation_fields = vec![("GlobalId", text("0wA1b2C3d4E5f6G7h8I9Jk"))];
        if schema.version() == Some(ifc_schema::SchemaVersion::Ifc4x3) {
            annotation_fields.push(("PredefinedType", Value::Enum(Arc::from("TEXT"))));
        }
        let annotation = model.push(named_entity(schema, "IfcAnnotation", annotation_fields));
        assert_eq!(
            StyleView::new(&model, schema)
                .annotation(annotation)
                .unwrap()
                .predefined_type()
                .unwrap(),
            (schema.version() == Some(ifc_schema::SchemaVersion::Ifc4x3))
                .then_some(AnnotationType::Text)
        );
    }
}
