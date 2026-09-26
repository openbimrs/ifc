//! `exact_unit`: a measure's effective unit, exactly or not at all (#53).

use ifc_model::{Codec, EntityId, Model};
use ifc_properties::{exact_unit, ExactPropertyError, ExactUnit, ExactUnitError, SchemaVersion};

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

fn ifc4(data: &str) -> Model {
    parse("IFC4", data)
}

fn id(n: u64) -> EntityId {
    EntityId(n)
}

fn resolved(result: Result<ExactUnit, ExactUnitError>) -> ExactUnit {
    result.unwrap_or_else(|error| panic!("expected a unit, got {error:?}"))
}

fn assert_scale(unit: &ExactUnit, expected: f64) {
    let error = (unit.scale - expected).abs() / expected.abs();
    assert!(error < 1e-12, "scale {} != {expected}", unit.scale);
}

const L: [i32; 7] = [1, 0, 0, 0, 0, 0, 0];

/// The fixture from issue #53, with a project so the assignment is in use.
fn issue_fixture() -> Model {
    ifc4(
        "#1=IFCSIUNIT(*,.AREAUNIT.,.MILLI.,.SQUARE_METRE.);
#2=IFCSIUNIT(*,.LENGTHUNIT.,.BOGUS.,.METRE.);
#3=IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.);
#4=IFCDIMENSIONALEXPONENTS(1,0,0,0,0,0,0);
#5=IFCMEASUREWITHUNIT(IFCLENGTHMEASURE(0.0254),#3);
#6=IFCCONVERSIONBASEDUNIT(#4,.LENGTHUNIT.,'INCH',#5);
#7=IFCSIUNIT(*,.ENERGYUNIT.,.MEGA.,.JOULE.);
#8=IFCSIUNIT(*,.ENERGYUNIT.,$,.JOULE.);
#9=IFCUNITASSIGNMENT((#8,#7));
#10=IFCPROJECT('p',$,'P',$,$,$,$,$,#9);",
    )
}

// ---- the five cases the issue lists ---------------------------------------

#[test]
fn a_prefixed_square_metre_raises_the_prefix_to_the_power() {
    let unit = resolved(exact_unit(&issue_fixture(), "IFCAREAMEASURE", Some(id(1))));
    assert_scale(&unit, 1e-6);
    assert_eq!(unit.dimensions, [2, 0, 0, 0, 0, 0, 0]);
    assert_eq!(
        (unit.unit, unit.from_project, unit.offset),
        (Some(id(1)), false, 0.0)
    );
}

#[test]
fn an_unknown_prefix_is_refused() {
    assert_eq!(
        exact_unit(&issue_fixture(), "IFCLENGTHMEASURE", Some(id(2))),
        Err(ExactUnitError::UnknownPrefix {
            unit: id(2),
            prefix: "BOGUS".into()
        })
    );
}

#[test]
fn a_conversion_based_unit_is_scaled_through_its_factor_unit() {
    let unit = resolved(exact_unit(
        &issue_fixture(),
        "IFCLENGTHMEASURE",
        Some(id(6)),
    ));
    assert_scale(&unit, 0.0254);
    assert_eq!(unit.dimensions, L);
}

#[test]
fn two_project_units_of_one_type_are_refused() {
    assert_eq!(
        exact_unit(&issue_fixture(), "IFCENERGYMEASURE", None),
        Err(ExactUnitError::DuplicateProjectUnit {
            unit_type: "ENERGYUNIT".into(),
            first: id(8),
            second: id(7),
        })
    );
}

#[test]
fn an_explicit_mega_joule_is_exact() {
    let unit = resolved(exact_unit(
        &issue_fixture(),
        "IFCENERGYMEASURE",
        Some(id(7)),
    ));
    assert_scale(&unit, 1e6);
    assert_eq!(unit.dimensions, [2, 1, -2, 0, 0, 0, 0]);
}

