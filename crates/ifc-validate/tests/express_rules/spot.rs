//! Rule-by-rule checks on hand-written violating models, per release.

use ifc_schema::SchemaVersion;
use ifc_validate::express::{EvalLimits, ExpressRules};
use ifc_validate::{
    validate_with_options, BudgetExceeded, FindingDetail, NotEvaluated, Path, RuleOutcome,
    RuleScope, Severity, ValidateOptions,
};

use super::corpus::evaluated;
use super::fixtures::{detail, step, under, GEOMETRY};
use super::{rules, schema, spec_available, RELEASES};

/// IFC2X3 calls `CorrectItemsForType` `WR24`.
fn items_for_type(version: SchemaVersion) -> &'static str {
    if version == SchemaVersion::Ifc2x3 {
        "IfcShapeRepresentation.WR24"
    } else {
        "IfcShapeRepresentation.CorrectItemsForType"
    }
}

/// A record of `entity` with the named attributes set, every other slot
/// `$`, laid out by the release's own tables.
fn record(version: SchemaVersion, id: u32, entity: &str, values: &[(&str, &str)]) -> String {
    let names = schema(version).attribute_names(entity);
    assert!(!names.is_empty(), "{entity} in {version:?}");
    let slots: Vec<&str> = names
        .iter()
        .map(|name| {
            values
                .iter()
                .find(|(attr, _)| attr == name)
                .map_or("$", |(_, value)| *value)
        })
        .collect();
    format!(
        "#{id}={}({});",
        entity.to_ascii_uppercase(),
        slots.join(",")
    )
}

/// `IfcShapeModel.WR11`: a shape representation belongs to exactly one
/// product shape, representation map or shape aspect. The rule reads three
/// INVERSE attributes and is not evaluated by the hand-written rules (#501).
#[test]
fn shape_model_wr11_flags_an_orphan_representation_in_every_release() {
    if !spec_available() {
        return;
    }
    for (version, _) in RELEASES {
        let model = step(
            version,
            &format!(
                "{GEOMETRY}
#6=IFCSHAPEREPRESENTATION(#3,'Body','Curve2D',(#5));
#7=IFCSHAPEREPRESENTATION(#3,'Body','Curve2D',(#5));
#8=IFCPRODUCTDEFINITIONSHAPE($,$,(#6));"
            ),
        );
        let report = evaluated(&model, version);
        let found = under(&report, "IfcShapeModel.WR11");
        assert_eq!(found.len(), 1, "{version:?}: {found:#?}");
        assert_eq!(found[0].severity, Severity::Error);
        assert_eq!(found[0].path, Path::Entity(ifc_model::EntityId(7)));
        let detail = detail(found[0]);
        assert_eq!(detail.label, "WR11");
        assert_eq!(
            detail.scope,
            RuleScope::Entity {
                declared_on: "IfcShapeModel".into()
            }
        );
        assert_eq!(detail.outcome, RuleOutcome::Violated);
        assert!(detail.release.eq_ignore_ascii_case(schema(version).name()));
    }
}

/// `CorrectItemsForType` (`WR24` in IFC2X3) calls the schema FUNCTION
/// `IfcShapeRepresentationTypes`: a 2D polyline is a `Curve2D`, never a
/// `SweptSolid`.
#[test]
fn correct_items_for_type_runs_its_function_in_every_release() {
    if !spec_available() {
        return;
    }
    for (version, _) in RELEASES {
        let model = step(
            version,
            &format!(
                "{GEOMETRY}
#6=IFCSHAPEREPRESENTATION(#3,'Body','Curve2D',(#5));
#7=IFCSHAPEREPRESENTATION(#3,'Body','SweptSolid',(#5));
#8=IFCPRODUCTDEFINITIONSHAPE($,$,(#6,#7));"
            ),
        );
        let report = evaluated(&model, version);
        let found = under(&report, items_for_type(version));
        assert_eq!(found.len(), 1, "{version:?}: {:#?}", report.findings());
        assert_eq!(found[0].path, Path::Entity(ifc_model::EntityId(7)));
        assert!(
            under(&report, "IfcShapeModel.WR11").is_empty(),
            "{version:?}"
        );
    }
}

