//! Unit resolution in a lenient model (#615): the reader's damage refuses
//! only where the resolution reads it.

use ifc_model::{Codec, Diagnostic, DiagnosticKind, EntityId, Model};
use ifc_properties::{
    exact_project_unit, exact_property, exact_schema, exact_unit, resolve_unit, ExactPropertyError,
    ExactUnitError,
};
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

fn missing(from: u64, to: u64) -> ExactUnitError {
    ExactUnitError::Structure(ExactPropertyError::MissingReference {
        from: id(from),
        to: id(to),
    })
}

fn incomplete(diagnostics: usize) -> ExactUnitError {
    ExactUnitError::Structure(ExactPropertyError::IncompleteModel { diagnostics })
}

#[test]
fn an_unrelated_skipped_record_leaves_units_resolvable() {
    let model = lenient(&units(&[], SITE));
    assert_eq!(model.diagnostics().len(), 1, "{:?}", model.diagnostics());
    assert_eq!(model.diagnostics()[0].kind(), DiagnosticKind::SkippedRecord);
    let unit = exact_unit(&model, "IFCLENGTHMEASURE", None).unwrap();
    assert_eq!((unit.unit, unit.from_project), (Some(id(1)), true));
    assert!((unit.scale - 1e-3).abs() < 1e-15);
    assert_eq!(exact_project_unit(&model, "LENGTHUNIT"), Ok(id(1)));
    assert!(resolve_unit(&model, id(1)).is_ok());
    assert!(matches!(
        exact_property(&model, id(1), None, "x"),
        Err(ExactPropertyError::IncompleteModel { .. })
    ));
    assert!(matches!(
        exact_schema(&model),
        Err(ExactPropertyError::IncompleteModel { .. })
    ));
}

#[test]
fn a_skipped_unit_record_still_refuses() {
    let model = lenient(&units(&[(1, "#1=XXBROKEN(;")], ""));
    assert_eq!(
        exact_unit(&model, "IFCLENGTHMEASURE", None),
        Err(missing(9, 1))
    );
}

#[test]
fn a_skipped_conversion_factor_still_refuses() {
    let model = lenient(&units(&[(5, "#5=XXBROKEN(;")], ""));
    assert_eq!(
        exact_unit(&model, "IFCPLANEANGLEMEASURE", None),
        Err(missing(6, 5))
    );
}

#[test]
fn a_skipped_assignment_still_refuses() {
    let model = lenient(&units(&[(9, "#9=XXBROKEN(;")], ""));
    assert_eq!(
        exact_unit(&model, "IFCLENGTHMEASURE", None),
        Err(missing(10, 9))
    );
}

#[test]
fn a_skipped_only_project_is_incomplete_not_absent() {
    let model = lenient(&units(&[(10, "#10=XXBROKEN(;")], ""));
    assert_eq!(
        exact_unit(&model, "IFCLENGTHMEASURE", None),
        Err(incomplete(1))
    );
    assert_eq!(exact_project_unit(&model, "LENGTHUNIT"), Err(incomplete(1)));
}

#[test]
fn a_model_without_a_project_and_without_damage_has_no_project() {
    let model = lenient(&units(&[(10, "")], ""));
    assert!(model.diagnostics().is_empty());
    assert_eq!(
        exact_project_unit(&model, "LENGTHUNIT"),
        Err(ExactUnitError::NoProject)
    );
}

#[test]
fn a_duplicate_unit_id_is_refused() {
    let model = checked(&units(&[], "#1=IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.);\n"));
    assert_eq!(
        exact_unit(&model, "IFCLENGTHMEASURE", None),
        Err(ExactUnitError::DamagedRecord {
            entity: id(1),
            diagnostic: 0
        })
    );
}

#[test]
fn a_duplicate_measure_record_is_refused() {
    let model = checked(&units(
        &[],
        "#5=IFCMEASUREWITHUNIT(IFCPLANEANGLEMEASURE(1.0),#3);\n",
    ));
    assert_eq!(
        exact_unit(&model, "IFCPLANEANGLEMEASURE", None),
        Err(ExactUnitError::DamagedRecord {
            entity: id(5),
            diagnostic: 0
        })
    );
}

#[test]
fn a_duplicate_project_id_is_refused() {
    let model = checked(&units(&[], "#10=IFCPROJECT('q',$,'Q',$,$,$,$,$,#9);\n"));
    assert_eq!(
        exact_unit(&model, "IFCLENGTHMEASURE", None),
        Err(ExactUnitError::DamagedRecord {
            entity: id(10),
            diagnostic: 0
        })
    );
}

