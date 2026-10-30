//! `StyleUnits` and the typed refusals of `StyleIndex::material_fill` (#597).

use ifc_model::{Codec, EntityId, Model};
use ifc_schema::{for_version, SchemaVersion};
use ifc_step::StepCodec;
use ifc_style::{MaterialFill, StyleError, StyleIndex, StyleResult, StyleUnits, StyleView};

#[test]
fn style_units_refuse_non_positive_factors() {
    for bad in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        let length = StyleUnits::new(Some(bad), Some(1.0));
        assert!(
            matches!(
                length,
                Err(StyleError::InvalidUnits {
                    factor: "metres_per_length",
                    ..
                })
            ),
            "{bad}: {length:?}"
        );
        let angle = StyleUnits::new(Some(1.0), Some(bad));
        assert!(
            matches!(
                angle,
                Err(StyleError::InvalidUnits {
                    factor: "radians_per_angle",
                    ..
                })
            ),
            "{bad}: {angle:?}"
        );
    }
    let none = StyleUnits::new(None, None).unwrap();
    assert_eq!(none.metres_per_length(), None);
    assert_eq!(none.radians_per_angle(), None);
    assert_eq!(
        StyleUnits::new(Some(1.0), Some(1.0)).unwrap(),
        StyleUnits::SI
    );
}

const MATERIAL: EntityId = EntityId(1);
const STYLED_ITEM: EntityId = EntityId(7);

/// A material (#1) with one styled representation in a 2D context; the
/// caller states the styled item (#7) and what it names.
const PREAMBLE: &str = "\
#1=IFCMATERIAL('M',$,$);
#2=IFCCARTESIANPOINT((0.,0.));
#3=IFCAXIS2PLACEMENT2D(#2,$);
#4=IFCGEOMETRICREPRESENTATIONCONTEXT($,'Plan',2,1.E-05,#3,$);
#5=IFCMATERIALDEFINITIONREPRESENTATION($,$,(#6),#1);
#6=IFCSTYLEDREPRESENTATION(#4,'Style','Material',(#7));
";

fn model(schema: &str, data: &str) -> Model {
    let source = format!(
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
         FILE_NAME('t.ifc','1970-01-01T00:00:00',(''),(''),'','','');\n\
         FILE_SCHEMA(('{schema}'));\nENDSEC;\nDATA;\n{PREAMBLE}{data}ENDSEC;\nEND-ISO-10303-21;\n"
    );
    StepCodec.read_bytes(source.as_bytes()).expect("parses")
}

fn resolve(model: &Model) -> StyleResult<Option<MaterialFill>> {
    let version = SchemaVersion::from_header_token(&model.header().schema[0]).unwrap();
    let schema = for_version(version).unwrap();
    StyleIndex::new(StyleView::new(model, schema))
        .unwrap()
        .material_fill(MATERIAL, StyleUnits::SI)
}

/// IFC4: the fill style #8 holds `members`; `rest` defines #9 onwards. A
/// curve style without font, width or colour is #9.
fn ifc4(members: &str, rest: &str) -> Model {
    model(
        "IFC4",
        &format!(
            "#7=IFCSTYLEDITEM($,(#8),$);\n#8=IFCFILLAREASTYLE('F',({members}),.T.);\n\
             #9=IFCCURVESTYLE($,$,$,$,$);\n{rest}"
        ),
    )
}

/// IFC4: one hatching (#12) at 0 radians with `distance` and `curve` as its
/// appearance, over the direction (#10) and vector (#11) `vector` states.
fn hatch(distance: &str, curve: &str, rest: &str) -> Model {
    ifc4(
        "#12",
        &format!(
            "#10=IFCDIRECTION((0.,1.));\n#11=IFCVECTOR(#10,0.004);\n\
             #12=IFCFILLAREASTYLEHATCHING({curve},{distance},$,$,0.);\n{rest}"
        ),
    )
}

fn only_line(model: &Model) -> (f64, f64) {
    let fill = resolve(model).unwrap().unwrap();
    assert_eq!(fill.lines.len(), 1);
    (fill.lines[0].spacing, fill.lines[0].shift)
}

#[test]
fn a_downward_vector_is_the_same_family_of_lines() {
    let model = hatch("#11", "#9", "");
    // (0, 1) * 0.004 is the upward vector; flip it.
    let (spacing, shift) = only_line(&model);
    assert!((spacing - 0.004).abs() < 1e-15 && shift.abs() < 1e-15);
    let down = ifc4(
        "#12",
        "#10=IFCDIRECTION((0.,-1.));\n#11=IFCVECTOR(#10,0.004);\n\
         #12=IFCFILLAREASTYLEHATCHING(#9,#11,$,$,0.);\n",
    );
    let (spacing, shift) = only_line(&down);
    assert!((spacing - 0.004).abs() < 1e-15 && shift.abs() < 1e-15);
}

