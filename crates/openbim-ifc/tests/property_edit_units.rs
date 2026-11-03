//! Property edits that read their numbers in SI (#487): the value written is
//! the SI number converted into the unit that applies, a refusal leaves the
//! model untouched, and the default writes numbers as given.
//!
//! One IFC4 wall (#22) carries a quantity `Test_Quantities.Width` in an
//! explicit millimetre unit (#23) and an enumerated property
//! `Test_Units.Size` whose enumeration states millimetres (#27). The project
//! units are millimetres or feet, by fixture.

#![cfg(all(feature = "step", feature = "properties", feature = "ifc4"))]

use std::f64::consts::FRAC_PI_2;

use ifc::properties::{ExactUnitError, UnitConversionError};
use ifc::{
    apply_property_edits, apply_property_edits_with, Codec, EntityId, InputUnits, Model,
    PropertyEdit, PropertyEditFailure, PropertyEditOptions, StepCodec, Value,
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
#22=IFCWALL('31t64Uzj98DhdKplMxBpF5',#5,'Wall',$,$,$,$,$,$);
#23=IFCQUANTITYLENGTH('Width',$,#6,200.,$);
#24=IFCELEMENTQUANTITY('2CazjTQP11iu6o3c47EEd8',#5,'Test_Quantities',$,$,(#23));
#25=IFCRELDEFINESBYPROPERTIES('0xtMgJ4Tb6LQGWVe9NZXe6',#5,$,$,(#22),#24);
#26=IFCPROPERTYENUMERATION('Sizes',(IFCLENGTHMEASURE(100.),IFCLENGTHMEASURE(200.)),#6);
#27=IFCPROPERTYENUMERATEDVALUE('Size',$,(IFCLENGTHMEASURE(100.)),#26);
#28=IFCPROPERTYSET('1jbZpUGIXFjB7a6sZlrkkE',#5,'Test_Units',$,(#27));
#29=IFCRELDEFINESBYPROPERTIES('3SWTIOzQn9Wuyh2$ztj4fM',#5,$,$,(#22),#28);";

/// Millimetre length, square millimetre area (`MILLI` applies before the
/// square, as `resolve_unit` reads it), SI volume, radian, kilopascal.
const MM: &str = "#8=IFCSIUNIT(*,.AREAUNIT.,.MILLI.,.SQUARE_METRE.);
#9=IFCSIUNIT(*,.VOLUMEUNIT.,$,.CUBIC_METRE.);
#10=IFCSIUNIT(*,.PLANEANGLEUNIT.,$,.RADIAN.);
#11=IFCSIUNIT(*,.PRESSUREUNIT.,.KILO.,.PASCAL.);
#20=IFCUNITASSIGNMENT((#6,#8,#9,#10,#11));";

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

fn typed(type_name: &str, value: Value) -> Value {
    Value::Typed {
        type_name: type_name.into(),
        value: Box::new(value),
    }
}

fn real(type_name: &str, number: f64) -> Value {
    typed(type_name, Value::Real(number))
}

fn si() -> PropertyEditOptions {
    PropertyEditOptions::default().with_input_units(InputUnits::Si)
}

fn bytes(model: &Model) -> Vec<u8> {
    StepCodec.write_bytes(model).expect("writes")
}

/// New properties of `Test_New` on the wall, one per `(name, value)`.
fn new_properties(values: &[(&str, Value)]) -> Vec<PropertyEdit> {
    values
        .iter()
        .map(|(name, value)| PropertyEdit::set(EntityId(22), "Test_New", *name, value.clone()))
        .collect()
}

/// The `NominalValue` of each property the batch wrote, in edit order.
fn nominal_values(model: &Model, properties: &[Option<EntityId>]) -> Vec<Value> {
    properties
        .iter()
        .map(|id| {
            let entity = model
                .get(id.expect("a property is written"))
                .expect("exists");
            entity.attributes[2].clone()
        })
        .collect()
}

fn refused(model: &mut Model, edits: &[PropertyEdit]) -> (Option<usize>, PropertyEditFailure) {
    let before = bytes(model);
    let error = apply_property_edits_with(model, edits, 7, &si()).expect_err("refused");
    assert_eq!(bytes(model), before, "a refused batch changed the model");
    (error.edit, error.failure)
}

#[test]
fn si_numbers_become_the_project_unit() {
    let mut model = model(MM);
    let cases = [
        ("Length", real("IFCLENGTHMEASURE", 1.234), 1234.0),
        ("Area", real("IFCAREAMEASURE", 2.5), 2_500_000.0),
        ("Volume", real("IFCVOLUMEMEASURE", 2.5), 2.5),
        ("Angle", real("IFCPLANEANGLEMEASURE", 1.5), 1.5),
        ("Pressure", real("IFCPRESSUREMEASURE", 2500.0), 2.5),
        ("Ratio", real("IFCRATIOMEASURE", 0.5), 0.5),
    ];
    let values: Vec<_> = cases.iter().map(|(n, v, _)| (*n, v.clone())).collect();
    let mut edits = new_properties(&values);
    let text = typed("IFCLABEL", Value::Text("x".into()));
    edits.push(PropertyEdit::set(
        EntityId(22),
        "Test_New",
        "Text",
        text.clone(),
    ));
    let outcome = apply_property_edits_with(&mut model, &edits, 7, &si()).expect("applies");
    let written = nominal_values(&model, &outcome.properties);
    for ((name, value, expected), written) in cases.iter().zip(&written) {
        let Value::Typed {
            type_name: wanted, ..
        } = value
        else {
            unreachable!()
        };
        assert_eq!(written, &real(wanted, *expected), "{name}");
    }
    assert_eq!(written[cases.len()], text, "a label is not a number");
}

#[test]
fn numbers_are_written_as_given_by_default() {
    assert_eq!(InputUnits::default(), InputUnits::Raw);
    assert_eq!(PropertyEditOptions::default().input_units, InputUnits::Raw);
    let mut model = model(MM);
    let edits = new_properties(&[("Length", real("IFCLENGTHMEASURE", 1.234))]);
    let outcome = apply_property_edits(&mut model, &edits, 7).expect("applies");
    assert_eq!(
        nominal_values(&model, &outcome.properties),
        [real("IFCLENGTHMEASURE", 1.234)]
    );
}

#[test]
fn conversion_based_project_units_and_explicit_units_apply() {
    let mut model = model(FOOT);
    let edits = new_properties(&[
        ("Length", real("IFCLENGTHMEASURE", 0.3048)),
        ("Area", real("IFCAREAMEASURE", 0.09290304)),
        ("Angle", real("IFCPLANEANGLEMEASURE", FRAC_PI_2)),
    ]);
    let outcome = apply_property_edits_with(&mut model, &edits, 7, &si()).expect("applies");
    assert_eq!(
        nominal_values(&model, &outcome.properties),
        [
            real("IFCLENGTHMEASURE", 1.0),
            real("IFCAREAMEASURE", 1.0),
            real("IFCPLANEANGLEMEASURE", 90.0),
        ]
    );
    // The quantity states millimetres, whatever the project unit is.
    let width = PropertyEdit::set(
        EntityId(22),
        "Test_Quantities",
        "Width",
        real("IFCLENGTHMEASURE", 0.25),
    );
    let outcome = apply_property_edits_with(&mut model, &[width], 8, &si()).expect("applies");
    assert_eq!(outcome.properties[0], Some(EntityId(23)));
    let quantity = model.get(EntityId(23)).unwrap();
    assert_eq!(quantity.attributes[3], Value::Real(250.0));
    // The enumeration states millimetres: 0.2 m is its member 200.
    let size = PropertyEdit::set(
        EntityId(22),
        "Test_Units",
        "Size",
        Value::List(vec![real("IFCLENGTHMEASURE", 0.2)]),
    );
    let outcome = apply_property_edits_with(&mut model, &[size], 9, &si()).expect("applies");
    let size = model.get(outcome.properties[0].unwrap()).unwrap();
    assert_eq!(
        size.attributes[2],
        Value::List(vec![real("IFCLENGTHMEASURE", 200.0)])
    );
}

#[test]
fn si_numbers_that_cannot_be_converted_are_refused() {
    let mut model = model(FOOT);
    let one = |value| [PropertyEdit::set(EntityId(22), "Test_New", "P", value)];
    let (edit, failure) = refused(&mut model, &one(real("IFCPRESSUREMEASURE", 1.0)));
    assert_eq!(edit, Some(0));
    assert!(
        matches!(
            &failure,
            PropertyEditFailure::UnitConversion {
                measure,
                error: UnitConversionError::Resolve(ExactUnitError::NoProjectUnit { unit_type }),
            } if measure == "IFCPRESSUREMEASURE" && &**unit_type == "PRESSUREUNIT"
        ),
        "{failure:?}"
    );
    assert!(failure
        .to_string()
        .starts_with("IFCPRESSUREMEASURE given in SI"));
    let (_, failure) = refused(&mut model, &one(real("IFCTHERMALCONDUCTIVITYMEASURE", 1.0)));
    assert!(
        matches!(
            failure,
            PropertyEditFailure::UnitConversion {
                error: UnitConversionError::Resolve(ExactUnitError::UnmappedMeasureType { .. }),
                ..
            }
        ),
        "{failure:?}"
    );
    let (_, failure) = refused(&mut model, &one(real("IFCLENGTHMEASURE", f64::NAN)));
    assert!(
        matches!(
            failure,
            PropertyEditFailure::UnitConversion {
                error: UnitConversionError::NonFinite { .. },
                ..
            }
        ),
        "{failure:?}"
    );
    let (_, failure) = refused(
        &mut model,
        &one(typed("IFCLENGTHMEASURE", Value::Integer(1))),
    );
    assert!(
        matches!(failure, PropertyEditFailure::InvalidValue(_)),
        "{failure:?}"
    );
    // Converted first, then checked against the enumeration: 0.3 m is 300 mm.
    let size = PropertyEdit::set(
        EntityId(22),
        "Test_Units",
        "Size",
        Value::List(vec![real("IFCLENGTHMEASURE", 0.3)]),
    );
    let (_, failure) = refused(&mut model, &[size]);
    assert!(
        matches!(failure, PropertyEditFailure::Template(_)),
        "{failure:?}"
    );
}

#[test]
fn raw_input_takes_what_si_input_refuses() {
    let mut model = model(FOOT);
    let edits = new_properties(&[
        ("Pressure", real("IFCPRESSUREMEASURE", 1.0)),
        ("Conductivity", real("IFCTHERMALCONDUCTIVITYMEASURE", 1.0)),
    ]);
    apply_property_edits(&mut model, &edits, 7).expect("raw input is not converted");
}

#[test]
fn a_quantity_takes_an_integer_as_a_real_within_the_exact_range() {
    let mut model = model(MM);
    let width = |value| {
        [PropertyEdit::set(
            EntityId(22),
            "Test_Quantities",
            "Width",
            typed("IFCLENGTHMEASURE", value),
        )]
    };
    let outcome = apply_property_edits_with(&mut model, &width(Value::Integer(1)), 7, &si())
        .expect("applies");
    let quantity = model.get(outcome.properties[0].unwrap()).unwrap();
    assert_eq!(quantity.attributes[3], Value::Real(1000.0));
    let (_, failure) = refused(&mut model, &width(Value::Integer(9_007_199_254_740_993)));
    assert!(
        matches!(failure, PropertyEditFailure::InvalidValue(_)),
        "{failure:?}"
    );
}

#[test]
fn every_item_of_a_list_is_converted() {
    let mut model = model(MM);
    let size = PropertyEdit::set(
        EntityId(22),
        "Test_Units",
        "Size",
        Value::List(vec![
            real("IFCLENGTHMEASURE", 0.1),
            real("IFCLENGTHMEASURE", 0.2),
        ]),
    );
    let outcome = apply_property_edits_with(&mut model, &[size], 7, &si()).expect("applies");
    let property = model.get(outcome.properties[0].unwrap()).unwrap();
    assert_eq!(
        property.attributes[2],
        Value::List(vec![
            real("IFCLENGTHMEASURE", 100.0),
            real("IFCLENGTHMEASURE", 200.0)
        ])
    );
}

#[test]
fn a_malformed_stated_unit_is_refused() {
    let mut model = model(MM);
    let text = String::from_utf8(bytes(&model)).unwrap();
    let broken = text.replace(
        "IFCQUANTITYLENGTH('Width',$,#6,",
        "IFCQUANTITYLENGTH('Width',$,'mm',",
    );
    assert_ne!(text, broken);
    model = StepCodec.read_bytes(broken.as_bytes()).expect("parses");
    let size = PropertyEdit::set(
        EntityId(22),
        "Test_Units",
        "Size",
        Value::List(vec![real("IFCLENGTHMEASURE", 0.2)]),
    );
    // The holder read refuses a unit that is no reference first; the
    // planner's own check of `Unit` is the second line of defence.
    let (_, failure) = refused(&mut model, &[size]);
    assert!(
        matches!(
            failure,
            PropertyEditFailure::Resolve(_)
                | PropertyEditFailure::UnitConversion {
                    error: UnitConversionError::Resolve(ExactUnitError::MalformedUnit {
                        attribute: "Unit",
                        ..
                    }),
                    ..
                }
        ),
        "{failure:?}"
    );
}