#[test]
fn a_duplicate_assignment_id_is_refused() {
    let model = checked(&units(&[], "#9=IFCUNITASSIGNMENT((#2));\n"));
    assert_eq!(
        exact_project_unit(&model, "LENGTHUNIT"),
        Err(ExactUnitError::DamagedRecord {
            entity: id(9),
            diagnostic: 0
        })
    );
}

#[test]
fn resolve_unit_refuses_a_duplicated_unit() {
    let model = checked(&units(&[], "#1=IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.);\n"));
    assert!(matches!(
        resolve_unit(&model, id(1)),
        Err(ifc_properties::UnitConversionError::Resolve(
            ExactUnitError::DamagedRecord { .. }
        ))
    ));
}

#[test]
fn an_unrelated_duplicate_leaves_units_resolvable() {
    let model = checked(&units(
        &[],
        "#20=IFCSIUNIT(*,.MASSUNIT.,$,.GRAM.);\n#20=IFCSIUNIT(*,.MASSUNIT.,$,.GRAM.);\n",
    ));
    assert!(!model.diagnostics().is_empty());
    assert_eq!(exact_project_unit(&model, "LENGTHUNIT"), Ok(id(1)));
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
    let unit = exact_unit(&model, "IFCPLANEANGLEMEASURE", None).unwrap();
    assert!((unit.scale - 0.01).abs() < 1e-15);
}

#[test]
fn an_unclassified_diagnostic_refuses_the_model() {
    let mut model = lenient(&units(&[], ""));
    model.push_diagnostic(Diagnostic::unlocated("x"));
    assert_eq!(
        exact_unit(&model, "IFCLENGTHMEASURE", None),
        Err(incomplete(1))
    );
    assert!(exact_unit(&model, "IFCCOUNTMEASURE", None).is_ok());
}

#[test]
fn a_skipped_record_whose_id_was_read_is_still_missing() {
    let model = lenient(&units(&[(1, "#1=IFCSIUNIT(*,.LENGTHUNIT.,;")], ""));
    let first = &model.diagnostics()[0];
    assert_eq!(first.kind(), DiagnosticKind::SkippedRecord);
    assert_eq!(first.instance(), Some(id(1)));
    // The record is absent, so the absence is what is reported.
    assert_eq!(
        exact_unit(&model, "IFCLENGTHMEASURE", None),
        Err(missing(9, 1))
    );
}

#[test]
fn a_duplicated_explicit_unit_is_refused() {
    let model = checked(&units(&[], "#1=IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.);\n"));
    assert_eq!(
        exact_unit(&model, "IFCLENGTHMEASURE", Some(id(1))),
        Err(ExactUnitError::DamagedRecord {
            entity: id(1),
            diagnostic: 0
        })
    );
}

#[test]
fn a_duplicated_unit_component_is_refused() {
    let model = checked(&units(
        &[],
        "#3=IFCSIUNIT(*,.PLANEANGLEUNIT.,$,.RADIAN.);\n",
    ));
    assert_eq!(
        exact_unit(&model, "IFCPLANEANGLEMEASURE", Some(id(6))),
        Err(ExactUnitError::DamagedRecord {
            entity: id(3),
            diagnostic: 0
        })
    );
}

#[test]
fn a_duplicated_dimensional_exponents_record_is_refused() {
    let model = checked(&units(&[], "#4=IFCDIMENSIONALEXPONENTS(0,0,0,0,0,0,0);\n"));
    assert_eq!(
        exact_unit(&model, "IFCPLANEANGLEMEASURE", Some(id(6))),
        Err(ExactUnitError::DamagedRecord {
            entity: id(4),
            diagnostic: 0
        })
    );
}

#[test]
fn a_duplicated_unit_of_another_type_in_the_assignment_is_refused() {
    let model = checked(&units(&[], "#7=IFCSIUNIT(*,.MASSUNIT.,$,.GRAM.);\n"));
    assert_eq!(
        exact_unit(&model, "IFCLENGTHMEASURE", None),
        Err(ExactUnitError::DamagedRecord {
            entity: id(7),
            diagnostic: 0
        })
    );
}

#[test]
fn resolve_unit_reports_damage_before_the_unit_kind() {
    let model = checked(&units(
        &[],
        "#20=IFCMONETARYUNIT(.USD.);\n#20=IFCMONETARYUNIT(.USD.);\n",
    ));
    assert!(matches!(
        resolve_unit(&model, id(20)),
        Err(ifc_properties::UnitConversionError::Resolve(
            ExactUnitError::DamagedRecord { .. }
        ))
    ));
}

#[test]
fn a_skipped_duplicate_of_a_unit_record_is_damaged() {
    let model = lenient(&units(&[], "#1=IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.;\n"));
    assert_eq!(model.diagnostics().len(), 1);
    assert_eq!(model.diagnostics()[0].instance(), Some(id(1)));
    assert_eq!(
        exact_unit(&model, "IFCLENGTHMEASURE", None),
        Err(ExactUnitError::DamagedRecord {
            entity: id(1),
            diagnostic: 0
        })
    );
}

