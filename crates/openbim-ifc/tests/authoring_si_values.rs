//! Attribute values of authoring batches read in SI (#638): what converts
//! into which unit, what is kept, and every refusal. Placements, the unit
//! guard and the batch-level refusals are in `authoring_si_input.rs`.
//!
//! The project units are millimetres (square metres) or feet, by fixture; #6
//! is an explicit millimetre unit in both.

#![cfg(all(
    feature = "step",
    feature = "authoring",
    feature = "properties",
    feature = "ifc4"
))]

use std::f64::consts::{FRAC_PI_2, PI};

use ifc::properties::{ExactUnitError, UnitConversionError};
use ifc::{
    apply_authoring, apply_authoring_with, authoring_handle, AuthorOp, AuthoringError,
    AuthoringFailure, AuthoringOptions, Codec, EntityId, InputUnits, Model, SiInputRefusal,
    StepCodec, Value,
};

const COMMON: &str = "#1=IFCPERSON($,'T',$,$,$,$,$,$);
#2=IFCORGANIZATION($,'T',$,$,$);
#3=IFCPERSONANDORGANIZATION(#1,#2,$);
#4=IFCAPPLICATION(#2,'1','t','t');
#5=IFCOWNERHISTORY(#3,#4,$,.NOCHANGE.,$,$,$,0);
#6=IFCSIUNIT(*,.LENGTHUNIT.,.MILLI.,.METRE.);
#7=IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.);
{UNITS}
#21=IFCPROJECT('0WtIqeFgfBFQdcY1sx7FNt',#5,'P',$,$,$,$,$,#20);
#30=IFCCARTESIANPOINT((0.,0.,0.));
#31=IFCAXIS2PLACEMENT3D(#30,$,$);
#26=IFCPROPERTYENUMERATION('Sizes',(IFCLENGTHMEASURE(100.)),#6);
#27=IFCPROPERTYSINGLEVALUE('P',$,IFCLENGTHMEASURE(1.),#6);";

/// Millimetres and degrees Celsius.
const MM_CELSIUS: &str = "#8=IFCSIUNIT(*,.AREAUNIT.,.MILLI.,.SQUARE_METRE.);
#9=IFCSIUNIT(*,.THERMODYNAMICTEMPERATUREUNIT.,$,.DEGREE_CELSIUS.);
#10=IFCSIUNIT(*,.PLANEANGLEUNIT.,$,.RADIAN.);
#20=IFCUNITASSIGNMENT((#6,#8,#9,#10));";

/// Millimetre length, square millimetre area, radian.
const MM: &str = "#8=IFCSIUNIT(*,.AREAUNIT.,.MILLI.,.SQUARE_METRE.);
#10=IFCSIUNIT(*,.PLANEANGLEUNIT.,$,.RADIAN.);
#20=IFCUNITASSIGNMENT((#6,#8,#10));";

/// Foot, square foot and degree, over metre, square metre and radian.
const FOOT: &str = "#8=IFCDIMENSIONALEXPONENTS(1,0,0,0,0,0,0);
#9=IFCMEASUREWITHUNIT(IFCLENGTHMEASURE(0.3048),#7);
#10=IFCCONVERSIONBASEDUNIT(#8,.LENGTHUNIT.,'foot',#9);
#11=IFCSIUNIT(*,.AREAUNIT.,$,.SQUARE_METRE.);
#12=IFCDIMENSIONALEXPONENTS(2,0,0,0,0,0,0);
#13=IFCMEASUREWITHUNIT(IFCAREAMEASURE(0.09290304),#11);
#14=IFCCONVERSIONBASEDUNIT(#12,.AREAUNIT.,'square foot',#13);
#15=IFCSIUNIT(*,.PLANEANGLEUNIT.,$,.RADIAN.);
#16=IFCDIMENSIONALEXPONENTS(0,0,0,0,0,0,0);
#17=IFCMEASUREWITHUNIT(IFCPLANEANGLEMEASURE(0.017453292519943295),#15);
#18=IFCCONVERSIONBASEDUNIT(#16,.PLANEANGLEUNIT.,'degree',#17);
#20=IFCUNITASSIGNMENT((#10,#14,#18));";