#[test]
fn an_offset_unit_is_refused_not_misapplied() {
    let model = ifc4(
        "#1=IFCSIUNIT(*,.THERMODYNAMICTEMPERATUREUNIT.,$,.KELVIN.);
#2=IFCDIMENSIONALEXPONENTS(0,0,0,0,1,0,0);
#3=IFCMEASUREWITHUNIT(IFCTHERMODYNAMICTEMPERATUREMEASURE(1.8),#1);
#4=IFCCONVERSIONBASEDUNITWITHOFFSET(#2,.THERMODYNAMICTEMPERATUREUNIT.,'Fahrenheit',#3,-459.67);",
    );
    assert_eq!(
        exact_unit(&model, "IFCTHERMODYNAMICTEMPERATUREMEASURE", Some(id(4))),
        Err(ExactUnitError::UnsupportedOffset { unit: id(4) })
    );
    // The permissive view keeps the offset instead of dropping it.
    match ifc_properties::unit(&model, id(4)) {
        Some(ifc_properties::UnitKind::Conversion { offset, .. }) => {
            assert_eq!(offset, Some(-459.67));
        }
        other => panic!("expected a conversion unit, got {other:?}"),
    }
}

// ---- project defaults -----------------------------------------------------

/// Four of the issue's real models: `MILLI METRE` lengths with unprefixed
/// `SQUARE_METRE` areas. Applying the length prefix to areas is off by 1000.
fn millimetre_project() -> Model {
    ifc4(
        "#1=IFCSIUNIT(*,.LENGTHUNIT.,.MILLI.,.METRE.);
#2=IFCSIUNIT(*,.AREAUNIT.,$,.SQUARE_METRE.);
#3=IFCSIUNIT(*,.PLANEANGLEUNIT.,$,.RADIAN.);
#4=IFCDIMENSIONALEXPONENTS(0,0,0,0,0,0,0);
#5=IFCMEASUREWITHUNIT(IFCPLANEANGLEMEASURE(0.017453292519943295),#3);
#6=IFCCONVERSIONBASEDUNIT(#4,.PLANEANGLEUNIT.,'DEGREE',#5);
#7=IFCSIUNIT(*,.MASSUNIT.,$,.GRAM.);
#8=IFCSIUNIT(*,.THERMODYNAMICTEMPERATUREUNIT.,$,.DEGREE_CELSIUS.);
#9=IFCUNITASSIGNMENT((#1,#2,#6,#7,#8));
#10=IFCPROJECT('p',$,'P',$,$,$,$,$,#9);",
    )
}

#[test]
fn project_defaults_apply_per_unit_type() {
    let model = millimetre_project();
    let length = resolved(exact_unit(&model, "IFCLENGTHMEASURE", None));
    assert_eq!((length.unit, length.from_project), (Some(id(1)), true));
    assert_scale(&length, 1e-3);
    let area = resolved(exact_unit(&model, "IFCAREAMEASURE", None));
    assert_eq!(area.unit, Some(id(2)));
    assert_scale(&area, 1.0);
}

#[test]
fn a_degree_resolves_to_radians() {
    let angle = resolved(exact_unit(
        &millimetre_project(),
        "IFCPLANEANGLEMEASURE",
        None,
    ));
    assert_eq!(angle.unit, Some(id(6)));
    assert_scale(&angle, std::f64::consts::PI / 180.0);
    assert_eq!(angle.dimensions, [0; 7]);
}

#[test]
fn a_defined_measure_type_follows_its_base() {
    let model = millimetre_project();
    let positive = resolved(exact_unit(&model, "IFCPOSITIVELENGTHMEASURE", None));
    assert_eq!(positive.unit, Some(id(1)));
    let angle = resolved(exact_unit(&model, "IfcPositivePlaneAngleMeasure", None));
    assert_eq!(angle.unit, Some(id(6)));
}

