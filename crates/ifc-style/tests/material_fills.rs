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
        Err(StyleError::WrongEntityType {
            expected: "IfcFillAreaStyle",
            ..
        })
    ));
    assert_eq!(
        index.fill_area_style(EntityId(999), StyleUnits::SI),
        Err(StyleError::UnknownEntity { id: EntityId(999) })
    );
}

/// Three representations on material #1, listed out of order, in the
/// contexts given; fills alternate grey (#20) and white (#21) as stated.
fn three(contexts: [u32; 3], fills: [u32; 3]) -> Model {
    model(&format!(
        "#5=IFCMATERIALDEFINITIONREPRESENTATION($,$,(#32,#30,#31),#1);\n\
         #30=IFCSTYLEDREPRESENTATION(#{},'Style','Material',(#40));\n\
         #31=IFCSTYLEDREPRESENTATION(#{},'Style','Material',(#41));\n\
         #32=IFCSTYLEDREPRESENTATION(#{},'Style','Material',(#42));\n\
         #40=IFCSTYLEDITEM($,(#{}),$);\n#41=IFCSTYLEDITEM($,(#{}),$);\n\
         #42=IFCSTYLEDITEM($,(#{}),$);\n",
        contexts[0], contexts[1], contexts[2], fills[0], fills[1], fills[2]
    ))
}

#[test]
fn representations_are_read_in_ascending_order_whatever_their_listing() {
    let model = two(11, 21, 10, 20);
    // #30 is in the elevation context and #31 in the section context.
    let fills = index(&model)
        .material_fills(EntityId(1), StyleUnits::SI)
        .unwrap();
    let reps: Vec<_> = fills.iter().map(|c| c.representation).collect();
    assert_eq!(reps, [EntityId(30), EntityId(31)]);
    let listed_backwards = model_with_backwards_listing();
    let fills = index(&listed_backwards)
        .material_fills(EntityId(1), StyleUnits::SI)
        .unwrap();
    let reps: Vec<_> = fills.iter().map(|c| c.representation).collect();
    assert_eq!(reps, [EntityId(30), EntityId(31)]);
}

fn model_with_backwards_listing() -> Model {
    model(
        "#5=IFCMATERIALDEFINITIONREPRESENTATION($,$,(#31,#30),#1);\n\
         #30=IFCSTYLEDREPRESENTATION(#10,'Style','Material',(#40));\n\
         #31=IFCSTYLEDREPRESENTATION(#11,'Style','Material',(#41));\n\
         #40=IFCSTYLEDITEM($,(#20),$);\n#41=IFCSTYLEDITEM($,(#21),$);\n",
    )
}

#[test]
fn interleaved_contexts_group_and_report_the_lowest_representation() {
    // #30 and #32 in the section context with one fill, #31 elsewhere.
    let model = three([10, 11, 10], [20, 21, 20]);
    let fills = index(&model)
        .material_fills(EntityId(1), StyleUnits::SI)
        .unwrap();
    let got: Vec<_> = fills
        .iter()
        .map(|c| (c.representation, c.context))
        .collect();
    assert_eq!(
        got,
        [(EntityId(30), EntityId(10)), (EntityId(31), EntityId(11))]
    );
    // The third differs from the first in the same context.
    let model = three([10, 11, 10], [20, 21, 21]);
    assert_eq!(
        index(&model).material_fills(EntityId(1), StyleUnits::SI),
        Err(StyleError::AmbiguousFill {
            owner: EntityId(10),
            styles: vec![EntityId(20), EntityId(21)],
        })
    );
}

#[test]
fn a_sub_context_states_its_target_view_and_parent() {
    let model = two(10, 20, 11, 21);
    let fills = index(&model)
        .material_fills(EntityId(1), StyleUnits::SI)
        .unwrap();
    assert_eq!(fills[0].parent_context, Some(EntityId(4)));
    assert_eq!(fills[0].user_defined_target_view, None);

    let user = model_with_sub_context(".USERDEFINED.", "'Detail'");
    let fills = index(&user)
        .material_fills(EntityId(1), StyleUnits::SI)
        .unwrap();
    assert_eq!(fills[0].target_view.as_deref(), Some("USERDEFINED"));
    assert_eq!(fills[0].user_defined_target_view.as_deref(), Some("Detail"));

    let plain = two(4, 20, 4, 20);
    let fills = index(&plain)
        .material_fills(EntityId(1), StyleUnits::SI)
        .unwrap();
    assert_eq!(fills[0].parent_context, None);
    assert_eq!(fills[0].user_defined_target_view, None);
}

fn model_with_sub_context(view: &str, user: &str) -> Model {
    let source = format!(
        "ISO-10303-21;\nHEADER;FILE_DESCRIPTION((''),'2;1');\
         FILE_NAME('','',(''),(''),'','','');FILE_SCHEMA(('IFC4'));ENDSEC;\nDATA;\n\
         #1=IFCMATERIAL('M',$,$);\n#2=IFCCARTESIANPOINT((0.,0.));\n\
         #3=IFCAXIS2PLACEMENT2D(#2,$);\n\
         #4=IFCGEOMETRICREPRESENTATIONCONTEXT($,'Plan',2,1.E-05,#3,$);\n\
         #10=IFCGEOMETRICREPRESENTATIONSUBCONTEXT('Body','Model',*,*,*,*,#4,$,{view},{user});\n\
         #20=IFCFILLAREASTYLE('Grey',(#22),.T.);\n#22=IFCCOLOURRGB($,0.5,0.5,0.5);\n\
         #5=IFCMATERIALDEFINITIONREPRESENTATION($,$,(#30),#1);\n\
         #30=IFCSTYLEDREPRESENTATION(#10,'Style','Material',(#40));\n\
         #40=IFCSTYLEDITEM($,(#20),$);\nENDSEC;\nEND-ISO-10303-21;\n"
    );
    StepCodec.read_bytes(source.as_bytes()).expect("parses")
}

#[test]
fn a_target_view_is_mandatory_and_from_the_projection_enumeration() {
    let missing = model_with_sub_context("$", "$");
    assert!(matches!(
        index(&missing).material_fills(EntityId(1), StyleUnits::SI),
        Err(StyleError::MissingAttribute {
            attribute: "TargetView",
            ..
        })
    ));
    let unknown = model_with_sub_context(".TOP_VIEW.", "$");
    assert!(matches!(
        index(&unknown).material_fills(EntityId(1), StyleUnits::SI),
        Err(StyleError::InvalidValue {
            attribute: "TargetView",
            ..
        })
    ));
}
