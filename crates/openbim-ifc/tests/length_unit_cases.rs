//! `convert_length_unit` case by case (#469): what is rescaled, what keeps
//! its unit, the tricky parameter spaces, georeferencing, the units written,
//! and every refusal.

#![cfg(all(feature = "step", feature = "properties", feature = "schema"))]

use ifc::properties::ConversionUnit;
use ifc::{
    convert_length_unit, stage_length_unit_conversion, Codec, ConvertedKind, EntityId, KeptReason,
    LengthUnitError, LengthUnitOptions, Model, RefusalReason, StepCodec, Value,
};

fn parse(schema: &str, data: &str) -> Model {
    let text = format!(
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
         FILE_NAME('','',(''),(''),'','','');\nFILE_SCHEMA(('{schema}'));\n\
         ENDSEC;\nDATA;\n{data}\nENDSEC;\nEND-ISO-10303-21;\n"
    );
    let model = StepCodec.read_bytes(text.as_bytes()).expect("parses");
    assert!(model.diagnostics().is_empty(), "{:?}", model.diagnostics());
    model
}

fn unit(spec: &str) -> ConversionUnit {
    ConversionUnit::parse(spec).unwrap()
}

fn real(model: &Model, id: u64, slot: usize) -> f64 {
    match &model.get(EntityId(id)).expect("entity").attributes[slot] {
        Value::Real(x) => *x,
        Value::Typed { value, .. } => value.as_f64().expect("number"),
        other => panic!("#{id} slot {slot}: {other:?}"),
    }
}

fn reals(model: &Model, id: u64, slot: usize) -> Vec<f64> {
    match &model.get(EntityId(id)).expect("entity").attributes[slot] {
        Value::List(items) => items
            .iter()
            .map(|v| v.unwrap_typed().as_f64().expect("number"))
            .collect(),
        other => panic!("#{id} slot {slot}: {other:?}"),
    }
}

/// Millimetres, square metres and cubic metres, the common authoring setup.
const UNITS: &str = "#1=IFCSIUNIT(*,.LENGTHUNIT.,.MILLI.,.METRE.);
#2=IFCSIUNIT(*,.AREAUNIT.,$,.SQUARE_METRE.);
#3=IFCSIUNIT(*,.VOLUMEUNIT.,$,.CUBIC_METRE.);
#4=IFCSIUNIT(*,.PLANEANGLEUNIT.,$,.RADIAN.);
#5=IFCUNITASSIGNMENT((#1,#2,#3,#4));
#6=IFCPROJECT('0000000000000000000006',$,'P',$,$,$,$,(#7),#5);
#7=IFCGEOMETRICREPRESENTATIONCONTEXT($,'Model',3,0.01,#8,$);
#8=IFCAXIS2PLACEMENT3D(#9,$,$);
#9=IFCCARTESIANPOINT((1000.,2000.,3000.));
#10=IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.);";

/// Geometry and values exercising every rule, IFC4.
fn ifc4_cases() -> Model {
    parse(
        "IFC4",
        &format!(
            "{UNITS}
#20=IFCDIRECTION((1.,0.,0.));
#21=IFCVECTOR(#20,500.);
#22=IFCLINE(#9,#21);
#23=IFCTRIMMEDCURVE(#22,(IFCPARAMETERVALUE(0.)),(IFCPARAMETERVALUE(2.)),.T.,.PARAMETER.);
#24=IFCCIRCLE(#8,250.);
#25=IFCTRIMMEDCURVE(#24,(IFCPARAMETERVALUE(0.)),(IFCPARAMETERVALUE(1.5)),.T.,.PARAMETER.);
#26=IFCPLANE(#8);
#27=IFCRECTANGULARTRIMMEDSURFACE(#26,0.,0.,1000.,500.,.T.,.T.);
#28=IFCCYLINDRICALSURFACE(#8,100.);
#29=IFCRECTANGULARTRIMMEDSURFACE(#28,0.,0.,3.,800.,.T.,.T.);
#30=IFCRECTANGLEPROFILEDEF(.AREA.,$,$,200.,400.);
#31=IFCEXTRUDEDAREASOLID(#30,#8,#20,3000.);
#32=IFCCARTESIANTRANSFORMATIONOPERATOR3D(#20,$,#9,2.,$);
#33=IFCQUANTITYLENGTH('Length',$,$,3000.,$);
#34=IFCQUANTITYAREA('Area',$,$,7.5,$);
#35=IFCQUANTITYVOLUME('Volume',$,$,1.5,$);
#36=IFCQUANTITYLENGTH('LengthInMetres',$,#10,3.,$);
#37=IFCPROPERTYSINGLEVALUE('Width',$,IFCPOSITIVELENGTHMEASURE(250.),$);
#38=IFCPROPERTYSINGLEVALUE('WidthInMetres',$,IFCPOSITIVELENGTHMEASURE(0.25),#10);
#39=IFCPROPERTYSINGLEVALUE('Ratio',$,IFCRATIOMEASURE(0.5),$);
#40=IFCMEASUREWITHUNIT(IFCLENGTHMEASURE(2.),#10);
#41=IFCCOLOURRGB($,1.,1.,1.);
#42=IFCLIGHTSOURCEPOSITIONAL($,#41,0.,1.,#9,10000.,1.,0.002,0.000004);
#43=IFCPROPERTYLISTVALUE('Lengths',$,(IFCLENGTHMEASURE(100.),IFCLENGTHMEASURE(200.)),$);
#44=IFCPROPERTYBOUNDEDVALUE('Span',$,IFCLENGTHMEASURE(9000.),IFCLENGTHMEASURE(1000.),$,$);"
        ),
    )
}

