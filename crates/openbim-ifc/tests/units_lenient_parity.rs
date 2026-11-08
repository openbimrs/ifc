//! `ifc::geometry::units::resolve` and `ifc::properties::exact_unit` give one
//! verdict on a lenient model's unit records (#684).

#![cfg(all(feature = "step", feature = "geometry-select", feature = "properties"))]

use ifc::geometry::units::resolve;
use ifc::geometry::GeometryError;
use ifc::properties::{exact_unit, ExactPropertyError, ExactUnitError};
use ifc::{Codec, Diagnostic, DiagnosticKind, EntityId, Model, ParseOptions, StepCodec};

#[derive(Debug, PartialEq)]
enum Verdict {
    Resolved,
    Damaged(EntityId),
    Incomplete,
    Refused,
}

fn text(data: &str) -> String {
    format!(
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
         FILE_NAME('','',(''),(''),'','','');\nFILE_SCHEMA(('IFC4'));\n\
         ENDSEC;\nDATA;\n{data}\nENDSEC;\nEND-ISO-10303-21;\n"
    )
}

fn lenient(data: &str) -> Model {
    StepCodec::lenient()
        .read_bytes(text(data).as_bytes())
        .expect("lenient read")
}

fn checked(data: &str) -> Model {
    StepCodec::with_options(ParseOptions::lenient().check_references(true))
        .read_bytes(text(data).as_bytes())
        .expect("lenient read")
}

