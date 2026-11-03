//! A material's cut fill in metres and radians, scaled by the project units
//! (#597): the `style` x `properties` join.

#![cfg(all(feature = "style", feature = "properties", feature = "step"))]

use ifc::style::{HatchLine, MaterialFill, StyleError};
use ifc::{
    fill_area_style, material_fill, material_fill_for_view, material_fills,
    material_fills_by_context, project_style_units, Codec, EntityId, FillResolver,
    MaterialFillError, Model, StepCodec,
};

fn fixture(name: &str) -> Model {
    let path = format!("../../test/fixtures/synthetic-fill/{name}");
    StepCodec
        .read_bytes(&std::fs::read(path).expect("fixture"))
        .expect("parses")
}

fn material(model: &Model, name: &str) -> EntityId {
    model
        .of_type("IFCMATERIAL")
        .find(|(_, entity)| entity.text(0) == Some(name))
        .map(|(id, _)| id)
        .unwrap_or_else(|| panic!("no material {name}"))
}

fn flat(line: &HatchLine) -> Vec<f64> {
    let mut out = vec![
        line.angle_rad,
        line.spacing,
        line.shift,
        line.origin[0],
        line.origin[1],
    ];
    out.extend(line.pattern_start.unwrap_or_default());
    out.extend(line.dashes.iter().flatten().flatten());
    out.extend(line.colour.unwrap_or_default());
    out.extend(line.width);
    out
}

fn assert_same(a: &MaterialFill, b: &MaterialFill) {
    assert_eq!(a.name, b.name);
    assert_eq!(a.lines.len(), b.lines.len());
    for (a, b) in a.lines.iter().zip(&b.lines) {
        let (a, b) = (flat(a), flat(b));
        assert_eq!(a.len(), b.len());
        for (a, b) in a.iter().zip(&b) {
            assert!((a - b).abs() < 1e-12, "{a} vs {b}");
        }
    }
}

#[test]
fn millimetres_and_degrees_resolve_like_metres_and_radians() {
    let metric = fixture("material_fill.ifc");
    let scaled = fixture("material_fill_mm_deg.ifc");
    for name in ["Concrete", "Scaled"] {
        let want = material_fill(&metric, material(&metric, name))
            .unwrap()
            .unwrap();
        let got = material_fill(&scaled, material(&scaled, name))
            .unwrap()
            .unwrap();
        assert_eq!(want.lines.len(), if name == "Concrete" { 2 } else { 1 });
        assert_same(&got, &want);
    }
}

#[test]
fn the_project_units_are_the_exact_factors() {
    let units = project_style_units(&fixture("material_fill_mm_deg.ifc")).unwrap();
    let length = units.metres_per_length().expect("length unit");
    let angle = units.radians_per_angle().expect("angle unit");
    assert!((length - 0.001).abs() < 1e-15, "{length}");
    assert!(
        (angle - std::f64::consts::PI / 180.0).abs() < 1e-15,
        "{angle}"
    );
}

/// A material (#1) styled by `fill` (#8, with `rest` after it), with no `IfcProject`.
fn without_project(schema: &str, fill: &str, rest: &str) -> Model {
    let source = format!(
        "ISO-10303-21;\nHEADER;FILE_DESCRIPTION((''),'2;1');\
         FILE_NAME('','',(''),(''),'','','');FILE_SCHEMA(('{schema}'));ENDSEC;\nDATA;\n\
         #1=IFCMATERIAL('M',$,$);\n#2=IFCCARTESIANPOINT((0.,0.));\n\
         #3=IFCAXIS2PLACEMENT2D(#2,$);\n\
         #4=IFCGEOMETRICREPRESENTATIONCONTEXT($,'Plan',2,1.E-05,#3,$);\n\
         #5=IFCMATERIALDEFINITIONREPRESENTATION($,$,(#6),#1);\n\
         #6=IFCSTYLEDREPRESENTATION(#4,'Style','Material',(#7));\n\
         #7=IFCSTYLEDITEM($,(#8),$);\n#8=IFCFILLAREASTYLE('F',({fill}),.T.);\n{rest}\
         ENDSEC;\nEND-ISO-10303-21;\n"
    );
    StepCodec.read_bytes(source.as_bytes()).expect("parses")
}

#[test]
fn a_project_without_units_resolves_only_what_needs_none() {
    let solid = without_project("IFC4", "#9", "#9=IFCCOLOURRGB($,0.1,0.2,0.3);\n");
    let fill = material_fill(&solid, EntityId(1)).unwrap().unwrap();
    assert_eq!(fill.background, Some([0.1, 0.2, 0.3]));
    assert!(fill.lines.is_empty());

    let hatched = without_project(
        "IFC4",
        "#10",
        "#9=IFCCURVESTYLE($,$,$,$,$);\n#10=IFCFILLAREASTYLEHATCHING(#9,0.005,$,$,0.);\n",
    );
    assert_eq!(
        material_fill(&hatched, EntityId(1)),
        Err(MaterialFillError::Style(StyleError::UnitUndeclared {
            entity: EntityId(10),
            unit: "PLANEANGLEUNIT",
        }))
    );
}

