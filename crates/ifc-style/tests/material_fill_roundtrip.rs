//! A resolved fill, authored back through the public writers, written as
//! STEP, read and resolved again, is the same fill (#597).

use ifc_model::{Codec, Entity, EntityId, Model, Transaction, Value};
use ifc_schema::{for_version, Schema, SchemaVersion};
use ifc_step::StepCodec;
use ifc_style::{
    create_colour_rgb, create_curve_style, create_curve_style_font,
    create_curve_style_font_pattern, create_draughting_predefined_curve_font,
    create_fill_area_style, create_fill_area_style_hatching, create_styled_item, ColourRgbDraft,
    CurveStyleDraft, CurveWidth, FillStyleKind, HatchLine, HatchLineDistance, MaterialFill,
    StyleIndex, StyleUnits, StyleView, StyledItemDraft,
};

fn fixture(name: &str) -> Model {
    let path = format!(
        "{}/../../test/fixtures/synthetic-fill/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    StepCodec
        .read_bytes(&std::fs::read(path).expect("fixture"))
        .expect("parses")
}

fn schema_of(model: &Model) -> &'static Schema {
    let version = SchemaVersion::from_header_token(&model.header().schema[0]).expect("release");
    for_version(version).expect("bundled")
}

fn resolve(model: &Model, material: EntityId) -> MaterialFill {
    StyleIndex::new(StyleView::new(model, schema_of(model)))
        .unwrap()
        .material_fill(material, StyleUnits::SI)
        .unwrap()
        .expect("a fill")
}

/// Stage `name` with its attributes placed by the schema's own slots.
fn raw(tx: &mut Transaction, schema: &Schema, name: &str, values: Vec<(&str, Value)>) -> EntityId {
    let attributes = schema.attributes(name);
    let mut slots = vec![Value::Null; attributes.len()];
    for (attribute, value) in values {
        let slot = attributes
            .iter()
            .position(|a| a.name.eq_ignore_ascii_case(attribute))
            .unwrap_or_else(|| panic!("{name} has no {attribute}"));
        slots[slot] = value;
    }
    tx.create(Entity::new(name.to_ascii_uppercase(), slots))
}

fn reals(values: &[f64]) -> Value {
    Value::List(values.iter().copied().map(Value::Real).collect())
}

/// Author `fill` back and return the model and its material.
fn author(fill: &MaterialFill, token: &str, schema: &Schema) -> (Model, EntityId) {
    let ifc2x3 = schema.entity("IfcOneDirectionRepeatFactor").is_some();
    let mut model = Model::default();
    model.header_mut().schema = vec![token.to_owned()];
    let mut tx = Transaction::new(&model);
    let colour = |tx: &mut Transaction, [red, green, blue]: [f64; 3]| {
        create_colour_rgb(tx, schema, ColourRgbDraft::new(red, green, blue)).unwrap()
    };
    let point = |tx: &mut Transaction, xy: [f64; 2]| {
        raw(
            tx,
            schema,
            "IfcCartesianPoint",
            vec![("Coordinates", reals(&xy))],
        )
    };

    let mut members = Vec::new();
    for line in &fill.lines {
        let mut curve = CurveStyleDraft::new();
        match &line.dashes {
            None => {}
            Some(pairs) if pairs.is_empty() => {
                let font =
                    create_draughting_predefined_curve_font(&mut tx, schema, "continuous").unwrap();
                curve = curve.curve_font(font);
            }
            Some(pairs) => {
                let patterns: Vec<_> = pairs
                    .iter()
                    .map(|[visible, invisible]| {
                        create_curve_style_font_pattern(&mut tx, schema, *visible, *invisible)
                            .unwrap()
                    })
                    .collect();
                let font = create_curve_style_font(&mut tx, schema, None, &patterns).unwrap();
                curve = curve.curve_font(font);
            }
        }
        if let Some(width) = line.width {
            curve = curve.curve_width(CurveWidth::PositiveLength(width));
        }
        if let Some(rgb) = line.colour {
            let colour = colour(&mut tx, rgb);
            curve = curve.curve_colour(colour);
        }
        let appearance = create_curve_style(&mut tx, schema, curve).unwrap();

        let distance = if line.shift == 0.0 {
            HatchLineDistance::Length(line.spacing)
        } else {
            let (sin, cos) = line.angle_rad.sin_cos();
            let v = [
                -line.spacing * sin + line.shift * cos,
                line.spacing * cos + line.shift * sin,
            ];
            let magnitude = v[0].hypot(v[1]);
            let direction = raw(
                &mut tx,
                schema,
                "IfcDirection",
                vec![(
                    "DirectionRatios",
                    reals(&[v[0] / magnitude, v[1] / magnitude]),
                )],
            );
            let vector = raw(
                &mut tx,
                schema,
                "IfcVector",
                vec![
                    ("Orientation", Value::Ref(direction)),
                    ("Magnitude", Value::Real(magnitude)),
                ],
            );
            HatchLineDistance::Vector(if ifc2x3 {
                raw(
                    &mut tx,
                    schema,
                    "IfcOneDirectionRepeatFactor",
                    vec![("RepeatFactor", Value::Ref(vector))],
                )
            } else {
                vector
            })
        };
        let origin = point(&mut tx, line.origin);
        let start = line.pattern_start.map(|xy| point(&mut tx, xy));
        let hatching = create_fill_area_style_hatching(
            &mut tx,
            &model,
            schema,
            appearance,
            distance,
            (Some(origin), start),
            line.angle_rad,
        )
        .unwrap();
        members.push((hatching, FillStyleKind::Hatching));
    }
    if let Some(rgb) = fill.background {
        members.push((colour(&mut tx, rgb), FillStyleKind::Colour));
    }
    let style = create_fill_area_style(
        &mut tx,
        schema,
        fill.name.as_deref(),
        &members,
        fill.model_or_draughting,
    )
    .unwrap();
    let styled =
        create_styled_item(&mut tx, &model, schema, StyledItemDraft::new(vec![style])).unwrap();

    let origin = point(&mut tx, [0.0, 0.0]);
    let axis = raw(
        &mut tx,
        schema,
        "IfcAxis2Placement2D",
        vec![("Location", Value::Ref(origin))],
    );
    let context = raw(
        &mut tx,
        schema,
        "IfcGeometricRepresentationContext",
        vec![
            ("ContextType", Value::Text("Plan".into())),
            ("CoordinateSpaceDimension", Value::Integer(2)),
            ("Precision", Value::Real(1e-5)),
            ("WorldCoordinateSystem", Value::Ref(axis)),
        ],
    );
    let representation = raw(
        &mut tx,
        schema,
        "IfcStyledRepresentation",
        vec![
            ("ContextOfItems", Value::Ref(context)),
            ("RepresentationIdentifier", Value::Text("Style".into())),
            ("RepresentationType", Value::Text("Material".into())),
            ("Items", Value::List(vec![Value::Ref(styled)])),
        ],
    );
    let material = raw(
        &mut tx,
        schema,
        "IfcMaterial",
        vec![("Name", Value::Text("Authored".into()))],
    );
    raw(
        &mut tx,
        schema,
        "IfcMaterialDefinitionRepresentation",
        vec![
            (
                "Representations",
                Value::List(vec![Value::Ref(representation)]),
            ),
            ("RepresentedMaterial", Value::Ref(material)),
        ],
    );
    tx.commit(&mut model).expect("commit");
    (model, material)
}

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-12
}