#[test]
fn a_skip_naming_a_held_id_before_a_unit_duplicate_reports_the_duplicate() {
    // The skip names #20, which a kept record holds; the conflicting #1 after
    // it is kept (openbim-step 0.11.4) and reported as a duplicate.
    let model = lenient(&units(
        &[],
        "#20=IFCSITE($);\n#20=IFCSITE($)\n#1=IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.);\n",
    ));
    assert_eq!(model.diagnostics().len(), 2, "{:?}", model.diagnostics());
    assert_eq!(model.diagnostics()[0].instance(), Some(id(20)));
    assert_eq!(model.diagnostics()[1].kind(), DiagnosticKind::DuplicateId);
    assert_eq!(model.diagnostics()[1].instance(), Some(id(1)));
    let damaged = ExactUnitError::DamagedRecord {
        entity: id(1),
        diagnostic: 1,
    };
    assert_eq!(
        exact_unit(&model, "IFCLENGTHMEASURE", None),
        Err(damaged.clone())
    );
    assert_eq!(exact_project_unit(&model, "LENGTHUNIT"), Err(damaged));
}

#[test]
fn a_skipped_duplicate_unit_after_a_bare_skip_is_damaged() {
    // The first repro of #668: #30 lacks its `;`, so the conflicting #1 that
    // follows is kept and reported rather than trusted.
    let model = lenient(&units(
        &[],
        "#30=IFCSITE($)\n#1=IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.);\n",
    ));
    assert_eq!(model.diagnostics().len(), 2, "{:?}", model.diagnostics());
    assert_eq!(model.diagnostics()[0].instance(), Some(id(30)));
    assert_eq!(model.diagnostics()[1].kind(), DiagnosticKind::DuplicateId);
    assert_eq!(
        exact_unit(&model, "IFCLENGTHMEASURE", None),
        Err(ExactUnitError::DamagedRecord {
            entity: id(1),
            diagnostic: 1
        })
    );
}

#[test]
fn a_cut_record_whose_tail_names_a_held_id_stays_incomplete() {
    let model = lenient(&units(&[], "#20=IFCSITE($);\n#11=IFCSITE(#1;#20,#46);\n"));
    assert_eq!(model.diagnostics().len(), 2, "{:?}", model.diagnostics());
    assert_eq!(model.diagnostics()[1].instance(), Some(id(20)));
    assert_eq!(
        exact_unit(&model, "IFCLENGTHMEASURE", None),
        Err(incomplete(2))
    );
}

#[test]
fn a_skip_before_any_id_is_incomplete() {
    let model = lenient(&units(&[], "@;\n"));
    assert_eq!(
        exact_unit(&model, "IFCLENGTHMEASURE", None),
        Err(incomplete(1))
    );
    assert_eq!(exact_project_unit(&model, "LENGTHUNIT"), Err(incomplete(1)));
}

#[test]
fn a_dangling_reference_elsewhere_resolves() {
    let model = checked(&units(&[], "#18=IFCRELAGGREGATES('r',$,$,$,#10,(#99));\n"));
    assert!(model
        .diagnostics()
        .iter()
        .any(|d| d.kind() == DiagnosticKind::DanglingReference));
    assert_eq!(exact_project_unit(&model, "LENGTHUNIT"), Ok(id(1)));
}

#[test]
fn a_dangling_reference_to_a_unit_record_is_missing() {
    let model = checked(&units(&[(9, "#9=IFCUNITASSIGNMENT((#1,#11));")], ""));
    assert_eq!(
        exact_unit(&model, "IFCLENGTHMEASURE", None),
        Err(missing(9, 11))
    );
}

#[test]
fn a_duplicate_without_an_id_is_incomplete() {
    let mut model = lenient(&units(&[], ""));
    model.push_diagnostic(Diagnostic::new(DiagnosticKind::DuplicateId, "x"));
    assert_eq!(
        exact_unit(&model, "IFCLENGTHMEASURE", None),
        Err(incomplete(1))
    );
}

#[test]
fn the_first_diagnostic_naming_a_record_is_reported() {
    let model = checked(&units(
        &[],
        "#20=IFCSIUNIT(*,.MASSUNIT.,$,.GRAM.);\n#20=IFCSIUNIT(*,.MASSUNIT.,$,.GRAM.);\n\
         #1=IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.);\n#1=IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.);\n",
    ));
    assert_eq!(
        exact_unit(&model, "IFCLENGTHMEASURE", None),
        Err(ExactUnitError::DamagedRecord {
            entity: id(1),
            diagnostic: 1
        })
    );
}