#[test]
fn a_vector_along_the_hatch_lines_is_refused() {
    let along = ifc4(
        "#12",
        "#10=IFCDIRECTION((1.,0.));\n#11=IFCVECTOR(#10,0.004);\n\
         #12=IFCFILLAREASTYLEHATCHING(#9,#11,$,$,0.);\n",
    );
    assert!(matches!(
        resolve(&along),
        Err(StyleError::InvalidValue { attribute: "StartOfNextHatchLine", id, .. }) if id == EntityId(12)
    ));
}

#[test]
fn a_3d_vector_is_unsupported() {
    let model = ifc4(
        "#12",
        "#10=IFCDIRECTION((0.,0.,1.));\n#11=IFCVECTOR(#10,0.004);\n\
         #12=IFCFILLAREASTYLEHATCHING(#9,#11,$,$,0.);\n",
    );
    assert!(matches!(
        resolve(&model),
        Err(StyleError::UnsupportedFill { entity, .. }) if entity == EntityId(11)
    ));
}

#[test]
fn ifc2x3_refuses_a_two_direction_repeat_factor_and_a_bare_vector() {
    let ifc2x3 = |distance: &str| {
        model(
            "IFC2X3",
            &format!(
                "#7=IFCSTYLEDITEM($,(#13),$);\n#13=IFCPRESENTATIONSTYLEASSIGNMENT((#8));\n\
                 #8=IFCFILLAREASTYLE('F',(#12));\n#9=IFCCURVESTYLE($,$,$,$);\n\
                 #10=IFCDIRECTION((0.,1.));\n#11=IFCVECTOR(#10,0.004);\n\
                 #14=IFCTWODIRECTIONREPEATFACTOR(#11,#11);\n\
                 #12=IFCFILLAREASTYLEHATCHING(#9,{distance},$,$,0.);\n"
            ),
        )
    };
    assert!(matches!(
        resolve(&ifc2x3("#14")),
        Err(StyleError::InvalidValue { attribute: "StartOfNextHatchLine", value, .. })
            if value.starts_with("WR21")
    ));
    assert!(matches!(
        resolve(&ifc2x3("#11")),
        Err(StyleError::ReferenceType {
            expected: "IfcOneDirectionRepeatFactor",
            ..
        })
    ));
}

#[test]
fn fill_styles_that_break_a_rule_are_invalid() {
    let colours = "#10=IFCCOLOURRGB($,0.1,0.1,0.1);\n#11=IFCCOLOURRGB($,0.2,0.2,0.2);\n";
    let two = ifc4("#10,#11", colours);
    assert!(matches!(
        resolve(&two),
        Err(StyleError::InvalidValue { attribute: "FillStyles", value, .. })
            if value.starts_with("MaxOneColour")
    ));
    let tiled = ifc4(
        "#12,#13",
        "#10=IFCDIRECTION((0.,1.));\n#11=IFCVECTOR(#10,0.004);\n\
         #12=IFCFILLAREASTYLEHATCHING(#9,#11,$,$,0.);\n\
         #14=IFCSTYLEDITEM($,(#9),$);\n#13=IFCFILLAREASTYLETILES((#11,#11),(#14),1.);\n",
    );
    assert!(matches!(
        resolve(&tiled),
        Err(StyleError::InvalidValue { attribute: "FillStyles", value, .. })
            if value.starts_with("ConsistentHatchStyleDef: hatching with tiles")
    ));
}

#[test]
fn a_distance_is_a_positive_length() {
    let bare = hatch("0.005", "#9", "");
    assert_eq!(only_line(&bare), (0.005, 0.0));
    let wrong = hatch("IFCLENGTHMEASURE(0.005)", "#9", "");
    assert!(matches!(
        resolve(&wrong),
        Err(StyleError::InvalidValue {
            attribute: "StartOfNextHatchLine",
            ..
        })
    ));
    let zero = hatch("IFCPOSITIVELENGTHMEASURE(0.)", "#9", "");
    assert!(matches!(
        resolve(&zero),
        Err(StyleError::OutOfRange {
            entity: "IfcPositiveLengthMeasure",
            ..
        })
    ));
}

#[test]
fn a_curve_width_is_a_positive_length() {
    let width = |width: &str| {
        hatch(
            "0.005",
            "#13",
            &format!("#13=IFCCURVESTYLE($,$,{width},$,$);\n"),
        )
    };
    assert!(matches!(
        resolve(&width("IFCRATIOMEASURE(0.5)")),
        Err(StyleError::InvalidValue {
            attribute: "CurveWidth",
            ..
        })
    ));
    assert!(matches!(
        resolve(&width("IFCDESCRIPTIVEMEASURE('by layer')")),
        Err(StyleError::UnsupportedFill { construct, .. })
            if construct == "IfcDescriptiveMeasure 'by layer'"
    ));
}

