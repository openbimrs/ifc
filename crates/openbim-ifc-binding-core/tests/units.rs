//! Unit conversion and a model's length unit through the binding core
//! (#469).

#![cfg(feature = "properties")]

use openbim_ifc_binding_core::value::Tagged;
use openbim_ifc_binding_core::{convert_unit, format_length, IfcModel, LengthUnitOptions};

const MODEL: &str = "ISO-10303-21;
HEADER;
FILE_DESCRIPTION((''),'2;1');
FILE_NAME('','',(''),(''),'','','');
FILE_SCHEMA(('IFC4'));
ENDSEC;
DATA;
#1=IFCSIUNIT(*,.LENGTHUNIT.,.MILLI.,.METRE.);
#2=IFCSIUNIT(*,.AREAUNIT.,$,.SQUARE_METRE.);
#3=IFCMONETARYUNIT('EUR');
#4=IFCUNITASSIGNMENT((#1,#2,#3));
#5=IFCPROJECT('0000000000000000000005',$,'P',$,$,$,$,(#6),#4);
#6=IFCGEOMETRICREPRESENTATIONCONTEXT($,'Model',3,0.01,#7,$);
#7=IFCAXIS2PLACEMENT3D(#8,$,$);
#8=IFCCARTESIANPOINT((1000.,2500.,0.));
#9=IFCQUANTITYAREA('Area',$,$,7.5,$);
ENDSEC;
END-ISO-10303-21;
";

#[test]
fn named_units_convert_and_format() {
    assert_eq!(convert_unit(1.0, "foot", "MILLIMETRE").unwrap(), 304.8);
    assert_eq!(convert_unit(1500.0, "MILLI METRE", "METRE").unwrap(), 1.5);
    assert_eq!(
        convert_unit(1.0, "METRE", "SQUARE_METRE")
            .unwrap_err()
            .code(),
        "type-mismatch"
    );
    assert_eq!(
        convert_unit(1.0, "furlong", "METRE").unwrap_err().code(),
        "invalid-value"
    );
    assert_eq!(
        convert_unit(1.0, "degree Fahrenheit", "KELVIN")
            .unwrap_err()
            .code(),
        "unsupported"
    );
    assert_eq!(
        format_length(444.5, "MILLIMETRE", "feet-inches", 8, None, true).unwrap(),
        "1' - 5 1/2\""
    );
    assert_eq!(
        format_length(1250.0, "MILLIMETRE", "decimal", 2, Some("METRE"), false).unwrap(),
        "1.25"
    );
    assert_eq!(
        format_length(1.0, "METRE", "roman", 2, None, false)
            .unwrap_err()
            .code(),
        "invalid-value"
    );
}

#[test]
fn a_model_converts_its_length_unit() {
    let mut model = IfcModel::parse(MODEL.as_bytes()).unwrap();
    assert_eq!(model.convert_unit_value(2.0, 1, 1).unwrap(), 2.0);
    assert_eq!(
        model.convert_unit_value(1.0, 3, 1).unwrap_err().code(),
        "unsupported"
    );
    assert_eq!(
        model.convert_unit_value(1.0, 99, 1).unwrap_err().code(),
        "missing-entity"
    );

    let mut dry = LengthUnitOptions::new();
    dry.dry_run = true;
    let staged = model.convert_length_unit("METRE", dry).unwrap();
    assert!(!staged.applied);
    assert_eq!(staged.removed, [1]);

    let report = model
        .convert_length_unit("METRE", LengthUnitOptions::new())
        .unwrap();
    assert!(report.applied);
    assert_eq!(report.from, "MILLI METRE");
    assert_eq!(report.length_factor, 0.001);
    assert_eq!(report.area_factor, Some(1.0));
    let point = report
        .converted
        .iter()
        .find(|c| c.entity_type == "IFCCARTESIANPOINT")
        .unwrap();
    assert_eq!((point.kind.as_str(), point.values), ("length", 3));
    assert_eq!(
        model.attribute(8, 0).unwrap(),
        Tagged::List(vec![
            Tagged::Real(1.0),
            Tagged::Real(2.5),
            Tagged::Real(0.0)
        ])
    );
    assert_eq!(
        model
            .convert_length_unit("SQUARE_METRE", LengthUnitOptions::new())
            .unwrap_err()
            .code(),
        "invalid-value"
    );
}
