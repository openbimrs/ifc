//! `StyleIndex::material_fills_by_context` and `material_fill_for_view` over
//! in-memory models (#624): a refusal stays in the context that owns it, and
//! a view is looked up by its `TargetView`.

use ifc_model::{Codec, EntityId, Model};
use ifc_schema::ifc4;
use ifc_step::StepCodec;
use ifc_style::{StyleError, StyleIndex, StyleUnits, StyleView};

/// Material #1 with four sub-contexts of the plan context #4 (#10 section,
/// #11 elevation, #12 top, #13 section again), grey #20 and white #21 fills,
/// a tiled fill #24 and a hatched fill #50; `representations` states the
/// styled representations (#30 upward) and their styled items.
fn model(representations: &str) -> Model {
    let source = format!(
        "ISO-10303-21;\nHEADER;FILE_DESCRIPTION((''),'2;1');\
         FILE_NAME('','',(''),(''),'','','');FILE_SCHEMA(('IFC4'));ENDSEC;\nDATA;\n\
         #1=IFCMATERIAL('M',$,$);\n#2=IFCCARTESIANPOINT((0.,0.));\n\
         #3=IFCAXIS2PLACEMENT2D(#2,$);\n\
         #4=IFCGEOMETRICREPRESENTATIONCONTEXT($,'Plan',2,1.E-05,#3,$);\n\
         #10=IFCGEOMETRICREPRESENTATIONSUBCONTEXT('Body','Model',*,*,*,*,#4,$,.SECTION_VIEW.,$);\n\
         #11=IFCGEOMETRICREPRESENTATIONSUBCONTEXT('Body','Model',*,*,*,*,#4,$,.ELEVATION_VIEW.,$);\n\
         #12=IFCGEOMETRICREPRESENTATIONSUBCONTEXT('Body','Model',*,*,*,*,#4,$,.TOP_VIEW.,$);\n\
         #13=IFCGEOMETRICREPRESENTATIONSUBCONTEXT('Body','Model',*,*,*,*,#4,$,.SECTION_VIEW.,$);\n\
         #20=IFCFILLAREASTYLE('Grey',(#22),.T.);\n#22=IFCCOLOURRGB($,0.5,0.5,0.5);\n\
         #21=IFCFILLAREASTYLE('White',(#23),.T.);\n#23=IFCCOLOURRGB($,1.,1.,1.);\n\
         #24=IFCFILLAREASTYLE('Tiled',(#25),.T.);\n\
         #25=IFCFILLAREASTYLETILES((#27,#27),(#28),1.);\n\
         #26=IFCDIRECTION((1.,0.));\n#27=IFCVECTOR(#26,0.01);\n\
         #28=IFCSTYLEDITEM($,(#29),$);\n#29=IFCCURVESTYLE($,$,$,$,$);\n\
         #50=IFCFILLAREASTYLE('Hatched',(#51),.T.);\n\
         #51=IFCFILLAREASTYLEHATCHING(#52,0.005,$,$,0.);\n\
         #52=IFCCURVESTYLE($,$,$,$,$);\n\
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

/// Representations `#30`, `#31`, `#32` (listed backwards) on material #1.
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

fn index(model: &Model) -> StyleIndex<'_, 'static> {
    StyleIndex::new(StyleView::new(model, ifc4())).unwrap()
}

const M: EntityId = EntityId(1);

/// The refusal of the tiled fill #24.
fn tiles() -> StyleError {
    StyleError::UnsupportedFill {
        style: EntityId(24),
        entity: EntityId(25),
        construct: "IfcFillAreaStyleTiles".to_owned(),
    }
}

#[test]
fn a_refused_context_leaves_the_others_readable() {
    let model = two(10, 20, 11, 24);
    let index = index(&model);
    let entries = index.material_fills_by_context(M, StyleUnits::SI).unwrap();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].representation, EntityId(30));
    assert_eq!(entries[0].context, EntityId(10));
    assert_eq!(entries[0].target_view.as_deref(), Some("SECTION_VIEW"));
    assert_eq!(entries[0].identifier.as_deref(), Some("Body"));
    assert_eq!(entries[0].context_type.as_deref(), Some("Model"));
    assert_eq!(entries[0].parent_context, Some(EntityId(4)));
    let fill = entries[0].fill.as_ref().unwrap();
    assert_eq!(fill.name.as_deref(), Some("Grey"));
    assert_eq!(fill.styled_item, Some(EntityId(40)));
    assert_eq!(fill.style, EntityId(20));
    assert_eq!(entries[1].representation, EntityId(31));
    assert_eq!(entries[1].context, EntityId(11));
    assert_eq!(entries[1].target_view.as_deref(), Some("ELEVATION_VIEW"));
    assert_eq!(entries[1].fill, Err(tiles()));
    assert_eq!(index.material_fills(M, StyleUnits::SI), Err(tiles()));
    assert_eq!(index.material_fill(M, StyleUnits::SI), Err(tiles()));
    let section = index
        .material_fill_for_view(M, "SECTION_VIEW", StyleUnits::SI)
        .unwrap()
        .unwrap();
    assert_eq!(section.style, EntityId(20));
    assert_eq!(
        index.material_fill_for_view(M, "ELEVATION_VIEW", StyleUnits::SI),
        Err(tiles())
    );
    assert_eq!(
        index.material_fill_for_view(M, "PLAN_VIEW", StyleUnits::SI),
        Ok(None)
    );
}