fn model(units: &str) -> Model {
    let data = COMMON.replace("{UNITS}", units);
    let text = format!(
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
         FILE_NAME('','',(''),(''),'','','');\nFILE_SCHEMA(('IFC4'));\n\
         ENDSEC;\nDATA;\n{data}\nENDSEC;\nEND-ISO-10303-21;\n"
    );
    let model = StepCodec
        .read_bytes(text.as_bytes())
        .expect("fixture parses");
    assert!(model.diagnostics().is_empty(), "{:?}", model.diagnostics());
    model
}

fn si() -> AuthoringOptions {
    AuthoringOptions::default().with_input_units(InputUnits::Si)
}

fn typed(type_name: &str, number: f64) -> Value {
    Value::Typed {
        type_name: type_name.into(),
        value: Box::new(Value::Real(number)),
    }
}

fn create(type_name: &str, attributes: &[(&str, Value)]) -> AuthorOp {
    AuthorOp::Create {
        type_name: type_name.into(),
        attributes: attributes
            .iter()
            .map(|(name, value)| ((*name).to_owned(), value.clone()))
            .collect(),
    }
}

/// Run `ops` in SI on `model`; the entity operation `op` produced.
fn run(model: &mut Model, ops: &[AuthorOp], op: usize) -> EntityId {
    let outcome =
        apply_authoring_with(model, ifc::schema::ifc4(), ops, 1, &si()).expect("accepted in SI");
    outcome.ids[op].expect("produced an entity")
}

/// The value of attribute `name` of `id`.
fn attr(model: &Model, id: EntityId, name: &str) -> Value {
    let entity = model.get(id).expect("exists");
    let slot = ifc::schema::ifc4()
        .attributes(&entity.type_name)
        .iter()
        .position(|attribute| attribute.name.eq_ignore_ascii_case(name))
        .expect("declared");
    entity.attributes[slot].clone()
}

fn number(value: &Value) -> f64 {
    value.unwrap_typed().as_f64().expect("a number")
}

fn near(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= 1e-9 * expected.abs().max(1.0),
        "{actual} is not {expected}"
    );
}

/// `ops` refused in SI, leaving the model as it was.
fn refused(model: &Model, ops: &[AuthorOp]) -> AuthoringFailure {
    let mut copy = model.clone();
    let (revision, len) = (copy.revision(), copy.len());
    let error: AuthoringError = apply_authoring_with(&mut copy, ifc::schema::ifc4(), ops, 1, &si())
        .expect_err("refused in SI");
    assert_eq!((copy.revision(), copy.len()), (revision, len));
    *error.failure
}

fn si_refusal(failure: &AuthoringFailure) -> &SiInputRefusal {
    match failure {
        AuthoringFailure::SiInput { reason, .. } => reason,
        other => panic!("{other:?}"),
    }
}

fn rectangle(x: f64, y: f64) -> AuthorOp {
    create(
        "IfcRectangleProfileDef",
        &[
            ("ProfileType", Value::Enum("AREA".into())),
            ("XDim", Value::Real(x)),
            ("YDim", Value::Real(y)),
        ],
    )
}

#[test]
fn profile_dimensions_are_lengths_in_the_project_unit() {
    let mut mm = model(MM);
    let id = run(&mut mm, &[rectangle(0.2, 0.4)], 0);
    near(number(&attr(&mm, id, "XDim")), 200.0);
    near(number(&attr(&mm, id, "YDim")), 400.0);

    let mut foot = model(FOOT);
    let id = run(&mut foot, &[rectangle(0.3048, 1.0)], 0);
    near(number(&attr(&foot, id, "XDim")), 1.0);
    near(number(&attr(&foot, id, "YDim")), 1.0 / 0.3048);

    // Raw writes the numbers as given.
    let mut raw = model(MM);
    let outcome =
        apply_authoring(&mut raw, ifc::schema::ifc4(), &[rectangle(0.2, 0.4)], 1).unwrap();
    near(number(&attr(&raw, outcome.ids[0].unwrap(), "XDim")), 0.2);
}