/// A millimetre project with a degree unit (#5, #6) and its assignment (#9).
fn units(replace: &[(u64, &str)], extra: &str) -> String {
    let mut lines = vec![
        "IFCSIUNIT(*,.LENGTHUNIT.,.MILLI.,.METRE.)",
        "IFCSIUNIT(*,.AREAUNIT.,$,.SQUARE_METRE.)",
        "IFCSIUNIT(*,.PLANEANGLEUNIT.,$,.RADIAN.)",
        "IFCDIMENSIONALEXPONENTS(0,0,0,0,0,0,0)",
        "IFCMEASUREWITHUNIT(IFCPLANEANGLEMEASURE(0.017453292519943295),#3)",
        "IFCCONVERSIONBASEDUNIT(#4,.PLANEANGLEUNIT.,'DEGREE',#5)",
        "IFCSIUNIT(*,.MASSUNIT.,$,.GRAM.)",
        "IFCSIUNIT(*,.THERMODYNAMICTEMPERATUREUNIT.,$,.DEGREE_CELSIUS.)",
        "IFCUNITASSIGNMENT((#1,#2,#6,#7,#8))",
        "IFCPROJECT('p',$,'P',$,$,$,$,$,#9)",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect::<Vec<_>>();
    let mut out = String::new();
    for (index, line) in lines.iter_mut().enumerate() {
        let n = index as u64 + 1;
        match replace.iter().find(|(at, _)| *at == n) {
            Some((_, with)) => out.push_str(&format!("{with}\n")),
            None => out.push_str(&format!("#{n}={line};\n")),
        }
    }
    out.push_str(extra);
    out
}

/// `resolve` reads the length and the plane angle unit, so the properties
/// side is asked for both and the first refusal counts.
fn properties_verdict(model: &Model) -> Verdict {
    for measure in ["IFCLENGTHMEASURE", "IFCPLANEANGLEMEASURE"] {
        match exact_unit(model, measure, None) {
            Ok(_) => {}
            Err(ExactUnitError::DamagedRecord { entity, .. }) => return Verdict::Damaged(entity),
            Err(ExactUnitError::Structure(ExactPropertyError::IncompleteModel { .. })) => {
                return Verdict::Incomplete
            }
            Err(_) => return Verdict::Refused,
        }
    }
    Verdict::Resolved
}

fn geometry_verdict(model: &Model) -> Verdict {
    match resolve(model) {
        Ok(_) => Verdict::Resolved,
        Err(GeometryError::UnitRecordDamaged { entity, .. }) => Verdict::Damaged(entity),
        Err(GeometryError::UnitsIncomplete { .. }) => Verdict::Incomplete,
        Err(_) => Verdict::Refused,
    }
}

fn pushed(diagnostic: Diagnostic) -> Model {
    let mut model = lenient(&units(&[], ""));
    model.push_diagnostic(diagnostic);
    model
}

#[test]
fn both_resolutions_give_one_verdict() {
    let cases: Vec<(&str, Model, Verdict)> =
        vec![
        ("intact", lenient(&units(&[], "")), Verdict::Resolved),
        (
            "duplicate unit",
            checked(&units(&[], "#1=IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.);\n")),
            Verdict::Damaged(EntityId(1)),
        ),
        (
            "skipped duplicate unit",
            lenient(&units(&[], "#1=IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.;\n")),
            Verdict::Damaged(EntityId(1)),
        ),
        (
            "duplicate project",
            checked(&units(&[], "#10=IFCPROJECT('q',$,'Q',$,$,$,$,$,#9);\n")),
            Verdict::Damaged(EntityId(10)),
        ),
        (
            "duplicate assignment",
            checked(&units(&[], "#9=IFCUNITASSIGNMENT((#2));\n")),
            Verdict::Damaged(EntityId(9)),
        ),
        (
            "duplicate unit of another type",
            checked(&units(&[], "#7=IFCSIUNIT(*,.MASSUNIT.,$,.GRAM.);\n")),
            Verdict::Damaged(EntityId(7)),
        ),
        (
            "duplicate measure",
            checked(&units(
                &[],
                "#5=IFCMEASUREWITHUNIT(IFCPLANEANGLEMEASURE(1.0),#3);\n",
            )),
            Verdict::Damaged(EntityId(5)),
        ),
        (
            "duplicate component",
            checked(&units(&[], "#3=IFCSIUNIT(*,.PLANEANGLEUNIT.,$,.RADIAN.);\n")),
            Verdict::Damaged(EntityId(3)),
        ),
        (
            "duplicate dimensions",
            checked(&units(&[], "#4=IFCDIMENSIONALEXPONENTS(0,0,0,0,0,0,0);\n")),
            Verdict::Damaged(EntityId(4)),
        ),
        (
            "skip without an id",
            lenient(&units(&[], "@;\n")),
            Verdict::Incomplete,
        ),
        (
            "duplicate without an id",
            pushed(Diagnostic::new(DiagnosticKind::DuplicateId, "x")),
            Verdict::Incomplete,
        ),
        (
            "unclassified diagnostic",
            pushed(Diagnostic::unlocated("x")),
            Verdict::Incomplete,
        ),
        (
            "held-id skip before a unit duplicate",
            lenient(&units(
                &[],
                "#20=IFCSITE($);\n#20=IFCSITE($)\n#1=IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.);\n",
            )),
            Verdict::Damaged(EntityId(1)),
        ),
        (
            "cut record",
            lenient(&units(&[], "#20=IFCSITE($);\n#11=IFCSITE(#1;#20,#46);\n")),
            Verdict::Incomplete,
        ),
        (
            "skipped project",
            lenient(&units(&[(10, "#10=XXBROKEN(;")], "")),
            Verdict::Incomplete,
        ),
        (
            "no project",
            lenient(&units(&[(10, "")], "")),
            Verdict::Refused,
        ),
        (
            "skipped unit",
            lenient(&units(&[(1, "#1=XXBROKEN(;")], "")),
            Verdict::Refused,
        ),
        (
            "unrelated duplicate",
            checked(&units(
                &[],
                "#20=IFCSIUNIT(*,.MASSUNIT.,$,.GRAM.);\n#20=IFCSIUNIT(*,.MASSUNIT.,$,.GRAM.);\n",
            )),
            Verdict::Resolved,
        ),
    ];
    for (name, model, expected) in cases {
        assert_eq!(properties_verdict(&model), expected, "properties: {name}");
        assert_eq!(geometry_verdict(&model), expected, "geometry: {name}");
    }
}