#[test]
fn ambiguity_stays_in_the_context_that_states_it() {
    // Representations #31 and #32 share context 11 and differ.
    let grouped = three([10, 11, 11], [20, 20, 21]);
    let index = index(&grouped);
    let entries = index.material_fills_by_context(M, StyleUnits::SI).unwrap();
    assert_eq!(entries.len(), 2);
    assert!(entries[0].fill.is_ok());
    assert_eq!(entries[1].representation, EntityId(31));
    assert_eq!(entries[1].context, EntityId(11));
    assert_eq!(
        entries[1].fill,
        Err(StyleError::AmbiguousFill {
            owner: EntityId(11),
            styles: vec![EntityId(20), EntityId(21)],
        })
    );
    let section = index
        .material_fill_for_view(M, "SECTION_VIEW", StyleUnits::SI)
        .unwrap()
        .unwrap();
    assert_eq!(section.style, EntityId(20));

    // Two items of one representation that differ: owned by the representation.
    let items = model(
        "#5=IFCMATERIALDEFINITIONREPRESENTATION($,$,(#30,#31),#1);\n\
         #30=IFCSTYLEDREPRESENTATION(#10,'Style','Material',(#40));\n\
         #31=IFCSTYLEDREPRESENTATION(#11,'Style','Material',(#41,#42));\n\
         #40=IFCSTYLEDITEM($,(#20),$);\n#41=IFCSTYLEDITEM($,(#20),$);\n\
         #42=IFCSTYLEDITEM($,(#21),$);\n",
    );
    let entries = index_of(&items);
    assert!(entries[0].fill.is_ok());
    assert_eq!(
        entries[1].fill,
        Err(StyleError::AmbiguousFill {
            owner: EntityId(31),
            styles: vec![EntityId(20), EntityId(21)],
        })
    );
}

fn index_of(model: &Model) -> Vec<ifc_style::ContextFillResult> {
    index(model)
        .material_fills_by_context(M, StyleUnits::SI)
        .unwrap()
}

#[test]
fn a_unit_refusal_stays_in_its_context() {
    let model = two(10, 20, 11, 50);
    let index = index(&model);
    let entries = index
        .material_fills_by_context(M, StyleUnits::new(None, None).unwrap())
        .unwrap();
    assert_eq!(entries.len(), 2);
    assert!(entries[0].fill.is_ok());
    assert_eq!(
        entries[1].fill,
        Err(StyleError::UnitUndeclared {
            entity: EntityId(51),
            unit: "PLANEANGLEUNIT",
        })
    );
    let entries = index.material_fills_by_context(M, StyleUnits::SI).unwrap();
    assert!(entries.iter().all(|entry| entry.fill.is_ok()));
}

#[test]
fn the_first_refusal_in_a_context_wins() {
    // #30 and #32 share context 11: #30 is refused, #32 would be ambiguous
    // with it; #31 is in context 10.
    let model = three([11, 10, 11], [24, 20, 21]);
    let entries = index_of(&model);
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].representation, EntityId(30));
    assert_eq!(entries[0].context, EntityId(11));
    assert_eq!(entries[0].fill, Err(tiles()));
    assert_eq!(entries[1].representation, EntityId(31));
    assert_eq!(entries[1].context, EntityId(10));
    assert!(entries[1].fill.is_ok());

    // Two refusals in one context: the first, not the last, is reported.
    let model = three([11, 11, 10], [24, 50, 20]);
    let entries = index(&model)
        .material_fills_by_context(M, StyleUnits::new(None, None).unwrap())
        .unwrap();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].representation, EntityId(30));
    assert_eq!(entries[0].fill, Err(tiles()));
}