#[test]
fn elevations_are_converted_and_a_compound_angle_is_not() {
    let mut mm = model(MM);
    let ops = [
        AuthorOp::Spatial {
            type_name: "IfcSite".into(),
            parent: EntityId(21),
            attributes: vec![
                ("RefElevation".into(), Value::Real(2.0)),
                (
                    "RefLatitude".into(),
                    Value::List(vec![
                        Value::Integer(50),
                        Value::Integer(30),
                        Value::Integer(15),
                    ]),
                ),
            ],
            placement: None,
            owner_history: None,
        },
        AuthorOp::Spatial {
            type_name: "IfcBuildingStorey".into(),
            parent: authoring_handle(0),
            attributes: vec![("Elevation".into(), Value::Real(3.0))],
            placement: None,
            owner_history: None,
        },
    ];
    let outcome = apply_authoring_with(&mut mm, ifc::schema::ifc4(), &ops, 1, &si()).unwrap();
    let site = outcome.ids[0].unwrap();
    let storey = outcome.ids[1].unwrap();
    near(number(&attr(&mm, site, "RefElevation")), 2000.0);
    assert_eq!(
        attr(&mm, site, "RefLatitude"),
        Value::List(vec![
            Value::Integer(50),
            Value::Integer(30),
            Value::Integer(15)
        ])
    );
    near(number(&attr(&mm, storey, "Elevation")), 3000.0);
}

fn single_value(nominal: Value, unit: Option<u64>) -> AuthorOp {
    let mut attributes = vec![("Name", Value::Text("P".into())), ("NominalValue", nominal)];
    if let Some(unit) = unit {
        attributes.push(("Unit", Value::Ref(EntityId(unit))));
    }
    create("IfcPropertySingleValue", &attributes)
}

#[test]
fn a_property_value_goes_into_the_unit_that_applies() {
    // No unit of its own: the project unit.
    let mut mm = model(MM);
    let id = run(
        &mut mm,
        &[single_value(typed("IFCLENGTHMEASURE", 2.5), None)],
        0,
    );
    near(number(&attr(&mm, id, "NominalValue")), 2500.0);
    // The typed wrapper survives.
    assert!(matches!(
        attr(&mm, id, "NominalValue"),
        Value::Typed { ref type_name, .. } if &**type_name == "IFCLENGTHMEASURE"
    ));

    // An explicit unit wins over the project's: millimetres on a foot model.
    let mut foot = model(FOOT);
    let id = run(
        &mut foot,
        &[single_value(typed("IFCLENGTHMEASURE", 2.5), Some(6))],
        0,
    );
    near(number(&attr(&foot, id, "NominalValue")), 2500.0);
    let id = run(
        &mut foot,
        &[single_value(typed("IFCLENGTHMEASURE", 1.0), None)],
        0,
    );
    near(number(&attr(&foot, id, "NominalValue")), 1.0 / 0.3048);
}

#[test]
fn angles_and_areas_follow_their_own_project_units() {
    let mut foot = model(FOOT);
    let id = run(
        &mut foot,
        &[single_value(typed("IFCPLANEANGLEMEASURE", FRAC_PI_2), None)],
        0,
    );
    near(number(&attr(&foot, id, "NominalValue")), 90.0);

    let mut mm = model(MM);
    let id = run(
        &mut mm,
        &[single_value(typed("IFCAREAMEASURE", 1.0), None)],
        0,
    );
    near(number(&attr(&mm, id, "NominalValue")), 1.0e6);
    // A dimensionless measure is kept.
    let id = run(
        &mut mm,
        &[single_value(typed("IFCRATIOMEASURE", 0.25), None)],
        0,
    );
    near(number(&attr(&mm, id, "NominalValue")), 0.25);
}

#[test]
fn a_quantity_converts_by_its_measure_and_a_count_is_kept() {
    let mut mm = model(MM);
    let length = create(
        "IfcQuantityLength",
        &[
            ("Name", Value::Text("Width".into())),
            ("LengthValue", Value::Real(1.5)),
        ],
    );
    let id = run(&mut mm, &[length], 0);
    near(number(&attr(&mm, id, "LengthValue")), 1500.0);
    let count = create(
        "IfcQuantityCount",
        &[
            ("Name", Value::Text("Doors".into())),
            ("CountValue", Value::Real(3.0)),
        ],
    );
    let id = run(&mut mm, &[count], 0);
    near(number(&attr(&mm, id, "CountValue")), 3.0);
}

#[test]
fn the_context_precision_is_a_length() {
    let mut mm = model(MM);
    let context = create(
        "IfcGeometricRepresentationContext",
        &[
            ("ContextType", Value::Text("Model".into())),
            ("CoordinateSpaceDimension", Value::Integer(3)),
            ("Precision", Value::Real(1.0e-5)),
            ("WorldCoordinateSystem", Value::Ref(EntityId(31))),
        ],
    );
    let id = run(&mut mm, &[context], 0);
    near(number(&attr(&mm, id, "Precision")), 0.01);
}

