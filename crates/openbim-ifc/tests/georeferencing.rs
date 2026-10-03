//! Every coordinate operation of a model, scaled by its project length unit
//! (#123): the `georef` x `properties` join.

#![cfg(all(feature = "georef", feature = "properties", feature = "step"))]

use ifc::georef::{GeorefError, OperationKind};
use ifc::{georeferencing, Codec, GeoreferencingError, StepCodec};

const FIXTURE: &str = "../../test/fixtures/synthetic-surfaces/synthetic_conic_offset_bounded.ifc";

fn read(text: &str) -> ifc::Model {
    StepCodec.read_bytes(text.as_bytes()).expect("parses")
}

#[test]
fn a_map_conversion_resolves_with_the_project_length_unit() {
    let model = StepCodec
        .read_bytes(&std::fs::read(FIXTURE).expect("fixture"))
        .expect("parses");
    let operations = georeferencing(&model).expect("resolves");
    assert_eq!(operations.len(), 1);
    let map = &operations[0];
    assert_eq!(map.operation, ifc::EntityId(51));
    assert_eq!(map.kind, OperationKind::MapConversion);
    assert_eq!(map.target_crs.name.as_deref(), Some("EPSG:25832"));
    assert_eq!(
        (map.eastings, map.northings, map.orthogonal_height),
        (1.0, 2.0, 0.01)
    );
    assert_eq!(map.project_unit.metres_per_unit, 1.0);
    assert_eq!(map.map_point([0.0, 0.0, 0.0]), [1.0, 2.0, 0.01]);
}

const MILLIMETRE_IFC4: &str = "ISO-10303-21;
HEADER;FILE_DESCRIPTION((''),'2;1');FILE_NAME('','',(''),(''),'','','');FILE_SCHEMA(('IFC4'));ENDSEC;
DATA;
#1=IFCSIUNIT(*,.LENGTHUNIT.,.MILLI.,.METRE.);
#2=IFCUNITASSIGNMENT((#1));
#3=IFCCARTESIANPOINT((0.,0.,0.));
#4=IFCAXIS2PLACEMENT3D(#3,$,$);
#5=IFCGEOMETRICREPRESENTATIONCONTEXT($,'Model',3,1.E-5,#4,$);
#6=IFCPROJECT('1ezb0oWf13sB2o5xBWMMb9',$,'p',$,$,$,$,(#5),#2);
#7=IFCSIUNIT(*,.AREAUNIT.,$,.SQUARE_METRE.);
#8=IFCPROJECTEDCRS('EPSG:25832',$,$,$,$,$,$);
#9=IFCMAPCONVERSION(#5,#8,500.,600.,10.,$,$,$);
ENDSEC;
END-ISO-10303-21;
";

#[test]
fn the_project_unit_scales_every_operation() {
    let operations = georeferencing(&read(MILLIMETRE_IFC4)).expect("resolves");
    assert_eq!(operations.len(), 1);
    let map = &operations[0];
    assert!((map.project_unit.metres_per_unit - 0.001).abs() < 1e-15);
    // No `MapUnit`: the eastings are in the project unit, millimetres.
    assert_eq!(map.declared_map_unit(), None);
    let origin = map.translation();
    assert!((origin[0] - 0.5).abs() < 1e-12, "{origin:?}");
    assert!((origin[1] - 0.6).abs() < 1e-12, "{origin:?}");
}

#[test]
fn no_operation_needs_no_units() {
    let text = MILLIMETRE_IFC4
        .replace("#9=IFCMAPCONVERSION(#5,#8,500.,600.,10.,$,$,$);\n", "")
        .replace("#2=IFCUNITASSIGNMENT((#1));", "#2=IFCUNITASSIGNMENT((#3));");
    assert_eq!(georeferencing(&read(&text)), Ok(Vec::new()));
}

#[test]
fn a_release_without_georeferencing_is_refused() {
    let text = MILLIMETRE_IFC4.replace("'IFC4'", "'IFC2X3'");
    assert!(matches!(
        georeferencing(&read(&text)),
        Err(GeoreferencingError::Georef(
            GeorefError::UnsupportedSchema { .. }
        ))
    ));
}

#[test]
fn an_unresolvable_project_unit_is_refused_not_assumed() {
    // The project assigns an area unit only: no length unit to scale by.
    let text =
        MILLIMETRE_IFC4.replace("#2=IFCUNITASSIGNMENT((#1));", "#2=IFCUNITASSIGNMENT((#7));");
    assert!(matches!(
        georeferencing(&read(&text)),
        Err(GeoreferencingError::ProjectLengthUnit(_))
    ));
}