#[test]
fn millimetres_to_metres_rescales_lengths_and_keeps_the_rest() {
    let mut model = ifc4_cases();
    let report = convert_length_unit(&mut model, &unit("METRE"), &LengthUnitOptions::new())
        .expect("converts");
    assert!(report.applied);
    assert_eq!(report.length.factor(), 0.001);
    // Areas and volumes were already in m² and m³: their factor is 1.
    assert!(report.area.unwrap().is_identity());
    assert!(report.volume.unwrap().is_identity());

    // Points, radii, depths, profile dimensions, vector magnitudes.
    assert_eq!(reals(&model, 9, 0), [1.0, 2.0, 3.0]);
    assert_eq!(real(&model, 21, 1), 0.5);
    assert_eq!(real(&model, 24, 1), 0.25);
    assert_eq!(real(&model, 31, 3), 3.0);
    assert_eq!((real(&model, 30, 3), real(&model, 30, 4)), (0.2, 0.4));
    // Precision is a length stated as a REAL.
    assert_eq!(real(&model, 7, 3), 0.00001);
    // Unitless: the transformation scale, a line's parameter (a multiple of
    // its rescaled vector), a circle's (an angle).
    assert_eq!(real(&model, 32, 3), 2.0);
    assert_eq!(
        model.get(EntityId(23)).unwrap().attributes[2],
        Value::List(vec![Value::Typed {
            type_name: "IFCPARAMETERVALUE".into(),
            value: Box::new(Value::Real(2.0))
        }])
    );
    assert_eq!(
        model.get(EntityId(25)).unwrap().attributes[2],
        Value::List(vec![Value::Typed {
            type_name: "IFCPARAMETERVALUE".into(),
            value: Box::new(Value::Real(1.5))
        }])
    );
    // A plane's parameters are lengths; a cylinder's u is an angle, v a length.
    assert_eq!(
        [1, 2, 3, 4].map(|slot| real(&model, 27, slot)),
        [0.0, 0.0, 1.0, 0.5]
    );
    assert_eq!(
        [1, 2, 3, 4].map(|slot| real(&model, 29, slot)),
        [0.0, 0.0, 3.0, 0.8]
    );
    // Quantities: the length rescales, area and volume keep their m² and
    // m³ values, and one with its own unit keeps it.
    assert_eq!(real(&model, 33, 3), 3.0);
    assert_eq!(real(&model, 34, 3), 7.5);
    assert_eq!(real(&model, 35, 3), 1.5);
    assert_eq!(real(&model, 36, 3), 3.0);
    // Properties: typed measures in a SELECT, with and without a unit.
    assert_eq!(real(&model, 37, 2), 0.25);
    assert_eq!(real(&model, 38, 2), 0.25);
    assert_eq!(real(&model, 39, 2), 0.5);
    assert_eq!(real(&model, 40, 0), 2.0);
    assert_eq!(reals(&model, 43, 2), [0.1, 0.2]);
    assert_eq!((real(&model, 44, 2), real(&model, 44, 3)), (9.0, 1.0));
    // A light's radius is a length; its attenuations scale inversely.
    assert_eq!(real(&model, 42, 5), 10.0);
    assert_eq!(real(&model, 42, 6), 1.0);
    assert_eq!(real(&model, 42, 7), 2.0);
    assert!((real(&model, 42, 8) - 4.0).abs() < 1e-12);

    let converted = |entity: &str, attribute: &str| {
        report
            .converted
            .iter()
            .find(|c| c.entity_type.as_ref() == entity && c.attribute.as_ref() == attribute)
            .unwrap_or_else(|| panic!("{entity}.{attribute} in {:#?}", report.converted))
    };
    assert_eq!(converted("IFCCARTESIANPOINT", "Coordinates").values, 3);
    assert_eq!(
        converted("IFCLIGHTSOURCEPOSITIONAL", "QuadricAttenuation").kind,
        ConvertedKind::InverseArea
    );
    let kept = |reason: KeptReason| {
        report
            .kept
            .iter()
            .filter(|k| k.reason == reason)
            .map(|k| k.values)
            .sum::<usize>()
    };
    assert_eq!(kept(KeptReason::ExplicitUnit), 3);
    assert_eq!(kept(KeptReason::UnitlessParameter), 6);
    // The new unit is the existing unprefixed METRE; the millimetre goes.
    assert_eq!(report.length_unit, Some(EntityId(10)));
    assert!(model.get(EntityId(1)).is_none());
    assert_eq!(
        model.get(EntityId(5)).unwrap().attributes[0],
        Value::List(vec![
            Value::Ref(EntityId(10)),
            Value::Ref(EntityId(2)),
            Value::Ref(EntityId(3)),
            Value::Ref(EntityId(4)),
        ])
    );
}