/// A type WHERE rule binds a value wherever it appears: here inside a
/// SELECT (`IfcValue`), wrapped as `IFCPOSITIVELENGTHMEASURE(...)`.
#[test]
fn a_type_rule_binds_a_wrapped_select_value_in_every_release() {
    if !spec_available() {
        return;
    }
    for (version, _) in RELEASES {
        let model = step(
            version,
            "#9=IFCPROPERTYSINGLEVALUE('Width',$,IFCPOSITIVELENGTHMEASURE(-1.),$);
#10=IFCPROPERTYSINGLEVALUE('Depth',$,IFCPOSITIVELENGTHMEASURE(1.),$);",
        );
        let report = evaluated(&model, version);
        let found = under(&report, "IfcPositiveLengthMeasure.WR1");
        assert_eq!(found.len(), 1, "{version:?}: {:#?}", report.findings());
        assert!(
            matches!(&found[0].path, Path::Attribute { entity, name, .. }
                if entity.0 == 9 && name.as_deref() == Some("NominalValue")),
            "{version:?}: {}",
            found[0].path
        );
        assert_eq!(
            detail(found[0]).scope,
            RuleScope::Type {
                type_name: "IfcPositiveLengthMeasure".into()
            }
        );
    }
}

/// A global RULE runs once over the file, and replaces the hand-written
/// rule of the same id rather than reporting beside it.
#[test]
fn a_global_rule_reports_once_and_supersedes_the_native_rule() {
    if !spec_available() {
        return;
    }
    for (version, _) in RELEASES {
        let two = format!(
            "{}\n{}",
            record(
                version,
                20,
                "IfcProject",
                &[("GlobalId", "'1aaaaaaaaaaaaaaaaaaaaa'")]
            ),
            record(
                version,
                21,
                "IfcProject",
                &[("GlobalId", "'1bbbbbbbbbbbbbbbbbbbbb'")]
            ),
        );
        let report = evaluated(&step(version, &two), version);
        let found = under(&report, "global.IfcSingleProjectInstance");
        assert_eq!(found.len(), 1, "{version:?}: {found:#?}");
        assert_eq!(found[0].path, Path::File);
        assert_eq!(
            detail(found[0]).scope,
            RuleScope::Global {
                rule: "IfcSingleProjectInstance".into()
            }
        );
        let one = record(
            version,
            20,
            "IfcProject",
            &[("GlobalId", "'1aaaaaaaaaaaaaaaaaaaaa'")],
        );
        let report = evaluated(&step(version, &one), version);
        assert!(under(&report, "global.IfcSingleProjectInstance").is_empty());
    }
}

/// `IfcRoot.UR1` is a UNIQUE clause, not a WHERE rule: it stays with
/// `global.UniqueGlobalId` and is reported once with the EXPRESS rules
/// supplied too.
#[test]
fn global_id_uniqueness_is_still_reported_once() {
    if !spec_available() {
        return;
    }
    for (version, _) in RELEASES {
        let same = format!(
            "{}\n{}",
            record(
                version,
                20,
                "IfcProject",
                &[("GlobalId", "'1aaaaaaaaaaaaaaaaaaaaa'")]
            ),
            record(
                version,
                21,
                "IfcWall",
                &[("GlobalId", "'1aaaaaaaaaaaaaaaaaaaaa'")]
            ),
        );
        let report = evaluated(&step(version, &same), version);
        assert_eq!(
            under(&report, "global.UniqueGlobalId").len(),
            1,
            "{version:?}"
        );
    }
}