fn circle() -> AuthorOp {
    create(
        "IfcCircle",
        &[
            ("Position", Value::Ref(EntityId(31))),
            ("Radius", Value::Real(1.0)),
        ],
    )
}

fn trim(basis: EntityId, from: f64, to: f64) -> AuthorOp {
    create(
        "IfcTrimmedCurve",
        &[
            ("BasisCurve", Value::Ref(basis)),
            ("Trim1", Value::List(vec![typed("IFCPARAMETERVALUE", from)])),
            ("Trim2", Value::List(vec![typed("IFCPARAMETERVALUE", to)])),
            ("SenseAgreement", Value::Bool(true)),
            ("MasterRepresentation", Value::Enum("PARAMETER".into())),
        ],
    )
}

fn trims(model: &Model, id: EntityId) -> (f64, f64) {
    let first = |name| match attr(model, id, name) {
        Value::List(items) => number(&items[0]),
        other => panic!("{other:?}"),
    };
    (first("Trim1"), first("Trim2"))
}

#[test]
fn a_trim_follows_its_curves_parameter_space() {
    // A circle's parameter is an angle: degrees on a foot/degree model.
    let mut foot = model(FOOT);
    let ops = [circle(), trim(authoring_handle(0), FRAC_PI_2, PI)];
    let id = run(&mut foot, &ops, 1);
    let (from, to) = trims(&foot, id);
    near(from, 90.0);
    near(to, 180.0);
    let radius = attr(&foot, EntityId(foot.next_id().0 - 2), "Radius");
    near(number(&radius), 1.0 / 0.3048);

    // The same trim on a millimetre/radian model is unchanged.
    let mut mm = model(MM);
    let id = run(&mut mm, &ops, 1);
    let (from, to) = trims(&mm, id);
    near(from, FRAC_PI_2);
    near(to, PI);

    // A line's parameter is a multiple of its vector: unitless.
    let line = [
        create(
            "IfcDirection",
            &[(
                "DirectionRatios",
                Value::List(vec![Value::Real(1.0), Value::Real(0.0), Value::Real(0.0)]),
            )],
        ),
        create(
            "IfcVector",
            &[
                ("Orientation", Value::Ref(authoring_handle(0))),
                ("Magnitude", Value::Real(1.0)),
            ],
        ),
        create(
            "IfcLine",
            &[
                ("Pnt", Value::Ref(EntityId(30))),
                ("Dir", Value::Ref(authoring_handle(1))),
            ],
        ),
        trim(authoring_handle(2), 0.25, 0.75),
    ];
    let id = run(&mut foot, &line, 3);
    let (from, to) = trims(&foot, id);
    near(from, 0.25);
    near(to, 0.75);
}

#[test]
fn a_trim_on_a_composite_of_arcs_is_refused() {
    let ops = [
        circle(),
        create(
            "IfcCompositeCurveSegment",
            &[
                ("Transition", Value::Enum("CONTINUOUS".into())),
                ("SameSense", Value::Bool(true)),
                ("ParentCurve", Value::Ref(authoring_handle(0))),
            ],
        ),
        create(
            "IfcCompositeCurve",
            &[
                (
                    "Segments",
                    Value::List(vec![Value::Ref(authoring_handle(1))]),
                ),
                ("SelfIntersect", Value::Bool(false)),
            ],
        ),
        trim(authoring_handle(2), 0.0, 1.0),
    ];
    let failure = refused(&model(MM), &ops);
    assert!(matches!(
        si_refusal(&failure),
        SiInputRefusal::ParameterSpace { .. }
    ));
}