#[test]
fn areas_and_volumes_follow_the_length_unit_by_their_exact_ratio() {
    let data = UNITS
        .replace(
            ".AREAUNIT.,$,.SQUARE_METRE.",
            ".AREAUNIT.,.MILLI.,.SQUARE_METRE.",
        )
        .replace(
            ".VOLUMEUNIT.,$,.CUBIC_METRE.",
            ".VOLUMEUNIT.,.MILLI.,.CUBIC_METRE.",
        );
    let model = parse(
        "IFC2X3",
        &format!(
            "{data}
#34=IFCQUANTITYAREA('Area',$,$,7500000.);
#35=IFCQUANTITYVOLUME('Volume',$,$,1500000000.);"
        ),
    );
    let mut followed = model.clone();
    let report = convert_length_unit(&mut followed, &unit("METRE"), &LengthUnitOptions::new())
        .expect("converts");
    assert_eq!(report.area.unwrap().factor(), 1e-6);
    assert_eq!(real(&followed, 34, 3), 7.5);
    assert_eq!(real(&followed, 35, 3), 1.5);
    // Without following, areas and volumes keep their units and values.
    let mut kept = model.clone();
    let report = convert_length_unit(
        &mut kept,
        &unit("METRE"),
        &LengthUnitOptions::new().area_and_volume(false),
    )
    .expect("converts");
    assert!(report.area.is_none());
    assert_eq!(real(&kept, 34, 3), 7_500_000.0);
    assert!(kept.get(EntityId(2)).is_some());
}

#[test]
fn a_named_target_is_written_as_a_conversion_based_unit_over_the_metre() {
    let mut model = ifc4_cases();
    let report = convert_length_unit(&mut model, &unit("foot"), &LengthUnitOptions::new())
        .expect("converts");
    let foot = model.get(report.length_unit.unwrap()).unwrap();
    assert_eq!(foot.type_name.as_ref(), "IFCCONVERSIONBASEDUNIT");
    assert_eq!(foot.attributes[2], Value::Text("foot".into()));
    let Value::Ref(measure) = foot.attributes[3] else {
        panic!("{foot:?}")
    };
    let measure = model.get(measure).unwrap();
    assert_eq!(
        measure.attributes,
        vec![
            Value::Typed {
                type_name: "IFCLENGTHMEASURE".into(),
                value: Box::new(Value::Real(0.3048))
            },
            // The model's own unprefixed metre.
            Value::Ref(EntityId(10)),
        ]
    );
    let area = model.get(report.area_unit.unwrap()).unwrap();
    assert_eq!(area.attributes[2], Value::Text("square foot".into()));
    // 3000 mm is 9.8425... ft; reading the units back gives 3 m.
    let back = ifc::properties::resolve_unit(&model, report.length_unit.unwrap()).unwrap();
    assert!((real(&model, 31, 3) * back.scale() - 3.0).abs() < 1e-12);
    // 7.5 m² in square feet.
    assert!((real(&model, 34, 3) - 7.5 / 0.09290304).abs() < 1e-9);
    #[cfg(feature = "validate")]
    {
        let report = ifc::validate::validate_declared(&model).unwrap();
        let errors: Vec<_> = report
            .findings()
            .iter()
            .filter(|f| f.rule.contains("Unit") || f.message.contains("UNIT"))
            .collect();
        assert!(errors.is_empty(), "{errors:#?}");
    }
}