#[test]
fn refusals_no_context_owns_fail_the_call() {
    let units = StyleUnits::SI;
    // A member of Representations that is no IfcStyledRepresentation.
    let invalid = model(
        "#5=IFCMATERIALDEFINITIONREPRESENTATION($,$,(#30,#4),#1);\n\
         #30=IFCSTYLEDREPRESENTATION(#10,'Style','Material',(#40));\n\
         #40=IFCSTYLEDITEM($,(#20),$);\n",
    );
    assert!(matches!(
        index(&invalid).material_fills_by_context(M, units),
        Err(StyleError::ReferenceType {
            target: EntityId(4),
            ..
        })
    ));

    // A refused representation whose ContextOfItems is no context.
    let broken = |item: &str| {
        model(&format!(
            "#5=IFCMATERIALDEFINITIONREPRESENTATION($,$,(#30,#31),#1);\n\
             #30=IFCSTYLEDREPRESENTATION(#10,'Style','Material',(#40));\n\
             #31=IFCSTYLEDREPRESENTATION(#20,'Style','Material',(#41));\n\
             #40=IFCSTYLEDITEM($,(#20),$);\n{item}\n"
        ))
    };
    let refused = broken("#41=IFCSTYLEDITEM($,(#24),$);");
    assert!(matches!(
        index(&refused).material_fills_by_context(M, units),
        Err(StyleError::ReferenceType {
            target: EntityId(20),
            ..
        })
    ));
    // The same representation states no fill: its context is never read.
    let silent = broken("#41=IFCSTYLEDITEM($,(#29),$);");
    let entries = index_of(&silent);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].context, EntityId(10));
    assert_eq!(index(&silent).material_fills(M, units).unwrap().len(), 1);

    // #12 states TOP_VIEW, which is no IfcGeometricProjectionEnum literal.
    for fill in [21, 24] {
        let model = two(10, 20, 12, fill);
        assert!(matches!(
            index(&model).material_fills_by_context(M, units),
            Err(StyleError::InvalidValue {
                attribute: "TargetView",
                id: EntityId(12),
                ..
            })
        ));
    }
}

#[test]
fn a_view_lookup_needs_its_matching_contexts_to_agree() {
    let units = StyleUnits::SI;
    let model = two(10, 20, 13, 20);
    let section = index(&model)
        .material_fill_for_view(M, "SECTION_VIEW", units)
        .unwrap()
        .unwrap();
    assert_eq!(section.styled_item, Some(EntityId(40)));

    let model = two(10, 20, 13, 21);
    let index_ = index(&model);
    assert!(index_
        .material_fills_by_context(M, units)
        .unwrap()
        .iter()
        .all(|entry| entry.fill.is_ok()));
    assert_eq!(
        index_.material_fill_for_view(M, "SECTION_VIEW", units),
        Err(StyleError::AmbiguousFill {
            owner: M,
            styles: vec![EntityId(20), EntityId(21)],
        })
    );

    let model = two(10, 20, 13, 24);
    let index_ = index(&model);
    assert_eq!(
        index_.material_fill_for_view(M, "SECTION_VIEW", units),
        Err(tiles())
    );
    assert_eq!(
        index_.material_fill_for_view(M, "ELEVATION_VIEW", units),
        Ok(None)
    );
}

#[test]
fn a_view_lookup_takes_a_projection_literal() {
    let units = StyleUnits::SI;
    let empty = model("");
    let index_ = index(&empty);
    for value in ["TOP_VIEW", ".SECTION_VIEW.", "section_view"] {
        assert_eq!(
            index_.material_fill_for_view(M, value, units),
            Err(StyleError::UnknownTargetView {
                value: value.to_owned()
            })
        );
    }
    assert_eq!(
        index_.material_fill_for_view(M, "SECTION_VIEW", units),
        Ok(None)
    );

    // A plain context matches no view, not even MODEL_VIEW.
    let model = two(4, 20, 4, 20);
    let index_ = index(&model);
    assert!(index_.material_fill(M, units).unwrap().is_some());
    for view in ["MODEL_VIEW", "NOTDEFINED"] {
        assert_eq!(index_.material_fill_for_view(M, view, units), Ok(None));
    }
}

#[test]
fn an_unknown_view_is_refused_before_anything_is_read() {
    let broken = model("#5=IFCMATERIALDEFINITIONREPRESENTATION($,$,(#4),#1);\n");
    assert!(matches!(
        index(&broken).material_fill_for_view(M, "nope", StyleUnits::SI),
        Err(StyleError::UnknownTargetView { .. })
    ));
    assert!(matches!(
        index(&broken).material_fill_for_view(M, "SECTION_VIEW", StyleUnits::SI),
        Err(StyleError::ReferenceType { .. })
    ));
}
