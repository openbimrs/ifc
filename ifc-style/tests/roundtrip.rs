use ifc_model::Codec;
use ifc_schema::ifc4;
use ifc_step::StepCodec;
use ifc_style::{BoxAlignment, StyleError, StyleView};

const SOURCE: &[u8] = br#"ISO-10303-21;
HEADER;
FILE_DESCRIPTION(('Style and annotation contract'),'2;1');
FILE_NAME('style.ifc','2026-08-31T00:00:00',(''),(''),'openbim','openbim','');
FILE_SCHEMA(('IFC4'));
ENDSEC;
DATA;
#1=IFCCOLOURRGB('blue',0.1,0.2,0.9);
#2=IFCSURFACESTYLESHADING(#1,0.25);
#3=IFCSURFACESTYLE('fill',.BOTH.,(#2));
#4=IFCCARTESIANPOINT((0.,0.));
#5=IFCAXIS2PLACEMENT2D(#4,$);
#6=IFCTEXTLITERAL('Hello',#5,.RIGHT.);
#7=IFCPLANAREXTENT(2.,1.);
#8=IFCTEXTLITERALWITHEXTENT('Hello',#5,.RIGHT.,#7,'middle-left');
#9=IFCPOLYLINE((#4,#4));
#10=IFCANNOTATIONFILLAREA(#9,$);
#11=IFCSTYLEDITEM(#10,(#3),$);
#12=IFCANNOTATION('0wA1b2C3d4E5f6G7h8I9Jk',$,'Note',$,$,$,$);
ENDSEC;
END-ISO-10303-21;
"#;

fn assert_style_contract(model: &ifc_model::Model) {
    let schema = ifc4();
    let view = StyleView::new(model, schema);
    let colour = model.of_type("IFCCOLOURRGB").next().unwrap().0;
    assert_eq!(view.colour_rgb(colour).unwrap().blue().unwrap(), 0.9);

    let text = model.of_type("IFCTEXTLITERAL").next().unwrap().0;
    assert_eq!(view.text_literal(text).unwrap().literal().unwrap(), "Hello");

    let extent = model.of_type("IFCTEXTLITERALWITHEXTENT").next().unwrap().0;
    assert_eq!(
        view.text_literal_with_extent(extent)
            .unwrap()
            .box_alignment()
            .unwrap(),
        BoxAlignment::MiddleLeft
    );

    let fill = model.of_type("IFCANNOTATIONFILLAREA").next().unwrap().0;
    let resolved = view.resolve_item_style(fill).unwrap();
    assert_eq!(resolved.effective_styles().len(), 1);

    let annotation = model.of_type("IFCANNOTATION").next().unwrap().0;
    assert_eq!(
        view.annotation(annotation).unwrap().name().unwrap(),
        Some("Note")
    );
}

#[test]
fn requested_style_and_annotation_graph_survives_step_roundtrip() {
    let codec = StepCodec;
    let model = codec.read_bytes(SOURCE).unwrap();
    assert_style_contract(&model);
    let bytes = codec.write_bytes(&model).unwrap();
    let reparsed = codec.read_bytes(&bytes).unwrap();
    assert_eq!(model.len(), reparsed.len());
    for (id, entity) in model.iter() {
        let other = reparsed.get(id).expect("entity must survive roundtrip");
        assert_eq!(entity.type_name, other.type_name);
        assert_eq!(entity.attributes, other.attributes);
    }
    assert_style_contract(&reparsed);
}

#[test]
fn dangling_style_reference_is_reported_instead_of_dropped() {
    let codec = StepCodec;
    let malformed = String::from_utf8(SOURCE.to_vec()).unwrap().replace(
        "#11=IFCSTYLEDITEM(#10,(#3),$);",
        "#11=IFCSTYLEDITEM(#10,(#999),$);",
    );
    let model = codec.read_bytes(malformed.as_bytes()).unwrap();
    let styled_id = model.of_type("IFCSTYLEDITEM").next().unwrap().0;

    let err = StyleView::new(&model, ifc4())
        .styled_item(styled_id)
        .unwrap()
        .styles()
        .unwrap_err();
    assert!(matches!(
        err,
        StyleError::DanglingReference { target, .. } if target == ifc_model::EntityId(999)
    ));
}