#[test]
fn staging_changes_nothing_and_an_unchanged_unit_stages_nothing() {
    let model = ifc4_cases();
    let staged =
        stage_length_unit_conversion(&model, &unit("METRE"), &LengthUnitOptions::new()).unwrap();
    assert!(!staged.transaction.is_empty());
    assert!(!staged.report.applied);
    assert_eq!(staged.report.removed, [EntityId(1)]);
    assert_eq!(reals(&model, 9, 0), [1000.0, 2000.0, 3000.0]);
    let mut same = model.clone();
    let report =
        convert_length_unit(&mut same, &unit("MILLI METRE"), &LengthUnitOptions::new()).unwrap();
    assert!(!report.applied);
    assert_eq!(report.values_converted, 0);
    assert_eq!(same.revision(), model.revision());
}

#[test]
fn targets_that_are_not_lengths_are_refused() {
    let mut model = ifc4_cases();
    for spec in ["SQUARE_METRE", "KILOGRAM", "DEGREE_CELSIUS"] {
        assert!(
            matches!(
                convert_length_unit(&mut model, &unit(spec), &LengthUnitOptions::new()),
                Err(LengthUnitError::Target { .. })
            ),
            "{spec}"
        );
    }
}

fn refusals(model: &Model) -> Vec<RefusalReason> {
    let mut copy = model.clone();
    match convert_length_unit(&mut copy, &unit("METRE"), &LengthUnitOptions::new()) {
        Err(LengthUnitError::Refused(refusals)) => {
            assert_eq!(copy.revision(), model.revision(), "nothing was written");
            refusals.into_iter().map(|r| r.reason).collect()
        }
        other => panic!("expected a refusal, got {other:?}"),
    }
}

#[test]
fn undefined_parameter_spaces_and_unitless_reals_are_refused_by_name() {
    // A p-curve in a cylinder's (angle, length) space.
    let pcurve = parse(
        "IFC4",
        &format!(
            "{UNITS}
#20=IFCCYLINDRICALSURFACE(#8,100.);
#21=IFCCARTESIANPOINT((0.,0.));
#22=IFCCARTESIANPOINT((1.5,500.));
#23=IFCPOLYLINE((#21,#22));
#24=IFCPCURVE(#20,#23);"
        ),
    );
    assert!(matches!(
        refusals(&pcurve).as_slice(),
        [RefusalReason::ParameterSpaceCurve { surface_type, .. }] if surface_type.as_ref() == "IFCCYLINDRICALSURFACE"
    ));
    // A swept disk whose directrix has no parameterisation this conversion
    // carries.
    let swept = parse(
        "IFC4",
        &format!(
            "{UNITS}
#20=IFCPCURVE(#21,#22);
#21=IFCPLANE(#8);
#22=IFCPOLYLINE((#9,#9));
#23=IFCSWEPTDISKSOLID(#20,10.,$,0.,1.);"
        ),
    );
    let swept = refusals(&swept);
    // StartParam and EndParam, each named.
    assert_eq!(swept.len(), 2, "{swept:?}");
    assert!(swept.iter().all(|reason| matches!(
        reason,
        RefusalReason::ParameterSpace { host_type, .. } if host_type.as_ref() == "IFCPCURVE"
    )));
    // An untyped number in an IfcValue select says no measure.
    let untyped = parse(
        "IFC4",
        &format!("{UNITS}\n#20=IFCPROPERTYSINGLEVALUE('Width',$,250.,$);"),
    );
    assert!(matches!(
        refusals(&untyped).as_slice(),
        [RefusalReason::UntypedSelectValue { .. }]
    ));
}

