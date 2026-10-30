//! `StyleIndex::material_fill` against the generated `synthetic-fill`
//! fixtures in IFC2X3, IFC4 and IFC4X3 and a millimetre/degree file (#597).

use ifc_model::{Codec, EntityId, Model};
use ifc_schema::{for_version, Schema, SchemaVersion};
use ifc_step::StepCodec;
use ifc_style::{
    HatchLine, MaterialFill, StyleError, StyleIndex, StyleResult, StyleUnits, StyleView,
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

/// The id of the entity of `type_name` whose attribute `slot` is the text `name`.
fn named(model: &Model, type_name: &str, slot: usize, name: &str) -> EntityId {
    model
        .of_type(type_name)
        .find(|(_, entity)| entity.text(slot) == Some(name))
        .map(|(id, _)| id)
        .unwrap_or_else(|| panic!("no {type_name} named {name}"))
}

fn material(model: &Model, name: &str) -> EntityId {
    named(model, "IFCMATERIAL", 0, name)
}

fn fill(model: &Model, name: &str, units: StyleUnits) -> StyleResult<Option<MaterialFill>> {
    let index = StyleIndex::new(StyleView::new(model, schema_of(model))).unwrap();
    index.material_fill(material(model, name), units)
}

/// The `n`th `FillStyles` member of the fill area style called `name`.
fn member(model: &Model, name: &str, n: usize) -> EntityId {
    let style = named(model, "IFCFILLAREASTYLE", 0, name);
    model
        .get(style)
        .unwrap()
        .attribute(1)
        .unwrap()
        .as_list()
        .unwrap()[n]
        .as_ref_id()
        .unwrap()
}

fn near(got: f64, want: f64) -> bool {
    (got - want).abs() < 1e-12
}

fn all_near(got: &[f64], want: &[f64]) -> bool {
    got.len() == want.len() && got.iter().zip(want).all(|(g, w)| near(*g, *w))
}

/// A hatch line as the fixtures' generator states it.
struct Line {
    angle: f64,
    spacing: f64,
    shift: f64,
    origin: [f64; 2],
    start: Option<[f64; 2]>,
    dashes: Option<Vec<[f64; 2]>>,
    colour: Option<[f64; 3]>,
    width: Option<f64>,
}

fn check_line(got: &HatchLine, want: &Line) {
    let flat = |dashes: &Option<Vec<[f64; 2]>>| {
        dashes
            .as_ref()
            .map(|pairs| pairs.iter().flatten().copied().collect::<Vec<_>>())
    };
    let ok = near(got.angle_rad, want.angle)
        && near(got.spacing, want.spacing)
        && near(got.shift, want.shift)
        && all_near(&got.origin, &want.origin)
        && got.pattern_start.is_some() == want.start.is_some()
        && got
            .pattern_start
            .zip(want.start)
            .is_none_or(|(g, w)| all_near(&g, &w))
        && match (flat(&got.dashes), flat(&want.dashes)) {
            (None, None) => true,
            (Some(g), Some(w)) => all_near(&g, &w),
            _ => false,
        }
        && got.colour.is_some() == want.colour.is_some()
        && got
            .colour
            .zip(want.colour)
            .is_none_or(|(g, w)| all_near(&g, &w))
        && got.width.is_some() == want.width.is_some()
        && got.width.zip(want.width).is_none_or(|(g, w)| near(g, w));
    assert!(ok, "{got:?}");
}

fn check_concrete(fill: &MaterialFill, model_or_draughting: Option<bool>) {
    assert_eq!(fill.name.as_deref(), Some("Concrete cut"));
    assert_eq!(fill.model_or_draughting, model_or_draughting);
    assert!(all_near(&fill.background.expect("background"), &[0.9; 3]));
    assert_eq!(fill.lines.len(), 2, "{fill:?}");
    check_line(
        &fill.lines[0],
        &Line {
            angle: std::f64::consts::FRAC_PI_4,
            spacing: 0.005,
            shift: 0.0,
            origin: [0.0, 0.0],
            start: None,
            dashes: Some(vec![]),
            colour: Some([0.2; 3]),
            width: Some(0.00025),
        },
    );
    check_line(
        &fill.lines[1],
        &Line {
            angle: 0.0,
            spacing: 0.004,
            shift: 0.003,
            origin: [0.0005, 0.0],
            start: Some([0.001, 0.0]),
            dashes: Some(vec![[0.002, 0.001]]),
            colour: Some([0.0; 3]),
            width: None,
        },
    );
}

#[test]
fn material_fill_resolves_in_every_release() {
    for (name, flag) in [
        ("material_fill.ifc", Some(true)),
        ("material_fill_ifc4x3.ifc", Some(true)),
        ("material_fill_ifc2x3.ifc", None),
    ] {
        let model = fixture(name);
        let resolved = fill(&model, "Concrete", StyleUnits::SI)
            .unwrap_or_else(|e| panic!("{name}: {e}"))
            .unwrap_or_else(|| panic!("{name}: no fill"));
        check_concrete(&resolved, flag);
        assert_eq!(
            resolved.style,
            named(&model, "IFCFILLAREASTYLE", 0, "Concrete cut")
        );
    }
}

#[test]
fn a_colour_alone_is_a_solid_fill() {
    for name in [
        "material_fill.ifc",
        "material_fill_ifc4x3.ifc",
        "material_fill_ifc2x3.ifc",
    ] {
        let model = fixture(name);
        let resolved = fill(&model, "Insulation", StyleUnits::SI).unwrap().unwrap();
        assert_eq!(resolved.background, Some([1.0; 3]), "{name}");
        assert!(resolved.lines.is_empty(), "{name}");
        assert_eq!(resolved.model_or_draughting, None, "{name}");
    }
}

#[test]
fn a_scaled_font_scales_its_dashes() {
    for name in [
        "material_fill.ifc",
        "material_fill_ifc4x3.ifc",
        "material_fill_ifc2x3.ifc",
    ] {
        let model = fixture(name);
        let resolved = fill(&model, "Scaled", StyleUnits::SI).unwrap().unwrap();
        assert_eq!(resolved.background, None, "{name}");
        assert_eq!(resolved.lines.len(), 1, "{name}");
        check_line(
            &resolved.lines[0],
            &Line {
                angle: std::f64::consts::FRAC_PI_2,
                spacing: 0.01,
                shift: 0.0,
                origin: [0.0, 0.0],
                start: None,
                dashes: Some(vec![[0.004, 0.002]]),
                colour: None,
                width: None,
            },
        );
    }
}

#[test]
fn millimetres_and_degrees_resolve_like_metres_and_radians() {
    let units = StyleUnits::new(Some(0.001), Some(std::f64::consts::PI / 180.0)).unwrap();
    let metric = fixture("material_fill.ifc");
    let scaled = fixture("material_fill_mm_deg.ifc");
    for name in ["Concrete", "Scaled"] {
        let want = fill(&metric, name, StyleUnits::SI).unwrap().unwrap();
        let got = fill(&scaled, name, units).unwrap().unwrap();
        assert_eq!(got.name, want.name);
        assert_eq!(got.lines.len(), want.lines.len(), "{name}");
        for (got, want) in got.lines.iter().zip(&want.lines) {
            check_line(
                got,
                &Line {
                    angle: want.angle_rad,
                    spacing: want.spacing,
                    shift: want.shift,
                    origin: want.origin,
                    start: want.pattern_start,
                    dashes: want.dashes.clone(),
                    colour: want.colour,
                    width: want.width,
                },
            );
        }
    }
}

#[test]
fn two_contexts_with_different_fills_are_ambiguous() {
    for name in [
        "material_fill.ifc",
        "material_fill_ifc4x3.ifc",
        "material_fill_ifc2x3.ifc",
    ] {
        let model = fixture(name);
        let want = vec![
            named(&model, "IFCFILLAREASTYLE", 0, "Layered model"),
            named(&model, "IFCFILLAREASTYLE", 0, "Layered plan"),
        ];
        assert_eq!(
            fill(&model, "Layered", StyleUnits::SI),
            Err(StyleError::AmbiguousFill {
                owner: material(&model, "Layered"),
                styles: want,
            }),
            "{name}"
        );
    }
}

#[test]
fn two_contexts_sharing_a_fill_resolve() {
    for name in [
        "material_fill.ifc",
        "material_fill_ifc4x3.ifc",
        "material_fill_ifc2x3.ifc",
    ] {
        let model = fixture(name);
        let resolved = fill(&model, "Shared", StyleUnits::SI).unwrap().unwrap();
        assert_eq!(
            resolved.style,
            named(&model, "IFCFILLAREASTYLE", 0, "Insulation cut"),
            "{name}"
        );
    }
}

#[test]
fn tiles_and_external_styles_are_unsupported() {
    for name in ["material_fill.ifc", "material_fill_ifc4x3.ifc"] {
        let model = fixture(name);
        for (material_name, style, construct) in [
            ("Tiled", "Tiled cut", "IfcFillAreaStyleTiles"),
            ("External", "External cut", "IfcExternallyDefinedHatchStyle"),
        ] {
            assert_eq!(
                fill(&model, material_name, StyleUnits::SI),
                Err(StyleError::UnsupportedFill {
                    style: named(&model, "IFCFILLAREASTYLE", 0, style),
                    entity: member(&model, style, 0),
                    construct: construct.to_owned(),
                }),
                "{name}"
            );
        }
    }
    let model = fixture("material_fill_ifc2x3.ifc");
    assert!(matches!(
        fill(&model, "External", StyleUnits::SI),
        Err(StyleError::UnsupportedFill { .. })
    ));
}

#[test]
fn by_layer_and_predefined_fonts_are_unsupported() {
    for name in [
        "material_fill.ifc",
        "material_fill_ifc4x3.ifc",
        "material_fill_ifc2x3.ifc",
    ] {
        let model = fixture(name);
        let by_layer = fill(&model, "ByLayer", StyleUnits::SI);
        assert!(
            matches!(&by_layer, Err(StyleError::UnsupportedFill { construct, .. })
                if construct == "IfcDraughtingPreDefinedColour 'by layer'"),
            "{name}: {by_layer:?}"
        );
        let dashed = fill(&model, "Dashed", StyleUnits::SI);
        assert!(
            matches!(&dashed, Err(StyleError::UnsupportedFill { construct, .. })
                if construct == "IfcDraughtingPreDefinedCurveFont 'dashed'"),
            "{name}: {dashed:?}"
        );
    }
}

#[test]
fn materials_without_a_fill_have_none() {
    for name in [
        "material_fill.ifc",
        "material_fill_ifc4x3.ifc",
        "material_fill_ifc2x3.ifc",
    ] {
        let model = fixture(name);
        for material_name in ["Plain", "SurfaceOnly"] {
            assert_eq!(
                fill(&model, material_name, StyleUnits::SI),
                Ok(None),
                "{name} {material_name}"
            );
        }
    }
}

#[test]
fn a_hatch_needs_the_units_it_reads() {
    let model = fixture("material_fill.ifc");
    let h1 = member(&model, "Concrete cut", 0);
    for (units, unit) in [
        (StyleUnits::new(Some(1.0), None).unwrap(), "PLANEANGLEUNIT"),
        (StyleUnits::new(None, Some(1.0)).unwrap(), "LENGTHUNIT"),
    ] {
        assert_eq!(
            fill(&model, "Concrete", units),
            Err(StyleError::UnitUndeclared { entity: h1, unit })
        );
    }
    let none = StyleUnits::new(None, None).unwrap();
    assert!(fill(&model, "Insulation", none).unwrap().is_some());
}

const RELEASES: [&str; 3] = [
    "material_fill.ifc",
    "material_fill_ifc4x3.ifc",
    "material_fill_ifc2x3.ifc",
];

#[test]
fn a_material_gives_one_fill_per_view() {
    for name in RELEASES {
        let model = fixture(name);
        let id = material(&model, "CutAndProjection");
        let index = StyleIndex::new(StyleView::new(&model, schema_of(&model))).unwrap();
        let fills = index.material_fills(id, StyleUnits::SI).unwrap();
        let views: Vec<_> = fills
            .iter()
            .map(|c| (c.target_view.as_deref(), c.fill.name.as_deref()))
            .collect();
        assert_eq!(
            views,
            [
                (Some("SECTION_VIEW"), Some("Section fill")),
                (Some("ELEVATION_VIEW"), Some("Elevation fill")),
            ],
            "{name}"
        );
        assert!(fills[0].representation < fills[1].representation);
        assert_ne!(fills[0].context, fills[1].context);
        assert_eq!(fills[0].identifier.as_deref(), Some("Body"), "{name}");
        assert_eq!(fills[0].context_type.as_deref(), Some("Model"), "{name}");
        assert_eq!(fills[0].fill.background, Some([0.7; 3]), "{name}");
        assert!(near(fills[0].fill.lines[0].spacing, 0.005), "{name}");
        assert!(near(fills[1].fill.lines[0].spacing, 0.02), "{name}");
        // The single-fill lookup refuses what differs per view.
        assert_eq!(
            index.material_fill(id, StyleUnits::SI),
            Err(StyleError::AmbiguousFill {
                owner: id,
                styles: vec![fills[0].fill.style, fills[1].fill.style],
            }),
            "{name}"
        );
    }
}

#[test]
fn materials_with_one_context_give_one_context_fill() {
    for name in RELEASES {
        let model = fixture(name);
        let index = StyleIndex::new(StyleView::new(&model, schema_of(&model))).unwrap();
        for (material_name, count) in [("Concrete", 1), ("Layered", 2), ("Plain", 0)] {
            let id = material(&model, material_name);
            let fills = index.material_fills(id, StyleUnits::SI).unwrap();
            assert_eq!(fills.len(), count, "{name} {material_name}");
            assert!(fills.iter().all(|c| c.target_view.is_none()));
        }
    }
}

#[test]
fn a_bare_style_resolves_and_a_non_fill_is_refused() {
    for name in RELEASES {
        let model = fixture(name);
        let index = StyleIndex::new(StyleView::new(&model, schema_of(&model))).unwrap();
        let style = named(&model, "IFCFILLAREASTYLE", 0, "Library pattern");
        let fill = index.fill_area_style(style, StyleUnits::SI).unwrap();
        assert_eq!(fill.style, style, "{name}");
        assert_eq!(fill.styled_item, None, "{name}");
        assert_eq!(fill.background, Some([0.8; 3]), "{name}");
        assert_eq!(fill.lines.len(), 1, "{name}");
        assert!(near(fill.lines[0].angle_rad, 0.5), "{name}");
        assert!(near(fill.lines[0].spacing, 0.003), "{name}");
        assert!(
            matches!(
                index.fill_area_style(material(&model, "Concrete"), StyleUnits::SI),
                Err(StyleError::WrongEntityType {
                    expected: "IfcFillAreaStyle",
                    ..
                })
            ),
            "{name}"
        );
    }
}
