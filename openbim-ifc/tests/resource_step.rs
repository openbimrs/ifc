//! Facade wiring: resource domain read through the STEP codec.

#![cfg(all(feature = "step", feature = "resource"))]

use ifc::resource::{ResourceError, ResourceTypeKind, ResourceView};
use ifc::{Codec, StepCodec};

const IFC4_SOURCE: &[u8] = br#"ISO-10303-21;
HEADER;
FILE_DESCRIPTION(('resource fixture'),'2;1');
FILE_NAME('resource.ifc','',(''),(''),'','','');
FILE_SCHEMA(('IFC4'));
ENDSEC;
DATA;
#1=IFCLABORRESOURCETYPE($,$,'Carpentry crew type',$,$,$,$,'Skilled',$,$,.CARPENTRY.);
#2=IFCLABORRESOURCE('2O2Fr$t4X7Zf8NOew3FLOH',$,'Crew A',$,$,$,$,$,$);
#3=IFCRELDEFINESBYTYPE('1O2Fr$t4X7Zf8NOew3FLOH',$,$,$,(#2),#1);
#4=IFCPERSON($,'Doe','Jane',$,$,$,$,$);
ENDSEC;
END-ISO-10303-21;"#;

const IFC4X3_SOURCE: &[u8] = br#"ISO-10303-21;
HEADER;
FILE_DESCRIPTION(('resource fixture'),'2;1');
FILE_NAME('resource.ifc','',(''),(''),'','','');
FILE_SCHEMA(('IFC4X3_ADD2'));
ENDSEC;
DATA;
#1=IFCLABORRESOURCETYPE($,$,'Formwork crew type',$,$,$,$,'Skilled',$,$,.CARPENTRY.);
#2=IFCLABORRESOURCE('2O2Fr$t4X7Zf8NOew3FLOH',$,'Crew B',$,$,$,$,$,$);
#3=IFCRELDEFINESBYTYPE('1O2Fr$t4X7Zf8NOew3FLOH',$,$,$,(#2),#1);
#4=IFCPERSON($,'Roe','Jan',$,$,$,$,$);
ENDSEC;
END-ISO-10303-21;"#;

const IFC2X3_SOURCE: &[u8] = br#"ISO-10303-21;
HEADER;
FILE_DESCRIPTION(('resource fixture'),'2;1');
FILE_NAME('resource.ifc','',(''),(''),'','','');
FILE_SCHEMA(('IFC2X3'));
ENDSEC;
DATA;
#1=IFCLABORRESOURCE('2O2Fr$t4X7Zf8NOew3FLOH',$,'Crew C',$,$,$,$,$,$,$);
ENDSEC;
END-ISO-10303-21;"#;

#[test]
fn ifc4_resource_data_round_trips_through_step() {
    let model = StepCodec.read_bytes(IFC4_SOURCE).unwrap();
    let view = ResourceView::for_model(&model).unwrap();
    let ty = ifc::EntityId(1);
    let labor = ifc::EntityId(2);
    let projected = view.resource_type(ty).unwrap();
    assert_eq!(projected.kind(), ResourceTypeKind::Labor);
    assert_eq!(projected.name().unwrap(), "Carpentry crew type");
    assert_eq!(view.assigned_resource_type(labor).unwrap(), Some(ty));

    let bytes = StepCodec.write_bytes(&model).unwrap();
    let decoded = StepCodec.read_bytes(&bytes).unwrap();
    let view = ResourceView::for_model(&decoded).unwrap();
    assert_eq!(view.assigned_resource_type(labor).unwrap(), Some(ty));
}

#[test]
fn ifc4x3_resource_data_round_trips_through_step() {
    let model = StepCodec.read_bytes(IFC4X3_SOURCE).unwrap();
    let view = ResourceView::for_model(&model).unwrap();
    let ty = ifc::EntityId(1);
    let labor = ifc::EntityId(2);
    let projected = view.resource_type(ty).unwrap();
    assert_eq!(projected.kind(), ResourceTypeKind::Labor);
    assert_eq!(projected.name().unwrap(), "Formwork crew type");
    assert_eq!(view.assigned_resource_type(labor).unwrap(), Some(ty));

    let bytes = StepCodec.write_bytes(&model).unwrap();
    let decoded = StepCodec.read_bytes(&bytes).unwrap();
    let view = ResourceView::for_model(&decoded).unwrap();
    assert_eq!(view.assigned_resource_type(labor).unwrap(), Some(ty));
}

#[test]
fn ifc2x3_step_file_is_a_typed_refusal_not_silently_dropped() {
    let model = StepCodec.read_bytes(IFC2X3_SOURCE).unwrap();
    assert!(matches!(
        ResourceView::for_model(&model),
        Err(ResourceError::UnsupportedSchema { .. })
    ));
}
