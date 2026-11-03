//! SI input in the unit that applies (#487): the project unit of the measure
//! (millimetre, foot, Celsius), an explicit unit, and the refusals (no
//! project unit, offset units, unmapped measures, non-finite input).

use ifc_model::{Codec, EntityId, Model};
use ifc_properties::{
    exact_unit, si_conversion, si_to_project, ExactUnitError, UnitConversion, UnitConversionError,
};

fn parse(schema: &str, data: &str) -> Model {
    let text = format!(
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
         FILE_NAME('','',(''),(''),'','','');\nFILE_SCHEMA(('{schema}'));\n\
         ENDSEC;\nDATA;\n{data}\nENDSEC;\nEND-ISO-10303-21;\n"
    );
    let model = ifc_step::StepCodec
        .read_bytes(text.as_bytes())
        .expect("fixture parses");
    assert!(model.diagnostics().is_empty(), "{:?}", model.diagnostics());
    model
}

const SCHEMAS: [&str; 3] = ["IFC2X3", "IFC4", "IFC4X3_ADD2"];

/// Millimetre length, SI area and volume, radian, kilogram, kilopascal,
/// Celsius, watt per square metre kelvin (#14) and per square millimetre
/// kelvin (#16); no frequency unit.
const MM: &str = "#1=IFCSIUNIT(*,.LENGTHUNIT.,.MILLI.,.METRE.);
#2=IFCSIUNIT(*,.AREAUNIT.,$,.SQUARE_METRE.);
#3=IFCSIUNIT(*,.VOLUMEUNIT.,$,.CUBIC_METRE.);
#4=IFCSIUNIT(*,.PLANEANGLEUNIT.,$,.RADIAN.);
#5=IFCSIUNIT(*,.MASSUNIT.,.KILO.,.GRAM.);
#6=IFCSIUNIT(*,.PRESSUREUNIT.,.KILO.,.PASCAL.);
#7=IFCSIUNIT(*,.THERMODYNAMICTEMPERATUREUNIT.,$,.DEGREE_CELSIUS.);
#8=IFCSIUNIT(*,.POWERUNIT.,$,.WATT.);
#9=IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.);
#10=IFCSIUNIT(*,.THERMODYNAMICTEMPERATUREUNIT.,$,.KELVIN.);
#11=IFCDERIVEDUNITELEMENT(#8,1);
#12=IFCDERIVEDUNITELEMENT(#9,-2);
#13=IFCDERIVEDUNITELEMENT(#10,-1);
#14=IFCDERIVEDUNIT((#11,#12,#13),.THERMALTRANSMITTANCEUNIT.,$);
#15=IFCDERIVEDUNITELEMENT(#1,-2);
#16=IFCDERIVEDUNIT((#11,#15,#13),.THERMALTRANSMITTANCEUNIT.,$);
#17=IFCUNITASSIGNMENT((#1,#2,#3,#4,#5,#6,#7,#14));
#18=IFCPROJECT('0000000000000000000018',$,'P',$,$,$,$,$,#17);";

const FOOT: &str = "#1=IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.);
#2=IFCDIMENSIONALEXPONENTS(1,0,0,0,0,0,0);
#3=IFCMEASUREWITHUNIT(IFCLENGTHMEASURE(0.3048),#1);
#4=IFCCONVERSIONBASEDUNIT(#2,.LENGTHUNIT.,'foot',#3);
#5=IFCSIUNIT(*,.AREAUNIT.,$,.SQUARE_METRE.);
#6=IFCDIMENSIONALEXPONENTS(2,0,0,0,0,0,0);
#7=IFCMEASUREWITHUNIT(IFCAREAMEASURE(0.09290304),#5);
#8=IFCCONVERSIONBASEDUNIT(#6,.AREAUNIT.,'square foot',#7);
#9=IFCSIUNIT(*,.VOLUMEUNIT.,$,.CUBIC_METRE.);
#10=IFCDIMENSIONALEXPONENTS(3,0,0,0,0,0,0);
#11=IFCMEASUREWITHUNIT(IFCVOLUMEMEASURE(0.028316846592),#9);
#12=IFCCONVERSIONBASEDUNIT(#10,.VOLUMEUNIT.,'cubic foot',#11);
#13=IFCSIUNIT(*,.PLANEANGLEUNIT.,$,.RADIAN.);
#14=IFCDIMENSIONALEXPONENTS(0,0,0,0,0,0,0);
#15=IFCMEASUREWITHUNIT(IFCPLANEANGLEMEASURE(0.017453292519943295),#13);
#16=IFCCONVERSIONBASEDUNIT(#14,.PLANEANGLEUNIT.,'degree',#15);
#17=IFCUNITASSIGNMENT((#4,#8,#12,#16));
#18=IFCPROJECT('0000000000000000000018',$,'P',$,$,$,$,$,#17);";

const FAHRENHEIT: &str = "#1=IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.);
#2=IFCSIUNIT(*,.THERMODYNAMICTEMPERATUREUNIT.,$,.KELVIN.);
#3=IFCDIMENSIONALEXPONENTS(0,0,0,0,1,0,0);
#4=IFCMEASUREWITHUNIT(IFCTHERMODYNAMICTEMPERATUREMEASURE(0.5555555555555556),#2);
#5=IFCCONVERSIONBASEDUNITWITHOFFSET(#3,.THERMODYNAMICTEMPERATUREUNIT.,'degree Fahrenheit',#4,459.67);
#6=IFCUNITASSIGNMENT((#1,#5));
#7=IFCPROJECT('0000000000000000000007',$,'P',$,$,$,$,$,#6);";