#[test]
fn mass_is_in_kilograms_and_celsius_carries_its_offset() {
    let model = millimetre_project();
    assert_scale(&resolved(exact_unit(&model, "IFCMASSMEASURE", None)), 1e-3);
    let celsius = resolved(exact_unit(
        &model,
        "IFCTHERMODYNAMICTEMPERATUREMEASURE",
        None,
    ));
    assert_scale(&celsius, 1.0);
    assert_eq!(celsius.offset, 273.15);
}

#[test]
fn a_measure_without_a_project_unit_is_refused() {
    assert_eq!(
        exact_unit(&millimetre_project(), "IFCVOLUMEMEASURE", None),
        Err(ExactUnitError::NoProjectUnit {
            unit_type: "VOLUMEUNIT".into()
        })
    );
}

#[test]
fn without_a_project_there_is_no_default() {
    let model = ifc4("#1=IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.);");
    assert_eq!(
        exact_unit(&model, "IFCLENGTHMEASURE", None),
        Err(ExactUnitError::NoProject)
    );
    // An explicit unit needs no project.
    assert_eq!(
        resolved(exact_unit(&model, "IFCLENGTHMEASURE", Some(id(1)))).unit,
        Some(id(1))
    );
}

// ---- dimensionless and non-measures --------------------------------------

#[test]
fn counts_and_ratios_are_dimensionless_not_errors() {
    let model = millimetre_project();
    for measure in [
        "IFCCOUNTMEASURE",
        "IFCRATIOMEASURE",
        "IFCNORMALISEDRATIOMEASURE",
    ] {
        let unit = resolved(exact_unit(&model, measure, None));
        assert_eq!(unit.unit, None, "{measure}");
        assert_eq!(
            (unit.dimensions, unit.scale, unit.offset),
            ([0; 7], 1.0, 0.0)
        );
        assert!(!unit.from_project);
    }
    assert_eq!(
        exact_unit(&model, "IFCCOUNTMEASURE", Some(id(1))),
        Err(ExactUnitError::UnexpectedUnit { unit: id(1) })
    );
}

#[test]
fn values_that_are_not_measures_or_not_mapped_are_refused() {
    let model = millimetre_project();
    assert_eq!(
        exact_unit(&model, "IFCLABEL", None),
        Err(ExactUnitError::NotAMeasure {
            measure_type: "IFCLABEL".into()
        })
    );
    assert_eq!(
        exact_unit(&model, "IFCWIDGETMEASURE", None),
        Err(ExactUnitError::MeasureNotInSchema {
            measure_type: "IFCWIDGETMEASURE".into(),
            schema: SchemaVersion::Ifc4,
        })
    );
    for measure in [
        "IFCMONETARYMEASURE",
        "IFCTHERMALCONDUCTIVITYMEASURE",
        "IFCCOMPOUNDPLANEANGLEMEASURE",
        "IFCSOUNDPRESSURELEVELMEASURE",
        "IFCDESCRIPTIVEMEASURE",
    ] {
        assert_eq!(
            exact_unit(&model, measure, None),
            Err(ExactUnitError::UnmappedMeasureType {
                measure_type: measure.into()
            }),
            "{measure}"
        );
    }
}

// ---- contradictions ---------------------------------------------------------

#[test]
fn an_explicit_unit_of_the_wrong_type_is_refused() {
    assert_eq!(
        exact_unit(&millimetre_project(), "IFCAREAMEASURE", Some(id(1))),
        Err(ExactUnitError::UnitTypeMismatch {
            unit: id(1),
            expected: "AREAUNIT".into(),
            found: "LENGTHUNIT".into(),
        })
    );
}

#[test]
fn a_unit_whose_name_contradicts_its_type_is_refused() {
    let model = ifc4("#1=IFCSIUNIT(*,.LENGTHUNIT.,$,.SQUARE_METRE.);");
    assert_eq!(
        exact_unit(&model, "IFCLENGTHMEASURE", Some(id(1))),
        Err(ExactUnitError::DimensionMismatch {
            unit: id(1),
            expected: L,
            found: [2, 0, 0, 0, 0, 0, 0],
        })
    );
}