#[test]
fn a_predefined_colour_name_is_case_sensitive() {
    let model = hatch(
        "0.005",
        "#13",
        "#13=IFCCURVESTYLE($,$,$,#14,$);\n#14=IFCDRAUGHTINGPREDEFINEDCOLOUR('Black');\n",
    );
    assert!(matches!(
        resolve(&model),
        Err(StyleError::InvalidValue { attribute: "Name", value, .. })
            if value.starts_with("PreDefinedColourNames")
    ));
}

#[test]
fn a_pattern_start_is_a_2d_point() {
    let model = ifc4(
        "#12",
        "#10=IFCCARTESIANPOINT((0.,0.,0.));\n\
         #12=IFCFILLAREASTYLEHATCHING(#9,0.005,$,#10,0.);\n",
    );
    assert!(matches!(
        resolve(&model),
        Err(StyleError::InvalidValue {
            attribute: "PatternStart",
            ..
        })
    ));
}

#[test]
fn one_styled_item_with_two_fills_is_ambiguous() {
    let model = model(
        "IFC4",
        "#7=IFCSTYLEDITEM($,(#8,#10),$);\n\
         #8=IFCFILLAREASTYLE('A',(#9),.T.);\n#9=IFCCOLOURRGB($,0.1,0.1,0.1);\n\
         #10=IFCFILLAREASTYLE('B',(#11),.T.);\n#11=IFCCOLOURRGB($,0.2,0.2,0.2);\n",
    );
    assert_eq!(
        resolve(&model),
        Err(StyleError::AmbiguousFill {
            owner: STYLED_ITEM,
            styles: vec![EntityId(8), EntityId(10)],
        })
    );
}

#[test]
fn a_hatch_line_without_a_font_has_no_dashes() {
    let model = hatch("0.005", "#9", "");
    let fill = resolve(&model).unwrap().unwrap();
    assert_eq!(fill.lines[0].dashes, None);
    assert_eq!(fill.lines[0].colour, None);
    assert_eq!(fill.lines[0].width, None);
}

fn vector_at_zero(direction: &str, magnitude: &str) -> Model {
    ifc4(
        "#12",
        &format!(
            "#10=IFCDIRECTION(({direction}));\n#11=IFCVECTOR(#10,{magnitude});\n\
             #12=IFCFILLAREASTYLEHATCHING(#9,#11,$,$,0.);\n"
        ),
    )
}

#[test]
fn a_vector_against_the_normal_flips_both_spacing_and_shift() {
    let (spacing, shift) = only_line(&vector_at_zero("0.6,-0.8", "0.005"));
    assert!((spacing - 0.004).abs() < 1e-15, "{spacing}");
    assert!((shift + 0.003).abs() < 1e-15, "{shift}");
}

#[test]
fn a_zero_vector_is_refused_as_such() {
    let zero = vector_at_zero("0.,1.", "0.");
    assert!(matches!(
        resolve(&zero),
        Err(StyleError::InvalidValue { attribute: "StartOfNextHatchLine", value, .. })
            if value == "zero vector"
    ));
}

#[test]
fn negative_zero_does_not_split_equal_fills() {
    // `Q` states a length (shift +0.0), `P` a vector against the normal
    // (shift -0.0); the two fills hold the same lines in swapped origins.
    let hatch = |id: u32, distance: &str, origin: u32| {
        format!("#{id}=IFCFILLAREASTYLEHATCHING(#9,{distance},#{origin},$,0.);\n")
    };
    let mut data = String::from(
        "#20=IFCCARTESIANPOINT((1.,0.));\n#21=IFCCARTESIANPOINT((0.,0.));\n\
         #22=IFCDIRECTION((0.,-1.));\n#23=IFCVECTOR(#22,0.004);\n",
    );
    data += &hatch(30, "#23", 20);
    data += &hatch(31, "0.004", 21);
    data += &hatch(32, "0.004", 20);
    data += &hatch(33, "#23", 21);
    data += "#8=IFCFILLAREASTYLE('A',(#30,#31),.T.);\n#14=IFCFILLAREASTYLE('B',(#32,#33),.T.);\n";
    let model = model(
        "IFC4",
        &format!("#7=IFCSTYLEDITEM($,(#8,#14),$);\n#9=IFCCURVESTYLE($,$,$,$,$);\n{data}"),
    );
    let fill = resolve(&model).unwrap().expect("agreeing fills resolve");
    assert_eq!(fill.lines.len(), 2);
}