#[test]
fn a_release_the_unit_resolver_does_not_read_is_refused() {
    let model = without_project("IFC4X1", "#9", "#9=IFCCOLOURRGB($,0.1,0.2,0.3);\n");
    assert!(matches!(
        material_fill(&model, EntityId(1)),
        Err(MaterialFillError::ProjectUnit {
            measure: "IFCLENGTHMEASURE",
            ..
        })
    ));
    let unknown = without_project("IFC9", "#9", "#9=IFCCOLOURRGB($,0.1,0.2,0.3);\n");
    assert!(matches!(
        material_fill(&unknown, EntityId(1)),
        Err(MaterialFillError::UnsupportedSchema { .. })
    ));
}

#[test]
fn per_view_fills_and_bare_styles_resolve() {
    let metric = fixture("material_fill.ifc");
    let id = material(&metric, "CutAndProjection");
    let fills = material_fills(&metric, id).unwrap();
    let views: Vec<_> = fills.iter().map(|c| c.target_view.as_deref()).collect();
    assert_eq!(views, [Some("SECTION_VIEW"), Some("ELEVATION_VIEW")]);
    // The single fill stays ambiguous; the per-view lookup is the answer.
    assert!(matches!(
        material_fill(&metric, id),
        Err(MaterialFillError::Style(StyleError::AmbiguousFill { .. }))
    ));

    let style = model_style(&metric, "Library pattern");
    let pattern = fill_area_style(&metric, style).unwrap();
    assert_eq!(pattern.styled_item, None);
    assert_eq!(pattern.background, Some([0.8; 3]));
    assert!(matches!(
        fill_area_style(&metric, id),
        Err(MaterialFillError::Style(StyleError::WrongEntityType { .. }))
    ));
}

fn model_style(model: &Model, name: &str) -> EntityId {
    model
        .of_type("IFCFILLAREASTYLE")
        .find(|(_, entity)| entity.text(0) == Some(name))
        .map(|(id, _)| id)
        .unwrap()
}

#[test]
fn a_refused_view_leaves_the_section_fill_readable() {
    let metric = fixture("material_fill.ifc");
    let id = material(&metric, "CutAndTiledProjection");
    let entries = material_fills_by_context(&metric, id).unwrap();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].target_view.as_deref(), Some("SECTION_VIEW"));
    assert!(entries[0].fill.is_ok());
    assert_eq!(entries[1].target_view.as_deref(), Some("ELEVATION_VIEW"));
    assert!(matches!(
        entries[1].fill,
        Err(StyleError::UnsupportedFill { .. })
    ));
    let section = material_fill_for_view(&metric, id, "SECTION_VIEW")
        .unwrap()
        .unwrap();
    assert_eq!(section.name.as_deref(), Some("Section fill"));
    assert!(matches!(
        material_fills(&metric, id),
        Err(MaterialFillError::Style(StyleError::UnsupportedFill { .. }))
    ));
    assert!(matches!(
        material_fill_for_view(&metric, id, "TOP_VIEW"),
        Err(MaterialFillError::Style(
            StyleError::UnknownTargetView { .. }
        ))
    ));
}

#[test]
fn one_resolver_serves_many_materials() {
    for name in ["material_fill.ifc", "material_fill_mm_deg.ifc"] {
        let model = fixture(name);
        let resolver = FillResolver::new(&model).unwrap();
        assert_eq!(resolver.units(), project_style_units(&model).unwrap());
        for material_name in [
            "Concrete",
            "Insulation",
            "Scaled",
            "Layered",
            "CutAndProjection",
            "CutAndTiledProjection",
            "Tiled",
            "Plain",
        ] {
            let Some((id, _)) = model
                .of_type("IFCMATERIAL")
                .find(|(_, entity)| entity.text(0) == Some(material_name))
            else {
                continue;
            };
            assert_eq!(
                resolver.material_fill(id),
                material_fill(&model, id),
                "{name} {material_name}"
            );
            assert_eq!(
                resolver.material_fills(id),
                material_fills(&model, id),
                "{name} {material_name}"
            );
            assert_eq!(
                resolver.material_fills_by_context(id),
                material_fills_by_context(&model, id),
                "{name} {material_name}"
            );
            assert_eq!(
                resolver.material_fill_for_view(id, "SECTION_VIEW"),
                material_fill_for_view(&model, id, "SECTION_VIEW"),
                "{name} {material_name}"
            );
        }
        if name == "material_fill.ifc" {
            let style = model_style(&model, "Library pattern");
            assert_eq!(
                resolver.fill_area_style(style),
                fill_area_style(&model, style)
            );
            assert!(resolver
                .styles()
                .material_fill(material(&model, "Plain"), resolver.units())
                .unwrap()
                .is_none());
        }
    }
    let unknown = without_project("IFC9", "#9", "#9=IFCCOLOURRGB($,0.1,0.2,0.3);\n");
    assert!(matches!(
        FillResolver::new(&unknown),
        Err(MaterialFillError::UnsupportedSchema { .. })
    ));
    let unread = without_project("IFC4X1", "#9", "#9=IFCCOLOURRGB($,0.1,0.2,0.3);\n");
    assert!(matches!(
        FillResolver::new(&unread),
        Err(MaterialFillError::ProjectUnit {
            measure: "IFCLENGTHMEASURE",
            ..
        })
    ));
}
