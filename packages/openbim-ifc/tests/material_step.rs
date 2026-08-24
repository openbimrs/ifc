#![cfg(all(feature = "step", feature = "material"))]

use ifc::material::{LayerSetDirection, MaterialView};
use ifc::{Codec, EntityId, StepCodec};

#[test]
fn step_material_resource_records_project_through_facade() {
    let source = br#"ISO-10303-21;
HEADER;
FILE_DESCRIPTION(('material fixture'),'2;1');
FILE_NAME('material.ifc','',(''),(''),'','','');
FILE_SCHEMA(('IFC4'));
ENDSEC;
DATA;
#1=IFCMATERIAL('Concrete',$,'Concrete');
#2=IFCMATERIALLAYER(#1,0.2,.F.,'Core',$,'Structure',80);
#3=IFCMATERIALLAYERSET((#2),'Wall',$);
#4=IFCMATERIALLAYERSETUSAGE(#3,.AXIS2.,.POSITIVE.,-0.1,3.0);
#5=IFCMATERIALPROPERTIES('Pset_MaterialCommon',$,(#6),#1);
#6=IFCPROPERTYSINGLEVALUE('Density',$,IFCMASSDENSITYMEASURE(2400.),$);
ENDSEC;
END-ISO-10303-21;"#;
    let model = StepCodec.read_bytes(source).unwrap();
    let view = MaterialView::new(&model);
    let material = view.materials().next().unwrap();
    assert_eq!(material.category().unwrap(), Some("Concrete"));
    assert_eq!(
        view.properties_for(material.id())
            .next()
            .unwrap()
            .unwrap()
            .name()
            .unwrap(),
        Some("Pset_MaterialCommon")
    );
    let set = view.layer_sets().next().unwrap();
    assert_eq!(set.layer_ids().unwrap(), vec![EntityId(2)]);
    assert_eq!(view.total_thickness(set).unwrap(), 0.2);
    let usage = view.layer_set_usages().next().unwrap();
    assert_eq!(
        usage.layer_set_direction().unwrap(),
        LayerSetDirection::Axis2
    );
    assert_eq!(usage.offset_from_reference_line().unwrap(), -0.1);
}