#[test]
fn ifc4x3_curve_segments_polynomials_and_project_libraries_are_refused() {
    let units = UNITS.replace(
        "#6=IFCPROJECT('0000000000000000000006',$,'P',$,$,$,$,(#7),#5);",
        "#6=IFCPROJECT('0000000000000000000006',$,'P',$,$,$,$,(#7),#5);\n\
         #11=IFCPROJECTLIBRARY('0000000000000000000011',$,'L',$,$,$,$,$,#5);",
    );
    let model = parse(
        "IFC4X3_ADD2",
        &format!(
            "{units}
#20=IFCAXIS2PLACEMENT2D(#21,$);
#21=IFCCARTESIANPOINT((0.,0.));
#22=IFCCIRCLE(#20,100.);
#23=IFCCURVESEGMENT(.CONTINUOUS.,#20,IFCPARAMETERVALUE(0.),IFCPARAMETERVALUE(1.),#22);
#24=IFCCURVESEGMENT(.CONTINUOUS.,#20,IFCLENGTHMEASURE(0.),IFCLENGTHMEASURE(100.),#22);
#25=IFCPOLYNOMIALCURVE(#20,(0.,1.),(0.,0.,1.),$);"
        ),
    );
    let reasons = refusals(&model);
    assert!(
        reasons.contains(&RefusalReason::ContextUnits),
        "{reasons:?}"
    );
    assert!(
        reasons.contains(&RefusalReason::PolynomialCoefficients),
        "{reasons:?}"
    );
    assert_eq!(
        reasons
            .iter()
            .filter(|r| matches!(r, RefusalReason::ParameterSpace { host_type, .. } if host_type.as_ref() == "IFCCURVESEGMENT"))
            .count(),
        2,
        "{reasons:?}"
    );
}

#[test]
fn values_with_units_of_their_own_keep_them_through_tables_and_series() {
    let model = parse(
        "IFC4",
        &format!(
            "{UNITS}
#20=IFCTABLECOLUMN($,'A',$,#10,$);
#21=IFCTABLECOLUMN($,'B',$,$,$);
#22=IFCTABLEROW((IFCLENGTHMEASURE(3.),IFCLENGTHMEASURE(3000.)),$);
#23=IFCTABLE('T',(#22),(#20,#21));
#24=IFCTIMESERIESVALUE((IFCLENGTHMEASURE(4.)));
#25=IFCREGULARTIMESERIES('S',$,'2026-01-01T00:00:00','2026-01-02T00:00:00',.CONTINUOUS.,.MEASURED.,$,#10,60.,(#24));
#26=IFCPROPERTYENUMERATION('E',(IFCLENGTHMEASURE(1.)),#10);
#27=IFCPROPERTYENUMERATEDVALUE('V',$,(IFCLENGTHMEASURE(1.)),#26);"
        ),
    );
    let mut converted = model.clone();
    convert_length_unit(&mut converted, &unit("METRE"), &LengthUnitOptions::new()).unwrap();
    let cells = reals(&converted, 22, 0);
    assert_eq!(cells, [3.0, 3.0]);
    assert_eq!(reals(&converted, 24, 0), [4.0]);
    assert_eq!(reals(&converted, 26, 1), [1.0]);
    assert_eq!(reals(&converted, 27, 2), [1.0]);
}

#[cfg(feature = "georef")]
#[test]
fn georeferencing_resolves_to_the_same_map_transform() {
    for fixture in [
        "synthetic-georef/map_conversion_ifc4.ifc",
        "synthetic-georef/epset_project_ifc2x3.ifc",
        "synthetic-georef/epset_sites_ifc2x3.ifc",
    ] {
        let path = format!(
            "{}/../../test/fixtures/{fixture}",
            env!("CARGO_MANIFEST_DIR")
        );
        let original = StepCodec.read_bytes(&std::fs::read(path).unwrap()).unwrap();
        for target in ["METRE", "foot", "CENTIMETRE"] {
            let mut model = original.clone();
            let report = convert_length_unit(&mut model, &unit(target), &LengthUnitOptions::new())
                .unwrap_or_else(|error| panic!("{fixture} to {target}: {error}"));
            assert!(
                report
                    .kept
                    .iter()
                    .any(|kept| kept.reason == KeptReason::MapUnit),
                "{fixture}: {report:#?}"
            );
            let before = ifc::georeferencing(&original).unwrap();
            let after = ifc::georeferencing(&model).unwrap();
            assert_eq!(before.len(), after.len(), "{fixture}");
            for (a, b) in before.iter().zip(&after) {
                for point in [[0.0, 0.0, 0.0], [10.0, -20.0, 3.0]] {
                    let (pa, pb) = (a.map_point(point), b.map_point(point));
                    for axis in 0..3 {
                        assert!(
                            (pa[axis] - pb[axis]).abs() < 1e-6,
                            "{fixture} to {target}: {pa:?} became {pb:?}"
                        );
                    }
                }
            }
        }
    }
}