/// A placement relative to a grid placement makes
/// `IfcCorrectLocalPlacement` return `?`: reported as indeterminate, which
/// leaves the report conformant as far as this rule goes.
#[test]
fn an_unknown_result_is_reported_as_indeterminate() {
    if !spec_available() {
        return;
    }
    for version in [
        SchemaVersion::Ifc4,
        SchemaVersion::Ifc4x1,
        SchemaVersion::Ifc4x2,
        SchemaVersion::Ifc4x3,
    ] {
        let data = format!(
            "{GEOMETRY}\n{}\n{}",
            record(version, 30, "IfcGridPlacement", &[]),
            record(
                version,
                32,
                "IfcLocalPlacement",
                &[("PlacementRelTo", "#30"), ("RelativePlacement", "#2")]
            ),
        );
        let report = evaluated(&step(version, &data), version);
        let found = under(&report, "IfcLocalPlacement.WR21");
        assert_eq!(found.len(), 1, "{version:?}: {:#?}", report.findings());
        assert_eq!(found[0].severity, Severity::Indeterminate);
        assert_eq!(detail(found[0]).outcome, RuleOutcome::Indeterminate);
        assert_eq!(report.summary().indeterminate, 1, "{version:?}");
    }
}

/// A string where a measure belongs cannot be compared with zero: an
/// evaluation error naming the data, not a pass.
#[test]
fn unreadable_data_is_an_evaluation_error() {
    if !spec_available() {
        return;
    }
    let model = step(
        SchemaVersion::Ifc4,
        "#9=IFCPROPERTYSINGLEVALUE('Width',$,IFCPOSITIVELENGTHMEASURE('1'),$);",
    );
    let report = evaluated(&model, SchemaVersion::Ifc4);
    let found = under(&report, "IfcPositiveLengthMeasure.WR1");
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].severity, Severity::EvaluationError);
    assert!(matches!(
        detail(found[0]).outcome,
        RuleOutcome::NotEvaluated(NotEvaluated::InvalidData { .. })
    ));
    assert!(!report.is_conformant());
}

fn with_limits(limits: EvalLimits) -> ifc_validate::Report {
    let model = step(
        SchemaVersion::Ifc4,
        &format!(
            "{GEOMETRY}
#6=IFCSHAPEREPRESENTATION(#3,'Body','Curve2D',(#5));
#8=IFCPRODUCTDEFINITIONSHAPE($,$,(#6));"
        ),
    );
    let rules = rules(SchemaVersion::Ifc4).clone().with_limits(limits);
    validate_with_options(
        &model,
        schema(SchemaVersion::Ifc4),
        &ValidateOptions::new().express(&rules),
    )
}

fn budget_of(report: &ifc_validate::Report) -> Vec<BudgetExceeded> {
    report
        .findings()
        .iter()
        .filter_map(|finding| match &finding.detail {
            Some(FindingDetail::Rule(detail)) => match &detail.outcome {
                RuleOutcome::NotEvaluated(NotEvaluated::BudgetExceeded(limit)) => Some(*limit),
                _ => None,
            },
            _ => None,
        })
        .collect()
}

/// Each limit is a typed refusal on the rules it stops, never a hang.
#[test]
fn every_limit_is_a_typed_refusal() {
    if !spec_available() {
        return;
    }
    let steps = with_limits(EvalLimits::DEFAULT.with_steps_per_rule(3));
    assert!(budget_of(&steps)
        .iter()
        .any(|limit| matches!(limit, BudgetExceeded::Steps { limit: 3 })));
    assert!(!steps.is_conformant());

    let depth = with_limits(EvalLimits::DEFAULT.with_max_depth(0));
    let stopped = under(&depth, "IfcShapeRepresentation.CorrectItemsForType");
    assert_eq!(stopped.len(), 1, "{:#?}", depth.findings());
    assert_eq!(
        budget_of(&depth).first(),
        Some(&BudgetExceeded::Depth { limit: 0 })
    );

    let total = with_limits(EvalLimits::DEFAULT.with_total_steps(10));
    assert!(total.is_truncated());
    assert_eq!(under(&total, "where.express.budget").len(), 1);
}