fn mm(schema: &str) -> Model {
    // IFC4X3 ADD2 adds `IfcDerivedUnit.Name`.
    let data = if schema.starts_with("IFC4X3") {
        MM.replace(
            ".THERMALTRANSMITTANCEUNIT.,$)",
            ".THERMALTRANSMITTANCEUNIT.,$,$)",
        )
    } else {
        MM.to_owned()
    };
    parse(schema, &data)
}

/// `exact.scale * p + exact.offset` gives back the SI value.
fn assert_round_trip(model: &Model, measure: &str, si: f64) {
    let p = si_to_project(model, measure, si).unwrap();
    let exact = exact_unit(model, measure, None).unwrap();
    let back = p * exact.scale + exact.offset;
    assert!(
        (back - si).abs() <= 1e-12 * si.abs().max(1.0),
        "{measure}: {si} -> {p} -> {back}"
    );
}

#[test]
fn si_input_becomes_the_project_unit_in_every_release() {
    for schema in SCHEMAS {
        let model = mm(schema);
        let cases = [
            ("IfcLengthMeasure", 1.234, 1234.0),
            ("IfcPositiveLengthMeasure", 2.5, 2500.0),
            ("IfcAreaMeasure", 2.5, 2.5),
            ("IfcVolumeMeasure", 2.5, 2.5),
            ("IfcPlaneAngleMeasure", 1.5, 1.5),
            ("IfcMassMeasure", 2.5, 2.5),
            ("IfcPressureMeasure", 2500.0, 2.5),
            ("IfcThermodynamicTemperatureMeasure", 273.15, 0.0),
            ("IfcThermodynamicTemperatureMeasure", 373.15, 100.0),
            ("IfcThermalTransmittanceMeasure", 0.3, 0.3),
            ("IfcRatioMeasure", 0.5, 0.5),
        ];
        for (measure, si, expected) in cases {
            assert_eq!(
                si_to_project(&model, measure, si),
                Ok(expected),
                "{schema} {measure}"
            );
            assert_round_trip(&model, measure, si);
        }
    }
}

#[test]
fn the_measure_type_is_case_insensitive() {
    let model = mm("IFC4");
    assert_eq!(si_to_project(&model, "IFCLENGTHMEASURE", 1.234), Ok(1234.0));
}

#[test]
fn an_explicit_unit_replaces_the_project_unit() {
    for schema in SCHEMAS {
        let model = mm(schema);
        let derived =
            si_conversion(&model, "IfcThermalTransmittanceMeasure", Some(EntityId(16))).unwrap();
        assert_eq!(derived.apply(0.3), 3e-7, "{schema}");
        assert!(si_conversion(&model, "IfcLengthMeasure", Some(EntityId(9)))
            .unwrap()
            .is_identity());
        assert_eq!(
            si_conversion(&model, "IfcRatioMeasure", None),
            Ok(UnitConversion::IDENTITY)
        );
    }
}

#[test]
fn undeclared_unusable_and_non_finite_input_is_refused() {
    let model = mm("IFC4");
    assert!(matches!(
        si_to_project(&model, "IfcFrequencyMeasure", 1.0),
        Err(UnitConversionError::Resolve(ExactUnitError::NoProjectUnit { unit_type }))
            if &*unit_type == "FREQUENCYUNIT"
    ));
    assert!(matches!(
        si_to_project(&model, "IfcLabel", 1.0),
        Err(UnitConversionError::Resolve(
            ExactUnitError::NotAMeasure { .. }
        ))
    ));
    assert!(matches!(
        si_to_project(&model, "IfcThermalConductivityMeasure", 1.0),
        Err(UnitConversionError::Resolve(
            ExactUnitError::UnmappedMeasureType { .. }
        ))
    ));
    assert!(matches!(
        si_conversion(&model, "IfcLengthMeasure", Some(EntityId(6))),
        Err(UnitConversionError::Resolve(ExactUnitError::UnitTypeMismatch { unit, .. }))
            if unit == EntityId(6)
    ));
    assert!(matches!(
        si_conversion(&model, "IfcRatioMeasure", Some(EntityId(1))),
        Err(UnitConversionError::Resolve(ExactUnitError::UnexpectedUnit { unit }))
            if unit == EntityId(1)
    ));
    for value in [f64::NAN, f64::INFINITY] {
        assert!(matches!(
            si_to_project(&model, "IfcLengthMeasure", value),
            Err(UnitConversionError::NonFinite { .. })
        ));
    }
}

#[test]
fn conversion_based_units_convert_exactly_and_round_trip() {
    for schema in SCHEMAS {
        let model = parse(schema, FOOT);
        let cases = [
            ("IfcLengthMeasure", 0.3048, 1.0),
            ("IfcLengthMeasure", 3.048, 10.0),
            ("IfcAreaMeasure", 0.09290304, 1.0),
            ("IfcVolumeMeasure", 0.028316846592, 1.0),
            ("IfcPlaneAngleMeasure", std::f64::consts::FRAC_PI_2, 90.0),
        ];
        for (measure, si, expected) in cases {
            assert_eq!(
                si_to_project(&model, measure, si),
                Ok(expected),
                "{schema} {measure}"
            );
            assert_round_trip(&model, measure, si);
        }
    }
}

#[test]
fn a_unit_with_an_offset_is_refused() {
    for schema in ["IFC4", "IFC4X3_ADD2"] {
        let model = parse(schema, FAHRENHEIT);
        assert_eq!(
            si_to_project(&model, "IfcThermodynamicTemperatureMeasure", 300.0),
            Err(UnitConversionError::OffsetUnit {
                name: "IfcConversionBasedUnitWithOffset".into(),
                unit: Some(EntityId(5)),
            }),
            "{schema}"
        );
        assert_eq!(si_to_project(&model, "IfcLengthMeasure", 1.0), Ok(1.0));
    }
}
