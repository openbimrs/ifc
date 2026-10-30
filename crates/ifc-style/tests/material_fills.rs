//! `StyleIndex::material_fills` and `StyleIndex::fill_area_style` over
//! in-memory models (#597): one fill per representation context, and bare
//! fill area styles.

use ifc_model::{Codec, EntityId, Model};
use ifc_schema::ifc4;
use ifc_step::StepCodec;
use ifc_style::{StyleError, StyleIndex, StyleUnits, StyleView};

/// Material #1 with two sub-contexts of the plan context #4 (#10 section,
/// #11 elevation) and two fills (#20 grey, #21 white); `representations`
/// states the styled representations (#30 upward) and their styled items.
fn model(representations: &str) -> Model {
    let source = format!(
        "ISO-10303-21;\nHEADER;FILE_DESCRIPTION((''),'2;1');\
         FILE_NAME('','',(''),(''),'','','');FILE_SCHEMA(('IFC4'));ENDSEC;\nDATA;\n\
         #1=IFCMATERIAL('M',$,$);\n#2=IFCCARTESIANPOINT((0.,0.));\n\
         #3=IFCAXIS2PLACEMENT2D(#2,$);\n\
         #4=IFCGEOMETRICREPRESENTATIONCONTEXT($,'Plan',2,1.E-05,#3,$);\n\
         #10=IFCGEOMETRICREPRESENTATIONSUBCONTEXT('Body','Model',*,*,*,*,#4,$,.SECTION_VIEW.,$);\n\
         #11=IFCGEOMETRICREPRESENTATIONSUBCONTEXT('Body','Model',*,*,*,*,#4,$,.ELEVATION_VIEW.,$);\n\
         #20=IFCFILLAREASTYLE('Grey',(#22),.T.);\n#22=IFCCOLOURRGB($,0.5,0.5,0.5);\n\
         #21=IFCFILLAREASTYLE('White',(#23),.T.);\n#23=IFCCOLOURRGB($,1.,1.,1.);\n\
         {representations}ENDSEC;\nEND-ISO-10303-21;\n"
    );
    StepCodec.read_bytes(source.as_bytes()).expect("parses")
}

/// Representations `#30` (context `ctx30`, fill `fill30`) and `#31`
/// likewise, both on material #1.
fn two(ctx30: u32, fill30: u32, ctx31: u32, fill31: u32) -> Model {
    model(&format!(
        "#5=IFCMATERIALDEFINITIONREPRESENTATION($,$,(#30,#31),#1);\n\
         #30=IFCSTYLEDREPRESENTATION(#{ctx30},'Style','Material',(#40));\n\
         #31=IFCSTYLEDREPRESENTATION(#{ctx31},'Style','Material',(#41));\n\
         #40=IFCSTYLEDITEM($,(#{fill30}),$);\n#41=IFCSTYLEDITEM($,(#{fill31}),$);\n"
    ))
}

fn index(model: &Model) -> StyleIndex<'_, 'static> {
    StyleIndex::new(StyleView::new(model, ifc4())).unwrap()
}

#[test]
fn each_context_gives_its_own_fill() {
    let model = two(10, 20, 11, 21);
    let index = index(&model);
    let fills = index.material_fills(EntityId(1), StyleUnits::SI).unwrap();
    assert_eq!(fills.len(), 2);
    assert_eq!(fills[0].representation, EntityId(30));
    assert_eq!(fills[0].context, EntityId(10));
    assert_eq!(fills[0].identifier.as_deref(), Some("Body"));
    assert_eq!(fills[0].context_type.as_deref(), Some("Model"));
    assert_eq!(fills[0].target_view.as_deref(), Some("SECTION_VIEW"));
    assert_eq!(fills[0].fill.name.as_deref(), Some("Grey"));
    assert_eq!(fills[0].fill.styled_item, Some(EntityId(40)));
    assert_eq!(fills[1].representation, EntityId(31));
    assert_eq!(fills[1].target_view.as_deref(), Some("ELEVATION_VIEW"));
    assert_eq!(fills[1].fill.name.as_deref(), Some("White"));
    // The single-fill lookup still refuses what differs per view.
    assert_eq!(
        index.material_fill(EntityId(1), StyleUnits::SI),
        Err(StyleError::AmbiguousFill {
            owner: EntityId(1),
            styles: vec![EntityId(20), EntityId(21)],
        })
    );
}

#[test]
fn a_plain_context_has_no_target_view() {
    let model = two(4, 20, 4, 20);
    let fills = index(&model)
        .material_fills(EntityId(1), StyleUnits::SI)
        .unwrap();
    assert_eq!(fills.len(), 1, "one context, one entry");
    assert_eq!(fills[0].representation, EntityId(30), "the lowest");
    assert_eq!(fills[0].target_view, None);
    assert_eq!(fills[0].identifier, None);
    assert_eq!(fills[0].context_type.as_deref(), Some("Plan"));
}

#[test]
fn two_fills_in_one_context_are_ambiguous() {
    let model = two(10, 20, 10, 21);
    assert_eq!(
        index(&model).material_fills(EntityId(1), StyleUnits::SI),
        Err(StyleError::AmbiguousFill {
            owner: EntityId(10),
            styles: vec![EntityId(20), EntityId(21)],
        })
    );
}

#[test]
fn two_fills_in_one_representation_are_ambiguous() {
    let model = model(
        "#5=IFCMATERIALDEFINITIONREPRESENTATION($,$,(#30),#1);\n\
         #30=IFCSTYLEDREPRESENTATION(#10,'Style','Material',(#40,#41));\n\
         #40=IFCSTYLEDITEM($,(#20),$);\n#41=IFCSTYLEDITEM($,(#21),$);\n",
    );
    assert_eq!(
        index(&model).material_fills(EntityId(1), StyleUnits::SI),
        Err(StyleError::AmbiguousFill {
            owner: EntityId(30),
            styles: vec![EntityId(20), EntityId(21)],
        })
    );
}

#[test]
fn a_material_without_a_fill_has_no_context_fills() {
    let model = model("");
    assert_eq!(
        index(&model).material_fills(EntityId(1), StyleUnits::SI),
        Ok(vec![])
    );
}

#[test]
fn a_bare_style_resolves_without_a_styled_item() {
    let model = model("");
    let index = index(&model);
    let fill = index.fill_area_style(EntityId(20), StyleUnits::SI).unwrap();
    assert_eq!(fill.style, EntityId(20));
    assert_eq!(fill.styled_item, None);
    assert_eq!(fill.background, Some([0.5; 3]));
    // Anything else is no fill area style.
    assert!(matches!(
        index.fill_area_style(EntityId(1), StyleUnits::SI),
        Err(StyleError::ReferenceType {
            expected: "IfcFillAreaStyle",
            ..
        })
    ));
    assert_eq!(
        index.fill_area_style(EntityId(999), StyleUnits::SI),
        Err(StyleError::UnknownEntity { id: EntityId(999) })
    );
}
