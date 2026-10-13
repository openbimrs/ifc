//! Every coordinate operation of a model, scaled by its project length unit
//! (#123): the `georef` x `properties` join.

#![cfg(all(feature = "georef", feature = "properties", feature = "step"))]

use ifc::georef::{GeorefError, OperationKind, OperationSource};
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
fn a_release_without_a_verified_georeferencing_layout_is_refused() {
    let text = MILLIMETRE_IFC4.replace("'IFC4'", "'IFC4X1'");
    assert!(matches!(
        georeferencing(&read(&text)),
        Err(GeoreferencingError::Georef(
            GeorefError::UnsupportedSchema { .. }
        ))
    ));
}

const GEOREF_FIXTURES: &str = "../../test/fixtures/synthetic-georef";

fn fixture(name: &str) -> ifc::Model {
    let text = std::fs::read_to_string(format!("{GEOREF_FIXTURES}/{name}")).expect("fixture");
    read(&text)
}

/// #451: IFC2X3 is read through `ePSet_MapConversion` and
/// `ePSet_ProjectedCRS`, with the project length unit resolved here, and
/// matches its IFC4 twin.
#[test]
fn ifc2x3_property_sets_resolve_like_their_ifc4_twin() {
    let ifc2x3 = georeferencing(&fixture("epset_project_ifc2x3.ifc")).expect("IFC2X3 resolves");
    let ifc4 = georeferencing(&fixture("map_conversion_ifc4.ifc")).expect("IFC4 resolves");
    let ([pset], [entity]) = (ifc2x3.as_slice(), ifc4.as_slice()) else {
        panic!("one operation each: {ifc2x3:?} {ifc4:?}");
    };
    assert_eq!(pset.source, OperationSource::PropertySet(ifc::EntityId(13)));
    assert!((pset.project_unit.metres_per_unit - 0.001).abs() < 1e-15);
    assert_eq!(pset.project_unit, entity.project_unit);
    assert_eq!(pset.map_unit, entity.map_unit);
    assert_eq!(pset.linear_part(), entity.linear_part());
    assert_eq!(pset.translation(), entity.translation());
    assert_eq!(pset.target_crs.name, entity.target_crs.name);
}

#[test]
fn an_ifc2x3_model_without_the_sets_has_no_operation_and_needs_no_units() {
    let text = MILLIMETRE_IFC4
        .replace("'IFC4'", "'IFC2X3'")
        .replace("#9=IFCMAPCONVERSION(#5,#8,500.,600.,10.,$,$,$);\n", "")
        .replace("#2=IFCUNITASSIGNMENT((#1));", "#2=IFCUNITASSIGNMENT((#3));");
    assert_eq!(georeferencing(&read(&text)), Ok(Vec::new()));
}

#[test]
fn a_malformed_ifc2x3_property_set_is_refused() {
    assert!(matches!(
        georeferencing(&fixture("epset_split_pair_ifc2x3.ifc")),
        Err(GeoreferencingError::Georef(
            GeorefError::IncompletePropertySets { .. }
        ))
    ));
}

/// The release decides: property sets of the georeferencing names in an
/// IFC4 file are not read, only its `IfcMapConversion`.
#[test]
fn ifc4_entities_are_read_and_ifc4_property_sets_are_not() {
    let text = MILLIMETRE_IFC4.replace(
        "ENDSEC;\nEND-ISO",
        "#20=IFCPROPERTYSINGLEVALUE('Eastings',$,IFCLENGTHMEASURE(1.),$);
#21=IFCPROPERTYSET('0ezb0oWf13sB2o5xBWMMb9',$,'ePSet_MapConversion',$,(#20));
#22=IFCRELDEFINESBYPROPERTIES('2ezb0oWf13sB2o5xBWMMb9',$,$,$,(#6),#21);
ENDSEC;\nEND-ISO",
    );
    let model = read(&text);
    assert!(ifc::georef::has_property_set_georeferencing(&model));
    let operations = georeferencing(&model).expect("resolves");
    assert_eq!(operations.len(), 1);
    assert_eq!(operations[0].operation, ifc::EntityId(9));
    assert_eq!(operations[0].eastings, 500.0);
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