#[test]
fn values_that_cannot_be_carried_are_refused() {
    let mm = model(MM);

    // A texture generator's parameters depend on the X3D mode.
    let texture = create(
        "IfcTextureCoordinateGenerator",
        &[
            ("Mode", Value::Text("COORD".into())),
            ("Parameter", Value::List(vec![typed("IFCREAL", 1.0)])),
        ],
    );
    assert!(matches!(
        si_refusal(&refused(&mm, &[texture])),
        SiInputRefusal::TextureGeneratorParameters
    ));

    // A map conversion's numbers are in the CRS's units.
    let map = create("IfcMapConversion", &[("Eastings", Value::Real(1.0))]);
    assert!(matches!(
        si_refusal(&refused(&mm, &[map])),
        SiInputRefusal::MapConversion
    ));

    // A unit another record states.
    let series = create(
        "IfcTimeSeriesValue",
        &[(
            "ListValues",
            Value::List(vec![typed("IFCLENGTHMEASURE", 1.0)]),
        )],
    );
    assert!(matches!(
        si_refusal(&refused(&mm, &[series])),
        SiInputRefusal::UnitStatedElsewhere
    ));

    // An untyped number where the select admits several numeric types.
    let untyped = single_value(Value::Real(1.0), None);
    assert!(matches!(
        si_refusal(&refused(&mm, &[untyped])),
        SiInputRefusal::UntypedSelectValue { .. }
    ));

    // A unit the batch creates.
    let ops = [
        create(
            "IfcSIUnit",
            &[
                ("UnitType", Value::Enum("LENGTHUNIT".into())),
                ("Name", Value::Enum("METRE".into())),
            ],
        ),
        {
            let AuthorOp::Create { attributes, .. } =
                single_value(typed("IFCLENGTHMEASURE", 1.0), None)
            else {
                unreachable!()
            };
            let mut attributes = attributes;
            attributes.push(("Unit".into(), Value::Ref(authoring_handle(0))));
            AuthorOp::Create {
                type_name: "IfcPropertySingleValue".into(),
                attributes,
            }
        },
    ];
    assert!(matches!(
        si_refusal(&refused(&mm, &ops)),
        SiInputRefusal::UnitCreatedInBatch { .. }
    ));
}

#[test]
fn a_non_finite_number_or_a_missing_project_unit_refuses_the_value() {
    let failure = refused(&model(MM), &[rectangle(f64::NAN, 1.0)]);
    let AuthoringFailure::UnitConversion {
        error, attribute, ..
    } = failure
    else {
        panic!("{failure:?}");
    };
    assert_eq!(attribute, "XDim");
    assert!(matches!(*error, UnitConversionError::NonFinite { .. }));

    let no_length = "#10=IFCSIUNIT(*,.PLANEANGLEUNIT.,$,.RADIAN.);
#20=IFCUNITASSIGNMENT((#10));";
    let failure = refused(&model(no_length), &[rectangle(1.0, 1.0)]);
    let AuthoringFailure::UnitConversion { error, .. } = failure else {
        panic!("{failure:?}");
    };
    assert!(matches!(
        *error,
        UnitConversionError::Resolve(ExactUnitError::NoProjectUnit { .. })
    ));
}

#[test]
fn an_edit_converts_by_the_edited_records_type() {
    let mut mm = model(MM);
    let created = run(&mut mm, &[rectangle(0.2, 0.4)], 0);
    let edit = AuthorOp::Edit {
        entity: created,
        attributes: vec![("XDim".into(), Value::Real(0.5))],
    };
    run(&mut mm, &[edit], 0);
    near(number(&attr(&mm, created, "XDim")), 500.0);
    // Untouched attributes keep what was written.
    near(number(&attr(&mm, created, "YDim")), 400.0);
}

#[cfg(feature = "ifc4x3")]
#[test]
fn polynomial_coefficients_are_refused() {
    let data = COMMON.replace("{UNITS}", MM);
    let text = format!(
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
         FILE_NAME('','',(''),(''),'','','');\nFILE_SCHEMA(('IFC4X3_ADD2'));\n\
         ENDSEC;\nDATA;\n{data}\nENDSEC;\nEND-ISO-10303-21;\n"
    );
    let mut model = StepCodec.read_bytes(text.as_bytes()).expect("parses");
    let curve = create(
        "IfcPolynomialCurve",
        &[
            ("Position", Value::Ref(EntityId(31))),
            ("CoefficientsX", Value::List(vec![Value::Real(1.0)])),
        ],
    );
    let error = apply_authoring_with(&mut model, ifc::schema::ifc4x3(), &[curve], 1, &si())
        .expect_err("refused in SI");
    assert!(matches!(
        si_refusal(&error.failure),
        SiInputRefusal::PolynomialCoefficients
    ));
}

