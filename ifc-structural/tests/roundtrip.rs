use ifc_model::Codec;
use ifc_step::StepCodec;
use ifc_structural::{LoadKind, StructuralView};

const SOURCE: &[u8] = b"ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\nFILE_NAME('structural.ifc','',(''),(''),'','','');\nFILE_SCHEMA(('IFC4'));\nENDSEC;\nDATA;\n#1=IFCSTRUCTURALLOADSINGLEFORCE('Wind',1.,2.,3.,4.,5.,6.);\nENDSEC;\nEND-ISO-10303-21;\n";

#[test]
fn structural_load_projection_survives_step_write_read() {
    let codec = StepCodec;
    let model = codec.read_bytes(SOURCE).unwrap();
    let view = StructuralView::for_model(&model).unwrap();
    let load = view.static_load(ifc_model::EntityId(1)).unwrap();
    assert_eq!(load.kind(), LoadKind::SingleForce);
    assert_eq!(
        load.components().unwrap(),
        [
            Some(1.0),
            Some(2.0),
            Some(3.0),
            Some(4.0),
            Some(5.0),
            Some(6.0)
        ]
    );

    let bytes = codec.write_bytes(&model).unwrap();
    let reparsed = codec.read_bytes(&bytes).unwrap();
    let reparsed_view = StructuralView::for_model(&reparsed).unwrap();
    assert_eq!(
        reparsed_view
            .static_load(ifc_model::EntityId(1))
            .unwrap()
            .components()
            .unwrap(),
        load.components().unwrap(),
    );
}