#[test]
fn a_conversion_factor_in_the_wrong_dimension_is_refused() {
    let model = ifc4(
        "#1=IFCSIUNIT(*,.AREAUNIT.,$,.SQUARE_METRE.);
#2=IFCDIMENSIONALEXPONENTS(1,0,0,0,0,0,0);
#3=IFCMEASUREWITHUNIT(IFCAREAMEASURE(0.0254),#1);
#4=IFCCONVERSIONBASEDUNIT(#2,.LENGTHUNIT.,'INCH',#3);",
    );
    assert_eq!(
        exact_unit(&model, "IFCLENGTHMEASURE", Some(id(4))),
        Err(ExactUnitError::DimensionMismatch {
            unit: id(4),
            expected: L,
            found: [2, 0, 0, 0, 0, 0, 0],
        })
    );
}

// ---- chains, cycles and budgets --------------------------------------------

#[test]
fn a_conversion_chain_multiplies_through() {
    let model = ifc4(
        "#1=IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.);
#2=IFCDIMENSIONALEXPONENTS(1,0,0,0,0,0,0);
#3=IFCMEASUREWITHUNIT(IFCLENGTHMEASURE(0.0254),#1);
#4=IFCCONVERSIONBASEDUNIT(#2,.LENGTHUNIT.,'inch',#3);
#5=IFCMEASUREWITHUNIT(IFCLENGTHMEASURE(12.),#4);
#6=IFCCONVERSIONBASEDUNIT(#2,.LENGTHUNIT.,'foot',#5);",
    );
    assert_scale(
        &resolved(exact_unit(&model, "IFCLENGTHMEASURE", Some(id(6)))),
        0.3048,
    );
}

#[test]
fn a_conversion_cycle_is_reported_with_its_members() {
    let model = ifc4(
        "#2=IFCDIMENSIONALEXPONENTS(1,0,0,0,0,0,0);
#3=IFCMEASUREWITHUNIT(IFCLENGTHMEASURE(2.),#6);
#4=IFCCONVERSIONBASEDUNIT(#2,.LENGTHUNIT.,'a',#3);
#5=IFCMEASUREWITHUNIT(IFCLENGTHMEASURE(3.),#4);
#6=IFCCONVERSIONBASEDUNIT(#2,.LENGTHUNIT.,'b',#5);",
    );
    assert_eq!(
        exact_unit(&model, "IFCLENGTHMEASURE", Some(id(4))),
        Err(ExactUnitError::CyclicConversion {
            cycle: vec![id(4), id(6), id(4)]
        })
    );
}

#[test]
fn a_chain_deeper_than_the_budget_is_refused() {
    let depth = ifc_model::Budget::DEFAULT.max_depth + 1;
    let mut data = String::from(
        "#1=IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.);\n#2=IFCDIMENSIONALEXPONENTS(1,0,0,0,0,0,0);\n",
    );
    let mut previous = 1;
    for step in 0..depth as u64 {
        let measure = 10 + 2 * step;
        let unit = measure + 1;
        data.push_str(&format!(
            "#{measure}=IFCMEASUREWITHUNIT(IFCLENGTHMEASURE(1.),#{previous});\n\
             #{unit}=IFCCONVERSIONBASEDUNIT(#2,.LENGTHUNIT.,'u{step}',#{measure});\n"
        ));
        previous = unit;
    }
    let result = exact_unit(&ifc4(&data), "IFCLENGTHMEASURE", Some(id(previous)));
    assert!(
        matches!(result, Err(ExactUnitError::ConversionChainTooDeep { max_depth, .. })
            if max_depth == ifc_model::Budget::DEFAULT.max_depth),
        "{result:?}"
    );
}

