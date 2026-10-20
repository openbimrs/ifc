//! Conversion between a model's units (#469): SI prefixes, conversion-based
//! units through their factors, derived units by their exponents, and the
//! typed refusals (dimensions, monetary, context-dependent, offset #110).

use ifc_model::{Codec, EntityId, Model};
use ifc_properties::{
    convert_between, exact_project_unit, measure_unit_type, resolve_unit, unit_conversion,
    ConversionUnit, ExactUnitError, MeasureUnitType, SchemaVersion, UnitConversionError,
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

/// Millimetre, metre, a foot over the metre, an inch over the millimetre,
/// square and cubic millimetres, a derived velocity (mm/s), a monetary
/// unit, a context-dependent unit and a Fahrenheit with an offset.
const UNITS: &str = "#1=IFCSIUNIT(*,.LENGTHUNIT.,.MILLI.,.METRE.);
#2=IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.);
#3=IFCDIMENSIONALEXPONENTS(1,0,0,0,0,0,0);
#4=IFCMEASUREWITHUNIT(IFCLENGTHMEASURE(0.3048),#2);
#5=IFCCONVERSIONBASEDUNIT(#3,.LENGTHUNIT.,'foot',#4);
#6=IFCMEASUREWITHUNIT(IFCLENGTHMEASURE(25.4),#1);
#7=IFCCONVERSIONBASEDUNIT(#3,.LENGTHUNIT.,'inch',#6);
#8=IFCSIUNIT(*,.AREAUNIT.,.MILLI.,.SQUARE_METRE.);
#9=IFCSIUNIT(*,.VOLUMEUNIT.,.MILLI.,.CUBIC_METRE.);
#10=IFCSIUNIT(*,.TIMEUNIT.,$,.SECOND.);
#11=IFCDERIVEDUNITELEMENT(#1,1);
#12=IFCDERIVEDUNITELEMENT(#10,-1);
#13=IFCDERIVEDUNIT((#11,#12),.LINEARVELOCITYUNIT.,$);
#14=IFCDERIVEDUNITELEMENT(#2,1);
#15=IFCDERIVEDUNIT((#14,#12),.LINEARVELOCITYUNIT.,$);
#16=IFCMONETARYUNIT('EUR');
#17=IFCDIMENSIONALEXPONENTS(0,0,0,0,0,0,0);
#18=IFCCONTEXTDEPENDENTUNIT(#17,.USERDEFINED.,'brick');
#19=IFCSIUNIT(*,.THERMODYNAMICTEMPERATUREUNIT.,$,.KELVIN.);
#20=IFCDIMENSIONALEXPONENTS(0,0,0,0,1,0,0);
#21=IFCMEASUREWITHUNIT(IFCTHERMODYNAMICTEMPERATUREMEASURE(0.5555555555555556),#19);
#22=IFCCONVERSIONBASEDUNITWITHOFFSET(#20,.THERMODYNAMICTEMPERATUREUNIT.,'degree Fahrenheit',#21,459.67);
#23=IFCUNITASSIGNMENT((#1,#8,#9,#16));
#24=IFCPROJECT('0000000000000000000024',$,'P',$,$,$,$,$,#23);";

fn units(schema: &str) -> Model {
    // IFC4X3 ADD2 adds `IfcDerivedUnit.Name`.
    let data = if schema.starts_with("IFC4X3") {
        UNITS.replace(".LINEARVELOCITYUNIT.,$)", ".LINEARVELOCITYUNIT.,$,$)")
    } else {
        UNITS.to_owned()
    };
    parse(schema, &data)
}

fn id(n: u64) -> EntityId {
    EntityId(n)
}

#[test]
fn prefixes_conversion_factors_and_derived_exponents_convert_in_every_release() {
    for schema in ["IFC2X3", "IFC4", "IFC4X3_ADD2"] {
        let model = units(schema);
        // mm -> m: an exact power of ten.
        assert_eq!(
            convert_between(&model, 1234.0, id(1), id(2)),
            Ok(1.234),
            "{schema}"
        );
        // foot -> mm through the foot's factor in metres.
        assert_eq!(
            convert_between(&model, 1.0, id(5), id(1)),
            Ok(304.8),
            "{schema}"
        );
        // inch -> foot: two conversion-based units.
        let twelve = convert_between(&model, 12.0, id(7), id(5)).unwrap();
        assert!((twelve - 1.0).abs() < 1e-15, "{schema}: {twelve}");
        // mm/s -> m/s, derived by exponents.
        assert_eq!(
            convert_between(&model, 1500.0, id(13), id(15)),
            Ok(1.5),
            "{schema}"
        );
        // mm² against mm: different dimensions.
        assert_eq!(
            convert_between(&model, 1.0, id(8), id(1)),
            Err(UnitConversionError::DimensionMismatch {
                from: [2, 0, 0, 0, 0, 0, 0],
                to: [1, 0, 0, 0, 0, 0, 0],
            }),
            "{schema}"
        );
    }
}

#[test]
fn a_model_unit_converts_to_a_named_or_si_unit() {
    let model = units("IFC4");
    let mm2 = resolve_unit(&model, id(8)).unwrap();
    assert_eq!(mm2.name.as_ref(), "MILLI SQUARE_METRE");
    assert_eq!(mm2.unit_type.as_deref(), Some("AREAUNIT"));
    assert_eq!((mm2.mantissa, mm2.decimal_exponent), (1.0, -6));
    let square_foot = ConversionUnit::parse("square foot").unwrap();
    let to_feet = unit_conversion(&mm2, &square_foot).unwrap();
    assert!((to_feet.apply(92_903.04) - 1.0).abs() < 1e-12);
    let foot = resolve_unit(&model, id(5)).unwrap();
    assert_eq!(foot.name.as_ref(), "foot");
    assert_eq!(
        unit_conversion(&foot, &ConversionUnit::parse("foot").unwrap())
            .unwrap()
            .apply(3.0),
        3.0
    );
}

#[test]
fn monetary_context_dependent_and_offset_units_are_refused() {
    let model = units("IFC4");
    assert_eq!(
        resolve_unit(&model, id(16)),
        Err(UnitConversionError::MonetaryUnit { unit: id(16) })
    );
    assert_eq!(
        resolve_unit(&model, id(18)),
        Err(UnitConversionError::ContextDependentUnit { unit: id(18) })
    );
    assert!(matches!(
        resolve_unit(&model, id(22)),
        Err(UnitConversionError::OffsetUnit { unit: Some(unit), .. }) if unit == id(22)
    ));
    assert!(matches!(
        convert_between(&model, 1.0, id(22), id(19)),
        Err(UnitConversionError::OffsetUnit { .. })
    ));
}

#[test]
fn the_project_unit_of_a_type_is_found_or_refused() {
    let model = units("IFC4");
    assert_eq!(exact_project_unit(&model, "LENGTHUNIT"), Ok(id(1)));
    assert_eq!(exact_project_unit(&model, "areaunit"), Ok(id(8)));
    assert_eq!(
        exact_project_unit(&model, "MASSUNIT"),
        Err(ExactUnitError::NoProjectUnit {
            unit_type: "MASSUNIT".into()
        })
    );
}

#[test]
fn measure_types_map_to_their_unit_types_per_release() {
    let named = |unit: &str| MeasureUnitType::Named(unit.into());
    for schema in [
        SchemaVersion::Ifc2x3,
        SchemaVersion::Ifc4,
        SchemaVersion::Ifc4x3,
    ] {
        assert_eq!(
            measure_unit_type(schema, "IfcPositiveLengthMeasure"),
            Ok(named("LENGTHUNIT"))
        );
        assert_eq!(
            measure_unit_type(schema, "IFCAREAMEASURE"),
            Ok(named("AREAUNIT"))
        );
        assert_eq!(
            measure_unit_type(schema, "IFCRATIOMEASURE"),
            Ok(MeasureUnitType::Dimensionless)
        );
    }
    assert_eq!(
        measure_unit_type(SchemaVersion::Ifc4, "IfcNonNegativeLengthMeasure"),
        Ok(named("LENGTHUNIT"))
    );
    assert!(measure_unit_type(SchemaVersion::Ifc4x1, "IfcLengthMeasure").is_err());
}