#[test]
fn a_temperature_difference_has_no_offset() {
    let mut model = model(MM_CELSIUS);
    let load = create(
        "IfcStructuralLoadTemperature",
        &[
            ("Name", Value::Text("L".into())),
            ("DeltaTConstant", Value::Real(10.0)),
        ],
    );
    let id = run(&mut model, &[load], 0);
    near(number(&attr(&model, id, "DeltaTConstant")), 10.0);
    // An absolute temperature does.
    let id = run(
        &mut model,
        &[single_value(
            typed("IFCTHERMODYNAMICTEMPERATUREMEASURE", 300.0),
            None,
        )],
        0,
    );
    near(number(&attr(&model, id, "NominalValue")), 26.85);
}

#[test]
fn an_enumerated_value_takes_its_enumerations_unit() {
    let mut foot = model(FOOT);
    let value = create(
        "IfcPropertyEnumeratedValue",
        &[
            ("Name", Value::Text("Size".into())),
            (
                "EnumerationValues",
                Value::List(vec![typed("IFCLENGTHMEASURE", 0.1)]),
            ),
            ("EnumerationReference", Value::Ref(EntityId(26))),
        ],
    );
    let id = run(&mut foot, &[value], 0);
    let Value::List(items) = attr(&foot, id, "EnumerationValues") else {
        panic!("a list");
    };
    near(number(&items[0]), 100.0);
}

#[test]
fn an_edit_takes_the_unit_its_record_states() {
    let mut foot = model(FOOT);
    let edit = AuthorOp::Edit {
        entity: EntityId(27),
        attributes: vec![("NominalValue".into(), typed("IFCLENGTHMEASURE", 2.0))],
    };
    run(&mut foot, &[edit], 0);
    near(number(&attr(&foot, EntityId(27), "NominalValue")), 2000.0);
}

#[test]
fn list_valued_and_integer_measures_are_kept_or_refused_by_the_table() {
    let mm = model(MM);
    let complex = Value::Typed {
        type_name: "IFCCOMPLEXNUMBER".into(),
        value: Box::new(Value::List(vec![Value::Real(1.0), Value::Real(2.0)])),
    };
    let failure = refused(&mm, &[single_value(complex, None)]);
    let AuthoringFailure::UnitConversion { error, measure, .. } = failure else {
        panic!("{failure:?}");
    };
    assert_eq!(measure, "IFCCOMPLEXNUMBER");
    assert!(matches!(
        *error,
        UnitConversionError::Resolve(ExactUnitError::UnmappedMeasureType { .. })
    ));

    let compound = Value::Typed {
        type_name: "IFCCOMPOUNDPLANEANGLEMEASURE".into(),
        value: Box::new(Value::List(vec![
            Value::Integer(50),
            Value::Integer(30),
            Value::Integer(15),
        ])),
    };
    let mut copy = mm.clone();
    run(&mut copy, &[single_value(compound.clone(), None)], 0);

    let integer = Value::Typed {
        type_name: "IFCINTEGERCOUNTRATEMEASURE".into(),
        value: Box::new(Value::Integer(5)),
    };
    let failure = refused(&mm, &[single_value(integer, None)]);
    assert!(
        matches!(si_refusal(&failure), SiInputRefusal::IntegerMeasure { .. }),
        "{failure:?}"
    );
}

fn cylinder() -> AuthorOp {
    create(
        "IfcCylindricalSurface",
        &[
            ("Position", Value::Ref(EntityId(31))),
            ("Radius", Value::Real(1.0)),
        ],
    )
}

#[test]
fn a_cylinders_angle_converts_and_its_axis_is_a_length() {
    let mut foot = model(FOOT);
    let point = create(
        "IfcPointOnSurface",
        &[
            ("BasisSurface", Value::Ref(authoring_handle(0))),
            ("PointParameterU", Value::Real(FRAC_PI_2)),
            ("PointParameterV", Value::Real(0.3048)),
        ],
    );
    let id = run(&mut foot, &[cylinder(), point], 1);
    near(number(&attr(&foot, id, "PointParameterU")), 90.0);
    near(number(&attr(&foot, id, "PointParameterV")), 1.0);
}

#[test]
fn a_pcurve_off_a_plane_is_refused() {
    let pcurve = create(
        "IfcPcurve",
        &[
            ("BasisSurface", Value::Ref(authoring_handle(0))),
            ("ReferenceCurve", Value::Ref(EntityId(30))),
        ],
    );
    let failure = refused(&model(MM), &[cylinder(), pcurve]);
    assert!(matches!(
        si_refusal(&failure),
        SiInputRefusal::ParameterSpaceCurve { .. }
    ));
}