// ---- derived units -----------------------------------------------------------

fn transmittance(length_prefix: &str) -> Model {
    ifc4(&format!(
        "#1=IFCSIUNIT(*,.POWERUNIT.,$,.WATT.);
#2=IFCSIUNIT(*,.LENGTHUNIT.,{length_prefix},.METRE.);
#3=IFCSIUNIT(*,.THERMODYNAMICTEMPERATUREUNIT.,$,.KELVIN.);
#4=IFCDERIVEDUNITELEMENT(#1,1);
#5=IFCDERIVEDUNITELEMENT(#2,-2);
#6=IFCDERIVEDUNITELEMENT(#3,-1);
#7=IFCDERIVEDUNIT((#4,#5,#6),.THERMALTRANSMITTANCEUNIT.,$);
#8=IFCUNITASSIGNMENT((#7));
#9=IFCPROJECT('p',$,'P',$,$,$,$,$,#8);"
    ))
}

#[test]
fn a_derived_unit_combines_its_elements() {
    let unit = resolved(exact_unit(
        &transmittance("$"),
        "IFCTHERMALTRANSMITTANCEMEASURE",
        None,
    ));
    assert_eq!(unit.unit, Some(id(7)));
    assert_eq!(unit.dimensions, [0, 1, -3, 0, -1, 0, 0]);
    assert_scale(&unit, 1.0);
    // W/mm²K is a million W/m²K.
    let milli = resolved(exact_unit(
        &transmittance(".MILLI."),
        "IFCTHERMALTRANSMITTANCEMEASURE",
        None,
    ));
    assert_scale(&milli, 1e6);
}

#[test]
fn a_derived_unit_with_an_offset_element_is_refused() {
    let model = ifc4(
        "#1=IFCSIUNIT(*,.POWERUNIT.,$,.WATT.);
#2=IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.);
#3=IFCSIUNIT(*,.THERMODYNAMICTEMPERATUREUNIT.,$,.DEGREE_CELSIUS.);
#4=IFCDERIVEDUNITELEMENT(#1,1);
#5=IFCDERIVEDUNITELEMENT(#2,-2);
#6=IFCDERIVEDUNITELEMENT(#3,-1);
#7=IFCDERIVEDUNIT((#4,#5,#6),.THERMALTRANSMITTANCEUNIT.,$);",
    );
    assert_eq!(
        exact_unit(&model, "IFCTHERMALTRANSMITTANCEMEASURE", Some(id(7))),
        Err(ExactUnitError::UnsupportedOffset { unit: id(3) })
    );
}

// ---- structure -----------------------------------------------------------------

#[test]
fn a_missing_or_malformed_unit_record_is_refused() {
    let model = ifc4("#1=IFCSIUNIT(*,.LENGTHUNIT.,$);");
    assert_eq!(
        exact_unit(&model, "IFCLENGTHMEASURE", Some(id(99))),
        Err(ExactUnitError::Structure(
            ExactPropertyError::MissingReference {
                from: id(99),
                to: id(99)
            }
        ))
    );
    assert!(matches!(
        exact_unit(&model, "IFCLENGTHMEASURE", Some(id(1))),
        Err(ExactUnitError::Structure(
            ExactPropertyError::MalformedEntitySlots { entity, .. }
        )) if entity == id(1)
    ));
}

#[test]
fn a_non_unit_is_refused() {
    let model = ifc4("#1=IFCDIMENSIONALEXPONENTS(1,0,0,0,0,0,0);");
    assert!(matches!(
        exact_unit(&model, "IFCLENGTHMEASURE", Some(id(1))),
        Err(ExactUnitError::UnsupportedUnit { unit, .. }) if unit == id(1)
    ));
}

// ---- release binding -------------------------------------------------------------