fn near_all(a: &[f64], b: &[f64]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(a, b)| near(*a, *b))
}

fn flat(dashes: &Option<Vec<[f64; 2]>>) -> Option<Vec<f64>> {
    dashes
        .as_ref()
        .map(|pairs| pairs.iter().flatten().copied().collect())
}

fn same_line(a: &HatchLine, b: &HatchLine) -> bool {
    near(a.angle_rad, b.angle_rad)
        && near(a.spacing, b.spacing)
        && near(a.shift, b.shift)
        && near_all(&a.origin, &b.origin)
        && a.pattern_start.is_some() == b.pattern_start.is_some()
        && near_all(
            &a.pattern_start.unwrap_or_default(),
            &b.pattern_start.unwrap_or_default(),
        )
        && match (flat(&a.dashes), flat(&b.dashes)) {
            (None, None) => true,
            (Some(a), Some(b)) => near_all(&a, &b),
            _ => false,
        }
        && a.colour.is_some() == b.colour.is_some()
        && near_all(&a.colour.unwrap_or_default(), &b.colour.unwrap_or_default())
        && a.width.is_some() == b.width.is_some()
        && near(a.width.unwrap_or_default(), b.width.unwrap_or_default())
}

#[test]
fn fixtures_round_trip_through_authoring_fill() {
    for (name, token) in [
        ("material_fill.ifc", "IFC4"),
        ("material_fill_ifc4x3.ifc", "IFC4X3_ADD2"),
        ("material_fill_ifc2x3.ifc", "IFC2X3"),
    ] {
        let source = fixture(name);
        for material in ["Concrete", "Insulation", "Scaled"] {
            let id = source
                .of_type("IFCMATERIAL")
                .find(|(_, e)| e.text(0) == Some(material))
                .map(|(id, _)| id)
                .unwrap();
            let a = resolve(&source, id);

            let (authored, authored_material) = author(&a, token, schema_of(&source));
            let bytes = StepCodec.write_bytes(&authored).expect("writes");
            let reread = StepCodec.read_bytes(&bytes).expect("reads");
            let b = resolve(&reread, authored_material);

            let context = format!("{name} {material}");
            assert_eq!(a.name, b.name, "{context}");
            assert_eq!(a.model_or_draughting, b.model_or_draughting, "{context}");
            assert_eq!(a.background, b.background, "{context}");
            assert_eq!(a.lines.len(), b.lines.len(), "{context}");
            for (a, b) in a.lines.iter().zip(&b.lines) {
                assert!(same_line(a, b), "{context}: {a:?} vs {b:?}");
            }
        }
    }
}