/// A chain of trimmed curves 5,000 deep and a curve trimming itself are
/// pathological: evaluation stops at the depth limit or at the cycle, with
/// a typed refusal, and the native stack survives it.
#[test]
fn deep_and_cyclic_models_stop_at_a_limit() {
    if !spec_available() {
        return;
    }
    let version = SchemaVersion::Ifc4;
    let mut data = String::from(
        "#1=IFCCARTESIANPOINT((0.,0.,0.));\n#2=IFCDIRECTION((1.,0.,0.));\n#3=IFCLINE(#1,#4);\n#4=IFCVECTOR(#2,1.);\n",
    );
    let mut basis = 3;
    for id in 10..5010 {
        data.push_str(&format!(
            "#{id}=IFCTRIMMEDCURVE(#{basis},(IFCPARAMETERVALUE(0.)),(IFCPARAMETERVALUE(1.)),.T.,.PARAMETER.);\n"
        ));
        basis = id;
    }
    data.push_str(
        "#6000=IFCTRIMMEDCURVE(#6000,(IFCPARAMETERVALUE(0.)),(IFCPARAMETERVALUE(1.)),.T.,.PARAMETER.);\n\
         #6002=IFCCOMPOSITECURVESEGMENT(.CONTINUOUS.,.T.,#6000);\n\
         #6004=IFCCOMPOSITECURVESEGMENT(.CONTINUOUS.,.T.,#5009);\n\
         #6003=IFCCOMPOSITECURVE((#6004,#6002),.F.);",
    );
    let report = evaluated(&step(version, &data), version);
    // `SameDim` reads each segment's `Dim`, which follows the trimmed
    // curves down: 5,000 deep on one side, round in a circle on the other.
    let same_dim = under(&report, "IfcCompositeCurve.SameDim");
    assert_eq!(same_dim.len(), 1, "{}", report.summary());
    assert_eq!(same_dim[0].severity, Severity::EvaluationError);
    let reasons: Vec<_> = report
        .findings()
        .iter()
        .filter(|f| f.severity == Severity::EvaluationError)
        .map(|f| f.message.clone())
        .collect();
    assert!(
        reasons
            .iter()
            .any(|m| m.contains("calls nested deeper than")),
        "{reasons:#?}"
    );
}

/// A rule needing something the evaluator lacks is reported once, as
/// unsupported, naming it -- and the coverage says so before any model is
/// read. The rule text is edited in memory; nothing is written.
#[test]
fn an_unsupported_construct_is_named_once() {
    if !spec_available() {
        return;
    }
    let root = super::spec_root().expect("present");
    let bytes = std::fs::read(root.join("ifc4-add2-tc1/IFC4.exp")).expect("reads");
    let text: String = bytes.iter().map(|&b| char::from(b)).collect();
    let label = "HasRepresentationType :";
    let start = text.find(label).expect("the rule exists") + label.len();
    let end = start + text[start..].find(';').expect("the rule ends");
    let edited = format!("{} FORMAT(1, 'x') = 'y'{}", &text[..start], &text[end..]);
    let rules = ExpressRules::parse(&edited, ifc_schema::ifc4()).expect("still parses");
    let coverage = rules.coverage();
    assert_eq!(
        coverage.not_evaluable,
        vec![(
            "IfcShapeRepresentation.HasRepresentationType".to_owned(),
            "the built-in FORMAT".to_owned()
        )]
    );
    let model = step(
        SchemaVersion::Ifc4,
        &format!(
            "{GEOMETRY}
#6=IFCSHAPEREPRESENTATION(#3,'Body','Curve2D',(#5));
#7=IFCSHAPEREPRESENTATION(#3,'Body','Curve2D',(#5));
#8=IFCPRODUCTDEFINITIONSHAPE($,$,(#6,#7));"
        ),
    );
    let report = validate_with_options(
        &model,
        ifc_schema::ifc4(),
        &ValidateOptions::new().express(&rules),
    );
    let found = under(&report, "IfcShapeRepresentation.HasRepresentationType");
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!(found[0].severity, Severity::Unsupported);
    assert!(found[0].message.contains("FORMAT"), "{}", found[0].message);
}