#[test]
fn an_ifc2x3_model_resolves_against_ifc2x3() {
    let model = parse(
        "IFC2X3",
        "#1=IFCSIUNIT(*,.LENGTHUNIT.,.MILLI.,.METRE.);
#2=IFCUNITASSIGNMENT((#1));
#3=IFCPROJECT('p',$,'P',$,$,$,$,$,#2);",
    );
    let unit = resolved(exact_unit(&model, "IFCLENGTHMEASURE", None));
    assert_scale(&unit, 1e-3);
    // IFC4-only measures are foreign to an IFC2X3 file.
    assert_eq!(
        exact_unit(&model, "IFCNONNEGATIVELENGTHMEASURE", None),
        Err(ExactUnitError::MeasureNotInSchema {
            measure_type: "IFCNONNEGATIVELENGTHMEASURE".into(),
            schema: SchemaVersion::Ifc2x3,
        })
    );
}

#[test]
fn an_ifc4_entity_in_an_ifc2x3_file_is_refused() {
    let model = parse(
        "IFC2X3",
        "#1=IFCSIUNIT(*,.THERMODYNAMICTEMPERATUREUNIT.,$,.KELVIN.);
#2=IFCDIMENSIONALEXPONENTS(0,0,0,0,1,0,0);
#3=IFCMEASUREWITHUNIT(IFCTHERMODYNAMICTEMPERATUREMEASURE(1.8),#1);
#4=IFCCONVERSIONBASEDUNITWITHOFFSET(#2,.THERMODYNAMICTEMPERATUREUNIT.,'F',#3,-459.67);",
    );
    assert!(matches!(
        exact_unit(&model, "IFCTHERMODYNAMICTEMPERATUREMEASURE", Some(id(4))),
        Err(ExactUnitError::Structure(ExactPropertyError::NotInSchema { entity, .. }))
            if entity == id(4)
    ));
}

/// IFC2X3 TC1's own tables disagree about the farad: `IfcDimensionsForSiUnit`
/// gives it current exponent 1, `IfcCorrectDimensions` requires mass +1 for
/// capacitance. IFC4 fixed both. Each release is read with its own tables.
#[test]
fn the_farad_is_read_with_each_releases_own_tables() {
    let data = "#1=IFCSIUNIT(*,.ELECTRICCAPACITANCEUNIT.,.MICRO.,.FARAD.);";
    let ifc4_unit = resolved(exact_unit(
        &parse("IFC4", data),
        "IFCELECTRICCAPACITANCEMEASURE",
        Some(id(1)),
    ));
    assert_eq!(ifc4_unit.dimensions, [-2, -1, 4, 2, 0, 0, 0]);
    assert_scale(&ifc4_unit, 1e-6);
    assert_eq!(
        exact_unit(
            &parse("IFC2X3", data),
            "IFCELECTRICCAPACITANCEMEASURE",
            Some(id(1))
        ),
        Err(ExactUnitError::DimensionMismatch {
            unit: id(1),
            expected: [-2, 1, 4, 1, 0, 0, 0],
            found: [-2, -1, 4, 1, 0, 0, 0],
        })
    );
}

// ---- the permissive view (#53 behaviour fixes) ----------------------------------

#[test]
fn the_permissive_view_scales_powers_and_does_not_invent_a_prefix() {
    let model = issue_fixture();
    let scale = |n| ifc_properties::unit(&model, id(n)).and_then(|kind| kind.si_scale());
    assert_eq!(scale(1), Some(1e-6), "mm² is (1e-3 m)²");
    assert_eq!(scale(3), Some(1.0));
    assert_eq!(scale(2), None, "an unknown prefix has no scale");
    match ifc_properties::unit(&model, id(2)) {
        Some(ifc_properties::UnitKind::Si {
            prefix,
            prefix_exponent,
            ..
        }) => {
            assert_eq!(prefix.as_deref(), Some("BOGUS"));
            assert_eq!(prefix_exponent, None);
        }
        other => panic!("expected an SI unit, got {other:?}"),
    }
}
