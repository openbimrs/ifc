//! `units::resolve` in a lenient model (#684): the reader's damage refuses
//! where the resolution reads it, with the verdicts `ifc-properties` gives.

use ifc_geometry::units::{resolve, UnitScale};
use ifc_geometry::GeometryError;
use ifc_model::{Codec, Diagnostic, DiagnosticKind, EntityId, Model};
use ifc_step::{ParseOptions, StepCodec};

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

fn id(n: u64) -> EntityId {
    EntityId(n)
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

const SITE: &str = "#17\n XXSITE('x',$);\n#18=IFCRELAGGREGATES('r',$,$,$,#10,(#17));\n";

fn damaged(entity: u64, diagnostic: usize) -> GeometryError {
    GeometryError::UnitRecordDamaged {
        entity: id(entity),
        diagnostic,
    }
}

fn incomplete(diagnostics: usize) -> GeometryError {
    GeometryError::UnitsIncomplete { diagnostics }
}

fn scale(model: &Model) -> UnitScale {
    resolve(model).expect("resolves")
}

#[test]
fn a_duplicate_unit_id_is_refused() {
    let model = checked(&units(&[], "#1=IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.);\n"));
    assert_eq!(resolve(&model), Err(damaged(1, 0)));
}

#[test]
fn a_skipped_duplicate_of_a_unit_record_is_damaged() {
    let model = lenient(&units(&[], "#1=IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.;\n"));
    assert_eq!(model.diagnostics().len(), 1);
    assert_eq!(resolve(&model), Err(damaged(1, 0)));
}

#[test]
fn a_duplicate_of_each_record_read_is_refused() {
    for (record, duplicate) in [
        (10, "#10=IFCPROJECT('q',$,'Q',$,$,$,$,$,#9);\n"),
        (9, "#9=IFCUNITASSIGNMENT((#2));\n"),
        (7, "#7=IFCSIUNIT(*,.MASSUNIT.,$,.GRAM.);\n"),
        (5, "#5=IFCMEASUREWITHUNIT(IFCPLANEANGLEMEASURE(1.0),#3);\n"),
        (3, "#3=IFCSIUNIT(*,.PLANEANGLEUNIT.,$,.RADIAN.);\n"),
        (4, "#4=IFCDIMENSIONALEXPONENTS(0,0,0,0,0,0,0);\n"),
    ] {
        let model = checked(&units(&[], duplicate));
        assert_eq!(resolve(&model), Err(damaged(record, 0)), "#{record}");
    }
}

#[test]
fn the_first_diagnostic_naming_a_record_is_reported() {
    let model = checked(&units(
        &[],
        "#20=IFCSIUNIT(*,.MASSUNIT.,$,.GRAM.);\n#20=IFCSIUNIT(*,.MASSUNIT.,$,.GRAM.);\n\
         #1=IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.);\n#1=IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.);\n",
    ));
    assert_eq!(resolve(&model), Err(damaged(1, 1)));
}

#[test]
fn a_skip_before_any_id_is_incomplete() {
    let model = lenient(&units(&[], "@;\n"));
    assert_eq!(resolve(&model), Err(incomplete(1)));
}

#[test]
fn diagnostics_that_cannot_be_attributed_are_incomplete() {
    for diagnostic in [
        Diagnostic::new(DiagnosticKind::DuplicateId, "x"),
        Diagnostic::unlocated("x"),
    ] {
        let mut model = lenient(&units(&[], ""));
        model.push_diagnostic(diagnostic);
        assert_eq!(resolve(&model), Err(incomplete(1)));
    }
}

#[test]
fn a_skip_naming_a_held_id_before_a_unit_duplicate_is_damaged() {
    // The skip names #20, which a kept record holds; the conflicting #1
    // after it is kept (openbim-step 0.11.4) and reported as a duplicate.
    let model = lenient(&units(
        &[],
        "#20=IFCSITE($);\n#20=IFCSITE($)\n#1=IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.);\n",
    ));
    assert_eq!(model.diagnostics().len(), 2, "{:?}", model.diagnostics());
    assert_eq!(resolve(&model), Err(damaged(1, 1)));
}

#[test]
fn a_cut_record_whose_tail_names_a_held_id_is_incomplete() {
    let model = lenient(&units(&[], "#20=IFCSITE($);\n#11=IFCSITE(#1;#20,#46);\n"));
    assert_eq!(model.diagnostics().len(), 2, "{:?}", model.diagnostics());
    assert_eq!(resolve(&model), Err(incomplete(2)));
}

#[test]
fn a_skipped_only_project_is_incomplete_not_absent() {
    let model = lenient(&units(&[(10, "#10=XXBROKEN(;")], ""));
    assert_eq!(resolve(&model), Err(incomplete(1)));
}

#[test]
fn a_model_without_a_project_and_without_damage_has_no_length_unit() {
    let model = lenient(&units(&[(10, "")], ""));
    assert!(model.diagnostics().is_empty());
    assert_eq!(
        resolve(&model),
        Err(GeometryError::LengthUnitUndeclared {
            project: None,
            assignment: None
        })
    );
}

#[test]
fn a_skipped_unit_record_is_missing() {
    let model = lenient(&units(&[(1, "#1=XXBROKEN(;")], ""));
    assert_eq!(
        resolve(&model),
        Err(GeometryError::MissingEntity {
            referrer: id(9),
            missing: id(1)
        })
    );
}

#[test]
fn an_unrelated_skipped_record_leaves_units_resolvable() {
    let model = lenient(&units(&[], SITE));
    assert_eq!(model.diagnostics().len(), 1, "{:?}", model.diagnostics());
    let scale = scale(&model);
    assert!((scale.length_to_metres - 1e-3).abs() < 1e-15);
    assert_eq!(scale.angle_to_radians, Some(0.017453292519943295));
}

#[test]
fn an_unrelated_duplicate_leaves_units_resolvable() {
    let model = checked(&units(
        &[],
        "#20=IFCSIUNIT(*,.MASSUNIT.,$,.GRAM.);\n#20=IFCSIUNIT(*,.MASSUNIT.,$,.GRAM.);\n",
    ));
    assert!(!model.diagnostics().is_empty());
    assert!((scale(&model).length_to_metres - 1e-3).abs() < 1e-15);
}

#[test]
fn a_dangling_reference_elsewhere_resolves() {
    let model = checked(&units(&[], "#18=IFCRELAGGREGATES('r',$,$,$,#10,(#99));\n"));
    assert!(model
        .diagnostics()
        .iter()
        .any(|d| d.kind() == DiagnosticKind::DanglingReference));
    assert!((scale(&model).length_to_metres - 1e-3).abs() < 1e-15);
}

#[test]
fn a_real_without_point_in_a_factor_resolves() {
    let model = lenient(&units(
        &[(5, "#5=IFCMEASUREWITHUNIT(IFCPLANEANGLEMEASURE(1E-02),#3);")],
        "",
    ));
    assert!(model
        .diagnostics()
        .iter()
        .any(|d| d.kind() == DiagnosticKind::RealWithoutPoint));
    let angle = scale(&model).angle_to_radians.expect("angle unit");
    assert!((angle - 0.01).abs() < 1e-15);
}
